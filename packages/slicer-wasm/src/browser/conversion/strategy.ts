import {
    buildSlicerZipOutputWithWorkers,
    convertSlicerLayersWithWorkers,
    createSlicerBinaryLayerQueue
} from "./controller.js";
import { assembleWasmBinaryOutputFromLayerStream } from "../output/wasm-binary.js";
import { snapshotBufferedOutputSinkStats, type PatchableSlicerOutputSink } from "../output/sink.js";
import type { SlicerConversionSession } from "./session.js";
import type { SlicerRuntimeMetrics } from "../runtime/metrics.js";
import type { FileMetadata, ParsedHeader, SlicerFormatId } from "../types.js";
import type { ManagedSlicerWorker } from "../worker/dispatch.js";

/**
 * @zh 转换策略执行结果。
 * @en Result of executing a conversion strategy.
 */
export interface SlicerConversionStrategyResult {
    readonly outputBlob: Blob | null;
    readonly outputSizeBytes: number | null;
    readonly computeMs: number;
    readonly layerPhaseMs: number;
    readonly assembleMs: number;
    readonly finalizeOnlyMs: number;
}

/**
 * @zh 转换策略输入。
 * @en Input for conversion strategy execution.
 */
export interface SlicerConversionStrategyInput {
    readonly isZipTarget: boolean;
    readonly metadata: FileMetadata;
    readonly header: ParsedHeader;
    readonly resolvedTargetFormat: SlicerFormatId;
    readonly targetAnycubicVersion?: number;
    readonly totalLayers: number;
    readonly workers: ManagedSlicerWorker[];
    readonly signal: AbortSignal;
    readonly sink: PatchableSlicerOutputSink;
    readonly session: SlicerConversionSession;
    readonly metrics: SlicerRuntimeMetrics;
    readonly layerMetadataJson: string;
    readonly jobName: string;
    readonly layerPhaseStartedAt: number;
    readonly updateProgress: (current: number, total: number, label: string) => void;
}

/**
 * @zh 根据目标格式选择 ZIP 或二进制转换策略。
 * @en Chooses ZIP or binary conversion strategy based on the target format.
 */
export function runSlicerConversionStrategy(
    input: SlicerConversionStrategyInput
): Promise<SlicerConversionStrategyResult> {
    return input.isZipTarget ? runZipConversion(input) : runBinaryConversion(input);
}

async function runZipConversion(input: SlicerConversionStrategyInput): Promise<SlicerConversionStrategyResult> {
    input.session.planZipStrategy();
    const zipResult = await buildSlicerZipOutputWithWorkers({
        metadata: input.metadata,
        targetFormat: input.resolvedTargetFormat,
        jobName: input.jobName,
        totalLayers: input.totalLayers,
        workers: input.workers,
        signal: input.signal,
        sink: input.sink,
        buildPreparedLayerPayload: input.session.buildPreparedZipLayerPayload,
        getDispatchTransferStats: () => input.metrics.snapshot().transferBuffers,
        updateProgress: input.updateProgress
    });
    input.metrics.recordFinalizeOnlyMs(zipResult.timings.finalizeOnlyWallMs);

    return {
        outputBlob: zipResult.blob,
        outputSizeBytes: zipResult.blob ? zipResult.blob.size : null,
        computeMs: zipResult.computeMs,
        layerPhaseMs: zipResult.timings.prepareWallMs,
        assembleMs: zipResult.timings.totalWallMs,
        finalizeOnlyMs: zipResult.timings.finalizeOnlyWallMs
    };
}

async function runBinaryConversion(input: SlicerConversionStrategyInput): Promise<SlicerConversionStrategyResult> {
    input.metrics.recordConversionMode(`workers x${input.workers.length}`);
    const layerQueue = createSlicerBinaryLayerQueue(input.totalLayers);
    const outputBlobPromise = assembleWasmBinaryOutputFromLayerStream({
        metadata: input.metadata,
        targetFormat: input.resolvedTargetFormat,
        targetAnycubicVersion: input.targetAnycubicVersion,
        layerCount: input.totalLayers,
        layerStream: layerQueue.consume(),
        sink: input.sink
    });

    let computeMs: number;
    try {
        const result = await convertSlicerLayersWithWorkers({
            workers: input.workers,
            target: input.resolvedTargetFormat,
            totalLayers: input.totalLayers,
            layerQueue,
            signal: input.signal,
            buildDispatchPayload: input.session.buildLayerDispatchPayload,
            updateProgress: input.updateProgress
        });
        computeMs = result.computeMs;
    } catch (error: unknown) {
        layerQueue.fail(toError(error));
        await outputBlobPromise.catch(() => undefined);
        throw error;
    }

    const layerPhaseMs = performance.now() - input.layerPhaseStartedAt;
    input.updateProgress(input.totalLayers, input.totalLayers, "Assembling file...");
    const assembleStartedAt = performance.now();
    const outputBlob = await outputBlobPromise;
    const assembleMs = performance.now() - assembleStartedAt;
    input.metrics.recordFinalizeOnlyMs(assembleMs);
    snapshotBufferedOutputSinkStats(input.sink);

    return {
        outputBlob,
        outputSizeBytes: outputBlob ? outputBlob.size : null,
        computeMs,
        layerPhaseMs,
        assembleMs,
        finalizeOnlyMs: assembleMs
    };
}

function toError(error: unknown): Error {
    return error instanceof Error ? error : new Error(String(error));
}
