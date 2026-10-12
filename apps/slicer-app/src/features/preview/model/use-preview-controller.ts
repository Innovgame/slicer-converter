import { useEffect, useRef, useState } from "react";

import {
    createSlicerPreviewSession,
    type ManagedSlicerWorker as ManagedConversionWorker,
    type ParsedHeader,
    type PreviewQuality,
    type SlicerDocument,
    type SlicerWorkerPreviewDoneMessage as WorkerPreviewDoneMessage,
    type SlicerWorkerResponse as WorkerResponse,
    type SlicerWorkerPoolState as WorkerPoolState
} from "@innovgame/slicer-converter";

import { getErrorMessage } from "../../../shared/lib/errors";
import type { PreviewImageState, StatusState } from "../../converter/model/converter-view-state";

type OpenedDocument = SlicerDocument;
const INTERACTIVE_PREVIEW_SIZE = 512;
const SETTLED_PREVIEW_IDLE_MS = 220;

/**
 * @zh layer preview 交互阶段；dragging 只请求低清，committed 才安排高清 settled。
 * @en Interaction phase for layer preview; dragging only requests low-res, committed schedules high-res settled.
 */
export type PreviewLayerInteraction = "dragging" | "committed";

/**
 * @zh 管理 layer preview 选择、滑动中低清请求和 idle 后高清请求。
 * @en Manages layer preview selection, low-res interactive requests, and high-res idle requests.
 */
