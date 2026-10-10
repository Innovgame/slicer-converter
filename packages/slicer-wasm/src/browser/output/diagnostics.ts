import type { PatchableSlicerOutputSink, SlicerOutputSink } from "./sink.js";

/**
 * @zh output sink 写入与完成阶段的诊断统计。
 * @en Diagnostic statistics for output sink writes and finalization.
 */
export interface OutputSinkWriteStats {
    writeCount: number;
    writeBytes: number;
    writeWallMs: number;
    maxWriteWallMs: number;
    finishCount: number;
    finishWallMs: number;
    patchCount?: number;
    patchBytes?: number;
    patchWallMs?: number;
    maxPatchWallMs?: number;
}

/**
 * @zh 为 output sink 包装写入/完成计时统计。
 * @en Wraps an output sink with write/finalize timing statistics.
 */
export function createInstrumentedOutputSink(
    sink: SlicerOutputSink,
    now: () => number = () => performance.now()
): { sink: SlicerOutputSink; stats: OutputSinkWriteStats } {
    const stats = createEmptyOutputSinkWriteStats();

    return {
        stats,
        sink: {
            get streaming() {
                return sink.streaming;
            },
            async write(data: Uint8Array): Promise<void> {
                const startedAt = now();
                try {
                    await sink.write(data);
                } finally {
                    const elapsed = now() - startedAt;
                    stats.writeCount += 1;
                    stats.writeBytes += data.byteLength;
                    stats.writeWallMs += elapsed;
                    stats.maxWriteWallMs = Math.max(stats.maxWriteWallMs, elapsed);
                }
            },
            async finish(): Promise<Blob | null> {
                const startedAt = now();
                try {
                    return await sink.finish();
                } finally {
                    stats.finishCount += 1;
                    stats.finishWallMs += now() - startedAt;
                }
            },
            abort(reason?: string): Promise<void> {
                return sink.abort(reason);
            }
        }
    };
}

/**
 * @zh 为 patchable output sink 包装写入、完成和 header patch 计时统计。
 * @en Wraps a patchable output sink with write, finalize, and header patch timing statistics.
 */
export function createInstrumentedPatchableOutputSink(
    sink: PatchableSlicerOutputSink,
    now: () => number = () => performance.now()
): { sink: PatchableSlicerOutputSink; stats: OutputSinkWriteStats } {
    const instrumented = createInstrumentedOutputSink(sink, now);
    instrumented.stats.patchCount = 0;
    instrumented.stats.patchBytes = 0;
    instrumented.stats.patchWallMs = 0;
    instrumented.stats.maxPatchWallMs = 0;

    return {
        stats: instrumented.stats,
        sink: {
            ...instrumented.sink,
            async patchHeader(data: Uint8Array): Promise<void> {
                const startedAt = now();
                try {
                    await sink.patchHeader(data);
                } finally {
                    const elapsed = now() - startedAt;
                    instrumented.stats.patchCount = (instrumented.stats.patchCount ?? 0) + 1;
                    instrumented.stats.patchBytes = (instrumented.stats.patchBytes ?? 0) + data.byteLength;
                    instrumented.stats.patchWallMs = (instrumented.stats.patchWallMs ?? 0) + elapsed;
                    instrumented.stats.maxPatchWallMs = Math.max(instrumented.stats.maxPatchWallMs ?? 0, elapsed);
                }
            }
        }
    };
}

/**
 * @zh 复制 output sink 诊断统计快照。
 * @en Copies an output sink diagnostic statistics snapshot.
 */
export function snapshotOutputSinkStats(stats: OutputSinkWriteStats): OutputSinkWriteStats {
    return { ...stats };
}

function createEmptyOutputSinkWriteStats(): OutputSinkWriteStats {
    return {
        writeCount: 0,
        writeBytes: 0,
        writeWallMs: 0,
        maxWriteWallMs: 0,
        finishCount: 0,
        finishWallMs: 0
    };
}
