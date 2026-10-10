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
 * @zh 测试用 `ConversionDiagnosticsTargetSummary` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsTargetSummary` used by tests.
 */
export interface ConversionDiagnosticsTargetSummary {
    runs: number;
    completeEvents: number;
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
 * @zh 测试用 `ConversionDiagnosticsReport` 诊断契约。
 * @en Diagnostics contract `ConversionDiagnosticsReport` used by tests.
 */
export interface ConversionDiagnosticsReport {
    targets: Record<string, ConversionDiagnosticsTargetSummary>;
    failures: string[];
}

/**
 * @zh 测试用 `buildConversionDiagnosticsReport` 诊断契约。
 * @en Diagnostics contract `buildConversionDiagnosticsReport` used by tests.
 */
export function buildConversionDiagnosticsReport(
    runs: readonly ConversionDiagnosticsRun[]
): ConversionDiagnosticsReport {
    const targets: Record<string, ConversionDiagnosticsTargetSummary> = {};
    const failures: string[] = [];

    for (const run of runs) {
        const completeEvents = run.diagnostics.filter((item) => item.event === "complete");
        if (completeEvents.length === 0) {
            failures.push(`${run.target} run ${run.iteration} did not emit a complete diagnostic event`);
        }
        if (/\bbatch=|\bbatch\b/.test(run.statusText)) {
            failures.push(`${run.target} run ${run.iteration} status still reports batch mode`);
        }

        const complete = completeEvents.at(-1);
        const details = complete?.details ?? {};
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

        targets[run.target] = mergeTargetSummary(targets[run.target], runSummary);
    }

    return { targets, failures };
}

/**
 * @zh 测试用 `assertConversionDiagnosticsReport` 诊断契约。
 * @en Diagnostics contract `assertConversionDiagnosticsReport` used by tests.
 */
export function assertConversionDiagnosticsReport(report: ConversionDiagnosticsReport): void {
    if (report.failures.length > 0) {
        throw new Error(report.failures.join("\n"));
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
