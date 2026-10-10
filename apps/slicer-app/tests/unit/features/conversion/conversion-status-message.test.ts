import { describe, expect, it } from "vitest";

import { buildConversionStatusMessage } from "../../../../src/features/conversion/model/conversion-status-message";

describe("buildConversionStatusMessage / 转换状态文案", () => {
    it("builds conversion status text / 构建 转换 status text", () => {
        const text = buildConversionStatusMessage({
            totalLayers: 10,
            targetFormat: "ctb",
            blobSizeBytes: 1536,
            totalElapsedMs: 1500,
            modeLabel: "workers x4",
            setupMs: 200,
            layerPhaseMs: 1000,
            computeMs: 800,
            assembleMs: 120,
            finalizeOnlyMs: 0
        });

        expect(text).toContain("Converted 10 layers to CTB in 1.50s");
        expect(text).toContain("Output 1.5 KB");
        expect(text).toContain("workers x4");
        expect(text).toContain("setup 200.0ms");
        expect(text).toContain("worker sum 800.0ms");
        expect(text).toContain("assemble 120.0ms");
        expect(text).toContain("finalize-only 0.0ms");
    });

    it("keeps zip conversion status on the same top-level timing fields as ctb / 保持 zip 转换 status on the 相同 top-level timing fields as ctb", () => {
        const text = buildConversionStatusMessage({
            totalLayers: 10,
            targetFormat: "vdt",
            blobSizeBytes: 1536,
            totalElapsedMs: 1500,
            modeLabel: "workers x4",
            setupMs: 200,
            layerPhaseMs: 1000,
            computeMs: 800,
            assembleMs: 120,
            finalizeOnlyMs: 0
        });

        expect(text).toContain("Converted 10 layers to VDT in 1.50s");
        expect(text).toContain("Output 1.5 KB");
        expect(text).toContain("workers x4");
        expect(text).toContain("setup 200.0ms");
        expect(text).toContain("worker sum 800.0ms");
        expect(text).toContain("assemble 120.0ms");
        expect(text).toContain("finalize-only 0.0ms");
        expect(text).not.toContain("zip assemble");
        expect(text).not.toContain("zip inflate");
        expect(text).not.toContain("decode/encode");
        expect(text).not.toContain("crc32");
        expect(text).not.toContain("deflate");
    });

    it("uses streamed output label when blob size is unavailable / 使用 streamed output label 当 blob size is unavailable", () => {
        const text = buildConversionStatusMessage({
            totalLayers: 10,
            targetFormat: "vdt",
            blobSizeBytes: null,
            totalElapsedMs: 1500,
            modeLabel: "workers x8 (streamed)",
            setupMs: 200,
            layerPhaseMs: 1000,
            computeMs: 800,
            assembleMs: 120,
            finalizeOnlyMs: 0
        });

        expect(text).toContain("Converted 10 layers to VDT in 1.50s");
        expect(text).toContain("Output streamed to disk");
        expect(text).toContain("workers x8 (streamed)");
        expect(text).not.toContain("(0 B)");
    });

    it("appends finalize-only suffix when finalizeOnlyMs is supplied / appends finalize-only suffix 当 finalizeOnlyMs is supplied", () => {
        const text = buildConversionStatusMessage({
            totalLayers: 10,
            targetFormat: "vdt",
            blobSizeBytes: 2048,
            totalElapsedMs: 1500,
            modeLabel: "workers x4",
            setupMs: 200,
            layerPhaseMs: 1000,
            computeMs: 800,
            assembleMs: 120,
            finalizeOnlyMs: 42
        });

        expect(text).toContain("assemble 120.0ms");
        expect(text).toContain("finalize-only 42.0ms");
    });
});
