import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { usePreviewController } from "../../../src/features/preview";
import type { SlicerWorkerResponse as WorkerResponse } from "@innovgame/slicer-converter";

class PreviewErrorWorker extends EventTarget {
    busy = false;

    postMessage(): void {
        queueMicrotask(() => {
            const response = {
                type: "error",
                index: 0,
                error: {
                    code: "E_PREVIEW_DECODE",
                    message: "preview exploded",
                    stage: "worker-preview",
                    retryable: false,
                    timestamp: 1
                }
            } as unknown as WorkerResponse;

            this.dispatchEvent(new MessageEvent("message", { data: response }));
        });
    }

    terminate(): void {}
}

class ProgressivePreviewWorker extends EventTarget {
    busy = false;
    readonly requests: unknown[] = [];

    postMessage(message: unknown): void {
        this.requests.push(message);
        this.busy = true;
    }

    emitPreviewDone(input: {
        index: number;
        requestId: number;
        quality: "interactive" | "settled";
        width: number;
        height: number;
        bitmap?: ImageBitmap;
    }): void {
        const response = {
            type: "preview-done",
            index: input.index,
            requestId: input.requestId,
            quality: input.quality,
            result: input.bitmap ?? ({ close: vi.fn() } as unknown as ImageBitmap),
            width: input.width,
            height: input.height
        } satisfies WorkerResponse;

        this.dispatchEvent(new MessageEvent("message", { data: response }));
    }

    terminate(): void {}
}

function createPreviewWorkerPool(worker: ProgressivePreviewWorker | PreviewErrorWorker) {
    return {
        size: 1,
        workers: [worker],
        findIdleWorker: () => (worker.busy ? null : worker),
        ensureWorkers: vi.fn(),
        pauseIdleTermination: vi.fn(),
        scheduleIdleTermination: vi.fn(),
        terminateAll: vi.fn()
    };
}

function createPreviewWorkerPoolFromWorkers(workers: ProgressivePreviewWorker[]) {
    return {
        get size() {
            return workers.length;
        },
        workers,
        findIdleWorker: () => workers.find((worker) => !worker.busy) ?? null,
        ensureWorkers: vi.fn(async () => workers),
        pauseIdleTermination: vi.fn(),
        scheduleIdleTermination: vi.fn(),
        terminateAll: vi.fn()
    };
}

