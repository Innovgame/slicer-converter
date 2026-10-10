import { buildFileMetadataJson } from "../format/header.js";
import { createLayerSpool, type LayerSpool } from "./layer-spool.js";
import type { PatchableSlicerOutputSink } from "./sink.js";
import type { FileMetadata, SlicerFormatId } from "../types.js";
import type { StreamingDocumentAssemblyHelpers } from "./document-assembly.js";

/**
 * @zh 二进制输出组装选项。
 * @en Options for binary output assembly.
 */
export interface AssembleBinaryOutputOptions {
    finalizedLayerLayout?: boolean;
}

/**
 * @zh 从 layer stream 组装非 archive 输出文件所需的输入。
 * @en Input needed to assemble a non-archive output file from a layer stream.
 */
export interface AssembleOutputFileInput {
    metadata: FileMetadata;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion?: number;
    layerCount: number;
    layerStream: AsyncIterable<Uint8Array>;
    sink: PatchableSlicerOutputSink;
    buildHeader: (metadataJson: string, targetFormat: SlicerFormatId, targetAnycubicVersion?: number) => Uint8Array;
    finalizeHeader: (
        headerBytes: Uint8Array,
        targetFormat: SlicerFormatId,
        layerSizes: Uint32Array,
        targetAnycubicVersion?: number
    ) => Uint8Array;
    buildFooter: (targetFormat: SlicerFormatId) => Uint8Array;
    documentAssembly?: StreamingDocumentAssemblyHelpers["streamingDocument"];
}

/**
 * @zh 将 header、layer stream、footer 依次写入输出 sink，并在 layer sizes 已知后生成最终 header。
 * @en Writes header, layer stream, and footer to an output sink while finalizing the header after layer sizes are known.
 */
export async function assembleBinaryOutput(
    sink: PatchableSlicerOutputSink,
    headerBytes: Uint8Array,
    footerBytes: Uint8Array,
    layerStream: AsyncIterable<Uint8Array>,
    layerCount: number,
    finalizeHeader: (headerBytes: Uint8Array, layerSizes: Uint32Array) => Uint8Array,
    spool: LayerSpool,
    options: AssembleBinaryOutputOptions = {}
): Promise<Blob | null> {
    const layerSizes = new Uint32Array(layerCount);
    let layerIndex = 0;
    for await (const chunk of layerStream) {
        if (layerIndex >= layerCount) {
            throw new Error("Layer stream produced more chunks than expected");
        }
        layerSizes[layerIndex] = chunk.length;
        await spool.write(chunk);
        layerIndex += 1;
    }

    if (layerIndex !== layerCount) {
        throw new Error(`Expected ${layerCount} layer chunks but received ${layerIndex}`);
    }

    await spool.closeForReplay();
    const finalHeader = finalizeHeader(headerBytes, layerSizes);
    if (options.finalizedLayerLayout) {
        await writeFinalizedBinaryLayout(sink, finalHeader, layerSizes, spool);
    } else {
        await sink.write(finalHeader);
        await spool.replay((chunk) => sink.write(chunk));
    }

    if (footerBytes.length > 0) {
        await sink.write(footerBytes);
    }

    return sink.finish();
}

/**
 * @zh 从 layer stream 组装目标格式输出；优先使用格式专用 document assembly helper。
 * @en Assembles target-format output from a layer stream, preferring format-specific document assembly helpers.
 */
export async function assembleOutputFileFromLayerStream(input: AssembleOutputFileInput): Promise<Blob | null> {
    const metadataJson = buildFileMetadataJson(input.metadata);
    if (input.documentAssembly) {
        if (input.documentAssembly.buildDocument) {
            return assembleBufferedDocumentOutput(input, metadataJson, input.documentAssembly.buildDocument);
        }
        if (
            input.documentAssembly.buildPrefix &&
            input.documentAssembly.checksumUpdate &&
            input.documentAssembly.buildChecksumTrailer
        ) {
            return assembleStreamingDocumentOutput(input, metadataJson, {
                buildPrefix: input.documentAssembly.buildPrefix,
                checksumUpdate: input.documentAssembly.checksumUpdate,
                buildChecksumTrailer: input.documentAssembly.buildChecksumTrailer
            });
        }
        throw new Error("Document assembly helper is missing a supported assembly strategy");
    }

    const headerBytes = input.buildHeader(metadataJson, input.targetFormat, input.targetAnycubicVersion);
    const footerBytes = input.buildFooter(input.targetFormat);
    const spool = await createLayerSpool();

    try {
        return await assembleBinaryOutput(
            input.sink,
            headerBytes,
            footerBytes,
            input.layerStream,
            input.layerCount,
            (hdr, sizes) => input.finalizeHeader(hdr, input.targetFormat, sizes, input.targetAnycubicVersion),
            spool,
            { finalizedLayerLayout: usesChituboxCtbLayerLayout(input.targetFormat) }
        );
    } finally {
        await spool.cleanup();
    }
}

function usesChituboxCtbLayerLayout(targetFormat: SlicerFormatId): boolean {
    return targetFormat === "ctb" || targetFormat.startsWith("ctb-v") || targetFormat.startsWith("gktwo-ctb");
}

