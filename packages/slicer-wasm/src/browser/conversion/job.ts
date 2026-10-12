import { buildLayerMetadataJson } from "../format/header.js";
import { getFormatFacts } from "../format/facts.js";
import { buildSlicerOutputJobName } from "./filename.js";
import { createSlicerConversionSession } from "./session.js";
import { runSlicerConversionStrategy } from "./strategy.js";
import { createSlicerRuntimeMetrics } from "../runtime/metrics.js";
import type { PatchableSlicerOutputSink } from "../output/sink.js";
import type { SlicerDocument } from "../document/document.js";
import type { FileMetadata, ParsedHeader, SlicerFormatId } from "../types.js";
import type { ManagedSlicerWorker } from "../worker/dispatch.js";

/**
 * @zh 转换耗时统计。
 * @en Conversion timing statistics.
 */
export interface SlicerConversionTimings {
    readonly totalMs: number;
    readonly setupMs: number;
    readonly layerPhaseMs: number;
    readonly computeMs: number;
    readonly assembleMs: number;
    readonly finalizeOnlyMs: number;
}

/**
 * @zh 单个已打开文档转换任务的结果。
 * @en Result of converting one opened slicer document.
 */
export interface SlicerConversionJobResult {
    readonly outputBlob: Blob | null;
    readonly outputSizeBytes: number | null;
    readonly timings: SlicerConversionTimings;
    readonly modeLabel: string;
}

/**
 * @zh 执行单个已打开文档的 browser conversion job。
 * @en Runs one browser conversion job for an opened slicer document.
 */
export async function runSlicerConversionJob(input: {
    readonly document: SlicerDocument;
    readonly targetFormat: SlicerFormatId;
    readonly targetAnycubicVersion?: number;
    readonly outputFileName: string;
    readonly sink: PatchableSlicerOutputSink;
    readonly workers: ManagedSlicerWorker[];
    readonly signal: AbortSignal;
    readonly updateProgress: (current: number, total: number, label: string) => void;
}): Promise<SlicerConversionJobResult> {
    input.signal.throwIfAborted();
    const header = input.document.parsedHeader;
    const metadata = header.metadata;
    const totalLayers = header.layers.length;
    const startedAt = performance.now();

    input.updateProgress(0, totalLayers, "Converting...");

    const result = await runOpenedDocumentConversion({
        document: input.document,
        metadata,
        header,
        resolvedTargetFormat: input.targetFormat,
        targetAnycubicVersion: input.targetAnycubicVersion,
        totalLayers,
        startedAt,
        workers: input.workers,
        signal: input.signal,
        sink: withCancellation(input.sink, input.signal),
        outputFileName: input.outputFileName,
        updateProgress: input.updateProgress
    });

    input.signal.throwIfAborted();
    return result;
}

async function runOpenedDocumentConversion(input: {
    readonly document: SlicerDocument;
    readonly metadata: FileMetadata;
    readonly header: ParsedHeader;
    readonly resolvedTargetFormat: SlicerFormatId;
    readonly targetAnycubicVersion?: number;
    readonly totalLayers: number;
    readonly startedAt: number;
    readonly workers: ManagedSlicerWorker[];
    readonly signal: AbortSignal;
    readonly sink: PatchableSlicerOutputSink;
    readonly outputFileName: string;
    readonly updateProgress: (current: number, total: number, label: string) => void;
}): Promise<SlicerConversionJobResult> {
    const width = input.metadata.resolution_x || 256;
    const height = input.metadata.resolution_y || 256;
    const layerMetadataJson = buildLayerMetadataJson(input.metadata);
    const metrics = createSlicerRuntimeMetrics();
    const session = createSlicerConversionSession({
        document: input.document,
        targetFormat: input.resolvedTargetFormat,
        targetAnycubicVersion: input.targetAnycubicVersion,
        width,
        height,
        layerMetadataJson,
        workerCount: input.workers.length,
        metrics
    });

    let primaryError: unknown;
    try {
        const layerPhaseStartedAt = performance.now();
        const setupMs = layerPhaseStartedAt - input.startedAt;
        const execution = await runSlicerConversionStrategy({
            isZipTarget: getFormatFacts(input.resolvedTargetFormat).isZip,
            metadata: input.metadata,
            header: input.header,
            resolvedTargetFormat: input.resolvedTargetFormat,
            targetAnycubicVersion: input.targetAnycubicVersion,
            totalLayers: input.totalLayers,
            workers: input.workers,
            signal: input.signal,
            sink: input.sink,
            session,
            metrics,
            layerMetadataJson,
            jobName: buildSlicerOutputJobName(input.outputFileName),
            layerPhaseStartedAt,
            updateProgress: input.updateProgress
        });

        const totalElapsedMs = performance.now() - input.startedAt;
        const modeLabel = metrics.latestConversionMode() ?? `workers x${input.workers.length}`;
        const finalizeOnlyMs = metrics.latestFinalizeOnlyMs();
        return {
            outputBlob: execution.outputBlob,
            outputSizeBytes: execution.outputSizeBytes,
            modeLabel: modeLabel + (input.sink.streaming ? " (streamed)" : ""),
            timings: {
                totalMs: totalElapsedMs,
                setupMs,
                layerPhaseMs: execution.layerPhaseMs,
                computeMs: execution.computeMs,
                assembleMs: execution.assembleMs,
                finalizeOnlyMs: finalizeOnlyMs ?? execution.finalizeOnlyMs
            }
        };
    } catch (error: unknown) {
        primaryError = error;
        throw error;
    } finally {
        await closeSessionPreservingPrimaryError(() => session.close(), primaryError);
    }
}

async function closeSessionPreservingPrimaryError(close: () => Promise<void>, primaryError: unknown): Promise<void> {
    try {
        await close();
    } catch (closeError: unknown) {
        if (primaryError !== undefined) {
            console.warn("[slicer-conversion-job] session close failed after conversion error", closeError);
            return;
        }
        throw closeError;
    }
}

function withCancellation(sink: PatchableSlicerOutputSink, signal: AbortSignal): PatchableSlicerOutputSink {
    return {
        streaming: sink.streaming,
        async write(bytes) {
            signal.throwIfAborted();
            await sink.write(bytes);
            signal.throwIfAborted();
        },
        async patchHeader(bytes) {
            signal.throwIfAborted();
            await sink.patchHeader(bytes);
            signal.throwIfAborted();
        },
        async finish() {
            signal.throwIfAborted();
            const result = await sink.finish();
            signal.throwIfAborted();
            return result;
        },
        abort: (reason) => sink.abort(reason)
    };
}
