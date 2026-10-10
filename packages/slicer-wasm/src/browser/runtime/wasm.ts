import initWasm, {
    crc32_bytes as crc32BytesRaw,
    diagnose_export as diagnoseExportRaw,
    list_supported_formats,
    type InitInput
} from "../../../lib/slicer_converter.js";

import { buildFileMetadataJson } from "../format/header.js";
import type {
    ArchiveLayerEntryPlan,
    ArchiveMetadataEntryPlan,
    DecodedPreviewLayer,
    ExportDiagnosticReport,
    FileMetadata,
    SlicerFormatId,
    SupportedFormatCapabilities,
    SupportedFormatInfo
} from "../types.js";

/**
 * @zh WASM 模块的初始化输入类型，支持直接传入 URL、`ArrayBuffer`、
 * `WebAssembly.Module` 或其 Promise 包装版。
 * @en Initialisation input for the WASM module. Accepts a URL, `ArrayBuffer`,
 * `WebAssembly.Module`, or a `Promise` wrapping any of the above.
 */
export type SlicerConverterInitInput = InitInput | Promise<InitInput>;

/**
 * @zh WASM 加载进度事件描述对象。
 * @en Progress event descriptor for WASM loading.
 */
export interface WasmLoadProgress {
    phase: "downloading" | "compiling" | "ready";
    loadedBytes: number;
    totalBytes: number | null;
    percent: number | null;
}

/**
 * @zh WASM 初始化或调用失败时可保留的结构化错误字段。
 * @en Structured error fields preserved from WASM initialization or calls.
 */
export interface WasmErrorInfo {
    message: string;
    code?: string;
    detail?: string;
    retryable?: boolean;
}

export type {
    ArchiveLayerEntryPlan,
    ArchiveMetadataEntryPlan,
    DecodedPreviewLayer,
    ExportDiagnosticReport,
    SupportedFormatCapabilities,
    SupportedFormatInfo
};

const defaultWasmUrl = new URL("../../../lib/slicer_converter_bg.wasm", import.meta.url);
let initPromise: Promise<void> | null = null;

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
}

/**
 * @zh 从未知异常值中提取 WASM 错误消息。
 * @en Extracts a WASM error message from an unknown thrown value.
 */
export function getWasmErrorMessage(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    if (isRecord(error) && typeof error.message === "string") {
        return error.message;
    }

    return String(error);
}

/**
 * @zh 将未知 WASM 异常规范化为 `Error`，并保留可用的结构化字段。
 * @en Normalizes an unknown WASM failure into `Error` while preserving structured fields.
 */
export function toWasmError(error: unknown): Error & Partial<WasmErrorInfo> {
    if (error instanceof Error) {
        return error as Error & Partial<WasmErrorInfo>;
    }

    const normalized = new Error(getWasmErrorMessage(error)) as Error & Partial<WasmErrorInfo>;
    if (isRecord(error)) {
        if (typeof error.code === "string") {
            normalized.code = error.code;
        }
        if (typeof error.detail === "string") {
            normalized.detail = error.detail;
        }
        if (typeof error.retryable === "boolean") {
            normalized.retryable = error.retryable;
        }
    }

    return normalized;
}

/**
 * @zh 计算字节数组的标准 CRC-32。
 * @en Computes the standard CRC-32 for a byte array.
 */
export function crc32Bytes(data: Uint8Array): number {
    return crc32BytesRaw(data);
}

/**
 * @zh 读取 Rust 注册表导出的支持格式列表。
 * @en Reads supported format information exported by the Rust registry.
 */
export function getSupportedFormats(): SupportedFormatInfo[] {
    return JSON.parse(list_supported_formats()) as SupportedFormatInfo[];
}

/**
 * @zh 从 Rust format registry 诊断非致命导出问题。
 * @en Diagnoses non-fatal export issues from the Rust format registry.
 */
export function diagnoseExport(input: {
    metadata: FileMetadata;
    targetFormat: SlicerFormatId;
}): ExportDiagnosticReport {
    return JSON.parse(
        diagnoseExportRaw(buildFileMetadataJson(input.metadata), input.targetFormat)
    ) as ExportDiagnosticReport;
}

/**
 * @zh 返回 WASM 二进制文件的默认 URL。
 * @en Returns the default URL for the WASM binary.
 *
 * @zh 该 URL 使用 `import.meta.url` 计算，因此可靠性依赖模块就近的文件路径。
 * @en Computed from `import.meta.url`, so it reliably resolves relative to the
 * module's own location regardless of the application's base URL.
 */
