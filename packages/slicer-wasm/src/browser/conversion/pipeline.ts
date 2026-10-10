import { diagnoseExport, initSlicerConverterWasm } from "../runtime/wasm.js";
import { buildSlicerOutputFilename } from "./filename.js";
import { resolveSlicerExportTarget, type ExportTargetFormat, type ResolvedSlicerExportTarget } from "./target.js";
import { openSlicerDocument, openSlicerDocumentFile } from "../document/source.js";
import { parseBrowserFileHeader } from "../format/header.js";
import { createBlobOutputSink, type PatchableSlicerOutputSink } from "../output/sink.js";
import { initSlicerWorkerPool, type SlicerWorkerPoolState } from "../worker/pool.js";
import { waitForSlicerWorkerReady } from "../worker/pool.js";
import { runSlicerConversionJob, type SlicerConversionJobResult, type SlicerConversionTimings } from "./job.js";
import type { SlicerDocument } from "../document/document.js";
import type { ExportDiagnosticWarning, FileMetadata, SlicerFormatId } from "../types.js";
import type { ManagedSlicerWorker } from "../worker/dispatch.js";

/**
 * @zh 高层 browser conversion API 的输入。
 * @en Input for the high-level browser conversion API.
 */
export interface ConvertSlicerFileInput {
    readonly file: File | Blob;
    readonly fileName?: string;
    readonly sourceExtension?: string;
    readonly target: SlicerFormatId | ExportTargetFormat | ResolvedSlicerExportTarget;
    readonly workerCount?: number;
    readonly signal?: AbortSignal;
    readonly output?: PatchableSlicerOutputSink;
    readonly workerPool?: SlicerWorkerPoolState<ManagedSlicerWorker>;
    readonly workerUrl?: URL;
    readonly createWorker?: () => ManagedSlicerWorker;
    readonly onProgress?: (progress: SlicerConversionProgress) => void;
}

/**
 * @zh 高层转换进度。
 * @en High-level conversion progress.
 */
export interface SlicerConversionProgress {
    readonly phase: "opening" | "converting-layers" | "assembling" | "complete";
    readonly current: number;
    readonly total: number;
    readonly label: string;
}

/**
 * @zh 高层转换结果。
 * @en High-level conversion result.
 */
export interface SlicerConversionResult {
    readonly blob: Blob | null;
    readonly fileName: string;
    readonly sourceFormat: SlicerFormatId;
    readonly targetFormat: SlicerFormatId;
    readonly metadata: FileMetadata;
    readonly warnings: ExportDiagnosticWarning[];
    readonly timings: SlicerConversionTimings;
}

/**
 * @zh DX conversion facade 的 worker 策略。
 * @en Worker strategy for the DX conversion facades.
 */
export type SlicerDxWorkerStrategy =
    | {
          readonly kind: "auto";
          readonly count?: number;
          readonly workerUrl?: URL;
          readonly createWorker?: () => ManagedSlicerWorker;
      }
    | {
          readonly kind: "pool";
          readonly pool: SlicerWorkerPoolState<ManagedSlicerWorker>;
      };

/**
 * @zh DX conversion facade 的共享输入。
 * @en Shared input for DX conversion facades.
 */
export interface SlicerDxConversionBaseInput {
    readonly file: File | Blob;
    readonly fileName?: string;
    readonly sourceExtension?: string;
    readonly target: SlicerFormatId | ExportTargetFormat | ResolvedSlicerExportTarget;
    readonly signal?: AbortSignal;
    readonly onProgress?: (progress: SlicerConversionProgress) => void;
    readonly worker?: SlicerDxWorkerStrategy;
}

/**
 * @zh 转换到 Blob 的 DX facade 输入。
 * @en DX facade input for converting to a Blob.
 */
export interface ConvertSlicerFileToBlobInput extends SlicerDxConversionBaseInput {
    readonly output?: never;
}

/**
 * @zh 转换到调用方 output sink 的 DX facade 输入。
 * @en DX facade input for converting to a caller-provided output sink.
 */
export interface ConvertSlicerFileToSinkInput extends SlicerDxConversionBaseInput {
    readonly output: PatchableSlicerOutputSink;
}

/**
 * @zh 始终包含 Blob 的 DX conversion result。
 * @en DX conversion result that always contains a Blob.
 */
export interface SlicerBlobConversionResult extends Omit<SlicerConversionResult, "blob"> {
    readonly blob: Blob;
}

/**
 * @zh 写入外部 sink 后不返回 Blob 的 DX conversion result。
 * @en DX conversion result for caller-provided sinks; it does not return a Blob.
 */
export interface SlicerSinkConversionResult extends Omit<SlicerConversionResult, "blob"> {
    readonly blob: null;
}

