import { describe, expect, it } from "vitest";

import { createSlicerDocument } from "../../src/browser/document/document.js";

describe("SlicerDocument / 切片文档", () => {
    it("reads encoded layers through readLayerPayload / 通过 readLayerPayload 读取 encoded layers", async () => {
        const file = createRawFile([1, 2, 3, 4]);
        const document = createSlicerDocument({
            sourceFile: file,
            parsedHeader: {
                format: "goo",
                metadata: { layer_count: 1 },
                layers: [{ offset: 1, size: 2 }]
            } as never,
            layerSource: { kind: "raw-file" }
        });

        await expect(document.readLayerPayload(0)).resolves.toMatchObject({
            kind: "encoded-layer",
            bytes: new Uint8Array([2, 3])
        });
        expect(Object.keys(document).sort()).toEqual([
            "close",
            "parsedHeader",
            "readLayerPayload",
            "sourceFile",
            "sourceName",
            "sourceSize"
        ]);
    });

    it("reads ZIP entry layers through readLayerPayload / 通过 readLayerPayload 读取 ZIP entry layers", async () => {
        const document = createSlicerDocument({
            sourceFile: new File([], "sample.vdt"),
            parsedHeader: { format: "vdt", metadata: { layer_count: 1 }, layers: [] } as never,
            layerSource: {
                kind: "zip",
                readCompressedLayerPayload: async () => ({
                    bytes: new Uint8Array([9, 8, 7]),
                    compressionMethod: 8,
                    uncompressedSize: 6
                })
            }
        });

        await expect(document.readLayerPayload(0)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([9, 8, 7]),
            compressionMethod: 8,
            uncompressedSize: 6
        });
    });

    it("rejects missing raw layer offsets / 拒绝 missing raw layer offsets", async () => {
        const document = createSlicerDocument({
            sourceFile: createRawFile([1, 2, 3, 4]),
            parsedHeader: { format: "goo", metadata: { layer_count: 1 }, layers: [] } as never,
            layerSource: { kind: "raw-file" }
        });

        await expect(document.readLayerPayload(0)).rejects.toThrow(/Layer 0 not found/);
    });
});

function createRawFile(bytes: number[]): File {
    return new File([new Uint8Array(bytes)], "sample.goo");
}