export function getSlicerConverterWasmUrl(): URL {
    return defaultWasmUrl;
}

/**
 * @internal
 * @zh 确保底层 WASM 模块已初始化；不返回 public API facade。
 * @en Ensures the underlying WASM module is initialised; it does not return the public API facade.
 */
export async function initSlicerConverterWasm(wasmUrl: SlicerConverterInitInput = defaultWasmUrl): Promise<void> {
    if (!initPromise) {
        initPromise = loadWasmModule(wasmUrl)
            .then((moduleOrPath) => initWasm({ module_or_path: moduleOrPath }))
            .then(() => undefined)
            .catch((error: unknown) => {
                initPromise = null;
                throw toWasmError(error);
            });
    }

    await initPromise;
}

/**
 * @internal
 * @zh 以流式方式加载并初始化底层 WASM 模块；不返回 public API facade。
 * @en Streams and initialises the underlying WASM module; it does not return the public API facade.
 */
export async function initSlicerConverterWasmWithProgress(
    onProgress: (progress: WasmLoadProgress) => void,
    wasmUrl: URL = defaultWasmUrl
): Promise<void> {
    if (!initPromise) {
        initPromise = streamWasmBytes(wasmUrl, onProgress)
            .then((wasmBytes) => initWasm({ module_or_path: wasmBytes }))
            .then(() => {
                onProgress({ phase: "ready", loadedBytes: 0, totalBytes: null, percent: 100 });
            })
            .catch((error: unknown) => {
                initPromise = null;
                throw toWasmError(error);
            });
    }

    await initPromise;
}

async function streamWasmBytes(wasmUrl: URL, onProgress: (progress: WasmLoadProgress) => void): Promise<Uint8Array> {
    if (isNodeFileUrl(wasmUrl)) {
        const bytes = await readNodeFileUrl(wasmUrl);
        onProgress({
            phase: "compiling",
            loadedBytes: bytes.byteLength,
            totalBytes: bytes.byteLength,
            percent: 100
        });
        return bytes;
    }

    const response = await fetch(wasmUrl);
    if (!response.ok) {
        throw new Error(`Failed to fetch WASM: HTTP ${response.status}`);
    }

    const totalBytesHeader = response.headers.get("content-length");
    const totalBytes = totalBytesHeader ? Number(totalBytesHeader) : null;
    onProgress({ phase: "downloading", loadedBytes: 0, totalBytes, percent: totalBytes ? 0 : null });

    if (!response.body) {
        const bytes = new Uint8Array(await response.arrayBuffer());
        onProgress({
            phase: "compiling",
            loadedBytes: bytes.byteLength,
            totalBytes: bytes.byteLength,
            percent: 100
        });
        return bytes;
    }

    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let loadedBytes = 0;
    while (true) {
        const { done, value } = await reader.read();
        if (done) {
            break;
        }

        chunks.push(value);
        loadedBytes += value.byteLength;
        onProgress({
            phase: "downloading",
            loadedBytes,
            totalBytes,
            percent: totalBytes ? Math.min(100, Math.round((loadedBytes / totalBytes) * 100)) : null
        });
    }

    const result = new Uint8Array(loadedBytes);
    let offset = 0;
    for (const chunk of chunks) {
        result.set(chunk, offset);
        offset += chunk.byteLength;
    }

    onProgress({
        phase: "compiling",
        loadedBytes,
        totalBytes: totalBytes ?? loadedBytes,
        percent: 100
    });

    return result;
}

async function loadWasmModule(input: SlicerConverterInitInput): Promise<InitInput> {
    const resolved = await input;
    if (isNodeFileUrl(resolved)) {
        return readNodeFileUrl(resolved);
    }

    return resolved;
}

function isNodeFileUrl(value: unknown): value is URL {
    return isNodeRuntime() && value instanceof URL && value.protocol === "file:";
}

function isNodeRuntime(): boolean {
    const runtime = globalThis as typeof globalThis & {
        process?: { versions?: { node?: string } };
    };

    return typeof runtime.process?.versions?.node === "string";
}

async function readNodeFileUrl(wasmUrl: URL): Promise<Uint8Array> {
    const { readFile } = await import("node:fs/promises");
    const { fileURLToPath } = await import("node:url");

    return new Uint8Array(await readFile(fileURLToPath(wasmUrl)));
}
