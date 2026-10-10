import { useEffect, useRef, useState } from "react";

import {
    getSupportedFormats,
    initSlicerConverterWithProgress,
    initSlicerWorkerPool as initManagedWorkerPool,
    syncSupportedFormatInfosForValidation,
    validateBrowserFormatBehaviors,
    type ManagedSlicerWorker as ManagedConversionWorker,
    type SlicerWorkerPoolState as WorkerPoolState,
    type WasmLoadProgress
} from "@slicer-converter/wasm";

import type { StatusState } from "../../features/converter/model/converter-view-state";
import { getErrorMessage } from "../../shared/lib/errors";

const MIN_WORKER_COUNT = 1;
const WORKER_COUNT = Math.min(Math.max((navigator.hardwareConcurrency || 4) - 1, 1), 8);
const WORKER_IDLE_TIMEOUT_MS = 30_000;

/**
 * @zh 初始化主线程 WASM runtime，并预热供转换/预览复用的 worker pool。
 * @en Initializes the main-thread WASM runtime and prewarms the worker pool reused by conversion/preview.
 */
export function useRuntimeController() {
    const [wasmReady, setWasmReady] = useState(false);
    const [status, setStatus] = useState<StatusState | null>(null);
    const workerPoolRef = useRef<WorkerPoolState<ManagedConversionWorker> | null>(null);

    useEffect(() => {
        let active = true;

        async function setup(): Promise<void> {
            try {
                setStatus({ type: "info", message: "Loading WASM..." });
                await initSlicerConverterWithProgress((progress: WasmLoadProgress) => {
                    if (!active) {
                        return;
                    }

                    setStatus({ type: "info", message: formatWasmLoadMessage(progress) });
                });
                if (!active) {
                    return;
                }

                const formatFacts = getSupportedFormats();
                syncSupportedFormatInfosForValidation(formatFacts);
                validateBrowserFormatBehaviors(formatFacts);
                setStatus({ type: "info", message: "WASM loaded. Initializing worker pool..." });
                const pool = await initManagedWorkerPool({
                    count: MIN_WORKER_COUNT,
                    minWorkers: MIN_WORKER_COUNT,
                    maxWorkers: WORKER_COUNT,
                    idleTimeoutMs: WORKER_IDLE_TIMEOUT_MS,
                    createWorker: () =>
                        Object.assign(new Worker(new URL("../../worker.ts", import.meta.url), { type: "module" }), {
                            busy: false
                        }) as ManagedConversionWorker
                });

                if (!active) {
                    pool.terminateAll();
                    return;
                }

                if (pool.size < MIN_WORKER_COUNT) {
                    throw new Error("Worker init failed: no workers became ready during prewarm.");
                }

                workerPoolRef.current = pool;
                setWasmReady(true);
                setStatus({
                    type: "success",
                    message: `Ready - prewarmed ${pool.size} worker(s), scales up to ${WORKER_COUNT} on demand.`
                });
            } catch (error: unknown) {
                if (!active) {
                    return;
                }

                setStatus({ type: "error", message: `Failed to load WASM: ${getErrorMessage(error)}` });
            }
        }

        void setup();

        return () => {
            active = false;
            workerPoolRef.current?.terminateAll();
            workerPoolRef.current = null;
        };
    }, []);

    return { wasmReady, status, setStatus, workerPoolRef };
}

function formatWasmLoadMessage(progress: WasmLoadProgress): string {
    if (progress.phase === "ready") {
        return "WASM loaded. Initializing worker pool...";
    }

    if (progress.phase === "compiling") {
        return "Compiling WASM module...";
    }

    if (progress.percent !== null) {
        return `Loading WASM... ${progress.percent}%`;
    }

    return `Loading WASM... ${formatKilobytes(progress.loadedBytes)} KB`;
}

function formatKilobytes(bytes: number): string {
    return (bytes / 1024).toFixed(bytes >= 1024 * 100 ? 0 : 1);
}
