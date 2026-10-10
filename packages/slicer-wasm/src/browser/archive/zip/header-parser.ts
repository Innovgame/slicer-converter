/**
 * @zh ZIP-based 切片格式的 header 解析入口，格式专属规则下沉到 `formats/*`。
 * @en Header parsing entry for ZIP-based slicer formats; format-specific rules live under `formats/*`.
 */

import type { CompressedZipLayerPayload } from "../../document/document.js";
import type { BrowserDocumentSource } from "../../document/source.js";
import type { FileMetadata, LayerOffset } from "../../types.js";
import { anycubicZipFormat } from "./formats/anycubic-zip.js";
import { chituboxZipFormat } from "./formats/chitubox-zip.js";
import { sl1ZipFormat } from "./formats/sl1.js";
import type { ZipFormatParser } from "./formats/shared.js";
import { vdtZipFormat } from "./formats/vdt.js";
import type { ZipEntryInfo } from "./directory.js";
import { createFileZipSource, parseZipDirectoryFromReader, type ZipSource } from "./source.js";

/**
 * @zh 从 SL1 ZIP 文件中解析 header metadata 和 layer entry table。
 * @en Parses header metadata and layer entry table from an SL1 ZIP file.
 */
export async function parseSl1File(file: File): Promise<BrowserDocumentSource> {
    return parseZipFormatFile(file, sl1ZipFormat);
}

/**
 * @zh 从 VDT ZIP 文件中解析 header metadata 和 layer entry table。
 * @en Parses header metadata and layer entry table from a VDT ZIP file.
 */
export async function parseVdtFile(file: File): Promise<BrowserDocumentSource> {
    return parseZipFormatFile(file, vdtZipFormat);
}

/**
 * @zh 从 Chitubox ZIP 文件中解析 run.gcode metadata 和 layer entry table。
 * @en Parses run.gcode metadata and layer entry table from a Chitubox ZIP file.
 */
export async function parseChituboxZipFile(file: File): Promise<BrowserDocumentSource> {
    return parseZipFormatFile(file, chituboxZipFormat);
}

/**
 * @zh 从 Anycubic ZIP 文件中解析 manifest metadata 和 layer entry table。
 * @en Parses manifest metadata and layer entry table from an Anycubic ZIP file.
 */
export async function parseAnycubicZipFile(file: File): Promise<BrowserDocumentSource> {
    return parseZipFormatFile(file, anycubicZipFormat);
}

async function parseZipFormatFile(file: File, parser: ZipFormatParser): Promise<BrowserDocumentSource> {
    const zipSource = createFileZipSource(file);
    const zipDir = await parseZipDirectoryFromReader(zipSource, file.size);
    const zipEntries = listZipLayerEntries(parser, zipDir.entries);
    const metadata = await parser.parseMetadata(zipSource, zipDir.entries, zipEntries.length);

    return createZipDocumentSource(parser.format, metadata, zipEntries, zipSource);
}

function createZipDocumentSource(
    format: ZipFormatParser["format"],
    metadata: FileMetadata,
    zipEntries: ZipEntryInfo[],
    zipSource: ZipSource
): BrowserDocumentSource {
    return {
        parsedHeader: {
            format,
            metadata,
            layers: zipEntries.map(
                (entry, index): LayerOffset => ({
                    offset: index,
                    size: entry.uncompressedSize
                })
            )
        },
        layerSource: { kind: "zip", readCompressedLayerPayload: createZipLayerPayloadReader(zipEntries, zipSource) }
    };
}

function createZipLayerPayloadReader(
    zipEntries: ZipEntryInfo[],
    zipSource: ZipSource
): (index: number) => Promise<CompressedZipLayerPayload> {
    return async (index) => {
        const entry = zipEntries[index];
        if (!entry) {
            throw new Error(`ZIP entry for layer ${index} not found`);
        }

        return {
            bytes: await zipSource.readCompressedEntryPayload(entry.localHeaderOffset, entry.compressedSize),
            compressionMethod: entry.compressionMethod,
            uncompressedSize: entry.uncompressedSize
        };
    };
}

function listZipLayerEntries(parser: ZipFormatParser, entries: readonly ZipEntryInfo[]): ZipEntryInfo[] {
    return entries
        .map((entry) => ({
            entry,
            layerIndex: parser.readLayerIndex(entry.name)
        }))
        .filter((entry): entry is { entry: ZipEntryInfo; layerIndex: number } => entry.layerIndex !== undefined)
        .sort((left, right) => left.layerIndex - right.layerIndex)
        .map(({ entry }) => entry);
}
