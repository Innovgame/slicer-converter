import {
    build_cxdlpv4_document_header as buildCxdLpV4DocumentHeaderRaw,
    build_cxdlpv4_layer_def as buildCxdLpV4LayerDefRaw,
    build_document_from_layers as buildDocumentFromLayersRaw,
    build_footer as buildFooterRaw,
    build_header as buildHeaderRaw,
    cxdlp_crc32_update as cxdLpCrc32UpdateRaw,
    finalize_header as finalizeHeaderRaw
} from "../../../lib/slicer_converter.js";

import type { SlicerFormatId } from "../types.js";

/**
 * @zh 支持流式文档组装的格式专用 helper 集合。
 * @en Format-specific helper set for streaming document assembly.
 */
export interface StreamingDocumentAssembler {
    buildDocument?(
        metadataJson: string,
        targetFormat: SlicerFormatId,
        targetAnycubicVersion: number | undefined,
        layers: Uint8Array[]
    ): Uint8Array;
    buildPrefix?(metadataJson: string, targetFormat: SlicerFormatId, layerSizes: Uint32Array): Uint8Array[];
    checksumUpdate?(previous: number, chunk: Uint8Array): number;
    buildChecksumTrailer?(checksum: number): Uint8Array;
}

/**
 * @zh 非 ZIP 二进制文档组装所需的 WASM helper 集合。
 * @en WASM helper set needed by non-ZIP binary document assembly.
 */
export interface StreamingDocumentAssemblyHelpers {
    buildHeader(metadataJson: string, targetFormat: SlicerFormatId, targetAnycubicVersion?: number): Uint8Array;
    finalizeHeader(
        headerBytes: Uint8Array,
        targetFormat: SlicerFormatId,
        layerSizes: Uint32Array,
        targetAnycubicVersion?: number
    ): Uint8Array;
    buildFooter(targetFormat: SlicerFormatId): Uint8Array;
    streamingDocument?: StreamingDocumentAssembler;
}

/**
 * @zh 为目标格式创建二进制文档组装 helper。
 * @en Creates binary document assembly helpers for the target format.
 */
export function createStreamingDocumentAssemblyHelpers(targetFormat: SlicerFormatId): StreamingDocumentAssemblyHelpers {
    return {
        buildHeader: (metadataJson, format, targetAnycubicVersion) =>
            buildHeaderRaw(metadataJson, format, targetAnycubicVersion),
        finalizeHeader: (headerBytes, format, layerSizes, targetAnycubicVersion) =>
            finalizeHeaderRaw(headerBytes, format, layerSizes, targetAnycubicVersion),
        buildFooter: (format) => buildFooterRaw(format),
        streamingDocument: isBufferedDocumentTarget(targetFormat)
            ? {
                  buildDocument: (metadataJson, format, targetAnycubicVersion, layers) =>
                      buildDocumentFromLayersRaw(metadataJson, format, layers, targetAnycubicVersion)
              }
            : targetFormat === "cxdlpv4"
              ? {
                    buildPrefix: buildCxdLpV4Prefix,
                    checksumUpdate: (previous, chunk) => cxdLpCrc32UpdateRaw(previous, chunk),
                    buildChecksumTrailer: u32ToBigEndian
                }
              : undefined
    };
}

function isBufferedDocumentTarget(targetFormat: SlicerFormatId): boolean {
    return targetFormat === "encrypted-ctb" || targetFormat === "encrypted-ctb-v4";
}

function buildCxdLpV4Prefix(metadataJson: string, targetFormat: SlicerFormatId, layerSizes: Uint32Array): Uint8Array[] {
    const header = buildCxdLpV4DocumentHeaderRaw(metadataJson, targetFormat, layerSizes);
    const layerTableBytes = CXDLP_V4_LAYER_DEF_SIZE * layerSizes.length;
    const prefixSize = header.byteLength + layerTableBytes;
    const chunks = [header];
    let dataAddress = prefixSize;

    for (let index = 0; index < layerSizes.length; index += 1) {
        const dataSize = layerSizes[index];
        chunks.push(buildCxdLpV4LayerDefRaw(metadataJson, index, dataAddress, dataSize));
        dataAddress += dataSize;
    }

    return chunks;
}

function u32ToBigEndian(value: number): Uint8Array {
    const bytes = new Uint8Array(4);
    new DataView(bytes.buffer).setUint32(0, value, false);
    return bytes;
}

const CXDLP_V4_LAYER_DEF_SIZE = 40;
