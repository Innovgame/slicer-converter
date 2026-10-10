import { describe, expect, it } from "vitest";

import {
    asSlicerFormatId,
    getFormatFacts,
    listImportableFileExtensions,
    listFormatFacts,
    resolveConcreteFormatId,
    syncFormatFacts,
    syncSupportedFormatInfosForValidation
} from "../../src/browser/format/format.js";
import { formatFact, IMPORT_EXPORT_CAPABILITIES } from "./support/format-facts.js";

describe("asSlicerFormatId / 格式 ID 校验", () => {
    it("accepts formats synced from WASM facts / 接受 格式化 synced 来自 WASM facts", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "ctb", aliases: ["ctb"], extensions: ["ctb"], is_zip: false }),
            formatFact({ name: "futurefmt", aliases: ["ffmt"], extensions: ["ffmt"], is_zip: false })
        ]);

        expect(asSlicerFormatId("ctb")).toBe("ctb");
        expect(asSlicerFormatId("futurefmt")).toBe("futurefmt");
        expect(asSlicerFormatId("ffmt")).toBe("ffmt");
    });

    it("throws when a format is not present in synced WASM facts / 抛出 当 a format is not present in synced WASM facts", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "ctb", aliases: ["ctb"], extensions: ["ctb"], is_zip: false })
        ]);

        expect(() => asSlicerFormatId("m5sp")).toThrow(/Unsupported format/);
        expect(() => asSlicerFormatId("zip")).toThrow(/Unsupported format/);
    });
});

describe("WASM format helpers / WASM 格式 helpers", () => {
    it("does not export app-only virtual target parsing / 不 export app-only virtual target parsing", async () => {
        const formatModule = await import("../../src/browser/format/format.js");
        const removedExportName = ["as", "Export", "Target", "Format"].join("");

        expect(removedExportName in formatModule).toBe(false);
    });
});

describe("format facts lookup / 格式事实查询", () => {
    it("looks up canonical formats aliases and extensions / 查询 canonical formats aliases and extensions", () => {
        syncFormatFacts([
            formatFact({
                name: "chitubox-zip",
                aliases: ["chituzip"],
                extensions: ["zip"],
                is_zip: true
            }),
            formatFact({
                name: "ctb",
                aliases: ["ctb-v2"],
                extensions: ["ctb"],
                is_zip: false
            })
        ]);

        expect(getFormatFacts("zip").format).toBe("chitubox-zip");
        expect(getFormatFacts("chituzip").isZip).toBe(true);
        expect(resolveConcreteFormatId("ctb-v2")).toBe("ctb");
        expect(listFormatFacts().map((facts) => facts.format).sort()).toEqual(["chitubox-zip", "ctb"]);
    });

    it("keeps first alias and extension lookup winners while canonical names stay direct / 保持 first alias and extension lookup winners 同时 canonical names stay direct", () => {
        syncFormatFacts([
            formatFact({
                name: "cxdlp",
                aliases: ["creality-cxdlp"],
                extensions: ["cxdlp"],
                is_zip: false
            }),
            formatFact({
                name: "cxdlp-v2",
                aliases: ["creality-cxdlp-v2", "cxdlp"],
                extensions: ["cxdlp"],
                is_zip: false
            })
        ]);

        expect(getFormatFacts("cxdlp").format).toBe("cxdlp");
        expect(resolveConcreteFormatId("cxdlp")).toBe("cxdlp");
        expect(getFormatFacts("cxdlp-v2").format).toBe("cxdlp-v2");
        expect(resolveConcreteFormatId("creality-cxdlp-v2")).toBe("cxdlp-v2");
    });

    it("rejects archive flag mismatches / 拒绝 archive flag mismatches", () => {
        expect(() =>
            syncFormatFacts([
                formatFact({
                    name: "badzip",
                    aliases: ["badzip"],
                    extensions: ["badzip"],
                    is_zip: true,
                    capabilities: {
                        ...IMPORT_EXPORT_CAPABILITIES,
                        is_archive: false
                    }
                })
            ])
        ).toThrow(/archive flag mismatch/);
    });

    it("lists sorted importable file extensions / 列出排序后的可导入文件扩展名", () => {
        syncFormatFacts([
            formatFact({ name: "ctb", aliases: ["ctb"], extensions: ["ctb"], is_zip: false }),
            formatFact({
                name: "exportonly",
                aliases: ["exportonly"],
                extensions: ["xonly"],
                is_zip: false,
                capabilities: {
                    ...IMPORT_EXPORT_CAPABILITIES,
                    can_import: false
                }
            }),
            formatFact({ name: "sl1", aliases: ["sl1"], extensions: ["sl1", "sl1s"], is_zip: true }),
            formatFact({ name: "cxdlp", aliases: ["creality-cxdlp"], extensions: ["cxdlp"], is_zip: false }),
            formatFact({ name: "cxdlpv4", aliases: ["creality-cxdlpv4"], extensions: ["cxdlpv4"], is_zip: false })
        ]);

        expect(listImportableFileExtensions()).toEqual([".ctb", ".cxdlp", ".cxdlpv4", ".sl1", ".sl1s"]);
    });
});
