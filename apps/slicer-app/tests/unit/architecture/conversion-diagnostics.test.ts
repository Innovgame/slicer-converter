import { describe, expect, it } from "vitest";

import {
    assertConversionDiagnosticsReport,
    buildConversionDiagnosticsReport,
    type ConversionDiagnosticsRun
} from "../../support/conversion-diagnostics";

function runFor(target: string, details: Record<string, unknown>): ConversionDiagnosticsRun {
    return {
        target,
        iteration: 1,
        statusText: "Converted 10 layers",
        elapsedMs: 1000,
        diagnostics: [
            {
                channel: target === "vdt" ? "slicer-zip" : "slicer-output",
                event: "complete",
                details,
                raw: `[complete ${target}]`
            }
        ]
    };
}

function dispatchTransferFixture(): Record<string, number> {
    return {
        bytes: 71_000_000,
        copiedBytes: 0,
        copiedCount: 0,
        count: 10
    };
}

function wasmBytesFixture(): Record<string, number> {
    return {
        inputBytes: 71_000_000,
        outputBytes: 72_000_000
    };
}

describe("conversion diagnostics report / 转换诊断报告", () => {
    it("summarizes key sink and finalize metrics per target / summarizes key sink 并且 finalize metrics per target", () => {
        const report = buildConversionDiagnosticsReport([
            runFor("ctb", {
                targetFormat: "ctb",
                assembleWaitMs: 450,
                layerPhaseMs: 370,
                computeMs: 260,
                dispatchTransfer: dispatchTransferFixture(),
                sink: {
                    writeCount: 1381,
                    writeBytes: 71_895_099,
                    writeWallMs: 118,
                    finishCount: 1,
                    finishWallMs: 450,
                    patchCount: 1
                }
            }),
            runFor("vdt", {
                targetFormat: "vdt",
                finalizeOnlyWallMs: 900,
                centralDirectoryWallMs: 1,
                sinkFinishWallMs: 899,
                dispatchTransfer: dispatchTransferFixture(),
                wasmBytes: wasmBytesFixture(),
                sink: {
                    writeCount: 2763,
                    writeBytes: 71_720_801,
                    writeWallMs: 107,
                    finishCount: 1,
                    finishWallMs: 899
                }
            })
        ]);

        expect(report.targets.ctb).toMatchObject({
            runs: 1,
            completeEvents: 1,
            sinkFinishWallMs: 450,
            sinkWriteCount: 1381,
            sinkWriteBytes: 71_895_099,
            assembleWaitMs: 450
        });
        expect(report.targets.vdt).toMatchObject({
            runs: 1,
            completeEvents: 1,
            sinkFinishWallMs: 899,
            sinkWriteCount: 2763,
            finalizeOnlyWallMs: 900,
            centralDirectoryWallMs: 1
        });
        expect(report.failures).toEqual([]);
    });

    it("aggregates repeated target runs using the slowest reported metrics / aggregates repeated target runs using the slowest reported metrics 用例", () => {
        const report = buildConversionDiagnosticsReport([
            runFor("vdt", {
                targetFormat: "vdt",
                finalizeOnlyWallMs: 500,
                sinkFinishWallMs: 450,
                dispatchTransfer: dispatchTransferFixture(),
                wasmBytes: wasmBytesFixture(),
                sink: {
                    writeCount: 2700,
                    writeBytes: 70_000_000,
                    writeWallMs: 80
                }
            }),
            runFor("vdt", {
                targetFormat: "vdt",
                finalizeOnlyWallMs: 900,
                sinkFinishWallMs: 800,
                dispatchTransfer: dispatchTransferFixture(),
                wasmBytes: wasmBytesFixture(),
                sink: {
                    writeCount: 2763,
                    writeBytes: 71_720_801,
                    writeWallMs: 107
                }
            })
        ]);

        expect(report.targets.vdt).toMatchObject({
            runs: 2,
            completeEvents: 2,
            finalizeOnlyWallMs: 900,
            sinkFinishWallMs: 800,
            sinkWriteCount: 2763,
            sinkWriteBytes: 71_720_801,
            sinkWriteWallMs: 107
        });
    });

    it("fails when diagnostics are missing or still report the removed batch mode / 失败 当 诊断 are 缺失 或 still report the removed batch mode", () => {
        const report = buildConversionDiagnosticsReport([
            {
                target: "vdt",
                iteration: 1,
                statusText: "Converted with workers x8 batch=8",
                elapsedMs: 1000,
                diagnostics: []
            }
        ]);

        expect(report.failures).toContain("vdt run 1 did not emit a complete diagnostic event");
        expect(report.failures).toContain("vdt run 1 status still reports batch mode");
        expect(() => assertConversionDiagnosticsReport(report)).toThrow(/complete diagnostic/);
    });
});

describe("conversion diagnostics bench gate / 转换诊断 bench gate", () => {
    it("applies optional thresholds only in the bench harness / applies optional thresholds only in the bench harness 用例", async () => {
        const { buildConversionDiagnosticsGate } = await import("../../bench/conversion-diagnostics-gate");
        const gate = buildConversionDiagnosticsGate(
            [
                runFor("vdt", {
                    targetFormat: "vdt",
                    finalizeOnlyWallMs: 950,
                    sinkFinishWallMs: 900,
                    dispatchTransfer: dispatchTransferFixture(),
                    wasmBytes: wasmBytesFixture(),
                    sink: {
                        writeCount: 2763,
                        writeBytes: 71_720_801,
                        writeWallMs: 107,
                        finishCount: 1,
                        finishWallMs: 900
                    }
                })
            ],
            {
                vdt: {
                    maxFinalizeOnlyWallMs: 500,
                    maxSinkFinishWallMs: 800
                }
            }
        );

        expect(gate.failures).toContain("vdt finalizeOnlyWallMs 950.0ms exceeds 500.0ms");
        expect(gate.failures).toContain("vdt sinkFinishWallMs 900.0ms exceeds 800.0ms");
    });
});