describe("usePreviewController / usePreviewController 用例", () => {
    beforeEach(() => {
        vi.useFakeTimers();
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("renders structured preview worker errors as user-facing messages / renders 结构化 preview worker 错误 as user-facing messages", async () => {
        const worker = new PreviewErrorWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPool(worker);
        const openedDocument = {
            sourceFile: createSourceFile("error.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 3840,
                    resolution_y: 2400,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [{ offset: 0, size: 2 }]
            },
            readLayerPayload: vi.fn(async () => createEncodedLayerPayload(new Uint8Array([1, 2])))
        };

        renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await act(async () => {
            await vi.advanceTimersByTimeAsync(80);
            await Promise.resolve();
        });

        expect(setStatus).toHaveBeenCalledWith({
            type: "error",
            message: "Layer decode error: preview exploded"
        });
    });

    it("requests low-res while changing and high-res after idle / 改变时请求低清并在 idle 后请求高清", async () => {
        const worker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPool(worker);
        const openedDocument = {
            sourceFile: createSourceFile("progressive.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 3840,
                    resolution_y: 2400,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 }
                ]
            },
            readLayerPayload: vi.fn(async (index: number) =>
                createEncodedLayerPayload(new Uint8Array([index, index + 1]))
            )
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(worker);
        worker.requests.splice(0);

        await act(async () => {
            await result.current.onPreviewLayerChange(1);
            await Promise.resolve();
        });

        expect(worker.requests).toEqual([
            expect.objectContaining({
                type: "decode-preview",
                index: 1,
                quality: "interactive",
                maxPreviewSize: 512,
                payload: expect.objectContaining({
                    kind: "encoded-layer"
                })
            })
        ]);

        const interactiveRequest = worker.requests[0] as { requestId: number };

        await act(async () => {
            worker.emitPreviewDone({
                index: 1,
                requestId: interactiveRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16
            });
            await Promise.resolve();
        });

        expect(result.current.previewImage).toMatchObject({ layerIndex: 1, width: 32, height: 16 });

        await act(async () => {
            await vi.advanceTimersByTimeAsync(220);
            await Promise.resolve();
        });

        expect(worker.requests).toHaveLength(2);
        expect(worker.requests[1]).toEqual(
            expect.objectContaining({
                type: "decode-preview",
                index: 1,
                quality: "settled",
                maxPreviewSize: 3840
            })
        );
    });

    it("coalesces interactive preview requests to the latest layer while a worker is busy / worker 忙时将 interactive preview 请求合并到最新层", async () => {
        const worker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPool(worker);
        const openedDocument = {
            sourceFile: createSourceFile("coalesced.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 64,
                    resolution_y: 32,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 },
                    { offset: 4, size: 2 },
                    { offset: 6, size: 2 }
                ]
            },
            readLayerPayload: vi.fn(async (index: number) =>
                createEncodedLayerPayload(new Uint8Array([index, index + 1]))
            )
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(worker);
        worker.requests.splice(0);

        await act(async () => {
            await result.current.onPreviewLayerChange(1);
            await Promise.resolve();
        });

        const firstRequest = worker.requests[0] as { requestId: number };

        await act(async () => {
            await result.current.onPreviewLayerChange(2);
            await result.current.onPreviewLayerChange(3);
            await Promise.resolve();
        });

        expect(worker.requests).toEqual([
            expect.objectContaining({
                index: 1,
                quality: "interactive"
            })
        ]);

        await act(async () => {
            worker.emitPreviewDone({
                index: 1,
                requestId: firstRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16
            });
            await Promise.resolve();
            await Promise.resolve();
        });

        expect(worker.requests).toHaveLength(2);
        expect(worker.requests[1]).toEqual(
            expect.objectContaining({
                index: 3,
                quality: "interactive",
                maxPreviewSize: 512
            })
        );
    });

    it("flushes the latest preview after an obsolete payload read completes / 过期 payload 读取完成后继续派发最新 preview", async () => {
        const worker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPool(worker);
        const delayedLayer = createDeferred<ReturnType<typeof createEncodedLayerPayload>>();
        const openedDocument = {
            sourceFile: createSourceFile("stale-payload.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 64,
                    resolution_y: 32,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 },
                    { offset: 4, size: 2 }
                ]
            },
            readLayerPayload: vi.fn((index: number) => {
                if (index === 1) {
                    return delayedLayer.promise;
                }
                return Promise.resolve(createEncodedLayerPayload(new Uint8Array([index, index + 1])));
            })
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(worker);
        worker.requests.splice(0);

        let firstChange: Promise<void>;
        await act(async () => {
            firstChange = result.current.onPreviewLayerChange(1);
            await Promise.resolve();
        });

        expect(worker.requests).toEqual([]);
        expect(worker.busy).toBe(true);

        await act(async () => {
            await result.current.onPreviewLayerChange(2);
            await Promise.resolve();
        });

        delayedLayer.resolve(createEncodedLayerPayload(new Uint8Array([1, 2])));

        await act(async () => {
            await firstChange;
            await Promise.resolve();
            await Promise.resolve();
        });

        expect(worker.requests).toEqual([
            expect.objectContaining({
                index: 2,
                quality: "interactive",
                maxPreviewSize: 512
            })
        ]);
    });

    it("skips settled preview while dragging and schedules it when committed / dragging 时跳过 settled preview 并在 committed 后安排", async () => {
        const worker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPool(worker);
        const openedDocument = {
            sourceFile: createSourceFile("continuous.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 3840,
                    resolution_y: 2400,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 },
                    { offset: 4, size: 2 }
                ]
            },
            readLayerPayload: vi.fn(async (index: number) =>
                createEncodedLayerPayload(new Uint8Array([index, index + 1]))
            )
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(worker);
        worker.requests.splice(0);

        await act(async () => {
            await result.current.onPreviewLayerChange(1, "dragging");
            await Promise.resolve();
        });
        let latestRequest = worker.requests.at(-1) as { index: number; requestId: number; quality: "interactive" };
        await act(async () => {
            worker.emitPreviewDone({
                index: 1,
                requestId: latestRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16
            });
            await result.current.onPreviewLayerChange(2, "dragging");
            await Promise.resolve();
        });

        latestRequest = worker.requests.at(-1) as { index: number; requestId: number; quality: "interactive" };
        await act(async () => {
            worker.emitPreviewDone({
                index: 2,
                requestId: latestRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16
            });
            await vi.advanceTimersByTimeAsync(700);
            await Promise.resolve();
        });

        expect(worker.requests).not.toEqual(
            expect.arrayContaining([
                expect.objectContaining({
                    index: 2,
                    quality: "settled"
                })
            ])
        );

        await act(async () => {
            await result.current.onPreviewLayerChange(2, "committed");
            await Promise.resolve();
            await vi.advanceTimersByTimeAsync(220);
            await Promise.resolve();
        });

        expect(worker.requests).toEqual(
            expect.arrayContaining([
                expect.objectContaining({
                    index: 2,
                    quality: "settled"
                })
            ])
        );
    });

    it("ignores an in-flight settled response after dragging starts / dragging 开始后忽略已在途 settled response", async () => {
        const interactiveWorker = new ProgressivePreviewWorker();
        const settledWorker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPoolFromWorkers([interactiveWorker, settledWorker]);
        const openedDocument = {
            sourceFile: createSourceFile("drag-invalidates-settled.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 3840,
                    resolution_y: 2400,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 }
                ]
            },
            readLayerPayload: vi.fn(async (index: number) =>
                createEncodedLayerPayload(new Uint8Array([index, index + 1]))
            )
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(interactiveWorker);
        interactiveWorker.requests.splice(0);
        settledWorker.requests.splice(0);

        await act(async () => {
            await result.current.onPreviewLayerChange(1, "committed");
            await Promise.resolve();
        });

        const interactiveRequest = interactiveWorker.requests[0] as { requestId: number };
        const interactiveBitmap = { close: vi.fn() } as unknown as ImageBitmap;
        await act(async () => {
            interactiveWorker.emitPreviewDone({
                index: 1,
                requestId: interactiveRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16,
                bitmap: interactiveBitmap
            });
            await Promise.resolve();
            await vi.advanceTimersByTimeAsync(220);
            await Promise.resolve();
        });

        expect(settledWorker.requests).toHaveLength(1);
        const settledRequest = settledWorker.requests[0] as { requestId: number };

        await act(async () => {
            await result.current.onPreviewLayerChange(1, "dragging");
            await Promise.resolve();
        });

        expect(interactiveWorker.requests).toHaveLength(1);

        const staleSettledBitmap = { close: vi.fn() } as unknown as ImageBitmap;
        await act(async () => {
            settledWorker.emitPreviewDone({
                index: 1,
                requestId: settledRequest.requestId,
                quality: "settled",
                width: 1024,
                height: 640,
                bitmap: staleSettledBitmap
            });
            await Promise.resolve();
        });

        expect(result.current.previewImage?.bitmap).toBe(interactiveBitmap);
        expect(staleSettledBitmap.close).toHaveBeenCalledOnce();
        expect(interactiveBitmap.close).not.toHaveBeenCalled();
    });

    it("dispatches settled preview on a separate worker lane from interactive preview / settled preview 使用独立 worker lane 不阻塞 interactive", async () => {
        const interactiveWorker = new ProgressivePreviewWorker();
        const settledWorker = new ProgressivePreviewWorker();
        const setStatus = vi.fn();
        const workerPool = createPreviewWorkerPoolFromWorkers([interactiveWorker, settledWorker]);
        const openedDocument = {
            sourceFile: createSourceFile("dual-lane.goo"),
            parsedHeader: {
                format: "goo",
                metadata: {
                    resolution_x: 3840,
                    resolution_y: 2400,
                    anti_alias_level: 1,
                    encryption_key: 0
                },
                layers: [
                    { offset: 0, size: 2 },
                    { offset: 2, size: 2 },
                    { offset: 4, size: 2 }
                ]
            },
            readLayerPayload: vi.fn(async (index: number) =>
                createEncodedLayerPayload(new Uint8Array([index, index + 1]))
            )
        };

        const { result } = renderHook(() =>
            usePreviewController({
                document: openedDocument as never,
                workerPool: workerPool as never,
                setStatus
            })
        );

        await completeLatestPreview(interactiveWorker);
        interactiveWorker.requests.splice(0);
        settledWorker.requests.splice(0);

        await act(async () => {
            await result.current.onPreviewLayerChange(1);
            await Promise.resolve();
        });

        expect(interactiveWorker.requests).toEqual([
            expect.objectContaining({
                index: 1,
                quality: "interactive"
            })
        ]);

        const interactiveRequest = interactiveWorker.requests[0] as { requestId: number };
        await act(async () => {
            interactiveWorker.emitPreviewDone({
                index: 1,
                requestId: interactiveRequest.requestId,
                quality: "interactive",
                width: 32,
                height: 16
            });
            await Promise.resolve();
            await vi.advanceTimersByTimeAsync(220);
            await Promise.resolve();
        });

        expect(settledWorker.requests).toEqual([
            expect.objectContaining({
                index: 1,
                quality: "settled"
            })
        ]);
        expect(settledWorker.busy).toBe(true);

        await act(async () => {
            await result.current.onPreviewLayerChange(2);
            await Promise.resolve();
        });

        expect(interactiveWorker.requests).toEqual([
            expect.objectContaining({ index: 1, quality: "interactive" }),
            expect.objectContaining({ index: 2, quality: "interactive" })
        ]);
        expect(settledWorker.requests).toHaveLength(1);
    });
});

async function completeLatestPreview(worker: ProgressivePreviewWorker): Promise<void> {
    await flushPreviewEffects();
    expect(worker.requests.length).toBeGreaterThan(0);

    const request = worker.requests.at(-1) as { index: number; requestId: number; quality: "interactive" | "settled" };

    await act(async () => {
        worker.emitPreviewDone({
            index: request.index,
            requestId: request.requestId,
            quality: request.quality,
            width: 32,
            height: 16
        });
        await Promise.resolve();
        await Promise.resolve();
    });
}

async function flushPreviewEffects(): Promise<void> {
    await act(async () => {
        for (let i = 0; i < 5; i += 1) {
            await Promise.resolve();
        }
    });
}

function createSourceFile(name: string): File {
    return new File([], name, { lastModified: 1 });
}

function createEncodedLayerPayload(bytes: Uint8Array) {
    return {
        kind: "encoded-layer",
        bytes
    } as const;
}

function createDeferred<T>(): { promise: Promise<T>; resolve(value: T): void } {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((innerResolve) => {
        resolve = innerResolve;
    });
    return { promise, resolve };
}