export function usePreviewController(input: {
    document: OpenedDocument | null;
    workerPool: WorkerPoolState<ManagedConversionWorker> | null;
    setStatus: (status: StatusState | null) => void;
}) {
    const [selectedLayer, setSelectedLayer] = useState(0);
    const [previewImage, setPreviewImage] = useState<PreviewImageState | null>(null);

    const documentRef = useRef<OpenedDocument | null>(input.document);
    const workerPoolRef = useRef<WorkerPoolState<ManagedConversionWorker> | null>(input.workerPool);
    const previewPendingRef = useRef<number | null>(null);
    const previewRequestSeqRef = useRef(0);
    const activePreviewCleanupsRef = useRef<Set<() => void>>(new Set());
    const settledPreviewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const previewImageRef = useRef<PreviewImageState | null>(null);
    const queuedInteractiveLayerRef = useRef<number | null>(null);
    const queuedSettledLayerRef = useRef<{ layerIndex: number; maxPreviewSize: number } | null>(null);
    const previewLaneWorkersRef = useRef<Partial<Record<PreviewQuality, ManagedConversionWorker>>>({});
    const latestAcceptedRequestRef = useRef<Record<PreviewQuality, number>>({
        interactive: 0,
        settled: 0
    });

    useEffect(() => {
        documentRef.current = input.document;
    }, [input.document]);

    useEffect(() => {
        workerPoolRef.current = input.workerPool;
    }, [input.workerPool]);

    useEffect(() => {
        cancelSettledPreviewTimer();
        cleanupActivePreviewRequests();
        queuedInteractiveLayerRef.current = null;
        queuedSettledLayerRef.current = null;
        previewLaneWorkersRef.current = {};
        replacePreviewImage(null);
        previewPendingRef.current = null;
        latestAcceptedRequestRef.current = { interactive: 0, settled: 0 };
        setSelectedLayer(0);

        if (input.document?.parsedHeader.layers.length) {
            void onPreviewLayerChange(0, "committed", input.document.parsedHeader);
        }
    }, [input.document]);

    useEffect(() => {
        return () => {
            cancelSettledPreviewTimer();
            cleanupActivePreviewRequests();
            queuedInteractiveLayerRef.current = null;
            queuedSettledLayerRef.current = null;
            previewLaneWorkersRef.current = {};
            closePreviewImage(previewImageRef.current);
            previewImageRef.current = null;
        };
    }, []);

    async function onPreviewLayerChange(
        layerIndex: number,
        interaction: PreviewLayerInteraction = "committed",
        headerOverride?: ParsedHeader
    ): Promise<void> {
        const currentHeader = headerOverride ?? documentRef.current?.parsedHeader;
        if (!currentHeader) {
            return;
        }

        const samePendingLayer = previewPendingRef.current === layerIndex;
        setSelectedLayer(layerIndex);
        previewPendingRef.current = layerIndex;
        latestAcceptedRequestRef.current.settled =
            interaction === "dragging" ? previewRequestSeqRef.current + 1 : previewRequestSeqRef.current;

        cancelSettledPreviewTimer();
        queuedSettledLayerRef.current = null;

        if (!samePendingLayer) {
            await queueInteractivePreview(layerIndex);
        }

        if (interaction === "dragging") {
            return;
        }

        settledPreviewTimerRef.current = setTimeout(() => {
            settledPreviewTimerRef.current = null;
            void queueSettledPreview(layerIndex, getSettledPreviewSize(currentHeader));
        }, SETTLED_PREVIEW_IDLE_MS);
    }

    async function dispatchPreviewRequest(
        layerIndex: number,
        quality: PreviewQuality,
        maxPreviewSize: number
    ): Promise<void> {
        if (previewPendingRef.current !== layerIndex) {
            return;
        }

        const pool = workerPoolRef.current;
        const currentDocument = documentRef.current;
        const worker = pool ? findPreviewWorkerForQuality(pool, quality) : undefined;
        if (!pool || !worker || !currentDocument) {
            return;
        }
        const dispatchStart = performance.now();
        pool.pauseIdleTermination();
        const requestId = previewRequestSeqRef.current + 1;
        previewRequestSeqRef.current = requestId;
        let postMessageAt = dispatchStart;

        let cleaned = false;
        const cleanup = (): void => {
            if (cleaned) return;
            cleaned = true;
            worker.removeEventListener("message", handler);
            worker.busy = false;
            if (previewLaneWorkersRef.current[quality] === worker) {
                previewLaneWorkersRef.current[quality] = worker;
            }
            activePreviewCleanupsRef.current.delete(cleanup);
            pool.scheduleIdleTermination();
        };

        const handler = (event: Event): void => {
            const message = (event as MessageEvent<WorkerResponse>).data;
            if (message.type === "preview-done" && message.requestId === requestId) {
                const responseAt = performance.now();
                cleanup();
                void flushPreviewQueue();

                if (message.timings) {
                    const t = message.timings;
                    logPreviewPerf({
                        layerIndex,
                        requestId,
                        quality: message.quality,
                        path: t.path,
                        buildPayloadMs: postMessageAt - dispatchStart,
                        workerRoundTripMs: responseAt - postMessageAt,
                        zipInflateMs: t.zipInflateMs,
                        wasmDecodeMs: t.wasmDecodeMs,
                        imageBitmapMs: t.imageBitmapMs,
                        workerTotalMs: t.totalMs
                    });
                }

                if (!shouldAcceptPreviewResponse(message)) {
                    closeImageBitmap(message.result);
                    return;
                }

                latestAcceptedRequestRef.current[message.quality] = message.requestId;
                replacePreviewImage({
                    bitmap: message.result,
                    greys: message.greys,
                    layerIndex: message.index,
                    width: message.width,
                    height: message.height
                });
                return;
            }

            if (message.type === "error" && message.index === layerIndex) {
                cleanup();
                void flushPreviewQueue();
                input.setStatus({ type: "error", message: `Layer decode error: ${getErrorMessage(message.error)}` });
            }
        };

        worker.busy = true;
        previewLaneWorkersRef.current[quality] = worker;
        worker.addEventListener("message", handler);
        activePreviewCleanupsRef.current.add(cleanup);

        const session = createSlicerPreviewSession({ document: currentDocument });
        const payload = await session.buildDispatchPayload({
            layerIndex,
            requestId,
            quality,
            maxPreviewSize
        });
        postMessageAt = performance.now();

        if (previewPendingRef.current !== layerIndex) {
            cleanup();
            void flushPreviewQueue();
            return;
        }

        worker.postMessage(payload.request, [payload.transfer]);
    }

    return { selectedLayer, previewImage, onPreviewLayerChange };

    async function queueInteractivePreview(layerIndex: number): Promise<void> {
        queuedInteractiveLayerRef.current = layerIndex;
        await flushPreviewQueue();
    }

    async function queueSettledPreview(layerIndex: number, maxPreviewSize: number): Promise<void> {
        queuedSettledLayerRef.current = { layerIndex, maxPreviewSize };
        await flushPreviewQueue();
    }

    async function flushPreviewQueue(): Promise<void> {
        const pool = workerPoolRef.current;
        if (!pool?.workers.some((worker) => !worker.busy)) {
            return;
        }

        const interactiveLayerIndex = queuedInteractiveLayerRef.current;
        if (interactiveLayerIndex !== null) {
            queuedInteractiveLayerRef.current = null;
            if (previewPendingRef.current === interactiveLayerIndex) {
                await dispatchPreviewRequest(interactiveLayerIndex, "interactive", INTERACTIVE_PREVIEW_SIZE);
                return;
            }
        }

        const settledRequest = queuedSettledLayerRef.current;
        if (settledRequest !== null) {
            queuedSettledLayerRef.current = null;
            if (previewPendingRef.current === settledRequest.layerIndex) {
                await dispatchPreviewRequest(settledRequest.layerIndex, "settled", settledRequest.maxPreviewSize);
            }
        }
    }

    function findPreviewWorkerForQuality(
        pool: WorkerPoolState<ManagedConversionWorker>,
        quality: PreviewQuality
    ): ManagedConversionWorker | undefined {
        const assigned = previewLaneWorkersRef.current[quality];
        if (assigned && !assigned.busy && pool.workers.includes(assigned)) {
            return assigned;
        }

        const otherQuality: PreviewQuality = quality === "interactive" ? "settled" : "interactive";
        const otherAssigned = previewLaneWorkersRef.current[otherQuality];
        const idleWorkers = pool.workers.filter((worker) => !worker.busy);
        const unassigned = idleWorkers.find((worker) => worker !== otherAssigned);
        return unassigned ?? idleWorkers[0];
    }

    function shouldAcceptPreviewResponse(message: WorkerPreviewDoneMessage): boolean {
        if (previewPendingRef.current !== message.index) {
            return false;
        }
        if (message.quality === "interactive") {
            return message.requestId >= latestAcceptedRequestRef.current.interactive;
        }

        return message.requestId >= latestAcceptedRequestRef.current.settled;
    }

    function replacePreviewImage(next: PreviewImageState | null): void {
        const previous = previewImageRef.current;
        previewImageRef.current = next;
        setPreviewImage(next);

        if (previous && previous.bitmap !== next?.bitmap) {
            closeImageBitmap(previous.bitmap);
        }
    }

    function cleanupActivePreviewRequests(): void {
        for (const cleanup of activePreviewCleanupsRef.current) {
            cleanup();
        }
        activePreviewCleanupsRef.current.clear();
    }

    function cancelSettledPreviewTimer(): void {
        if (settledPreviewTimerRef.current !== null) {
            clearTimeout(settledPreviewTimerRef.current);
            settledPreviewTimerRef.current = null;
        }
    }
}