async function assembleStreamingDocumentOutput(
    input: AssembleOutputFileInput,
    metadataJson: string,
    document: Required<
        Pick<
            NonNullable<AssembleOutputFileInput["documentAssembly"]>,
            "buildPrefix" | "checksumUpdate" | "buildChecksumTrailer"
        >
    >
): Promise<Blob | null> {
    const spool = await createLayerSpool();
    try {
        const layerSizes = new Uint32Array(input.layerCount);
        let layerIndex = 0;
        for await (const chunk of input.layerStream) {
            if (layerIndex >= input.layerCount) {
                throw new Error("Layer stream produced more chunks than expected");
            }
            layerSizes[layerIndex] = chunk.byteLength;
            await spool.write(chunk);
            layerIndex += 1;
        }

        if (layerIndex !== input.layerCount) {
            throw new Error(`Expected ${input.layerCount} layer chunks but received ${layerIndex}`);
        }

        await spool.closeForReplay();
        const prefixChunks = document.buildPrefix(metadataJson, input.targetFormat, layerSizes);

        let checksum = 0;
        const writeChecked = async (chunk: Uint8Array): Promise<void> => {
            checksum = document.checksumUpdate(checksum, chunk);
            await input.sink.write(chunk);
        };

        for (const chunk of prefixChunks) {
            await writeChecked(chunk);
        }
        await spool.replay(writeChecked);
        await input.sink.write(document.buildChecksumTrailer(checksum));
        return input.sink.finish();
    } finally {
        await spool.cleanup();
    }
}

async function assembleBufferedDocumentOutput(
    input: AssembleOutputFileInput,
    metadataJson: string,
    buildDocument: NonNullable<NonNullable<AssembleOutputFileInput["documentAssembly"]>["buildDocument"]>
): Promise<Blob | null> {
    const layers: Uint8Array[] = [];
    for await (const chunk of input.layerStream) {
        if (layers.length >= input.layerCount) {
            throw new Error("Layer stream produced more chunks than expected");
        }
        layers.push(chunk);
    }

    if (layers.length !== input.layerCount) {
        throw new Error(`Expected ${input.layerCount} layer chunks but received ${layers.length}`);
    }

    await input.sink.write(buildDocument(metadataJson, input.targetFormat, input.targetAnycubicVersion, layers));
    return input.sink.finish();
}

async function writeFinalizedBinaryLayout(
    sink: PatchableSlicerOutputSink,
    finalHeader: Uint8Array,
    layerSizes: Uint32Array,
    spool: LayerSpool
): Promise<void> {
    const layerRanges = readLayerRanges(finalHeader, layerSizes);
    let cursor = 0;
    let layerIndex = 0;

    await spool.replay(async (chunk) => {
        if (layerIndex >= layerRanges.length) {
            throw new Error("Layer spool replay produced more chunks than expected");
        }

        const range = layerRanges[layerIndex];
        if (range.start < cursor) {
            throw new Error(`Layer ${layerIndex} overlaps previously written binary document region`);
        }
        if (chunk.byteLength !== range.size) {
            throw new Error(
                `Layer ${layerIndex} finalized size ${range.size} does not match encoded size ${chunk.byteLength}`
            );
        }

        if (cursor < range.start) {
            await writeBinaryGap(sink, finalHeader, cursor, range.start);
        }
        await sink.write(chunk);
        cursor = range.end;
        layerIndex += 1;
    });

    if (layerIndex !== layerRanges.length) {
        throw new Error(`Expected ${layerRanges.length} layer chunks but replayed ${layerIndex}`);
    }
    if (cursor < finalHeader.byteLength) {
        await sink.write(finalHeader.subarray(cursor));
    }
}

async function writeBinaryGap(
    sink: PatchableSlicerOutputSink,
    finalHeader: Uint8Array,
    cursor: number,
    start: number
): Promise<void> {
    if (cursor < finalHeader.byteLength) {
        await sink.write(finalHeader.subarray(cursor, Math.min(start, finalHeader.byteLength)));
    }
    if (start <= finalHeader.byteLength) {
        return;
    }

    let remaining = start - Math.max(cursor, finalHeader.byteLength);
    const zeroChunk = new Uint8Array(1024);
    while (remaining > 0) {
        const count = Math.min(remaining, zeroChunk.byteLength);
        await sink.write(zeroChunk.subarray(0, count));
        remaining -= count;
    }
}

function readLayerRanges(header: Uint8Array, layerSizes: Uint32Array): Array<{ start: number; end: number; size: number }> {
    const view = new DataView(header.buffer, header.byteOffset, header.byteLength);
    if (header.byteLength < 0x48) {
        throw new Error("Finalized binary header is too small to contain a layer table");
    }

    const layerTableOffset = view.getUint32(0x40, true);
    const layerCount = view.getUint32(0x44, true);
    if (layerCount !== layerSizes.length) {
        throw new Error(`Finalized header has ${layerCount} layer entries but ${layerSizes.length} layer chunks`);
    }

    const ranges: Array<{ start: number; end: number; size: number }> = [];
    for (let index = 0; index < layerCount; index += 1) {
        const defOffset = layerTableOffset + index * 36;
        if (defOffset + 24 > header.byteLength) {
            throw new Error(`Finalized header layer table entry ${index} is out of bounds`);
        }
        const dataAddress = view.getUint32(defOffset + 12, true);
        const dataSize = view.getUint32(defOffset + 16, true);
        const pageNumber = view.getUint32(defOffset + 20, true);
        if (pageNumber !== 0) {
            throw new Error("Streaming binary assembler does not support page-numbered layer offsets");
        }
        ranges.push({ start: dataAddress, end: dataAddress + dataSize, size: dataSize });
    }

    return ranges.sort((left, right) => left.start - right.start);
}
