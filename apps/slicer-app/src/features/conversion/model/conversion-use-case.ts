import {
    convertSlicerFileToBlob,
    convertSlicerFileToSink,
    type ManagedSlicerWorker as ManagedConversionWorker,
    type ResolvedSlicerExportTarget,
    type SlicerDocument,
    type SlicerConversionProgress,
    type SlicerConversionResult,
    type SlicerWorkerPoolState as WorkerPoolState
} from "@slicer-converter/wasm";

import { buildConversionStatusMessage } from "./conversion-status-message";
import { openConversionOutputTarget } from "./conversion-output-target";

type OpenedDocument = SlicerDocument;

/**
 * @zh `ConversionUseCaseResult` 接口契约。
 * @en Interface contract for `ConversionUseCaseResult`.
 */
export interface ConversionUseCaseResult {
    statusMessage: string;
}

/**
 * @zh `ConversionUseCaseInput` 接口契约。
 * @en Interface contract for `ConversionUseCaseInput`.
 */
export interface ConversionUseCaseInput {
    document: OpenedDocument;
    workerPool: WorkerPoolState<ManagedConversionWorker>;
    target: ResolvedSlicerExportTarget;
    outputFileName?: string;
    signal: AbortSignal;
    updateProgress(current: number, total: number, label: string): void;
    downloadBlob(input: { blob: Blob; downloadName: string }): void;
}

/**
 * @zh `runConversionUseCase` 导出函数。
 * @en Exported function `runConversionUseCase`.
 */
export async function runConversionUseCase(input: ConversionUseCaseInput): Promise<ConversionUseCaseResult> {
    const header = input.document.parsedHeader;
    const totalLayers = header.layers.length;

    const outputTarget = await openConversionOutputTarget({
        sourceFileName: input.outputFileName?.trim() || input.document.sourceFile.name,
        target: input.target
    });

    try {
        const conversionInput = {
            file: input.document.sourceFile,
            fileName: outputTarget.downloadName,
            sourceExtension: input.document.parsedHeader.format,
            target: {
                ...input.target,
                statusMessage: null
            },
            signal: input.signal,
            worker: { kind: "pool" as const, pool: input.workerPool },
            onProgress: (progress: SlicerConversionProgress) => updateProgress(input.updateProgress, progress)
        };
        if (outputTarget.mode === "streaming") {
            const result = await convertSlicerFileToSink({
                ...conversionInput,
                output: outputTarget.sink
            });
            return {
                statusMessage: buildStatusMessage(result, outputTarget.mode, totalLayers)
            };
        }

        const result = await convertSlicerFileToBlob(conversionInput);
        input.downloadBlob({ blob: result.blob, downloadName: outputTarget.downloadName });

        return {
            statusMessage: buildStatusMessage(result, outputTarget.mode, totalLayers)
        };
    } catch (error: unknown) {
        await outputTarget.sink.abort(error instanceof Error ? error.message : "Conversion failed");
        throw error;
    }
}

function updateProgress(
    update: (current: number, total: number, label: string) => void,
    progress: SlicerConversionProgress
): void {
    update(progress.current, progress.total, progress.label);
}

function buildStatusMessage(
    result: SlicerConversionResult,
    outputMode: "streaming" | "blob",
    totalLayers: number
): string {
    return buildConversionStatusMessage({
        totalLayers,
        targetFormat: result.targetFormat,
        blobSizeBytes: result.blob ? result.blob.size : null,
        totalElapsedMs: result.timings.totalMs,
        modeLabel: `workers${outputMode === "streaming" ? " (streamed)" : ""}`,
        setupMs: result.timings.setupMs,
        layerPhaseMs: result.timings.layerPhaseMs,
        computeMs: result.timings.computeMs,
        assembleMs: result.timings.assembleMs,
        finalizeOnlyMs: result.timings.finalizeOnlyMs
    });
}
