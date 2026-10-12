import { describe, expect, it } from "vitest";
import { openSlicerDocumentFile } from "../../src/browser/document/source.js";
import { syncFormatFacts } from "../../src/browser/format/facts.js";
import { createInMemoryZipSource, readZipEntryBytes } from "../../src/browser/archive/zip/source.js";
import { formatFact } from "./support/format-facts.js";

describe("complete input validation / 完整输入校验", () => {
    it("rejects a complete file with missing layer definitions / 拒绝缺少图层定义的完整文件", async () => {
        syncFormatFacts([formatFact({ name: "ctb", aliases: [], extensions: ["ctb"], is_zip: false })]);
        await expect(
            openSlicerDocumentFile({
                file: new File([new Uint8Array(128)], "bad.ctb"),
                parseHeader: () => ({ format: "ctb" as never, metadata: { layer_count: 10 }, layers: [] })
            })
        ).rejects.toThrow(/layer count/i);
    });

    it("rejects layer payload ranges outside the file / 拒绝超出文件范围的图层数据", async () => {
        syncFormatFacts([formatFact({ name: "ctb", aliases: [], extensions: ["ctb"], is_zip: false })]);
        await expect(
            openSlicerDocumentFile({
                file: new File([new Uint8Array(128)], "bad.ctb"),
                parseHeader: () => ({
                    format: "ctb" as never,
                    metadata: { layer_count: 1 },
                    layers: [{ offset: 120, size: 16 }]
                })
            })
        ).rejects.toThrow(/layer 0.*range/i);
    });

    it("rejects a truncated stored ZIP payload / 拒绝截断的 stored ZIP 数据", async () => {
        const bytes = new Uint8Array(31);
        new DataView(bytes.buffer).setUint32(0, 0x04034b50, true);
        await expect(
            readZipEntryBytes(createInMemoryZipSource(bytes), {
                index: 0,
                name: "config.ini",
                compressionMethod: 0,
                compressedSize: 4,
                uncompressedSize: 4,
                localHeaderOffset: 0,
                usesDataDescriptor: false
            })
        ).rejects.toThrow(/range|truncat/i);
    });
});
