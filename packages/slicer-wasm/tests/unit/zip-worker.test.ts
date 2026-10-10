import { describe, expect, it, vi } from "vitest";

import { convertZipLayerForTarget, decodeZipPreview } from "../../src/browser/archive/zip/worker.js";

describe("zip worker helpers / ZIP worker helpers", () => {
    it("uses decodeEncodeLayer for ZIP conversion after decompression / 使用 decodeEncodeLayer 在 ZIP 解压后转换", async () => {
        const decodeEncodeLayer = vi.fn(() => new Uint8Array([9, 9]));
        const inflate = vi.fn(async () => new Uint8Array([1, 2, 3]));

        const result = await convertZipLayerForTarget({
            compressed: new Uint8Array([7, 8]),
            compressionMethod: 8,
            uncompressedSize: 3,
            index: 4,
            sourceFormat: "sl1",
            targetFormat: "ctb",
            targetAnycubicVersion: 515,
            width: 256,
            height: 128,
            layerMetadataJson: "{}",
            inflate,
            decodeEncodeLayer
        });

        expect(inflate).toHaveBeenCalledWith(new Uint8Array([7, 8]), 3);
        expect(decodeEncodeLayer).toHaveBeenCalledWith(
            new Uint8Array([1, 2, 3]),
            4,
            "sl1",
            "ctb",
            256,
            128,
            1,
            0,
            "{}",
            515
        );
        expect(Array.from(result)).toEqual([9, 9]);
    });

    it("uses decodeLayerPreview for ZIP preview after decompression / 使用 decodeLayerPreview 在 ZIP 解压后预览", async () => {
        const decodeLayerPreview = vi.fn(() => ({ pixels: new Uint8Array([3, 4]), width: 32, height: 16 }));
        const inflate = vi.fn(async () => new Uint8Array([1, 2]));

        const result = await decodeZipPreview({
            compressed: new Uint8Array([5, 6]),
            compressionMethod: 8,
            uncompressedSize: 2,
            index: 1,
            sourceFormat: "sl1",
            width: 64,
            height: 32,
            inflate,
            decodeLayerPreview,
            maxPreviewSize: 32
        });

        expect(inflate).toHaveBeenCalledWith(new Uint8Array([5, 6]), 2);
        expect(decodeLayerPreview).toHaveBeenCalledWith(new Uint8Array([1, 2]), 1, "sl1", 64, 32, 1, 0, 32);
        expect(result).toMatchObject({
            pixels: new Uint8Array([3, 4]),
            width: 32,
            height: 16,
            timings: {
                compressedBytes: 2,
                pngPayloadBytes: 2
            }
        });
        expect(result.timings.zipInflateMs).toBeGreaterThanOrEqual(0);
        expect(result.timings.wasmPreviewDecodeMs).toBeGreaterThanOrEqual(0);
        expect(result.timings.totalMs).toBeGreaterThanOrEqual(0);
    });

    it("throws on unsupported compression methods / 抛出 on 不支持 compression 方法", async () => {
        const inflate = vi.fn(async () => new Uint8Array([1, 2, 3]));
        const decodeEncodeLayer = vi.fn(() => new Uint8Array([9, 9]));

        await expect(
            convertZipLayerForTarget({
                compressed: new Uint8Array([7, 8]),
                compressionMethod: 12,
                uncompressedSize: 3,
                index: 4,
                sourceFormat: "sl1",
                targetFormat: "ctb",
                width: 256,
                height: 128,
                layerMetadataJson: "{}",
                inflate,
                decodeEncodeLayer
            })
        ).rejects.toThrow(/Unsupported ZIP compression method: 12/);
    });
});
