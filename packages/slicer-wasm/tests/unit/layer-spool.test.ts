import { afterEach, describe, expect, it, vi } from "vitest";

import { createLayerSpool } from "../../src/browser/output/layer-spool.js";

const originalNavigator = globalThis.navigator;

afterEach(() => {
    vi.restoreAllMocks();
    Object.defineProperty(globalThis, "navigator", {
        configurable: true,
        value: originalNavigator
    });
});

describe("createLayerSpool / createLayerSpool 用例", () => {
    it("replays layers from memory when OPFS is unavailable / replays layers 来自 memory 当 OPFS is unavailable", async () => {
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {}
        });
        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));
        await spool.write(new Uint8Array([2, 3]));
        await spool.closeForReplay();

        const replayed: number[][] = [];
        await spool.replay((chunk) => {
            replayed.push(Array.from(chunk));
        });
        await spool.cleanup();

        expect(replayed).toEqual([[1], [2, 3]]);
    });

    it("uses OPFS when available and replays original layer boundaries / 使用 OPFS 当 available 并且 replays original layer boundaries", async () => {
        let fileBytes = new Uint8Array();
        const writable = {
            write: vi.fn(async (chunk: Uint8Array) => {
                const next = new Uint8Array(fileBytes.byteLength + chunk.byteLength);
                next.set(fileBytes, 0);
                next.set(chunk, fileBytes.byteLength);
                fileBytes = next;
            }),
            close: vi.fn(async () => undefined),
            abort: vi.fn(async () => undefined)
        };
        const handle = {
            createWritable: vi.fn(async () => writable),
            getFile: vi.fn(async () => new Blob([fileBytes]))
        };
        const root = {
            getFileHandle: vi.fn(async (_name: string, _options?: { create?: boolean }) => handle),
            removeEntry: vi.fn(async (_name: string) => undefined)
        };
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {
                storage: {
                    getDirectory: vi.fn(async () => root)
                }
            }
        });
        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));
        await spool.write(new Uint8Array([2, 3]));
        await spool.closeForReplay();

        const replayed: number[][] = [];
        await spool.replay((chunk) => {
            replayed.push(Array.from(chunk));
        });
        await spool.cleanup();

        expect(writable.write).toHaveBeenCalledTimes(2);
        expect(writable.close).toHaveBeenCalledTimes(1);
        expect(replayed).toEqual([[1], [2, 3]]);
        expect(root.removeEntry).toHaveBeenCalledTimes(1);
    });

    it("removes OPFS temp file before memory fallback when writable setup fails / writable setup 失败时先移除 OPFS 临时文件再退回 memory", async () => {
        const handle = {
            createWritable: vi.fn(async () => {
                throw new Error("createWritable failed");
            })
        };
        const root = {
            getFileHandle: vi.fn(async (_name: string, _options?: { create?: boolean }) => handle),
            removeEntry: vi.fn(async (_name: string) => undefined)
        };
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {
                storage: {
                    getDirectory: vi.fn(async () => root)
                }
            }
        });

        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));
        await spool.write(new Uint8Array([2, 3]));
        await spool.closeForReplay();

        const replayed: number[][] = [];
        await spool.replay((chunk) => {
            replayed.push(Array.from(chunk));
        });

        const [[createdName]] = root.getFileHandle.mock.calls;
        expect(replayed).toEqual([[1], [2, 3]]);
        expect(root.removeEntry).toHaveBeenCalledWith(createdName);
    });

    it("preserves OPFS layer boundaries across browser stream chunks / preserves OPFS layer boundaries across browser stream chunks", async () => {
        const writable = {
            write: vi.fn(async (_chunk: Uint8Array) => undefined),
            close: vi.fn(async () => undefined),
            abort: vi.fn(async () => undefined)
        };
        const handle = {
            createWritable: vi.fn(async () => writable),
            getFile: vi.fn(async () => ({
                stream: () =>
                    new ReadableStream<Uint8Array>({
                        start(controller) {
                            controller.enqueue(new Uint8Array([1, 2, 3]));
                            controller.enqueue(new Uint8Array([4]));
                            controller.enqueue(new Uint8Array([5, 6]));
                            controller.close();
                        }
                    })
            }))
        };
        const root = {
            getFileHandle: vi.fn(async () => handle),
            removeEntry: vi.fn(async () => undefined)
        };
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {
                storage: {
                    getDirectory: vi.fn(async () => root)
                }
            }
        });
        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));
        await spool.write(new Uint8Array([2, 3, 4]));
        await spool.write(new Uint8Array([5, 6]));
        await spool.closeForReplay();

        const replayed: number[][] = [];
        await spool.replay((chunk) => {
            replayed.push(Array.from(chunk));
        });
        await spool.cleanup();

        expect(replayed).toEqual([[1], [2, 3, 4], [5, 6]]);
    });

    it("cleans up memory fallback chunks after replay / cleanup 后清理 memory fallback chunks", async () => {
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {}
        });
        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));
        await spool.closeForReplay();
        await spool.cleanup();

        const replayed: number[][] = [];
        await spool.replay((chunk) => {
            replayed.push(Array.from(chunk));
        });

        expect(replayed).toEqual([]);
    });

    it("rejects replay before the spool is closed / replay before close 会 reject", async () => {
        Object.defineProperty(globalThis, "navigator", {
            configurable: true,
            value: {}
        });
        const spool = await createLayerSpool();

        await spool.write(new Uint8Array([1]));

        await expect(spool.replay(async () => undefined)).rejects.toThrow("Layer spool must be closed before replay");
    });
});
