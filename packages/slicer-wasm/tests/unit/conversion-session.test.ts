import { describe, expect, it } from "vitest";

import {
    asSlicerFormatId,
    createSlicerConversionSession,
    createSlicerRuntimeMetrics,
    syncSupportedFormatInfosForValidation
} from "../../src/index.js";
import type { SlicerDocument } from "../../src/index.js";
import { formatFact } from "./support/format-facts.js";

describe("conversion session / 转换读取会话", () => {
    it("builds encoded layer worker payloads / 构建普通编码层 worker payload", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", aliases: ["vdt"], extensions: ["vdt"], is_zip: true })
        ]);
        const document = createFakeDocument({
            kind: "encoded-layer",
            bytes: new Uint8Array([1, 2, 3])
        });
        const metrics = createSlicerRuntimeMetrics();
        const session = createSlicerConversionSession({
            document,
            targetFormat: asSlicerFormatId("vdt"),
            targetAnycubicVersion: 515,
            width: 4,
            height: 5,
            layerMetadataJson: "{}",
            workerCount: 2,
            metrics
        });

        const payload = await session.buildLayerDispatchPayload(0);

        expect(payload.request).toMatchObject({
            type: "decode-encode",
            index: 0,
            sourceFormat: "goo",
            targetFormat: "vdt",
            targetAnycubicVersion: 515,
            width: 4,
            height: 5,
            antiAliasLevel: 1,
            encryptionKey: 0,
            layerMetadataJson: "{}"
        });
        expect(payload.transfer.byteLength).toBe(3);
        expect(metrics.snapshot().transferBuffers).toEqual({ count: 1, bytes: 3, copiedCount: 0, copiedBytes: 0 });
    });

    it("builds compressed ZIP layer worker payloads / 构建压缩 ZIP 层 worker payload", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const document = createFakeDocument({
            kind: "zip-entry",
            bytes: new Uint8Array([9, 8, 7]),
            compressionMethod: 8,
            uncompressedSize: 99
        });
        const session = createSlicerConversionSession({
            document,
            targetFormat: asSlicerFormatId("goo"),
            width: 4,
            height: 5,
            layerMetadataJson: null,
            workerCount: 1,
            metrics: createSlicerRuntimeMetrics()
        });

        const payload = await session.buildPreparedZipLayerPayload(0, {
            name: "layers/000.png",
            compressionMethod: 0
        });

        expect(payload.request).toMatchObject({
            type: "zip-decode-encode-prepare",
            compressionMethod: 8,
            uncompressedSize: 99,
            entryName: "layers/000.png",
            entryCompressionMethod: 0
        });
    });
});

function createFakeDocument(payload: Awaited<ReturnType<SlicerDocument["readLayerPayload"]>>): SlicerDocument {
    return {
        sourceFile: new File([new Uint8Array([1])], "sample.goo"),
        sourceName: "sample.goo",
        sourceSize: 1,
        parsedHeader: {
            format: asSlicerFormatId("goo"),
            metadata: { resolution_x: 4, resolution_y: 5, layer_count: 1 },
            layers: [{ offset: 0, size: 1 }]
        },
        async readLayerPayload() {
            return payload;
        },
        async close() {}
    };
}
