import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
    createSlicerWorkerPoolState,
    initSlicerWorkerPool,
    waitForSlicerWorkerReady
} from "../../src/browser/worker/pool.js";

interface FakeWorker {
    busy: boolean;
    terminate: ReturnType<typeof vi.fn>;
    name: string;
}

type EventedWorker = EventTarget &
    FakeWorker & {
        addEventListener: Worker["addEventListener"];
        removeEventListener: Worker["removeEventListener"];
        dispatchEvent: EventTarget["dispatchEvent"];
    };

function makeWorker(name: string, busy = false): FakeWorker {
    return {
        busy,
        name,
        terminate: vi.fn()
    };
}

function makeEventedWorker(name: string): EventedWorker {
    const worker = new EventTarget() as EventedWorker;
    worker.busy = false;
    worker.name = name;
    worker.terminate = vi.fn();
    return worker;
}

describe("worker pool lifecycle / worker pool 生命周期", () => {
    beforeEach(() => {
        vi.useFakeTimers();
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("terminates idle workers after the idle timeout when above minWorkers / terminates idle workers 在之后 the idle timeout 当 above minWorkers", () => {
        const workerA = makeWorker("A");
        const workerB = makeWorker("B");
        const pool = createSlicerWorkerPoolState([workerA, workerB], {
            idleTimeoutMs: 30_000,
            minWorkers: 1,
            maxWorkers: 2
        });

        pool.scheduleIdleTermination();
        vi.advanceTimersByTime(30_000);

        expect(workerA.terminate.mock.calls.length + workerB.terminate.mock.calls.length).toBe(1);
        expect(pool.size).toBe(1);
    });

    it("does not terminate the last worker when minWorkers is 1 / 不 terminate the last worker 当 minWorkers is 1", () => {
        const worker = makeWorker("solo");
        const pool = createSlicerWorkerPoolState([worker], {
            idleTimeoutMs: 30_000,
            minWorkers: 1,
            maxWorkers: 1
        });

        pool.scheduleIdleTermination();
        vi.advanceTimersByTime(30_000);

        expect(worker.terminate).not.toHaveBeenCalled();
        expect(pool.size).toBe(1);
    });

    it("recreates a worker when capacity drops below maxWorkers / recreates a worker 当 capacity drops below maxWorkers", async () => {
        const workerA = makeWorker("A");
        const workerB = makeWorker("B");
        const workerC = makeWorker("C");
        const createWorker = vi.fn().mockReturnValue(workerC);
        const pool = createSlicerWorkerPoolState([workerA, workerB], {
            idleTimeoutMs: 30_000,
            minWorkers: 1,
            maxWorkers: 2,
            createWorker,
            waitForReady: async () => undefined
        });

        pool.scheduleIdleTermination();
        vi.advanceTimersByTime(30_000);
        await pool.ensureWorkers(2);

        expect(createWorker).toHaveBeenCalledTimes(1);
        expect(pool.size).toBe(2);
        expect(pool.workers).toContain(workerC);
    });

    it("uses structured worker error messages when readiness fails / 使用 结构化 worker 错误 messages 当 readiness 失败", async () => {
        const worker = makeEventedWorker("init");

        const readyPromise = waitForSlicerWorkerReady(worker as unknown as Worker);
        worker.dispatchEvent(
            new MessageEvent("message", {
                data: {
                    type: "error",
                    error: {
                        code: "E_WORKER_INIT",
                        message: "structured init failed",
                        stage: "worker-init",
                        retryable: false,
                        timestamp: 1
                    }
                }
            })
        );

        await expect(readyPromise).rejects.toThrow("structured init failed");
    });

    it("preserves retryable init metadata when readiness fails / preserves 可重试 init metadata 当 readiness 失败", async () => {
        const worker = makeEventedWorker("retryable-init");

        const readyPromise = waitForSlicerWorkerReady(worker as unknown as Worker);
        worker.dispatchEvent(
            new MessageEvent("message", {
                data: {
                    type: "error",
                    error: {
                        code: "E_WORKER_INIT_TRANSIENT",
                        message: "temporary init failure",
                        stage: "worker-init",
                        retryable: true,
                        timestamp: 1
                    }
                }
            })
        );

        await expect(readyPromise).rejects.toMatchObject({
            code: "E_WORKER_INIT_TRANSIENT",
            retryable: true,
            stage: "worker-init",
            message: "temporary init failure"
        });
    });

    it("retries worker creation once when waitForReady fails with a retryable error / 重试 worker creation once 当 waitForReady 失败 带有 a 可重试 错误", async () => {
        const workerA = makeEventedWorker("A");
        const workerB = makeEventedWorker("B");
        const createWorker = vi.fn<() => EventedWorker>().mockReturnValueOnce(workerA).mockReturnValueOnce(workerB);
        const waitForReady = vi
            .fn<(worker: EventedWorker) => Promise<void>>()
            .mockRejectedValueOnce({
                code: "E_WORKER_INIT_TRANSIENT",
                message: "temporary init failure",
                stage: "worker-init",
                retryable: true,
                timestamp: 1
            })
            .mockResolvedValueOnce(undefined);

        const pool = await initSlicerWorkerPool({
            count: 1,
            minWorkers: 1,
            maxWorkers: 1,
            createWorker: createWorker as unknown as () => Worker & FakeWorker,
            waitForReady: waitForReady as unknown as (worker: Worker & FakeWorker) => Promise<void>,
            setTimeoutFn(handler) {
                handler();
                return 0 as unknown as ReturnType<typeof setTimeout>;
            }
        });

        expect(createWorker).toHaveBeenCalledTimes(2);
        expect(pool.size).toBe(1);
        expect(workerA.terminate).toHaveBeenCalled();
        expect(pool.workers).toContain(workerB as unknown as Worker & FakeWorker);
    });
});