function closePreviewImage(preview: PreviewImageState | null): void {
    if (preview) {
        closeImageBitmap(preview.bitmap);
    }
}

function closeImageBitmap(bitmap: ImageBitmap): void {
    if (typeof bitmap.close === "function") {
        bitmap.close();
    }
}

function logPreviewPerf(input: {
    layerIndex: number;
    requestId: number;
    quality: PreviewQuality;
    path: string;
    buildPayloadMs: number;
    workerRoundTripMs: number;
    zipInflateMs?: number;
    wasmDecodeMs: number;
    imageBitmapMs: number;
    workerTotalMs: number;
}): void {
    console.info(
        `[preview-perf] app layer=${input.layerIndex} request=${input.requestId} quality=${input.quality} path=${input.path} build=${formatMs(input.buildPayloadMs)} roundTrip=${formatMs(input.workerRoundTripMs)} inflate=${formatMs(input.zipInflateMs ?? 0)} decode=${formatMs(input.wasmDecodeMs)} bitmap=${formatMs(input.imageBitmapMs)} worker=${formatMs(input.workerTotalMs)}`
    );
}

function formatMs(value: number): string {
    return `${value.toFixed(1)}ms`;
}

function getSettledPreviewSize(header: ParsedHeader): number {
    return Math.max(1, Math.floor(Math.max(header.metadata.resolution_x || 0, header.metadata.resolution_y || 0)));
}
