import type { SlicerWorkerResponse } from "./protocol.js";
import { getWorkerErrorMessage } from "./errors.js";
import { createSlicerLayerPrefetch } from "./layer-prefetch.js";
import type { ManagedSlicerWorker, SlicerWorkerDispatchPayload } from "./dispatch.js";

const WORKER_RETRY_LIMIT = 1;

/**
 * @zh `SlicerLayerWorkerSchedulerInput` 接口契约。
 * @en Interface contract for `SlicerLayerWorkerSchedulerInput`.
 */
export interface SlicerLayerWorkerSchedulerInput<TDone extends SlicerWorkerResponse> {
    workers: ManagedSlicerWorker[];
    totalLayers: number;
    signal: AbortSignal;
    buildDispatchPayload(layerIndex: number): Promise<SlicerWorkerDispatchPayload>;
    isDoneMessage(message: SlicerWorkerResponse, layerIndex: number): message is TDone;
    onDone(message: TDone, layerIndex: number): Promise<void> | void;
}

/**
 * @zh `runSlicerLayerWorkerScheduler` 导出函数。
 * @en Exported function `runSlicerLayerWorkerScheduler`.
 */
export async function runSlicerLayerWorkerScheduler<TDone extends SlicerWorkerResponse>(
    input: SlicerLayerWorkerSchedulerInput<TDone>
): Promise<void> {
    input.signal.throwIfAborted();
    if (input.totalLayers <= 0) return;
    if (input.workers.length === 0) throw new Error("Layer worker scheduler requires at least one worker");
    while (input.workers.every((worker) => worker.busy)) {
        await new Promise<void>((resolve, reject) => {
            const onAbort = (): void => {
                clearTimeout(timer);
                reject(new DOMException("Conversion cancelled", "AbortError"));
            };
            const timer = setTimeout(() => {
                input.signal.removeEventListener("abort", onAbort);
                resolve();
            }, 10);
            input.signal.addEventListener("abort", onAbort, { once: true });
        });
        input.signal.throwIfAborted();
        if (input.workers.length === 0) throw new Error("Layer worker scheduler requires at least one worker");
    }
    const workers = input.workers.filter((worker) => !worker.busy);
    if (workers.length === 0) {
        throw new Error("Layer worker scheduler requires at least one worker (idle)");
    }
    const prefetch = createSlicerLayerPrefetch(
        input.totalLayers,
        input.buildDispatchPayload,
        workers.length + 2,
        input.signal
    );
    if (input.signal.aborted) {
        prefetch.dispose();
        input.signal.throwIfAborted();
    }
    let nextLayer = 0;

    await new Promise<void>((resolve, reject) => {
        let settled = false;
        let completed = 0;
        const active = new Map<ManagedSlicerWorker, () => void>();
        const finish = (error?: unknown): void => {
            if (settled) return;
            settled = true;
            input.signal.removeEventListener("abort", onAbort);
            prefetch.dispose();
            for (const [worker, cleanup] of active) {
                cleanup();
                worker.busy = false;
                worker.terminate();
                const index = input.workers.indexOf(worker);
                if (index >= 0) input.workers.splice(index, 1);
            }
            active.clear();
            if (error !== undefined) reject(error);
            else resolve();
        };
        const onAbort = (): void => finish(new DOMException("Conversion cancelled", "AbortError"));
        input.signal.addEventListener("abort", onAbort);

        const dispatch = (worker: ManagedSlicerWorker): void => {
            if (settled || nextLayer >= input.totalLayers) return;
            const layerIndex = nextLayer++;
            worker.busy = true;
            let attempt = 0;
            let handlingDone = false;
            const cleanup = (): void => {
                worker.removeEventListener("message", handler);
                worker.removeEventListener("error", onWorkerError);
                worker.removeEventListener("messageerror", onWorkerError);
            };
            const onWorkerError = (): void => finish(new Error(`Layer ${layerIndex}: Worker failed`));
            const postDispatchPayload = (): void => {
                const payload = attempt === 0 ? prefetch.next() : input.buildDispatchPayload(layerIndex);
                void payload
                    .then(({ request, transfer }) => {
                        if (!settled && !input.signal.aborted) worker.postMessage(request, [transfer]);
                    })
                    .catch(finish);
            };
            const handler = (event: Event): void => {
                if (settled || handlingDone) return;
                const message = (event as MessageEvent<SlicerWorkerResponse>).data;
                if (input.isDoneMessage(message, layerIndex)) {
                    handlingDone = true;
                    void Promise.resolve()
                        .then(() => {
                            if (!settled) return input.onDone(message, layerIndex);
                        })
                        .then(() => {
                            if (settled) return;
                            cleanup();
                            active.delete(worker);
                            worker.busy = false;
                            completed++;
                            if (completed === input.totalLayers) finish();
                            else dispatch(worker);
                        })
                        .catch(finish);
                } else if (message.type === "error" && message.index === layerIndex) {
                    if (message.error.retryable && attempt < WORKER_RETRY_LIMIT) {
                        attempt++;
                        postDispatchPayload();
                    } else {
                        finish(new Error(`Layer ${layerIndex}: ${getWorkerErrorMessage(message.error)}`));
                    }
                }
            };
            active.set(worker, cleanup);
            worker.addEventListener("message", handler);
            worker.addEventListener("error", onWorkerError);
            worker.addEventListener("messageerror", onWorkerError);
            postDispatchPayload();
        };
        for (const worker of workers) dispatch(worker);
    });
}
