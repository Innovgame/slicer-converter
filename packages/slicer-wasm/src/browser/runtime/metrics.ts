/**
 * @zh 转换运行期指标快照。
 * @en Snapshot of conversion runtime metrics.
 */
export interface SlicerRuntimeMetricsSnapshot {
    readonly conversionModes: string[];
    readonly finalizeOnlyMs: number[];
    readonly transferBuffers: SlicerTransferBufferStats;
}

/**
 * @zh worker transfer buffer 统计。
 * @en Worker transfer-buffer statistics.
 */
export interface SlicerTransferBufferStats {
    count: number;
    bytes: number;
    copiedCount: number;
    copiedBytes: number;
}

/**
 * @zh 单次 browser conversion 运行期指标收集器。
 * @en Runtime metrics collector for one browser conversion.
 */
export interface SlicerRuntimeMetrics {
    recordConversionMode(modeLabel: string): void;
    recordFinalizeOnlyMs(durationMs: number): void;
    recordTransferBuffer(input: { byteLength: number; copied: boolean }): void;
    latestConversionMode(): string | null;
    latestFinalizeOnlyMs(): number | null;
    snapshot(): SlicerRuntimeMetricsSnapshot;
    reset(): void;
}

/**
 * @zh 创建 browser conversion 运行期指标收集器。
 * @en Creates a browser conversion runtime metrics collector.
 */
export function createSlicerRuntimeMetrics(): SlicerRuntimeMetrics {
    const conversionModes: string[] = [];
    const finalizeOnlyMs: number[] = [];
    const transferBuffers: SlicerTransferBufferStats = {
        count: 0,
        bytes: 0,
        copiedCount: 0,
        copiedBytes: 0
    };

    return {
        recordConversionMode(modeLabel) {
            conversionModes.push(modeLabel);
        },
        recordFinalizeOnlyMs(durationMs) {
            finalizeOnlyMs.push(durationMs);
        },
        recordTransferBuffer(input) {
            transferBuffers.count += 1;
            transferBuffers.bytes += input.byteLength;
            if (input.copied) {
                transferBuffers.copiedCount += 1;
                transferBuffers.copiedBytes += input.byteLength;
            }
        },
        latestConversionMode() {
            return conversionModes.at(-1) ?? null;
        },
        latestFinalizeOnlyMs() {
            return finalizeOnlyMs.at(-1) ?? null;
        },
        snapshot() {
            return {
                conversionModes: [...conversionModes],
                finalizeOnlyMs: [...finalizeOnlyMs],
                transferBuffers: { ...transferBuffers }
            };
        },
        reset() {
            conversionModes.length = 0;
            finalizeOnlyMs.length = 0;
            transferBuffers.count = 0;
            transferBuffers.bytes = 0;
            transferBuffers.copiedCount = 0;
            transferBuffers.copiedBytes = 0;
        }
    };
}
