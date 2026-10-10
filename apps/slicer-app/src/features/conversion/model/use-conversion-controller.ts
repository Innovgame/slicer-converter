import { useMemo, useRef, useState } from "react";

import {
    type ManagedSlicerWorker as ManagedConversionWorker,
    type ResolvedSlicerExportTarget,
    type SlicerDocument,
    type SlicerFormatId,
    type SlicerWorkerPoolState as WorkerPoolState,
    diagnoseExport
} from "@slicer-converter/wasm";

import type { ProgressState, StatusState } from "../../converter/model/converter-view-state";
import { getErrorMessage } from "../../../shared/lib/errors";
import { buildAnycubicExportWarning } from "./anycubic-export-warning";
import { runConversionUseCase } from "./conversion-use-case";

type OpenedDocument = SlicerDocument;

/**
 * @zh 管理一次转换任务的启动、取消和进度上报。
 * @en Manages conversion task start, cancellation, and progress reporting.
 */
export function useConversionController(input: {
    document: OpenedDocument | null;
    workerPool: WorkerPoolState<ManagedConversionWorker> | null;
    wasmReady: boolean;
    outputFileName?: string;
    conversionTarget: ResolvedSlicerExportTarget | null;
    setStatus: (status: StatusState | null) => void;
}) {
    const [progress, setProgress] = useState<ProgressState | null>(null);
    const [converting, setConverting] = useState(false);
    const conversionAbortControllerRef = useRef<AbortController | null>(null);
    const lastProgressTimeRef = useRef(0);
    const exportMetadata = input.document?.parsedHeader.metadata;
    const exportTargetFormat = input.conversionTarget?.resolvedTargetFormat;
    const exportWarning = useMemo(
        () => readExportWarning(input.wasmReady, exportMetadata, exportTargetFormat),
        [input.wasmReady, exportMetadata, exportTargetFormat]
    );

    async function onConvert(): Promise<void> {
        const currentDocument = input.document;
        const currentHeader = currentDocument?.parsedHeader;
        const pool = input.workerPool;
        const conversionTarget = input.conversionTarget;
        if (!currentHeader || !currentDocument || !pool || !conversionTarget || converting || !input.wasmReady) {
            return;
        }

        setConverting(true);
        const abortController = new AbortController();
        conversionAbortControllerRef.current = abortController;

        if (conversionTarget.statusMessage) {
            input.setStatus({
                type: "info",
                message: conversionTarget.statusMessage
            });
        }

        try {
            const result = await runConversionUseCase({
                document: currentDocument,
                workerPool: pool,
                target: conversionTarget,
                outputFileName: input.outputFileName,
                signal: abortController.signal,
                updateProgress,
                downloadBlob: downloadBlobOutput
            });
            input.setStatus({
                type: "success",
                message: result.statusMessage
            });
            const warning = buildAnycubicExportWarning(diagnoseExport({
                metadata: currentHeader.metadata,
                targetFormat: conversionTarget.resolvedTargetFormat
            }));
            if (warning) {
                input.setStatus({ type: "warning", message: `${result.statusMessage} ${warning}` });
            }
        } catch (error: unknown) {
            if (abortController.signal.aborted) {
                input.setStatus({ type: "info", message: "Conversion cancelled." });
            } else {
                input.setStatus({ type: "error", message: `Convert error: ${getErrorMessage(error)}` });
            }
        } finally {
            conversionAbortControllerRef.current = null;
            setConverting(false);
            setProgress(null);
        }
    }

    function onCancel(): void {
        conversionAbortControllerRef.current?.abort();
    }

    function updateProgress(current: number, total: number, label: string): void {
        const percent = total > 0 ? Math.round((current / total) * 100) : 0;
        const now = performance.now();
        if (percent >= 100 || now - lastProgressTimeRef.current >= 50) {
            lastProgressTimeRef.current = now;
            setProgress({ label, percent });
        }
    }

    return { converting, progress, exportWarning, onConvert, onCancel };
}

function readExportWarning(
    wasmReady: boolean,
    metadata: OpenedDocument["parsedHeader"]["metadata"] | undefined,
    targetFormat: SlicerFormatId | undefined
): string | null {
    if (!wasmReady || !metadata || !targetFormat) {
        return null;
    }

    try {
        return buildAnycubicExportWarning(
            diagnoseExport({
                metadata,
                targetFormat
            })
        );
    } catch {
        return null;
    }
}

function downloadBlobOutput(input: { blob: Blob; downloadName: string }): void {
    const url = URL.createObjectURL(input.blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = input.downloadName;
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
