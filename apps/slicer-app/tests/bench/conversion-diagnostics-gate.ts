/**
 * @zh 测试用 `DiagnosticEvent` 契约。
 * @en Test contract `DiagnosticEvent`.
 */
export interface DiagnosticEvent {
    channel: "slicer-output" | "slicer-zip";
    event: string;
    details: Record<string, unknown>;
    raw: string;
}

/**
 * @zh 测试用 `ConversionDiagnosticsRun` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsRun` used by tests.
 */
export interface ConversionDiagnosticsRun {
    target: string;
    iteration: number;
    statusText: string;
    elapsedMs: number;
    diagnostics: DiagnosticEvent[];
    completionEvidence?: "public-status";
    fileSystemSink?: FileSystemSinkStats;
}

/**
 * @zh 测试用 `FileSystemSinkStats` 契约。
 * @en Test contract `FileSystemSinkStats`.
 */
export interface FileSystemSinkStats {
    writeCount: number;
    writeBytes: number;
    writeWallMs: number;
    maxWriteWallMs: number;
    seekCount: number;
    seekWallMs: number;
    maxSeekWallMs: number;
    closeCount: number;
    closeWallMs: number;
    maxCloseWallMs: number;
    abortCount: number;
}

/**
 * @zh 测试用 `ConversionDiagnosticsThresholds` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsThresholds` used by tests.
 */
export interface ConversionDiagnosticsThresholds {
    maxFinalizeOnlyWallMs?: number;
    maxSinkFinishWallMs?: number;
    maxSinkWriteCount?: number;
}

/**
 * @zh 测试用 `ConversionDiagnosticsThresholdMap` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsThresholdMap` used by tests.
 */
export type ConversionDiagnosticsThresholdMap = Record<string, ConversionDiagnosticsThresholds>;

/**
 * @zh 测试用 `ConversionDiagnosticsTargetSummary` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsTargetSummary` used by tests.
 */
export interface ConversionDiagnosticsTargetSummary {
    runs: number;
    completeEvents: number;
    publicStatusCompletions: number;
    elapsedMs: number;
    assembleWaitMs: number | null;
    centralDirectoryWallMs: number | null;
    computeMs: number | null;
    finalizeOnlyWallMs: number | null;
    layerPhaseMs: number | null;
    sinkFinishWallMs: number | null;
    sinkWriteBytes: number | null;
    sinkWriteCount: number | null;
    sinkWriteWallMs: number | null;
    dispatchTransferBytes: number | null;
    dispatchTransferCopiedBytes: number | null;
    dispatchTransferCopiedCount: number | null;
    dispatchTransferCount: number | null;
    wasmInputBytes: number | null;
    wasmOutputBytes: number | null;
}

/**
 * @zh 测试用 `ConversionDiagnosticsGate` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsGate` used by tests.
 */
export interface ConversionDiagnosticsGate {
    targets: Record<string, ConversionDiagnosticsTargetSummary>;
    failures: string[];
}

/**
 * @zh 测试用 `buildConversionDiagnosticsGate` 诊断契约。
 * @en Diagnostics contract `buildConversionDiagnosticsGate` used by tests.
 */