/**
 * @internal
 * @zh `convertSlicerFileWithDeps` 的依赖注入契约，用于测试和自定义 runtime 适配。
 * @en Dependency contract for `convertSlicerFileWithDeps`, used by tests and custom runtime adapters.
 */
export interface SlicerConversionDeps {
    init(): Promise<unknown>;
    openDocument(input: { file: File; extension?: string; fileName?: string }): Promise<SlicerDocument>;
    createDefaultSink(): PatchableSlicerOutputSink;
    createWorkerPool(input: {
        readonly count: number;
        readonly workerUrl?: URL;
        readonly createWorker?: () => ManagedSlicerWorker;
    }): Promise<SlicerWorkerPoolState<ManagedSlicerWorker>> | SlicerWorkerPoolState<ManagedSlicerWorker>;
    diagnoseExport(input: { metadata: FileMetadata; targetFormat: SlicerFormatId }): {
        warnings: ExportDiagnosticWarning[];
    };
    runJob(input: {
        readonly document: SlicerDocument;
        readonly targetFormat: SlicerFormatId;
        readonly targetAnycubicVersion?: number;
        readonly outputFileName: string;
        readonly sink: PatchableSlicerOutputSink;
        readonly workers: ManagedSlicerWorker[];
        readonly signal: AbortSignal;
        readonly updateProgress: (current: number, total: number, label: string) => void;
    }): Promise<SlicerConversionJobResult>;
}

/**
 * @zh 转换浏览器 `File | Blob`，默认返回内存 Blob。
 * @en Converts a browser `File | Blob`, returning an in-memory Blob by default.
 */
export function convertSlicerFile(input: ConvertSlicerFileInput): Promise<SlicerConversionResult> {
    return convertSlicerFileWithDeps(input, defaultSlicerConversionDeps);
}

/**
 * @zh 转换浏览器 `File | Blob`，并始终返回内存 Blob。
 * @en Converts a browser `File | Blob` and always returns an in-memory Blob.
 */
export function convertSlicerFileToBlob(input: ConvertSlicerFileToBlobInput): Promise<SlicerBlobConversionResult> {
    return convertSlicerFileToBlobWithDeps(input, defaultSlicerConversionDeps);
}

/**
 * @zh 转换浏览器 `File | Blob`，并写入调用方提供的 output sink。
 * @en Converts a browser `File | Blob` into a caller-provided output sink.
 */
export function convertSlicerFileToSink(input: ConvertSlicerFileToSinkInput): Promise<SlicerSinkConversionResult> {
    return convertSlicerFileToSinkWithDeps(input, defaultSlicerConversionDeps);
}

/**
 * @internal
 * @zh 带依赖注入的 Blob DX facade。
 * @en Dependency-injected Blob DX facade.
 */
export async function convertSlicerFileToBlobWithDeps(
    input: ConvertSlicerFileToBlobInput,
    deps: SlicerConversionDeps
): Promise<SlicerBlobConversionResult> {
    const result = await convertSlicerFileWithDeps(toConvertSlicerFileInput(input), deps);
    if (!result.blob) {
        throw new Error("Blob conversion completed without a Blob output");
    }
    return { ...result, blob: result.blob };
}

/**
 * @internal
 * @zh 带依赖注入的 output sink DX facade。
 * @en Dependency-injected output sink DX facade.
 */
export async function convertSlicerFileToSinkWithDeps(
    input: ConvertSlicerFileToSinkInput,
    deps: SlicerConversionDeps
): Promise<SlicerSinkConversionResult> {
    const result = await convertSlicerFileWithDeps(toConvertSlicerFileInput(input), deps);
    return { ...result, blob: null };
}

/**
 * @internal
 * @zh 带依赖注入的高层转换入口。
 * @en Dependency-injected high-level conversion entry.
 */
