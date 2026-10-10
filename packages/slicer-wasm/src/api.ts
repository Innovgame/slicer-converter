import { convertSlicerFile } from "./browser/conversion/pipeline.js";
import {
    listExportTargetOptions,
    resolveSlicerExportTarget,
    selectDefaultExportTarget
} from "./browser/conversion/target.js";
import {
    findSlicerExportTargetMatrixSelection,
    listSlicerExportTargetMatrix
} from "./browser/conversion/target-matrix.js";
import {
    loadSlicerDocumentFileSource,
    loadSlicerDocumentSource,
    openSlicerDocument,
    openSlicerDocumentFile
} from "./browser/document/source.js";
import {
    crc32Bytes,
    diagnoseExport,
    getSupportedFormats,
    initSlicerConverterWasm,
    initSlicerConverterWasmWithProgress
} from "./browser/runtime/wasm.js";
import type {
    ArchiveLayerEntryPlan,
    ArchiveMetadataEntryPlan,
    DecodedPreviewLayer,
    ExportDiagnosticReport,
    SlicerConverterInitInput,
    SupportedFormatCapabilities,
    SupportedFormatInfo,
    WasmLoadProgress
} from "./browser/runtime/wasm.js";
export {
    crc32Bytes,
    diagnoseExport,
    getSlicerConverterWasmUrl,
    getSupportedFormats,
    getWasmErrorMessage,
    toWasmError
} from "./browser/runtime/wasm.js";
export type { SlicerConverterInitInput, WasmErrorInfo, WasmLoadProgress } from "./browser/runtime/wasm.js";

/**
 * @zh 浏览器包公开的 WASM facade 类型集合；低层 raw 组装函数由 typed helper 封装。
 * @en Public WASM facade exposed by the browser package; low-level raw assembly
 * functions are wrapped by typed helpers.
 *
 * @zh 可通过 {@link initSlicerConverter} 或 {@link initSlicerConverterWithProgress} 获取实例。
 * @en Obtain an instance via {@link initSlicerConverter} or
 * {@link initSlicerConverterWithProgress}.
 */
export interface SlicerConverterApi {
    convertSlicerFile: typeof convertSlicerFile;
    crc32Bytes: typeof crc32Bytes;
    diagnoseExport: typeof diagnoseExport;
    getSupportedFormats: typeof getSupportedFormats;
    findSlicerExportTargetMatrixSelection: typeof findSlicerExportTargetMatrixSelection;
    listExportTargetOptions: typeof listExportTargetOptions;
    listSlicerExportTargetMatrix: typeof listSlicerExportTargetMatrix;
    loadSlicerDocumentFileSource: typeof loadSlicerDocumentFileSource;
    loadSlicerDocumentSource: typeof loadSlicerDocumentSource;
    openSlicerDocument: typeof openSlicerDocument;
    openSlicerDocumentFile: typeof openSlicerDocumentFile;
    resolveSlicerExportTarget: typeof resolveSlicerExportTarget;
    selectDefaultExportTarget: typeof selectDefaultExportTarget;
}

export type {
    ArchiveLayerEntryPlan,
    ArchiveMetadataEntryPlan,
    DecodedPreviewLayer,
    ExportDiagnosticReport,
    SupportedFormatCapabilities,
    SupportedFormatInfo
};

const slicerConverterApi: SlicerConverterApi = {
    convertSlicerFile,
    crc32Bytes,
    diagnoseExport,
    getSupportedFormats,
    findSlicerExportTargetMatrixSelection,
    listExportTargetOptions,
    listSlicerExportTargetMatrix,
    loadSlicerDocumentFileSource,
    loadSlicerDocumentSource,
    openSlicerDocument,
    openSlicerDocumentFile,
    resolveSlicerExportTarget,
    selectDefaultExportTarget
};

/**
 * @zh 初始化 Slicer Converter WASM 模块。
 * @en Initialises the Slicer Converter WASM module.
 *
 * @zh 多次调用安全—— WASM 只会加载一次。不需要下载进度时使用此函数。
 * @en Safe to call multiple times — WASM is loaded exactly once. Use this
 * function when download progress is not required.
 *
 * @param wasmUrl - @zh 可选的自定义 WASM 载入输入；默认使用内置路径
 *   @en Optional custom WASM init input; defaults to the bundled path
 * @returns @zh 已初始化的 {@link SlicerConverterApi} @en The initialised {@link SlicerConverterApi}
 */
export async function initSlicerConverter(wasmUrl?: SlicerConverterInitInput): Promise<SlicerConverterApi> {
    await initSlicerConverterWasm(wasmUrl);
    return slicerConverterApi;
}

/**
 * @zh 以流式方式加载 WASM 并通过回调上报进度。
 * @en Loads WASM by streaming the bytes and reports progress via callback.
 *
 * @zh 多次调用安全—— WASM 只会加载一次。当需要展示加载进度条时使用此函数。
 * @en Safe to call multiple times — WASM is loaded exactly once. Use this
 * function when you want to display a loading progress bar.
 *
 * @param onProgress - @zh 每当下载/编译进度变化时触发的回调
 *   @en Callback fired whenever download or compilation progress changes
 * @param wasmUrl - @zh 可选的自定义 WASM URL；默认使用内置路径
 *   @en Optional custom WASM URL; defaults to the bundled path
 * @returns @zh 已初始化的 {@link SlicerConverterApi} @en The initialised {@link SlicerConverterApi}
 */
export async function initSlicerConverterWithProgress(
    onProgress: (progress: WasmLoadProgress) => void,
    wasmUrl?: URL
): Promise<SlicerConverterApi> {
    await initSlicerConverterWasmWithProgress(onProgress, wasmUrl);
    return slicerConverterApi;
}
