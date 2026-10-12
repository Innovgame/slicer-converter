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
    it("does not let a measured run hide another run's missing metric / 不允许已观测运行掩盖其他运行的指标缺失", async () => {
        const { buildConversionDiagnosticsGate } = await import("../../bench/conversion-diagnostics-gate");
        const gate = buildConversionDiagnosticsGate(
            [
                runFor("vdt", {
                    sinkFinishWallMs: 5,
                    dispatchTransfer: dispatchTransferFixture(),
                    wasmBytes: wasmBytesFixture()
                }),
                {
                    target: "vdt",
                    iteration: 2,
                    elapsedMs: 1000,
                    diagnostics: [],
                    completionEvidence: "public-status",
                    statusText:
                        "Converted 10 layers to VDT in 1s. Output 1 MB. workers; layers 1s; worker sum 2s; assemble 3ms; finalize-only 2ms."
                }
            ],
            { vdt: { maxSinkFinishWallMs: 100 } }
        );
        expect(gate.failures.join(" ")).toMatch(/run 2.*sinkFinishWallMs.*unavailable/);
    });
    it("checks current public completion timings without inventing private diagnostics / 校验公开完成耗时且不伪造内部诊断", async () => {
        const { buildConversionDiagnosticsGate } = await import("../../bench/conversion-diagnostics-gate");
        const gate = buildConversionDiagnosticsGate(
            [
                {
                    target: "vdt",
                    iteration: 1,
                    elapsedMs: 1300,
                    diagnostics: [],
                    completionEvidence: "public-status",
                    statusText:
                        "Converted 914 layers to VDT in 1.28s. Output 16.1 MB. workers; setup 0.2ms; layers 1.28s; worker sum 8.85s; assemble 1.28s; finalize-only 4.3ms."
                }
            ],
            { vdt: { maxFinalizeOnlyWallMs: 4 } }
        );
        expect(gate.targets.vdt.finalizeOnlyWallMs).toBe(4.3);
        expect(gate.targets.vdt.completeEvents).toBe(0);
        expect(gate.targets.vdt.dispatchTransferBytes).toBeNull();
        expect(gate.failures).toEqual(["vdt finalizeOnlyWallMs 4.3ms exceeds 4.0ms"]);
    });

    it("rejects incomplete completion evidence and unavailable threshold measurements / 拒绝不完整完成证据及不可用阈值观测", async () => {
        const { buildConversionDiagnosticsGate } = await import("../../bench/conversion-diagnostics-gate");
        const gate = buildConversionDiagnosticsGate(
            [
                {
                    target: "vdt",
                    iteration: 1,
                    elapsedMs: 1300,
                    diagnostics: [],
                    completionEvidence: "public-status",
                    statusText: "Converting..."
                }
            ],
            { vdt: { maxSinkFinishWallMs: 100 } }
        );
        expect(gate.failures.join(" ")).toMatch(/completion/i);
        expect(gate.failures.join(" ")).toMatch(/sinkFinishWallMs.*unavailable/i);
    });

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