export async function convertSlicerFileWithDeps(
    input: ConvertSlicerFileInput,
    deps: SlicerConversionDeps
): Promise<SlicerConversionResult> {
    const signal = input.signal ?? new AbortController().signal;
    reportProgress(input, { phase: "opening", current: 0, total: 0, label: "Opening file..." });
    await deps.init();

    const sourceFile = toNamedFile(input.file, input.fileName);
    const document = await deps.openDocument({
        file: sourceFile,
        extension: input.sourceExtension,
        fileName: input.fileName
    });
    const sink = input.output ?? deps.createDefaultSink();
    let pool: SlicerWorkerPoolState<ManagedSlicerWorker> | null = null;

    try {
        const target = resolveTargetInput(input.target, document.parsedHeader.format);
        const outputFileName = buildSlicerOutputFilename({
            sourceFileName: document.sourceName,
            requestedTargetFormat: target.requestedTargetFormat,
            resolvedExtension: target.resolvedExtension
        });
        const diagnostics = deps.diagnoseExport({
            metadata: document.parsedHeader.metadata,
            targetFormat: target.resolvedTargetFormat
        });

        pool =
            input.workerPool ??
            (await deps.createWorkerPool({
                count: input.workerCount ?? navigator.hardwareConcurrency ?? 1,
                workerUrl: input.workerUrl,
                createWorker: input.createWorker
            }));
        const workers = await pool.ensureWorkers(input.workerCount);
        pool.pauseIdleTermination();
        const job = await deps.runJob({
            document,
            targetFormat: target.resolvedTargetFormat,
            targetAnycubicVersion: target.resolvedAnycubicVersion,
            outputFileName,
            sink,
            workers,
            signal,
            updateProgress(current, total, label) {
                reportProgress(input, {
                    phase:
                        label.toLowerCase().includes("assembling") || label.toLowerCase().includes("finalizing")
                            ? "assembling"
                            : "converting-layers",
                    current,
                    total,
                    label
                });
            }
        });

        reportProgress(input, {
            phase: "complete",
            current: document.parsedHeader.layers.length,
            total: document.parsedHeader.layers.length,
            label: "Conversion complete."
        });

        return {
            blob: job.outputBlob,
            fileName: outputFileName,
            sourceFormat: document.parsedHeader.format,
            targetFormat: target.resolvedTargetFormat,
            metadata: document.parsedHeader.metadata,
            warnings: diagnostics.warnings,
            timings: job.timings
        };
    } catch (error: unknown) {
        await sink.abort(error instanceof Error ? error.message : "Conversion failed");
        throw error;
    } finally {
        pool?.scheduleIdleTermination();
        await document.close();
    }
}

const defaultSlicerConversionDeps: SlicerConversionDeps = {
    init: initSlicerConverterWasm,
    openDocument(input) {
        if (!input.extension) {
            return openSlicerDocumentFile({
                file: input.file,
                parseHeader: parseBrowserFileHeader
            });
        }

        return openSlicerDocument({
            file: input.file,
            extension: input.extension,
            parseHeader: parseBrowserFileHeader
        });
    },
    createDefaultSink: createBlobOutputSink,
    createWorkerPool(input) {
        return initSlicerWorkerPool({
            count: input.count,
            minWorkers: Math.min(1, input.count),
            maxWorkers: input.count,
            createWorker: input.createWorker ?? (() => createDefaultSlicerWorker(input.workerUrl)),
            waitForReady: waitForSlicerWorkerReady
        });
    },
    diagnoseExport,
    runJob: runSlicerConversionJob
};

function resolveTargetInput(
    target: ConvertSlicerFileInput["target"],
    sourceFormat: SlicerFormatId
): ResolvedSlicerExportTarget {
    if (typeof target === "object") {
        return target;
    }
    return resolveSlicerExportTarget({
        sourceFormat,
        selectedTarget: target
    });
}

function reportProgress(input: ConvertSlicerFileInput, progress: SlicerConversionProgress): void {
    input.onProgress?.(progress);
}

function toConvertSlicerFileInput(
    input: ConvertSlicerFileToBlobInput | ConvertSlicerFileToSinkInput
): ConvertSlicerFileInput {
    const workerInput = toWorkerInput(input.worker);
    return {
        file: input.file,
        fileName: input.fileName,
        sourceExtension: input.sourceExtension,
        target: input.target,
        signal: input.signal,
        output: "output" in input ? input.output : undefined,
        onProgress: input.onProgress,
        ...workerInput
    };
}

function toWorkerInput(
    worker?: SlicerDxWorkerStrategy
): Pick<ConvertSlicerFileInput, "workerCount" | "workerPool" | "workerUrl" | "createWorker"> {
    if (!worker || worker.kind === "auto") {
        return {
            workerCount: worker?.count,
            workerUrl: worker?.workerUrl,
            createWorker: worker?.createWorker
        };
    }

    return {
        workerPool: worker.pool
    };
}

function toNamedFile(file: File | Blob, fileName?: string): File {
    if (file instanceof File && !fileName) {
        return file;
    }
    return new File([file], fileName ?? (file instanceof File ? file.name : "input.slicer"));
}

function createDefaultSlicerWorker(workerUrl?: URL): ManagedSlicerWorker {
    if (!workerUrl) {
        throw new Error("convertSlicerFile requires workerUrl or createWorker when no workerPool is provided");
    }
    return Object.assign(new Worker(workerUrl, { type: "module" }), { busy: false });
}