export function buildConversionDiagnosticsGate(
    runs: readonly ConversionDiagnosticsRun[],
    thresholds: ConversionDiagnosticsThresholdMap = {}
): ConversionDiagnosticsGate {
    const targets: Record<string, ConversionDiagnosticsTargetSummary> = {};
    const failures: string[] = [];

    for (const run of runs) {
        const completeEvents = run.diagnostics.filter((item) => item.event === "complete");
        const publicTimings =
            run.completionEvidence === "public-status" ? readPublicCompletionTimings(run.statusText, run.target) : null;
        if (completeEvents.length === 0 && publicTimings === null) {
            failures.push(`${run.target} run ${run.iteration} did not provide valid completion evidence`);
        }
        if (/\bbatch=|\bbatch\b/.test(run.statusText)) {
            failures.push(`${run.target} run ${run.iteration} status still reports batch mode`);
        }

        const complete = completeEvents.at(-1);
        const details = complete?.details ?? publicTimings ?? {};
        const sink = readObject(details.sink);
        const dispatchTransfer = readObject(details.dispatchTransfer);
        const wasmBytes = readObject(details.wasmBytes);
        if (complete) {
            if (Object.keys(dispatchTransfer).length === 0) {
                failures.push(`${run.target} run ${run.iteration} complete diagnostic is missing dispatchTransfer`);
            }
            if (run.target === "vdt" && Object.keys(wasmBytes).length === 0) {
                failures.push(`${run.target} run ${run.iteration} complete diagnostic is missing wasmBytes`);
            }
        }
        const runSummary: ConversionDiagnosticsTargetSummary = {
            runs: 1,
            completeEvents: completeEvents.length,
            publicStatusCompletions: complete ? 0 : publicTimings ? 1 : 0,
            elapsedMs: run.elapsedMs,
            assembleWaitMs: readNumber(details.assembleWaitMs),
            centralDirectoryWallMs: readNumber(details.centralDirectoryWallMs),
            computeMs: readNumber(details.computeMs),
            finalizeOnlyWallMs: readNumber(details.finalizeOnlyWallMs),
            layerPhaseMs: readNumber(details.layerPhaseMs),
            sinkFinishWallMs: readNumber(details.sinkFinishWallMs) ?? readNumber(sink.finishWallMs),
            sinkWriteBytes: readNumber(sink.writeBytes),
            sinkWriteCount: readNumber(sink.writeCount),
            sinkWriteWallMs: readNumber(sink.writeWallMs),
            dispatchTransferBytes: readNumber(dispatchTransfer.bytes),
            dispatchTransferCopiedBytes: readNumber(dispatchTransfer.copiedBytes),
            dispatchTransferCopiedCount: readNumber(dispatchTransfer.copiedCount),
            dispatchTransferCount: readNumber(dispatchTransfer.count),
            wasmInputBytes: readNumber(wasmBytes.inputBytes),
            wasmOutputBytes: readNumber(wasmBytes.outputBytes)
        };

        const required = thresholds[run.target];
        for (const [limit, metric] of [
            ["maxFinalizeOnlyWallMs", "finalizeOnlyWallMs"],
            ["maxSinkFinishWallMs", "sinkFinishWallMs"],
            ["maxSinkWriteCount", "sinkWriteCount"]
        ] as const) {
            if (required?.[limit] !== undefined && runSummary[metric] === null)
                failures.push(`${run.target} run ${run.iteration} ${metric} measurement unavailable`);
        }

        targets[run.target] = mergeTargetSummary(targets[run.target], runSummary);
    }

    for (const [target, summary] of Object.entries(targets)) {
        applyThresholds(target, summary, thresholds[target], failures);
    }

    return { targets, failures };
}

/**
 * @zh 测试用 `assertConversionDiagnosticsGate` 诊断契约。
 * @en Diagnostics contract `assertConversionDiagnosticsGate` used by tests.
 */
export function assertConversionDiagnosticsGate(gate: ConversionDiagnosticsGate): void {
    if (gate.failures.length > 0) {
        throw new Error(gate.failures.join("\n"));
    }
}

function applyThresholds(
    target: string,
    summary: ConversionDiagnosticsTargetSummary,
    thresholds: ConversionDiagnosticsThresholds | undefined,
    failures: string[]
): void {
    if (!thresholds) {
        return;
    }

    if (
        thresholds.maxFinalizeOnlyWallMs !== undefined &&
        summary.finalizeOnlyWallMs !== null &&
        summary.finalizeOnlyWallMs > thresholds.maxFinalizeOnlyWallMs
    ) {
        failures.push(
            `${target} finalizeOnlyWallMs ${formatMs(summary.finalizeOnlyWallMs)} exceeds ${formatMs(
                thresholds.maxFinalizeOnlyWallMs
            )}`
        );
    }

    if (
        thresholds.maxSinkFinishWallMs !== undefined &&
        summary.sinkFinishWallMs !== null &&
        summary.sinkFinishWallMs > thresholds.maxSinkFinishWallMs
    ) {
        failures.push(
            `${target} sinkFinishWallMs ${formatMs(summary.sinkFinishWallMs)} exceeds ${formatMs(
                thresholds.maxSinkFinishWallMs
            )}`
        );
    }

    if (
        thresholds.maxSinkWriteCount !== undefined &&
        summary.sinkWriteCount !== null &&
        summary.sinkWriteCount > thresholds.maxSinkWriteCount
    ) {
        failures.push(`${target} sinkWriteCount ${summary.sinkWriteCount} exceeds ${thresholds.maxSinkWriteCount}`);
    }
}

