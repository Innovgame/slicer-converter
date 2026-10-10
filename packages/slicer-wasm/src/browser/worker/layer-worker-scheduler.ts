import type { SlicerWorkerError, SlicerWorkerResponse } from "./protocol.js";
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
    if (input.totalLayers <= 0) {
        return;
    }
    if (input.workers.length === 0) {
        throw new Error("Layer worker scheduler requires at least one worker");
    }

    let nextLayer = 0;
    const prefetch = createSlicerLayerPrefetch(
        input.totalLayers,
        input.buildDispatchPayload,
        input.workers.length + 2,
        input.signal
    );

    await new Promise<void>((resolve, reject) => {
        const onAbort = (): void => {
            input.signal.removeEventListener("abort", onAbort);
            reject(new DOMException("Conversion cancelled", "AbortError"));
        };
        input.signal.addEventListener("abort", onAbort);

        let completed = 0;
        const settleError = (error: unknown): void => {
            input.signal.removeEventListener("abort", onAbort);
            reject(error);
        };

        const dispatch = (worker: ManagedSlicerWorker): void => {
            if (input.signal.aborted || nextLayer >= input.totalLayers) {
                return;
            }

            const layerIndex = nextLayer;
            nextLayer += 1;
            worker.busy = true;
            let attempt = 0;

            const postDispatchPayload = (): void => {
                const payloadPromise =
                    attempt === 0
                        ? prefetch.next()
                        : input.buildDispatchPayload(layerIndex).then(({ request, transfer }) => ({
                              request,
                              transfer
                          }));

                payloadPromise
                    .then(({ request, transfer }) => {
                        worker.postMessage(request, [transfer]);
                    })
                    .catch((error: unknown) => {
                        worker.removeEventListener("message", handler);
                        worker.busy = false;
                        settleError(error);
                    });
            };

            const handleError = (error: SlicerWorkerError): void => {
                if (error.retryable && attempt < WORKER_RETRY_LIMIT) {
                    attempt += 1;
                    postDispatchPayload();
                    return;
                }

                worker.removeEventListener("message", handler);
                worker.busy = false;
                settleError(new Error(`Layer ${layerIndex}: ${getWorkerErrorMessage(error)}`));
            };

            const completeLayer = (message: TDone): void => {
                let done: Promise<void> | void;
                try {
                    done = input.onDone(message, layerIndex);
                } catch (error: unknown) {
                    worker.removeEventListener("message", handler);
                    worker.busy = false;
                    settleError(error);
                    return;
                }

                Promise.resolve(done)
                    .then(() => {
                        worker.removeEventListener("message", handler);
                        worker.busy = false;
                        completed += 1;
                        if (completed === input.totalLayers) {
                            input.signal.removeEventListener("abort", onAbort);
                            resolve();
                            return;
                        }
                        dispatch(worker);
                    })
                    .catch((error: unknown) => {
                        worker.removeEventListener("message", handler);
                        worker.busy = false;
                        settleError(error);
                    });
            };

            const handler = (event: Event): void => {
                const message = (event as MessageEvent<SlicerWorkerResponse>).data;
                if (input.isDoneMessage(message, layerIndex)) {
                    completeLayer(message);
                    return;
                }

                if (message.type === "error" && message.index === layerIndex) {
                    handleError(message.error);
                }
            };

            worker.addEventListener("message", handler);
            postDispatchPayload();
        };

        for (const worker of input.workers) {
            dispatch(worker);
        }
    });
}
