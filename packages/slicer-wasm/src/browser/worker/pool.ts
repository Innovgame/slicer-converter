import type { SlicerWorkerResponse } from "./protocol.js";
import { getWorkerErrorMessage, isRetryableWorkerError, toSlicerWorkerError } from "./errors.js";

/**
 * @zh 带有忙碌标志的 Worker 最小接口。
 * @en Minimal interface for a Worker tracked by the pool.
 */
export interface BusySlicerWorker {
    busy: boolean;
}

interface PoolWorkerLifecycle {
    terminate(): void;
}

type IdleTimeoutHandle = ReturnType<typeof globalThis.setTimeout>;

/**
 * @zh `SlicerWorkerPoolOptions` 接口契约。
 * @en Interface contract for `SlicerWorkerPoolOptions`.
 */
export interface SlicerWorkerPoolOptions<TWorker extends BusySlicerWorker> {
    idleTimeoutMs?: number;
    minWorkers?: number;
    maxWorkers?: number;
    createWorker?(): TWorker;
    waitForReady?(worker: TWorker): Promise<void>;
    setTimeoutFn?(handler: () => void, timeoutMs: number): IdleTimeoutHandle;
    clearTimeoutFn?(handle: IdleTimeoutHandle): void;
}

/**
 * @zh Worker 池的运行时状态：对外提供总数、worker 列表和空闲查找。
 * @en Runtime state of a Worker pool: exposes the total count, worker list,
 * and an idle-worker lookup.
 */
export interface SlicerWorkerPoolState<TWorker extends BusySlicerWorker> {
    readonly size: number;
    workers: TWorker[];
    findIdleWorker(): TWorker | undefined;
    ensureWorkers(targetCount?: number): Promise<TWorker[]>;
    pauseIdleTermination(): void;
    scheduleIdleTermination(): void;
    terminateAll(): void;
}

/**
 * @zh 用已初始化的 worker 列表创建一个 {@link SlicerWorkerPoolState}。
 * @en Creates a {@link SlicerWorkerPoolState} from an already-initialised worker list.
 *
 * @param workers - @zh 已就绪的 worker 列表 @en Ready-to-use worker array
 */
export function createSlicerWorkerPoolState<TWorker extends BusySlicerWorker>(
    workers: TWorker[],
    options: SlicerWorkerPoolOptions<TWorker> = {}
): SlicerWorkerPoolState<TWorker> {
    const setTimeoutFn = options.setTimeoutFn ?? ((handler, timeoutMs) => globalThis.setTimeout(handler, timeoutMs));
    const clearTimeoutFn = options.clearTimeoutFn ?? ((handle) => globalThis.clearTimeout(handle));
    const waitForReady = options.waitForReady ?? (async () => undefined);
    const idleTimers = new Map<TWorker, IdleTimeoutHandle>();
    let generation = 0;
    let ensureChain: Promise<void> = Promise.resolve();
    const minWorkers = options.minWorkers ?? workers.length;
    const maxWorkers = options.maxWorkers ?? Math.max(minWorkers, workers.length);

    function clearIdleTimer(worker: TWorker): void {
        const handle = idleTimers.get(worker);
        if (handle === undefined) {
            return;
        }
        clearTimeoutFn(handle);
        idleTimers.delete(worker);
    }

    function terminateWorker(worker: TWorker): void {
        clearIdleTimer(worker);
        const index = workers.indexOf(worker);
        if (index >= 0) {
            workers.splice(index, 1);
        }

        const lifecycle = worker as TWorker & Partial<PoolWorkerLifecycle>;
        if (typeof lifecycle.terminate === "function") {
            lifecycle.terminate();
        }
    }

    const pool: SlicerWorkerPoolState<TWorker> = {
        get size() {
            return workers.length;
        },
        workers,
        findIdleWorker() {
            return workers.find((worker) => !worker.busy);
        },
        ensureWorkers(targetCount = maxWorkers) {
            const requestGeneration = generation;
            const ensure = ensureChain.then(async () => {
                if (requestGeneration !== generation) return workers;
                const desiredCount = Math.min(Math.max(targetCount, minWorkers), maxWorkers);
                while (workers.length < desiredCount) {
                    if (!options.createWorker) break;
                    const worker = options.createWorker();
                    const terminate = (): void => {
                        const lifecycle = worker as TWorker & Partial<PoolWorkerLifecycle>;
                        lifecycle.terminate?.();
                    };
                    try {
                        await waitForReady(worker);
                        if (requestGeneration !== generation) {
                            terminate();
                            break;
                        }
                        workers.push(worker);
                    } catch {
                        terminate();
                        break;
                    }
                }
                return workers;
            });
            ensureChain = ensure.then(
                () => undefined,
                () => undefined
            );
            return ensure;
        },
        pauseIdleTermination() {
            for (const worker of idleTimers.keys()) {
                clearIdleTimer(worker);
            }
        },
        scheduleIdleTermination() {
            pool.pauseIdleTermination();

            const idleTimeoutMs = options.idleTimeoutMs;
            if (idleTimeoutMs === undefined || idleTimeoutMs <= 0) {
                return;
            }

            for (const worker of workers) {
                if (worker.busy || workers.length <= minWorkers) {
                    continue;
                }

                const handle = setTimeoutFn(() => {
                    idleTimers.delete(worker);

                    if (worker.busy || workers.length <= minWorkers) {
                        return;
                    }

                    terminateWorker(worker);
                    pool.scheduleIdleTermination();
                }, idleTimeoutMs);

                idleTimers.set(worker, handle);
            }
        },
        terminateAll() {
            generation++;
            pool.pauseIdleTermination();
            for (const worker of [...workers]) {
                terminateWorker(worker);
            }
        }
    };

    return pool;
}

