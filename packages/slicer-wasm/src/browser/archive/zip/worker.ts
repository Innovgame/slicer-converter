import type { LayerCodecRuntime } from "../../runtime/codec.js";
import type { DecodedPreviewLayer, SlicerFormatId } from "../../types.js";

/**
 * @zh ZIP preview 解码的分段耗时。
 * @en Segmented timings for ZIP preview decoding.
 */
export interface DecodeZipPreviewTimings {
    compressedBytes: number;
    pngPayloadBytes: number;
    zipInflateMs: number;
    wasmPreviewDecodeMs: number;
    totalMs: number;
}

/**
 * @zh 带分段耗时的 ZIP preview 解码结果。
 * @en ZIP preview decode result with segmented timings.
 */
export interface DecodedZipPreviewLayer extends DecodedPreviewLayer {
    timings: DecodeZipPreviewTimings;
}

interface ZipInflateDeps {
    inflate: (compressed: Uint8Array, expectedUncompressedSize?: number) => Promise<Uint8Array>;
}

interface ZipCompressedInput extends ZipInflateDeps {
    compressed: Uint8Array;
    compressionMethod: number;
    uncompressedSize: number;
}

interface ConvertZipLayerDeps extends ZipInflateDeps {
    decodeEncodeLayer: LayerCodecRuntime["decodeEncodeLayer"];
}

interface DecodeZipPreviewDeps extends ZipInflateDeps {
    decodeLayerPreview: LayerCodecRuntime["decodeLayerPreview"];
}

/**
 * @zh `ConvertZipLayerInput` 接口契约。
 * @en Interface contract for `ConvertZipLayerInput`.
 */
export interface ConvertZipLayerInput extends ZipCompressedInput, ConvertZipLayerDeps {
    index: number;
    sourceFormat: SlicerFormatId;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion?: number;
    width: number;
    height: number;
    layerMetadataJson: string | null;
}

/**
 * @zh `DecodeZipPreviewInput` 接口契约。
 * @en Interface contract for `DecodeZipPreviewInput`.
 */
export interface DecodeZipPreviewInput extends ZipCompressedInput, DecodeZipPreviewDeps {
    index: number;
    sourceFormat: SlicerFormatId;
    width: number;
    height: number;
    maxPreviewSize: number;
}

async function decompressZipPayload(input: ZipCompressedInput): Promise<Uint8Array> {
    if (input.compressionMethod === 0) {
        return input.compressed;
    }
    if (input.compressionMethod === 8) {
        return input.inflate(input.compressed, input.uncompressedSize);
    }
    throw new Error(`Unsupported ZIP compression method: ${input.compressionMethod}`);
}

/**
 * @zh `convertZipLayerForTarget` 导出函数。
 * @en Exported function `convertZipLayerForTarget`.
 */
export async function convertZipLayerForTarget(input: ConvertZipLayerInput): Promise<Uint8Array> {
    const pngBytes = await decompressZipPayload(input);

    return input.decodeEncodeLayer(
        pngBytes,
        input.index,
        input.sourceFormat,
        input.targetFormat,
        input.width,
        input.height,
        1,
        0,
        input.layerMetadataJson,
        input.targetAnycubicVersion
    );
}

/**
 * @zh `decodeZipPreview` 导出函数。
 * @en Exported function `decodeZipPreview`.
 */
export async function decodeZipPreview(input: DecodeZipPreviewInput): Promise<DecodedZipPreviewLayer> {
    const t0 = now();
    const pngBytes = await decompressZipPayload(input);
    const t1 = now();
    const preview = input.decodeLayerPreview(
        pngBytes,
        input.index,
        input.sourceFormat,
        input.width,
        input.height,
        1,
        0,
        input.maxPreviewSize
    );
    const t2 = now();

    return {
        ...preview,
        timings: {
            compressedBytes: input.compressed.byteLength,
            pngPayloadBytes: pngBytes.byteLength,
            zipInflateMs: t1 - t0,
            wasmPreviewDecodeMs: t2 - t1,
            totalMs: t2 - t0
        }
    } satisfies DecodedZipPreviewLayer;
}

function now(): number {
    return typeof performance !== "undefined" ? performance.now() : Date.now();
}
