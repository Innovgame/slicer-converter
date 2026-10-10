import { buildArchiveMetadataEntries, createArchiveLayerDescriptorProvider } from "../archive/plan.js";
import { getFormatFacts } from "../format/facts.js";
import { createOrderedLayerQueue, type OrderedLayerQueue } from "../output/stream.js";
import type { SlicerOutputSink } from "../output/sink.js";
import { prepareZipEntry } from "../archive/zip/archive.js";
import { ZipStreamAssembler } from "../archive/zip/stream-assembler.js";
import type { ZipAssemblyTimings } from "../archive/zip/assembly-timings.js";
import { runSlicerLayerWorkerScheduler } from "../worker/layer-worker-scheduler.js";
import type { SlicerTransferBufferStats } from "../runtime/metrics.js";
import type { SlicerZipLayerDescriptor } from "./session.js";
import type { FileMetadata, SlicerFormatId } from "../types.js";
import type { ManagedSlicerWorker, SlicerWorkerDispatchPayload } from "../worker/dispatch.js";
import type { SlicerWorkerResponse } from "../worker/protocol.js";

/**
 * @zh 单次 worker layer 转换通道的结果。
 * @en Result of a single worker layer conversion pass.
 */
export interface SlicerConversionPassResult {
    readonly computeMs: number;
}

/**
 * @zh ZIP 输出转换结果。
 * @en Conversion result for ZIP output targets.
 */
export interface SlicerZipConversionResult extends SlicerConversionPassResult {
    readonly blob: Blob | null;
    readonly timings: ZipAssemblyTimings;
}

/**
 * @zh 使用 worker 转换普通二进制目标的所有图层。
 * @en Converts all layers for a regular binary target with workers.
 */
export async function convertSlicerLayersWithWorkers(input: {
    readonly workers: ManagedSlicerWorker[];
    readonly target: SlicerFormatId;
    readonly totalLayers: number;
    readonly layerQueue: OrderedLayerQueue;
    readonly signal: AbortSignal;
    readonly buildDispatchPayload: (layerIndex: number) => Promise<SlicerWorkerDispatchPayload>;
    readonly updateProgress: (current: number, total: number, label: string) => void;
}): Promise<SlicerConversionPassResult> {
    let totalWorkerMs = 0;
    let completed = 0;

    await runSlicerLayerWorkerScheduler({
        workers: input.workers,
        totalLayers: input.totalLayers,
        signal: input.signal,
        buildDispatchPayload: input.buildDispatchPayload,
        isDoneMessage: (message, layerIndex): message is Extract<SlicerWorkerResponse, { type: "layer-done" }> =>
            message.type === "layer-done" && message.index === layerIndex,
        onDone(message, layerIndex) {
            input.layerQueue.push(layerIndex, new Uint8Array(message.result));
            totalWorkerMs += message.elapsedMs;
            completed += 1;
            input.updateProgress(completed, input.totalLayers, `Converting layer ${completed}/${input.totalLayers}...`);
        }
    });

    return { computeMs: totalWorkerMs };
}

/**
 * @zh 使用 worker 转换并准备 ZIP 输出。
 * @en Converts layers with workers and prepares ZIP output.
 */
