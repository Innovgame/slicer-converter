import { describe, expect, it, vi } from "vitest";

import type { ManagedSlicerWorker, SlicerWorkerDispatchPayload } from "../../src/browser/worker/dispatch.js";
import type { SlicerWorkerRequest, SlicerWorkerResponse } from "../../src/browser/worker/protocol.js";

function createTestPayload(index: number): SlicerWorkerDispatchPayload {
    const transfer = new Uint8Array([index + 1]).buffer as ArrayBuffer;
    return {
        transfer,
        request: {
            type: "decode-encode",
            index,
            data: transfer,
            sourceFormat: "goo",
            targetFormat: "ctb",
            targetAnycubicVersion: null,
            width: 4,
            height: 4,
            antiAliasLevel: 1,
            encryptionKey: 0,
            layerMetadataJson: null
        } satisfies SlicerWorkerRequest
    };
}

describe("runSlicerLayerWorkerScheduler / runSlicerLayerWorkerScheduler 用例", () => {
    it("resolves immediately when there are no layers to dispatch / resolves immediately 当 there are no layers to dispatch", async () => {
        const { runSlicerLayerWorkerScheduler } = await import("../../src/browser/worker/layer-worker-scheduler.js");
        const buildDispatchPayload = vi.fn(async (index: number) => createTestPayload(index));
        const onDone = vi.fn();

        await runSlicerLayerWorkerScheduler({
            workers: [],
            totalLayers: 0,
            signal: new AbortController().signal,
            buildDispatchPayload,
            isDoneMessage: (message, layerIndex): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
                message.type === "layer-done" && message.index === layerIndex,
            onDone
        });

        expect(buildDispatchPayload).not.toHaveBeenCalled();
        expect(onDone).not.toHaveBeenCalled();
    });

    it("rejects an empty worker pool when layers need dispatching / 拒绝 an 空 worker pool 当 layers need dispatching", async () => {
        const { runSlicerLayerWorkerScheduler } = await import("../../src/browser/worker/layer-worker-scheduler.js");

        await expect(
            runSlicerLayerWorkerScheduler({
                workers: [],
                totalLayers: 1,
                signal: new AbortController().signal,
                buildDispatchPayload: async (index: number) => createTestPayload(index),
                isDoneMessage: (message, layerIndex): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
                    message.type === "layer-done" && message.index === layerIndex,
                onDone: vi.fn()
            })
        ).rejects.toThrow("Layer worker scheduler requires at least one worker");
    });

    it("retries retryable layer errors with a fresh payload and completes the layer / 重试 可重试 layer 错误 带有 a fresh payload 并且 completes the layer", async () => {
        const { runSlicerLayerWorkerScheduler } = await import("../../src/browser/worker/layer-worker-scheduler.js");
        const onDone = vi.fn();

        class RetryWorker extends EventTarget {
            busy = false;
            calls = 0;
            onerror: ((this: AbstractWorker, ev: ErrorEvent) => unknown) | null = null;
            onmessage: ((this: Worker, ev: MessageEvent) => unknown) | null = null;
            onmessageerror: ((this: Worker, ev: MessageEvent) => unknown) | null = null;

            postMessage(): void {
                this.calls += 1;
                if (this.calls === 1) {
                    queueMicrotask(() => {
                        this.dispatchEvent(
                            new MessageEvent("message", {
                                data: {
                                    type: "error",
                                    index: 0,
                                    error: {
                                        code: "E_TRANSIENT",
                                        message: "temporary failure",
                                        stage: "worker-convert",
                                        retryable: true,
                                        timestamp: 1
                                    }
                                } satisfies SlicerWorkerResponse
                            })
                        );
                    });
                    return;
                }

                queueMicrotask(() => {
                    this.dispatchEvent(
                        new MessageEvent("message", {
                            data: {
                                type: "layer-done",
                                index: 0,
                                result: new Uint8Array([7]).buffer as ArrayBuffer,
                                elapsedMs: 3
                            } satisfies SlicerWorkerResponse
                        })
                    );
                });
            }

            terminate(): void {}
        }

        const worker = new RetryWorker();
        const buildDispatchPayload = vi.fn(async (index: number) => createTestPayload(index));

        await runSlicerLayerWorkerScheduler({
            workers: [worker as unknown as ManagedSlicerWorker],
            totalLayers: 1,
            signal: new AbortController().signal,
            buildDispatchPayload,
            isDoneMessage: (message, layerIndex): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
                message.type === "layer-done" && message.index === layerIndex,
            onDone
        });

        expect(worker.calls).toBe(2);
        expect(buildDispatchPayload).toHaveBeenCalledTimes(2);
        expect(onDone).toHaveBeenCalledWith(expect.objectContaining({ type: "layer-done", index: 0, elapsedMs: 3 }), 0);
        expect(worker.busy).toBe(false);
    });

    it("removes the message listener when onDone rejects / removes the message listener 当 onDone 拒绝", async () => {
        const { runSlicerLayerWorkerScheduler } = await import("../../src/browser/worker/layer-worker-scheduler.js");

        class ListenerTrackingWorker extends EventTarget {
            busy = false;
            activeMessageListeners = 0;
            onerror: ((this: AbstractWorker, ev: ErrorEvent) => unknown) | null = null;
            onmessage: ((this: Worker, ev: MessageEvent) => unknown) | null = null;
            onmessageerror: ((this: Worker, ev: MessageEvent) => unknown) | null = null;

            override addEventListener(
                type: string,
                callback: EventListenerOrEventListenerObject | null,
                options?: boolean | AddEventListenerOptions
            ): void {
                if (type === "message") {
                    this.activeMessageListeners += 1;
                }
                super.addEventListener(type, callback, options);
            }

            override removeEventListener(
                type: string,
                callback: EventListenerOrEventListenerObject | null,
                options?: boolean | EventListenerOptions
            ): void {
                if (type === "message") {
                    this.activeMessageListeners -= 1;
                }
                super.removeEventListener(type, callback, options);
            }

            postMessage(): void {
                queueMicrotask(() => {
                    this.dispatchEvent(
                        new MessageEvent("message", {
                            data: {
                                type: "layer-done",
                                index: 0,
                                result: new Uint8Array([7]).buffer as ArrayBuffer,
                                elapsedMs: 3
                            } satisfies SlicerWorkerResponse
                        })
                    );
                });
            }

            terminate(): void {}
        }

        const worker = new ListenerTrackingWorker();

        await expect(
            runSlicerLayerWorkerScheduler({
                workers: [worker as unknown as ManagedSlicerWorker],
                totalLayers: 1,
                signal: new AbortController().signal,
                buildDispatchPayload: async (index: number) => createTestPayload(index),
                isDoneMessage: (message, layerIndex): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
                    message.type === "layer-done" && message.index === layerIndex,
                onDone: () => {
                    throw new Error("assembler failed");
                }
            })
        ).rejects.toThrow("assembler failed");

        expect(worker.activeMessageListeners).toBe(0);
        expect(worker.busy).toBe(false);
    });
});