function readObject(value: unknown): Record<string, unknown> {
    return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function readNumber(value: unknown): number | null {
    return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function mergeTargetSummary(
    previous: ConversionDiagnosticsTargetSummary | undefined,
    next: ConversionDiagnosticsTargetSummary
): ConversionDiagnosticsTargetSummary {
    if (!previous) {
        return next;
    }

    return {
        runs: previous.runs + next.runs,
        completeEvents: previous.completeEvents + next.completeEvents,
        publicStatusCompletions: previous.publicStatusCompletions + next.publicStatusCompletions,
        elapsedMs: Math.max(previous.elapsedMs, next.elapsedMs),
        assembleWaitMs: maxNullable(previous.assembleWaitMs, next.assembleWaitMs),
        centralDirectoryWallMs: maxNullable(previous.centralDirectoryWallMs, next.centralDirectoryWallMs),
        computeMs: maxNullable(previous.computeMs, next.computeMs),
        finalizeOnlyWallMs: maxNullable(previous.finalizeOnlyWallMs, next.finalizeOnlyWallMs),
        layerPhaseMs: maxNullable(previous.layerPhaseMs, next.layerPhaseMs),
        sinkFinishWallMs: maxNullable(previous.sinkFinishWallMs, next.sinkFinishWallMs),
        sinkWriteBytes: maxNullable(previous.sinkWriteBytes, next.sinkWriteBytes),
        sinkWriteCount: maxNullable(previous.sinkWriteCount, next.sinkWriteCount),
        sinkWriteWallMs: maxNullable(previous.sinkWriteWallMs, next.sinkWriteWallMs),
        dispatchTransferBytes: maxNullable(previous.dispatchTransferBytes, next.dispatchTransferBytes),
        dispatchTransferCopiedBytes: maxNullable(
            previous.dispatchTransferCopiedBytes,
            next.dispatchTransferCopiedBytes
        ),
        dispatchTransferCopiedCount: maxNullable(
            previous.dispatchTransferCopiedCount,
            next.dispatchTransferCopiedCount
        ),
        dispatchTransferCount: maxNullable(previous.dispatchTransferCount, next.dispatchTransferCount),
        wasmInputBytes: maxNullable(previous.wasmInputBytes, next.wasmInputBytes),
        wasmOutputBytes: maxNullable(previous.wasmOutputBytes, next.wasmOutputBytes)
    };
}

function maxNullable(left: number | null, right: number | null): number | null {
    if (left === null) {
        return right;
    }
    if (right === null) {
        return left;
    }
    return Math.max(left, right);
}

function formatMs(value: number): string {
    return `${value.toFixed(1)}ms`;
}

function readPublicCompletionTimings(status: string, target: string): Record<string, number> | null {
    const completed = /^Converted \d+ layers to (\S+) in [\d.]+(?:ms|s)\./u.exec(status);
    if (completed?.[1] !== target.toUpperCase()) return null;
    const details: Record<string, number> = {};
    for (const [label, key] of [
        ["layers", "layerPhaseMs"],
        ["worker sum", "computeMs"],
        ["assemble", "assembleWaitMs"],
        ["finalize-only", "finalizeOnlyWallMs"]
    ]) {
        const match = new RegExp(`; ${label} ([\\d.]+)(ms|s)`).exec(status);
        if (!match) return null;
        const value = Number(match[1]) * (match[2] === "s" ? 1000 : 1);
        if (!Number.isFinite(value)) return null;
        details[key] = value;
    }
    return details;
}
