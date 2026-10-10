import { describe, expect, it, vi } from "vitest";

import { createSlicerPreviewSession } from "../../src/browser/preview/session.js";
import type { SlicerDocument } from "../../src/browser/document/document.js";
import type { ParsedHeader } from "../../src/browser/types.js";

describe("createSlicerPreviewSession / 创建 preview session", () => {
    it("builds encoded-layer preview worker requests / 构造 encoded-layer preview worker 请求", async () => {
        const document = createDocument({
            format: "goo",
            metadata: {
                resolution_x: 3840,
                resolution_y: 2400,
                anti_alias_level: 2,
                encryption_key: 7
            },
            layers: [{ offset: 0, size: 3 }]
        });
        vi.mocked(document.readLayerPayload).mockResolvedValue({
            kind: "encoded-layer",
            bytes: new Uint8Array([1, 2, 3])
        });

        const payload = await createSlicerPreviewSession({ document }).buildDispatchPayload({
            layerIndex: 0,
            requestId: 9,
            quality: "interactive",
            maxPreviewSize: 512
        });

        expect(payload.transfer).toBeInstanceOf(ArrayBuffer);
        expect(payload.request).toEqual({
            type: "decode-preview",
            requestId: 9,
            quality: "interactive",
            index: 0,
            maxPreviewSize: 512,
            payload: {
                kind: "encoded-layer",
                data: payload.transfer,
                format: "goo",
                width: 3840,
                height: 2400,
                antiAliasLevel: 2,
                encryptionKey: 7
            }
        });
    });

    it("builds ZIP-entry preview worker requests without inflating bytes / 构造 ZIP-entry preview worker 请求且不解压", async () => {
        const document = createDocument({
            format: "vdt",
            metadata: {
                resolution_x: 2560,
                resolution_y: 1440
            },
            layers: [{ offset: 0, size: 2 }]
        });
        vi.mocked(document.readLayerPayload).mockResolvedValue({
            kind: "zip-entry",
            bytes: new Uint8Array([5, 6]),
            compressionMethod: 8,
            uncompressedSize: 100
        });

        const payload = await createSlicerPreviewSession({ document }).buildDispatchPayload({
            layerIndex: 3,
            requestId: 10,
            quality: "settled",
            maxPreviewSize: 1024
        });

        expect(payload.request).toEqual({
            type: "decode-preview",
            requestId: 10,
            quality: "settled",
            index: 3,
            maxPreviewSize: 1024,
            payload: {
                kind: "zip-entry",
                data: payload.transfer,
                compressionMethod: 8,
                uncompressedSize: 100,
                sourceFormat: "vdt",
                width: 2560,
                height: 1440
            }
        });
    });
});

function createDocument(parsedHeader: ParsedHeader): SlicerDocument {
    return {
        sourceFile: new File([], "sample.goo"),
        sourceName: "sample.goo",
        sourceSize: 0,
        parsedHeader,
        readLayerPayload: vi.fn(),
        close: vi.fn()
    };
}