export async function buildSlicerZipOutputWithWorkers(input: {
    readonly metadata: FileMetadata;
    readonly targetFormat: SlicerFormatId;
    readonly jobName: string;
    readonly totalLayers: number;
    readonly workers: ManagedSlicerWorker[];
    readonly signal: AbortSignal;
    readonly sink: SlicerOutputSink;
    readonly buildPreparedLayerPayload: (
        layerIndex: number,
        descriptor: SlicerZipLayerDescriptor
    ) => Promise<SlicerWorkerDispatchPayload>;
    readonly getDispatchTransferStats?: () => SlicerTransferBufferStats;
    readonly updateProgress: (current: number, total: number, label: string) => void;
}): Promise<SlicerZipConversionResult> {
    const facts = getFormatFacts(input.targetFormat);
    if (!facts.isZip) {
        throw new Error(`ZIP archive entries are not supported for target format: ${input.targetFormat}`);
    }

    const startedAt = performance.now();
    const assembler = await createZipAssemblerWithMetadata({
        metadata: input.metadata,
        targetFormat: input.targetFormat,
        jobName: input.jobName,
        totalLayers: input.totalLayers,
        sink: input.sink
    });
    const descriptorForLayer = createArchiveLayerDescriptorProvider({
        targetFormat: input.targetFormat,
        jobName: input.jobName,
        startIndex: 0,
        count: input.totalLayers
    });

    let completed = 0;
    let computeMs = 0;
    input.updateProgress(0, input.totalLayers, "Converting and preparing ZIP layers...");

    await runSlicerLayerWorkerScheduler({
        workers: input.workers,
        totalLayers: input.totalLayers,
        signal: input.signal,
        buildDispatchPayload: (layerIndex) => {
            const descriptor = descriptorForLayer(layerIndex);
            return input.buildPreparedLayerPayload(layerIndex, {
                name: descriptor.name,
                compressionMethod: descriptor.compressionMethod
            });
        },
        isDoneMessage: (
            message,
            layerIndex
        ): message is Extract<SlicerWorkerResponse, { type: "zip-layer-prepared" }> =>
            message.type === "zip-layer-prepared" && message.index === layerIndex,
        async onDone(message, layerIndex) {
            await assembler.pushLayerEntry(layerIndex, {
                name: message.name,
                compressionMethod: message.compressionMethod,
                compressedData: new Uint8Array(message.data),
                uncompressedSize: message.uncompressedSize,
                crc32: message.crc32
            });
            computeMs += message.computeMs;
            completed += 1;
            input.updateProgress(
                completed,
                input.totalLayers,
                `Converting and preparing ZIP layers ${completed}/${input.totalLayers}...`
            );
        }
    });

    input.updateProgress(input.totalLayers, input.totalLayers, "Finalizing archive...");
    const finalizeStartedAt = performance.now();
    const finalizeResult = await assembler.finalize();
    const totalWallMs = performance.now() - startedAt;

    return {
        blob: finalizeResult.blob,
        timings: {
            prepareWallMs: finalizeStartedAt - startedAt,
            finalizeOnlyWallMs: finalizeResult.timings.finalizeOnlyWallMs,
            totalWallMs,
            centralDirectoryWallMs: finalizeResult.timings.centralDirectoryWallMs,
            centralDirectoryBytes: finalizeResult.timings.centralDirectoryBytes,
            centralDirectoryEntryCount: finalizeResult.timings.centralDirectoryEntryCount,
            sinkFinishWallMs: finalizeResult.timings.sinkFinishWallMs
        },
        computeMs
    };
}

/**
 * @zh 创建二进制输出使用的有序 layer 队列。
 * @en Creates the ordered layer queue used by binary output.
 */
export function createSlicerBinaryLayerQueue(totalLayers: number): OrderedLayerQueue {
    return createOrderedLayerQueue(totalLayers);
}

async function createZipAssemblerWithMetadata(input: {
    readonly metadata: FileMetadata;
    readonly targetFormat: SlicerFormatId;
    readonly jobName: string;
    readonly totalLayers: number;
    readonly sink: SlicerOutputSink;
}): Promise<ZipStreamAssembler> {
    const metadataEntries = buildArchiveMetadataEntries({
        metadata: input.metadata,
        targetFormat: input.targetFormat,
        jobName: input.jobName,
        totalLayers: input.totalLayers
    });
    const metadataPrepared = await Promise.all(metadataEntries.map((entry) => prepareZipEntry(entry)));
    const assembler = new ZipStreamAssembler(input.sink);
    for (const entry of metadataPrepared) {
        await assembler.writePreparedEntry(entry);
    }
    return assembler;
}
