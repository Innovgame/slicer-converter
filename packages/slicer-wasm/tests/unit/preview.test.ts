import { afterEach, describe, expect, it, vi } from "vitest";

import { decodeSlicerLayerPreviewWithDeps } from "../../src/browser/preview/preview.js";
import type { SlicerDocument } from "../../src/browser/document/document.js";
import type { ParsedHeader } from "../../src/browser/types.js";

describe("decodeSlicerLayerPreview / 解码 slicer layer preview", () => {
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    it("decodes encoded layers through bounded WASM preview / encoded layer 通过 bounded WASM preview 解码", async () => {
        const bitmap = createBitmap(2, 1);
        vi.stubGlobal(
            "ImageData",
            class FakeImageData {
                constructor(
                    readonly data: Uint8ClampedArray,
                    readonly width: number,
                    readonly height: number
                ) {}
            }
        );
        vi.stubGlobal(
            "createImageBitmap",
            vi.fn(async () => bitmap)
        );
        const decodeLayerPreview = vi.fn(() => ({
            pixels: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]),
            width: 2,
            height: 1
        }));
        const document = createDocument("goo");
        vi.mocked(document.readLayerPayload).mockResolvedValue({
            kind: "encoded-layer",
            bytes: new Uint8Array([9, 10])
        });

        const result = await decodeSlicerLayerPreviewWithDeps({
            document,
            layerIndex: 0,
            quality: "interactive",
            maxPreviewSize: 512,
            decodeLayerPreview,
            decodeZipPreview: vi.fn(),
            inflateRaw: vi.fn(),
            now: createClock([0, 3, 5])
        });

        expect(decodeLayerPreview).toHaveBeenCalledWith(expect.any(Uint8Array), 0, "goo", 100, 50, 1, 0, 512);
        expect(result).toMatchObject({
            bitmap,
            width: 2,
            height: 1,
            timings: { path: "encoded-layer-preview", wasmDecodeMs: 3, imageBitmapMs: 2, totalMs: 5 }
        });
    });

    it("decodes interactive ZIP layers through bounded WASM preview / interactive ZIP layer 通过 bounded WASM preview 解码", async () => {
        const bitmap = createBitmap(3, 1);
        vi.stubGlobal(
            "ImageData",
            class FakeImageData {
                constructor(
                    readonly data: Uint8ClampedArray,
                    readonly width: number,
                    readonly height: number
                ) {}
            }
        );
        vi.stubGlobal(
            "createImageBitmap",
            vi.fn(async () => bitmap)
        );
        const decodeZipPreview = vi.fn(async () => ({
            pixels: new Uint8Array(12),
            width: 3,
            height: 1,
            timings: {
                compressedBytes: 2,
                pngPayloadBytes: 4,
                zipInflateMs: 1,
                wasmPreviewDecodeMs: 4,
                totalMs: 5
            }
        }));
        const document = createDocument("vdt");
        vi.mocked(document.readLayerPayload).mockResolvedValue({
            kind: "zip-entry",
            bytes: new Uint8Array([5, 6]),
            compressionMethod: 8,
            uncompressedSize: 4
        });

        const result = await decodeSlicerLayerPreviewWithDeps({
            document,
            layerIndex: 2,
            quality: "interactive",
            maxPreviewSize: 512,
            decodeLayerPreview: vi.fn(),
            decodeZipPreview,
            inflateRaw: vi.fn(),
            now: createClock([10, 16, 18])
        });

        expect(decodeZipPreview).toHaveBeenCalledWith({
            compressed: new Uint8Array([5, 6]),
            compressionMethod: 8,
            uncompressedSize: 4,
            index: 2,
            sourceFormat: "vdt",
            width: 100,
            height: 50,
            inflate: expect.any(Function),
            decodeLayerPreview: expect.any(Function),
            maxPreviewSize: 512
        });
        expect(result.timings).toMatchObject({
            path: "archive-layer-preview",
            zipInflateMs: 1,
            wasmDecodeMs: 4,
            imageBitmapMs: 2,
            totalMs: 8
        });
    });

    it("uses native PNG for settled SL1 and VDT ZIP layers / settled SL1 和 VDT ZIP layer 使用 native PNG", async () => {
        const bitmap = createBitmap(100, 50);
        vi.stubGlobal(
            "Blob",
            class FakeBlob {
                constructor(
                    readonly parts: unknown[],
                    readonly options: unknown
                ) {}
            }
        );
        vi.stubGlobal(
            "createImageBitmap",
            vi.fn(async () => bitmap)
        );
        const inflateRaw = vi.fn(async () => new Uint8Array([137, 80, 78, 71]));
        const document = createDocument("sl1");
        vi.mocked(document.readLayerPayload).mockResolvedValue({
            kind: "zip-entry",
            bytes: new Uint8Array([7, 8]),
            compressionMethod: 8,
            uncompressedSize: 4
        });

        const result = await decodeSlicerLayerPreviewWithDeps({
            document,
            layerIndex: 1,
            quality: "settled",
            maxPreviewSize: 2048,
            decodeLayerPreview: vi.fn(),
            decodeZipPreview: vi.fn(),
            inflateRaw,
            now: createClock([20, 25, 31])
        });

        expect(inflateRaw).toHaveBeenCalledWith(new Uint8Array([7, 8]), 4);
        expect(result).toMatchObject({
            bitmap,
            width: 100,
            height: 50,
            timings: {
                path: "archive-original-png",
                zipInflateMs: 5,
                wasmDecodeMs: 0,
                imageBitmapMs: 6,
                totalMs: 11
            }
        });
    });
});

function createDocument(format: ParsedHeader["format"]): SlicerDocument {
    return {
        sourceFile: new File([], `sample.${format}`),
        sourceName: `sample.${format}`,
        sourceSize: 0,
        parsedHeader: {
            format,
            metadata: {
                resolution_x: 100,
                resolution_y: 50,
                anti_alias_level: 1,
                encryption_key: 0
            },
            layers: [
                { offset: 0, size: 2 },
                { offset: 2, size: 2 },
                { offset: 4, size: 2 }
            ]
        },
        readLayerPayload: vi.fn(),
        close: vi.fn()
    };
}

function createBitmap(width: number, height: number): ImageBitmap {
    return { width, height, close: vi.fn() } as unknown as ImageBitmap;
}

function createClock(values: number[]): () => number {
    let index = 0;
    return () => values[Math.min(index++, values.length - 1)] ?? 0;
}