/**
 * @zh `initSlicerWorkerPool` 导出函数。
 * @en Exported function `initSlicerWorkerPool`.
 */
export async function initSlicerWorkerPool<TWorker extends Worker & BusySlicerWorker>(
    input: {
        count: number;
        createWorker(): TWorker;
        waitForReady?: (worker: TWorker) => Promise<void>;
    } & Pick<
        SlicerWorkerPoolOptions<TWorker>,
        "idleTimeoutMs" | "minWorkers" | "maxWorkers" | "setTimeoutFn" | "clearTimeoutFn"
    >
): Promise<SlicerWorkerPoolState<TWorker>> {
    const workerEntries: TWorker[] = [];
    const readyWorkers: TWorker[] = [];
    const waitForReady = input.waitForReady ?? waitForSlicerWorkerReady;
    const initialCount = Math.min(input.count, input.maxWorkers ?? input.count);

    await Promise.all(
        Array.from({ length: initialCount }, async () => {
            const result = await createWorkerWithRetry({
                createWorker: input.createWorker,
                waitForReady,
                setTimeoutFn: input.setTimeoutFn
            });

            workerEntries.push(...result.attemptedWorkers);
            if (result.readyWorker) {
                readyWorkers.push(result.readyWorker);
            }
        })
    );

    for (const worker of workerEntries) {
        if (!readyWorkers.includes(worker)) {
            worker.terminate();
        }
    }

    return createSlicerWorkerPoolState(readyWorkers, {
        idleTimeoutMs: input.idleTimeoutMs,
        minWorkers: input.minWorkers,
        maxWorkers: input.maxWorkers,
        createWorker: input.createWorker,
        waitForReady,
        setTimeoutFn: input.setTimeoutFn,
        clearTimeoutFn: input.clearTimeoutFn
    });
}

/**
 * @zh 等待 Worker 发出 `ready` 响应。若 10 s 内未响应则 reject。
 * @en Waits for a Worker to emit a `ready` response. Rejects after 10 s.
 *
 * @param worker - @zh 待初始化的 Worker @en Worker to wait for
 */
export function waitForSlicerWorkerReady(worker: Worker): Promise<void> {
    return new Promise<void>((resolve, reject) => {
        const timeoutId = globalThis.setTimeout(() => {
            cleanup();
            reject(
                toSlicerWorkerError(new Error("Worker init timeout"), {
                    code: "E_WORKER_INIT_TIMEOUT",
                    stage: "worker-init",
                    retryable: true
                })
            );
        }, 10000);

        const cleanup = (): void => {
            globalThis.clearTimeout(timeoutId);
            worker.removeEventListener("message", onMessage);
            worker.removeEventListener("error", onError);
        };

        const onMessage = (event: Event): void => {
            const message = (event as MessageEvent<SlicerWorkerResponse>).data;
            if (message.type === "ready") {
                cleanup();
                resolve();
            } else if (message.type === "error") {
                cleanup();
                reject(
                    toSlicerWorkerError(message.error, {
                        code: "E_WORKER_INIT",
                        message: getWorkerErrorMessage(message.error),
                        stage: "worker-init",
                        retryable: true
                    })
                );
            }
        };

        const onError = (event: Event): void => {
            const errorEvent = event as ErrorEvent;
            cleanup();
            reject(
                toSlicerWorkerError(errorEvent.error ?? new Error(errorEvent.message || "Worker init failed"), {
                    code: "E_WORKER_INIT",
                    message: errorEvent.message || "Worker init failed",
                    stage: "worker-init",
                    retryable: true
                })
            );
        };

        worker.addEventListener("message", onMessage);
        worker.addEventListener("error", onError);
    });
}

async function createWorkerWithRetry<TWorker extends Worker & BusySlicerWorker>(input: {
    createWorker(): TWorker;
    waitForReady(worker: TWorker): Promise<void>;
    setTimeoutFn?: SlicerWorkerPoolOptions<TWorker>["setTimeoutFn"];
}): Promise<{ readyWorker: TWorker | null; attemptedWorkers: TWorker[] }> {
    const attemptedWorkers: TWorker[] = [];
    const retryDelaysMs = [100];

    for (let attempt = 0; attempt <= retryDelaysMs.length; attempt += 1) {
        const worker = input.createWorker();
        attemptedWorkers.push(worker);

        try {
            await input.waitForReady(worker);
            return { readyWorker: worker, attemptedWorkers };
        } catch (error: unknown) {
            worker.terminate();

            if (!isRetryableWorkerError(error) || attempt >= retryDelaysMs.length) {
                return { readyWorker: null, attemptedWorkers };
            }

            await waitForDelay(retryDelaysMs[attempt]!, input.setTimeoutFn);
        }
    }

    return { readyWorker: null, attemptedWorkers };
}

function waitForDelay<TWorker extends BusySlicerWorker>(
    timeoutMs: number,
    setTimeoutFn?: SlicerWorkerPoolOptions<TWorker>["setTimeoutFn"]
): Promise<void> {
    return new Promise<void>((resolve) => {
        const schedule =
            setTimeoutFn ?? ((handler: () => void, delay: number) => globalThis.setTimeout(handler, delay));
        schedule(() => resolve(), timeoutMs);
    });
}
