import { createStreamingDocumentAssemblyHelpers } from "./document-assembly.js";
import { assembleOutputFileFromLayerStream } from "./binary.js";
import type { PatchableSlicerOutputSink } from "./sink.js";
import type { FileMetadata, SlicerFormatId } from "../types.js";

/**
 * @zh 使用 package 内 WASM document assembly helper 从 layer stream 组装二进制输出。
 * @en Assembles binary output from a layer stream using package-local WASM document assembly helpers.
 */
export interface AssembleWasmBinaryOutputInput {
    metadata: FileMetadata;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion?: number;
    layerCount: number;
    layerStream: AsyncIterable<Uint8Array>;
    sink: PatchableSlicerOutputSink;
}

/**
 * @zh 使用当前 WASM helper 为目标格式组装二进制输出文件。
 * @en Assembles a binary output file for the target format using the current WASM helpers.
 */
export function assembleWasmBinaryOutputFromLayerStream(input: AssembleWasmBinaryOutputInput): Promise<Blob | null> {
    const assembly = createStreamingDocumentAssemblyHelpers(input.targetFormat);
    return assembleOutputFileFromLayerStream({
        metadata: input.metadata,
        targetFormat: input.targetFormat,
        targetAnycubicVersion: input.targetAnycubicVersion,
        layerCount: input.layerCount,
        layerStream: input.layerStream,
        sink: input.sink,
        buildHeader: assembly.buildHeader,
        finalizeHeader: assembly.finalizeHeader,
        buildFooter: assembly.buildFooter,
        documentAssembly: assembly.streamingDocument
    });
}
