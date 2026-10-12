import { describe, expect, it, vi } from "vitest";
import { runSlicerLayerWorkerScheduler } from "../../src/browser/worker/layer-worker-scheduler.js";
import { createSlicerWorkerPoolState } from "../../src/browser/worker/pool.js";
import type { ManagedSlicerWorker, SlicerWorkerDispatchPayload } from "../../src/browser/worker/dispatch.js";
import type { SlicerWorkerResponse } from "../../src/browser/worker/protocol.js";

class ControlledWorker extends EventTarget {
    busy = false;
    terminated = false;
    posts = 0;
    postMessage(): void {
        this.posts++;
    }
    terminate(): void {
        this.terminated = true;
    }
    done(index = 0): void {
        this.dispatchEvent(
            new MessageEvent("message", {
                data: { type: "layer-done", index, result: new ArrayBuffer(0), elapsedMs: 1 }
            })
        );
    }
}
const flush = async (): Promise<void> => {
    for (let i = 0; i < 8; i++) await Promise.resolve();
};
const payload = async (index: number): Promise<SlicerWorkerDispatchPayload> => ({
    request: {
        type: "decode-encode",
        index,
        data: new ArrayBuffer(0),
        sourceFormat: "goo",
        targetFormat: "ctb",
        targetAnycubicVersion: null,
        width: 1,
        height: 1,
        antiAliasLevel: 1,
        encryptionKey: 0,
        layerMetadataJson: null
    },
    transfer: new ArrayBuffer(0)
});
function input(workers: ControlledWorker[], signal: AbortSignal, onDone = vi.fn()) {
    return {
        workers: workers as unknown as ManagedSlicerWorker[],
        signal,
        totalLayers: workers.length,
        buildDispatchPayload: payload,
        isDoneMessage: (
            message: SlicerWorkerResponse,
            index: number
        ): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
            message.type === "layer-done" && message.index === index,
        onDone
    };
}

describe("scheduler cancellation / 调度器取消边界", () => {
    it("rejects cancellation triggered when prefetch starts / 预读启动时触发取消也会拒绝", async () => {
        const controller = new AbortController();
        const worker = new ControlledWorker();
        let outcome = "pending";
        const run = runSlicerLayerWorkerScheduler({
            ...input([worker], controller.signal),
            buildDispatchPayload(index) {
                controller.abort();
                return payload(index);
            }
        }).then(
            () => {
                outcome = "resolved";
            },
            (error) => {
                outcome = error.name;
            }
        );
        await flush();
        expect(outcome).toBe("AbortError");
        await run;
        expect(worker.posts).toBe(0);
    });
    it("waits for a busy preview worker before starting conversion / 预览 worker 忙时等待再启动转换", async () => {
        const worker = new ControlledWorker();
        worker.busy = true;
        let failure: unknown;
        const run = runSlicerLayerWorkerScheduler(input([worker], new AbortController().signal)).catch((error) => {
            failure = error;
        });
        await flush();
        expect(failure).toBeUndefined();
        expect(worker.posts).toBe(0);
        worker.busy = false;
        await vi.waitFor(() => expect(worker.posts).toBe(1));
        worker.done();
        await run;
    });

    it("rejects a worker crash and removes it from the pool / worker 崩溃时拒绝并从池中移除", async () => {
        const worker = new ControlledWorker();
        const workers = [worker];
        const run = runSlicerLayerWorkerScheduler(input(workers, new AbortController().signal)).catch((error) => error);
        await flush();
        worker.dispatchEvent(new Event("error"));
        let error: Error | undefined;
        void run.then((value) => {
            error = value;
        });
        await flush();
        expect(error?.message).toContain("Worker failed");
        expect(workers).toHaveLength(0);
        await run;
    });

    it("does not report completion after cancellation during async output / 异步输出期间取消后不报告完成", async () => {
        const worker = new ControlledWorker();
        const controller = new AbortController();
        const output = Promise.withResolvers<void>();
        let completed = false;
        const run = runSlicerLayerWorkerScheduler({
            ...input([worker], controller.signal),
            onDone: () => output.promise
        }).then(
            () => {
                completed = true;
            },
            (error) => error
        );
        await flush();
        worker.done();
        await flush();
        controller.abort();
        expect((await run).name).toBe("AbortError");
        output.resolve();
        await flush();
        expect(completed).toBe(false);
        expect(worker.terminated).toBe(true);
    });

    it("rejects an already aborted signal without starting work / 提前取消时拒绝且不启动任务", async () => {
        const controller = new AbortController();
        controller.abort();
        const worker = new ControlledWorker();
        let outcome = "pending";
        const run = runSlicerLayerWorkerScheduler(input([worker], controller.signal)).then(
            () => (outcome = "resolved"),
            (error) => (outcome = error.name)
        );
        await flush();
        expect(outcome).toBe("AbortError");
        await run;
        expect(worker.posts).toBe(0);
    });

    it("ignores late results and replaces in-flight workers for the next task / 忽略迟到结果并为后续任务替换运行中的 worker", async () => {
        const worker = new ControlledWorker();
        const fresh = new ControlledWorker();
        const pool = createSlicerWorkerPoolState([worker], { minWorkers: 1, maxWorkers: 1, createWorker: () => fresh });
        const controller = new AbortController();
        const onDone = vi.fn();
        const run = runSlicerLayerWorkerScheduler(input(pool.workers, controller.signal, onDone)).catch(
            (error) => error
        );
        await flush();
        controller.abort();
        expect((await run).name).toBe("AbortError");
        worker.done();
        await flush();
        expect(onDone).not.toHaveBeenCalled();
        expect(worker.terminated).toBe(true);
        const next = await pool.ensureWorkers();
        expect(next).toEqual([fresh]);
        const nextDone = vi.fn();
        const nextRun = runSlicerLayerWorkerScheduler(input(next, new AbortController().signal, nextDone));
        await flush();
        worker.done();
        fresh.done();
        await nextRun;
        expect(nextDone).toHaveBeenCalledTimes(1);
    });

    it("does not dispatch a read that finishes after cancellation / 取消后不派发已完成的读取", async () => {
        const worker = new ControlledWorker();
        const controller = new AbortController();
        const read = Promise.withResolvers<SlicerWorkerDispatchPayload>();
        const run = runSlicerLayerWorkerScheduler({
            ...input([worker], controller.signal),
            buildDispatchPayload: () => read.promise
        }).catch((error) => error);
        await flush();
        controller.abort();
        await run;
        read.resolve(await payload(0));
        await flush();
        expect(worker.posts).toBe(0);
    });

    it("cleans up every worker after one layer fails / 单层失败时清理所有运行中的 worker", async () => {
        const workers = [new ControlledWorker(), new ControlledWorker()];
        const onDone = vi.fn();
        const run = runSlicerLayerWorkerScheduler(input(workers, new AbortController().signal, onDone)).catch(
            (error) => error
        );
        const late = workers[1];
        const failed = workers[0];
        await flush();
        failed.dispatchEvent(
            new MessageEvent("message", {
                data: {
                    type: "error",
                    index: 0,
                    error: {
                        code: "E_LAYER",
                        message: "broken",
                        stage: "worker-convert",
                        retryable: false,
                        timestamp: 1
                    }
                }
            })
        );
        expect((await run).message).toContain("broken");
        late.done(1);
        await flush();
        expect(onDone).not.toHaveBeenCalled();
        expect(late.terminated).toBe(true);
    });
});
