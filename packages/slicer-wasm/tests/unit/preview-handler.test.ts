import { describe, expect, it, vi } from "vitest";

import { createSlicerPreviewHandlers } from "../../src/browser/worker/handlers/preview.js";
import type { SlicerWorkerResponse } from "../../src/browser/worker/protocol.js";

describe("preview worker handlers / preview worker handlers 用例", () => {
    it("samples native PNG greys in the worker and transfers their buffer / 在 Worker 中采样原生 PNG 灰度并转移缓存", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const previousCanvas = globalThis.OffscreenCanvas;
        const restore = installImageBitmapMocks({ bitmapWidth: 2, bitmapHeight: 1 });
        vi.stubGlobal(
            "OffscreenCanvas",
            class {
                width = 2;
                height = 1;
                getContext() {
                    return {
                        drawImage() {},
                        getImageData() {
                            return { data: new Uint8ClampedArray([17, 17, 17, 255, 203, 203, 203, 255]) };
                        }
                    };
                }
            }
        );
        try {
            const handlers = createSlicerPreviewHandlers({
                postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                    posted.push({ message, transfer }),
                now: () => 0
            } as never);
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 10,
                quality: "settled",
                index: 0,
                maxPreviewSize: 2,
                payload: {
                    kind: "zip-entry",
                    data: new ArrayBuffer(1),
                    compressionMethod: 0,
                    uncompressedSize: 1,
                    sourceFormat: "sl1",
                    width: 2,
                    height: 1
                }
            });
            await waitForPostedMessages(posted, 1);
            const message = posted[0]!.message;
            expect(message).toMatchObject({ type: "preview-done", greys: expect.any(ArrayBuffer) });
            if (message.type !== "preview-done") throw new Error("Expected preview");
            expect(new Uint8Array(message.greys!)).toEqual(new Uint8Array([17, 203]));
            expect(posted[0]!.transfer).toContain(message.greys);
        } finally {
            restore();
            vi.stubGlobal("OffscreenCanvas", previousCanvas);
        }
    });
    it("transfers native-resolution grey samples for settled previews / 完整预览传输原生分辨率灰度采样", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const handlers = createSlicerPreviewHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                posted.push({ message, transfer }),
            now: () => 0,
            decodeLayerPreview: () => ({
                pixels: new Uint8Array([17, 17, 17, 255, 203, 203, 203, 255]),
                width: 2,
                height: 1
            })
        } as never);
        const restore = installImageBitmapMocks();
        try {
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 9,
                quality: "settled",
                index: 0,
                maxPreviewSize: 2,
                payload: {
                    kind: "encoded-layer",
                    data: new ArrayBuffer(1),
                    format: "prz",
                    width: 2,
                    height: 1,
                    antiAliasLevel: 1,
                    encryptionKey: 0
                }
            });
            await waitForPostedMessages(posted, 1);
            const message = posted[0]!.message;
            expect(message).toMatchObject({ type: "preview-done", greys: expect.any(ArrayBuffer) });
            if (message.type !== "preview-done") throw new Error("Expected preview");
            expect(new Uint8Array(message.greys!)).toEqual(new Uint8Array([17, 203]));
            expect(posted[0]!.transfer).toContain(message.greys);
        } finally {
            restore();
        }
    });
    it("keeps interactive preview dimensions proportional to the source layer / interactive preview 尺寸与 source layer 保持比例", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const decodeLayerPreview = vi.fn(
            (
                _data: Uint8Array,
                _index: number,
                _format: string,
                width: number,
                height: number,
                _antiAliasLevel: number,
                _encryptionKey: number,
                maxPreviewSize: number
            ) => {
                const scale = Math.min(maxPreviewSize / width, maxPreviewSize / height);
                return {
                    pixels: new Uint8Array(Math.round(width * scale) * Math.round(height * scale) * 4),
                    width: Math.round(width * scale),
                    height: Math.round(height * scale)
                };
            }
        );

        const handlers = createSlicerPreviewHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                posted.push({ message, transfer }),
            now: () => 0,
            decodeLayerPreview,
            inflateRaw: vi.fn(),
            decodeEncodeLayer: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview: vi.fn()
        } as never);

        const restoreImageData = installImageBitmapMocks();

        try {
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 1,
                quality: "interactive",
                index: 4,
                maxPreviewSize: 512,
                payload: {
                    kind: "encoded-layer",
                    data: new Uint8Array([1, 2, 3]).buffer as ArrayBuffer,
                    format: "goo",
                    width: 3840,
                    height: 2400,
                    antiAliasLevel: 1,
                    encryptionKey: 0
                }
            });

            await waitForPostedMessages(posted, 1);
        } finally {
            restoreImageData();
        }

        expect(decodeLayerPreview).toHaveBeenCalledWith(expect.any(Uint8Array), 4, "goo", 3840, 2400, 1, 0, 512);
        expect(posted[0]?.message).toMatchObject({
            type: "preview-done",
            index: 4,
            requestId: 1,
            quality: "interactive",
            width: 512,
            height: 320,
            greys: undefined
        });
    });

    it.each([
        ["interactive", 512],
        ["settled", 3840]
    ] as const)(
        "uses WASM layer preview for %s encoded layers / %s encoded layer 使用 WASM layer preview",
        async (quality, maxPreviewSize) => {
            const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
            const decodeLayerPreview = vi.fn(
                (
                    _data: Uint8Array,
                    _index: number,
                    _format: string,
                    width: number,
                    height: number,
                    _antiAliasLevel: number,
                    _encryptionKey: number,
                    previewSize: number
                ) => {
                    const scale = Math.min(previewSize / width, previewSize / height);
                    return {
                        pixels: new Uint8Array(Math.round(width * scale) * Math.round(height * scale) * 4),
                        width: Math.round(width * scale),
                        height: Math.round(height * scale)
                    };
                }
            );

            const handlers = createSlicerPreviewHandlers({
                postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                    posted.push({ message, transfer }),
                now: () => 0,
                decodeLayerPreview,
                inflateRaw: vi.fn(),
                decodeEncodeLayer: vi.fn(),
                prepareZipEntry: vi.fn(),
                convertZipLayerForTarget: vi.fn(),
                decodeZipPreview: vi.fn()
            } as never);

            const restoreImageData = installImageBitmapMocks();

            try {
                handlers["decode-preview"]({
                    type: "decode-preview",
                    requestId: quality === "interactive" ? 20 : 21,
                    quality,
                    index: 4,
                    maxPreviewSize,
                    payload: {
                        kind: "encoded-layer",
                        data: new Uint8Array([1, 2, 3]).buffer as ArrayBuffer,
                        format: "goo",
                        width: 3840,
                        height: 2400,
                        antiAliasLevel: 1,
                        encryptionKey: 0
                    }
                });

                await waitForPostedMessages(posted, 1);
            } finally {
                restoreImageData();
            }

            expect(decodeLayerPreview).toHaveBeenCalledWith(
                expect.any(Uint8Array),
                4,
                "goo",
                3840,
                2400,
                1,
                0,
                maxPreviewSize
            );
            expect(posted[0]?.message).toMatchObject({
                type: "preview-done",
                index: 4,
                quality,
                timings: {
                    path: "encoded-layer-preview"
                }
            });
        }
    );

    it.each(["vdt", "sl1"] as const)(
        "uses WASM PNG preview for interactive %s ZIP layers / interactive %s ZIP layer 使用 WASM PNG preview",
        async (sourceFormat) => {
            const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
            const decodeZipPreview = vi.fn(async () => ({
                pixels: new Uint8Array([1, 2, 3, 4]),
                width: 2,
                height: 1
            }));
            const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));

            const handlers = createSlicerPreviewHandlers({
                postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                    posted.push({ message, transfer }),
                now: () => 0,
                decodeLayerPreview: vi.fn(),
                inflateRaw,
                decodeEncodeLayer: vi.fn(),
                prepareZipEntry: vi.fn(),
                convertZipLayerForTarget: vi.fn(),
                decodeZipPreview
            } as never);

            const restoreImageData = installImageBitmapMocks();

            try {
                handlers["decode-preview"]({
                    type: "decode-preview",
                    requestId: 2,
                    quality: "interactive",
                    index: 6,
                    maxPreviewSize: 512,
                    payload: {
                        kind: "zip-entry",
                        data: new Uint8Array([5, 6]).buffer as ArrayBuffer,
                        compressionMethod: 8,
                        uncompressedSize: 4,
                        sourceFormat,
                        width: 3840,
                        height: 2400
                    }
                });

                await waitForPostedMessages(posted, 1);
            } finally {
                restoreImageData();
            }

            expect(inflateRaw).not.toHaveBeenCalled();
            expect(decodeZipPreview).toHaveBeenCalledWith({
                compressed: new Uint8Array([5, 6]),
                compressionMethod: 8,
                uncompressedSize: 4,
                index: 6,
                sourceFormat,
                width: 3840,
                height: 2400,
                inflate: expect.any(Function),
                decodeLayerPreview: expect.any(Function),
                maxPreviewSize: 512
            });
            expect(posted[0]?.message).toMatchObject({
                type: "preview-done",
                index: 6,
                requestId: 2,
                quality: "interactive",
                width: 2,
                height: 1,
                timings: {
                    path: "archive-layer-preview"
                }
            });
        }
    );

    it("uses native PNG bitmap for settled VDT ZIP layers / settled VDT ZIP layer 使用 native PNG bitmap", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const decodeZipPreview = vi.fn(async () => ({
            pixels: new Uint8Array([1, 2, 3, 4]),
            width: 2,
            height: 1
        }));
        const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));

        const handlers = createSlicerPreviewHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                posted.push({ message, transfer }),
            now: () => 0,
            decodeLayerPreview: vi.fn(),
            inflateRaw,
            decodeEncodeLayer: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview
        } as never);
        const restoreImageData = installImageBitmapMocks({ bitmapWidth: 3840, bitmapHeight: 2400 });
        const createImageBitmapMock = globalThis.createImageBitmap;

        try {
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 3,
                quality: "settled",
                index: 6,
                maxPreviewSize: 3840,
                payload: {
                    kind: "zip-entry",
                    data: new Uint8Array([5, 6]).buffer as ArrayBuffer,
                    compressionMethod: 8,
                    uncompressedSize: 4,
                    sourceFormat: "vdt",
                    width: 3840,
                    height: 2400
                }
            });

            await waitForPostedMessages(posted, 1);
        } finally {
            restoreImageData();
        }

        expect(inflateRaw).toHaveBeenCalledWith(new Uint8Array([5, 6]), 4);
        expect(decodeZipPreview).not.toHaveBeenCalled();
        expect(createImageBitmapMock).toHaveBeenCalledWith(expect.any(Blob));
        expect(posted[0]?.message).toMatchObject({
            type: "preview-done",
            index: 6,
            requestId: 3,
            quality: "settled",
            width: 3840,
            height: 2400,
            timings: {
                path: "archive-original-png",
                wasmDecodeMs: 0
            }
        });
    });

    it("reports settled SL1/VDT native PNG decode failures without WASM fallback / settled SL1/VDT native PNG decode 失败时不回退 WASM", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const decodeZipPreview = vi.fn(async () => ({
            pixels: new Uint8Array([1, 2, 3, 4]),
            width: 2,
            height: 1
        }));
        const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));

        const handlers = createSlicerPreviewHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                posted.push({ message, transfer }),
            now: () => 0,
            decodeLayerPreview: vi.fn(),
            inflateRaw,
            decodeEncodeLayer: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview
        } as never);
        const restoreImageData = installImageBitmapMocks({ rejectBitmap: true });

        try {
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 4,
                quality: "settled",
                index: 6,
                maxPreviewSize: 3840,
                payload: {
                    kind: "zip-entry",
                    data: new Uint8Array([5, 6]).buffer as ArrayBuffer,
                    compressionMethod: 8,
                    uncompressedSize: 4,
                    sourceFormat: "vdt",
                    width: 3840,
                    height: 2400
                }
            });

            await waitForPostedMessages(posted, 1);
        } finally {
            restoreImageData();
        }

        expect(inflateRaw).toHaveBeenCalledWith(new Uint8Array([5, 6]), 4);
        expect(decodeZipPreview).not.toHaveBeenCalled();
        expect(posted[0]?.message).toMatchObject({
            type: "error",
            index: 6,
            error: {
                code: "E_WORKER_ZIP_PREVIEW",
                stage: "worker-preview",
                format: "vdt",
                layerIndex: 6
            }
        });
    });

    it("uses stored PNG bytes directly for settled SL1 ZIP layers / settled SL1 ZIP layer 直接使用 stored PNG bytes", async () => {
        const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
        const decodeZipPreview = vi.fn(async () => ({
            pixels: new Uint8Array([1, 2, 3, 4]),
            width: 2,
            height: 1
        }));
        const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));

        const handlers = createSlicerPreviewHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                posted.push({ message, transfer }),
            now: () => 0,
            decodeLayerPreview: vi.fn(),
            inflateRaw,
            decodeEncodeLayer: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview
        } as never);
        const restoreImageData = installImageBitmapMocks({ bitmapWidth: 2560, bitmapHeight: 1440 });

        try {
            handlers["decode-preview"]({
                type: "decode-preview",
                requestId: 5,
                quality: "settled",
                index: 8,
                maxPreviewSize: 2560,
                payload: {
                    kind: "zip-entry",
                    data: new Uint8Array([137, 80, 78, 71]).buffer as ArrayBuffer,
                    compressionMethod: 0,
                    uncompressedSize: 4,
                    sourceFormat: "sl1",
                    width: 2560,
                    height: 1440
                }
            });

            await waitForPostedMessages(posted, 1);
        } finally {
            restoreImageData();
        }

        expect(inflateRaw).not.toHaveBeenCalled();
        expect(decodeZipPreview).not.toHaveBeenCalled();
        expect(posted[0]?.message).toMatchObject({
            type: "preview-done",
            index: 8,
            requestId: 5,
            quality: "settled",
            width: 2560,
            height: 1440,
            timings: {
                path: "archive-original-png"
            }
        });
    });

    it.each([
        ["interactive", 512],
        ["settled", 2560]
    ] as const)(
        "keeps WASM ZIP preview for %s non-native PNG archive formats / %s 非 native PNG archive format 保持 WASM ZIP preview",
        async (quality, maxPreviewSize) => {
            const posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }> = [];
            const decodeZipPreview = vi.fn(async () => ({
                pixels: new Uint8Array([1, 2, 3, 4]),
                width: 2,
                height: 1
            }));
            const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));

            const handlers = createSlicerPreviewHandlers({
                postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) =>
                    posted.push({ message, transfer }),
                now: () => 0,
                decodeLayerPreview: vi.fn(),
                inflateRaw,
                decodeEncodeLayer: vi.fn(),
                prepareZipEntry: vi.fn(),
                convertZipLayerForTarget: vi.fn(),
                decodeZipPreview
            } as never);
            const restoreImageData = installImageBitmapMocks({ bitmapWidth: 2560, bitmapHeight: 1440 });

            try {
                handlers["decode-preview"]({
                    type: "decode-preview",
                    requestId: quality === "interactive" ? 6 : 7,
                    quality,
                    index: 9,
                    maxPreviewSize,
                    payload: {
                        kind: "zip-entry",
                        data: new Uint8Array([5, 6]).buffer as ArrayBuffer,
                        compressionMethod: 8,
                        uncompressedSize: 4,
                        sourceFormat: "anycubic-zip",
                        width: 2560,
                        height: 1440
                    }
                });

                await waitForPostedMessages(posted, 1);
            } finally {
                restoreImageData();
            }

            expect(decodeZipPreview).toHaveBeenCalledWith({
                compressed: new Uint8Array([5, 6]),
                compressionMethod: 8,
                uncompressedSize: 4,
                index: 9,
                sourceFormat: "anycubic-zip",
                width: 2560,
                height: 1440,
                inflate: expect.any(Function),
                decodeLayerPreview: expect.any(Function),
                maxPreviewSize
            });
            expect(inflateRaw).not.toHaveBeenCalled();
            expect(posted[0]?.message).toMatchObject({
                type: "preview-done",
                index: 9,
                requestId: quality === "interactive" ? 6 : 7,
                quality,
                width: 2,
                height: 1,
                timings: {
                    path: "archive-layer-preview"
                }
            });
        }
    );
});

