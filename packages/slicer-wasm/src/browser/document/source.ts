import { createSlicerDocument, type SlicerDocument, type SlicerLayerSource } from "./document.js";
import { getDocumentOpenerForFormat } from "../format/browser-format-behaviors.js";
import { getFormatFacts } from "../format/facts.js";
import type { ParsedHeader } from "../types.js";

/**
 * @zh 浏览器 File 文档打开输入；从 `file.name` 推导 extension。
 * @en Browser File document-opening input; derives the extension from `file.name`.
 */
export interface SlicerDocumentFileInput {
    readonly file: File;
    parseHeader(data: Uint8Array, extension: string): ParsedHeader;
}

/**
 * @zh 浏览器文档 opener 返回的解析结果和 layer source。
 * @en Parsed header and layer source returned by a browser document opener.
 */
export interface BrowserDocumentSource {
    readonly parsedHeader: ParsedHeader;
    readonly layerSource: SlicerLayerSource;
}

/**
 * @zh 打开浏览器 `File` 并封装成 package 公开的统一 slicer document 读取边界。
 * @en Opens a browser `File` and wraps it in the package public slicer document read boundary.
 */
export async function openSlicerDocument(input: {
    file: File;
    extension: string;
    parseHeader(data: Uint8Array, extension: string): ParsedHeader;
}): Promise<SlicerDocument> {
    const source = await loadSlicerDocumentSource(input);

    return createSlicerDocument({
        sourceFile: input.file,
        parsedHeader: source.parsedHeader,
        layerSource: source.layerSource
    });
}

/**
 * @zh 打开浏览器 `File`，并从文件名推导 source extension。
 * @en Opens a browser `File` and derives the source extension from the file name.
 */
export function openSlicerDocumentFile(input: SlicerDocumentFileInput): Promise<SlicerDocument> {
    return openSlicerDocument({
        file: input.file,
        extension: getFileExtension(input.file.name),
        parseHeader: input.parseHeader
    });
}

/**
 * @zh 选择格式专属 opener；没有专属 opener 时回退到 WASM header retry parser。
 * @en Selects a format-specific opener, falling back to the WASM header retry parser when none is registered.
 */
export async function loadSlicerDocumentSource(input: {
    file: File;
    extension: string;
    parseHeader(data: Uint8Array, extension: string): ParsedHeader;
}): Promise<BrowserDocumentSource> {
    const documentOpener = getDocumentOpenerForFormat(input.extension);
    if (documentOpener) {
        return documentOpener.open(input.file);
    }

    return parseDocumentSourceWithWasmRetry(input);
}

/**
 * @zh 加载浏览器 `File` source，并从文件名推导 source extension。
 * @en Loads a browser `File` source and derives the source extension from the file name.
 */
export function loadSlicerDocumentFileSource(input: SlicerDocumentFileInput): Promise<BrowserDocumentSource> {
    return loadSlicerDocumentSource({
        file: input.file,
        extension: getFileExtension(input.file.name),
        parseHeader: input.parseHeader
    });
}

const MAX_HEADER_READ_BYTES = 192 * 1024 * 1024;

async function parseDocumentSourceWithWasmRetry(input: {
    file: File;
    extension: string;
    parseHeader(data: Uint8Array, extension: string): ParsedHeader;
}): Promise<BrowserDocumentSource> {
    if (requiresFullDocumentParse(input.extension)) {
        return {
            parsedHeader: input.parseHeader(new Uint8Array(await input.file.arrayBuffer()), input.extension),
            layerSource: { kind: "raw-file" }
        };
    }

    let headerSize = Math.min(input.file.size, 512 * 1024);

    while (true) {
        const headerBuffer = new Uint8Array(await input.file.slice(0, headerSize).arrayBuffer());

        try {
            const header = input.parseHeader(headerBuffer, input.extension);
            const expectedLayers = header.metadata.layer_count || 0;
            if (header.layers.length < expectedLayers && headerSize < input.file.size) {
                const nextSize = getNextHeaderReadSize(input.file.size, headerSize);
                if (nextSize) {
                    headerSize = nextSize;
                    continue;
                }
            }

            return {
                parsedHeader: header,
                layerSource: { kind: "raw-file" }
            };
        } catch (error: unknown) {
            const requiredBytes = getRequiredBytesFromParseError(error);
            if (requiredBytes && headerSize < input.file.size) {
                const nextSize = getNextHeaderReadSize(input.file.size, headerSize, requiredBytes);
                if (nextSize) {
                    headerSize = nextSize;
                    continue;
                }
            }

            throw error;
        }
    }
}

function requiresFullDocumentParse(extension: string): boolean {
    try {
        return getFormatFacts(extension).capabilities.requiresFullDocumentParse;
    } catch {
        return false;
    }
}

function getNextHeaderReadSize(fileSize: number, currentSize: number, requiredBytes?: number): number | null {
    const maxReadableSize = Math.min(fileSize, MAX_HEADER_READ_BYTES);
    if (currentSize >= maxReadableSize) {
        return null;
    }

    const doubledSize = Math.min(currentSize * 2, maxReadableSize);
    const requiredSize = requiredBytes ? Math.min(requiredBytes + 64 * 1024, maxReadableSize) : 0;
    const nextSize = Math.max(doubledSize, requiredSize, 4 * 1024 * 1024);
    return nextSize > currentSize ? nextSize : null;
}

function getRequiredBytesFromParseError(error: unknown): number | null {
    const message = String(error instanceof Error ? error.message : (error ?? ""));
    const match = message.match(/Buffer too small: need (\d+) bytes, got (\d+)/i);
    return match ? Number.parseInt(match[1], 10) : null;
}

function getFileExtension(fileName: string): string {
    const separator = fileName.lastIndexOf(".");
    if (separator <= 0 || separator === fileName.length - 1) {
        return "";
    }

    return fileName.slice(separator + 1).toLowerCase();
}
