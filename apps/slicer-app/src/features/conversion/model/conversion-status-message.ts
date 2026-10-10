import { formatBytes, formatDuration } from "../../../shared/lib/format";

/**
 * @zh 构建转换状态提示文案所需的输入数据。
 * @en Input data needed to build the conversion status message.
 */
export interface ConversionStatusInput {
    totalLayers: number;
    targetFormat: string;
    blobSizeBytes: number | null;
    totalElapsedMs: number;
    modeLabel: string;
    /**
     * @zh 从转换请求开始到 layer 主阶段真正启动之间的前置开销，
     *     包括保存目标选择、worker 预热、session 组装等。
     * @en Setup wall-clock before the layer phase starts, including save-target
     *     selection, worker warm-up, and session preparation.
     */
    setupMs?: number;
    layerPhaseMs: number;
    computeMs: number;
    assembleMs: number;
    /**
     * @zh 仅统计 finalize 阶段的墙上时间；二进制路径缺少细分指标时由调用方传 0。
     * @en Wall-clock time spent strictly inside finalization; binary paths pass 0 when no finer metric is available.
     */
    finalizeOnlyMs: number;
}

/**
 * @zh 根据转换结果生成转换完成状态文案。
 * @en Builds a conversion completion status message from the provided result inputs.
 */
export function buildConversionStatusMessage({
    totalLayers,
    targetFormat,
    blobSizeBytes,
    totalElapsedMs,
    modeLabel,
    setupMs,
    layerPhaseMs,
    computeMs,
    assembleMs,
    finalizeOnlyMs
}: ConversionStatusInput): string {
    const outputLabel = blobSizeBytes === null ? "streamed to disk" : formatBytes(blobSizeBytes);
    const setupSegment = typeof setupMs === "number" ? `; setup ${formatDuration(setupMs)}` : "";
    const finalizeSuffix = `; finalize-only ${formatDuration(finalizeOnlyMs)}`;
    return `Converted ${totalLayers} layers to ${targetFormat.toUpperCase()} in ${formatDuration(totalElapsedMs)}. Output ${outputLabel}. ${modeLabel}${setupSegment}; layers ${formatDuration(layerPhaseMs)}; worker sum ${formatDuration(computeMs)}; assemble ${formatDuration(assembleMs)}${finalizeSuffix}.`;
}