function installImageBitmapMocks(input?: {
    bitmapWidth?: number;
    bitmapHeight?: number;
    rejectBitmap?: boolean;
}): () => void {
    const previousImageData = globalThis.ImageData;
    const previousCreateImageBitmap = globalThis.createImageBitmap;

    vi.stubGlobal(
        "ImageData",
        class {
            constructor(
                readonly data: Uint8ClampedArray,
                readonly width: number,
                readonly height: number
            ) {}
        }
    );
    vi.stubGlobal(
        "createImageBitmap",
        vi.fn(async (source: unknown) => {
            if (input?.rejectBitmap && source instanceof Blob) {
                throw new Error("native PNG decode failed");
            }

            return {
                close: vi.fn(),
                width: input?.bitmapWidth,
                height: input?.bitmapHeight
            } as unknown as ImageBitmap;
        })
    );

    return () => {
        vi.stubGlobal("ImageData", previousImageData);
        vi.stubGlobal("createImageBitmap", previousCreateImageBitmap);
    };
}

async function waitForPostedMessages(
    posted: Array<{ message: SlicerWorkerResponse; transfer?: Transferable[] }>,
    count: number
): Promise<void> {
    const startedAt = performance.now();
    while (posted.length < count) {
        if (performance.now() - startedAt > 250) {
            throw new Error(`Timed out waiting for ${count} posted messages; got ${posted.length}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 0));
    }
}
