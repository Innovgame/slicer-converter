import { slicerLayerCodecRuntime, type LayerCodecRuntime } from "../runtime/codec.js";
import type { SlicerDocument } from "../document/document.js";
import type { DecodedPreviewLayer, SlicerFormatId } from "../types.js";
import type { PreviewQuality, PreviewTimings } from "../worker/protocol.js";
import { inflateRaw } from "../archive/zip/source.js";
import { decodeZipPreview, type DecodedZipPreviewLayer, type DecodeZipPreviewInput } from "../archive/zip/worker.js";

/**
 * @zh `decodeSlicerLayerPreview` 的输入参数。
 * @en Input parameters for `decodeSlicerLayerPreview`.
 */
export interface DecodeSlicerLayerPreviewInput {
    document: SlicerDocument;
    layerIndex: number;
    quality: PreviewQuality;
    maxPreviewSize: number;
}

/**
 * @zh 单层 preview 解码结果。
 * @en Decoded preview result for one layer.
 */
export interface SlicerLayerPreviewResult {
    bitmap: ImageBitmap;
    width: number;
    height: number;
    timings: PreviewTimings;
}

interface DecodeSlicerLayerPreviewDeps {
    decodeLayerPreview: LayerCodecRuntime["decodeLayerPreview"];
    decodeZipPreview: (input: DecodeZipPreviewInput) => Promise<DecodedZipPreviewLayer>;
    inflateRaw: (compressed: Uint8Array, expectedUncompressedSize?: number) => Promise<Uint8Array>;
    now: () => number;
}

/**
 * @zh 解码已打开文档的单层 preview。
 * @en Decodes one layer preview from an opened slicer document.
 */
export function decodeSlicerLayerPreview(input: DecodeSlicerLayerPreviewInput): Promise<SlicerLayerPreviewResult> {
    return decodeSlicerLayerPreviewWithDeps({
        ...input,
        decodeLayerPreview: slicerLayerCodecRuntime.decodeLayerPreview,
        decodeZipPreview,
        inflateRaw,
        now
    });
}

/**
 * @internal
 * @zh 注入依赖版本，用于测试 preview route 和计时。
 * @en Dependency-injected variant used to test preview routes and timings.
 */
export async function decodeSlicerLayerPreviewWithDeps(
    input: DecodeSlicerLayerPreviewInput & DecodeSlicerLayerPreviewDeps
): Promise<SlicerLayerPreviewResult> {
    const header = input.document.parsedHeader;
    const width = header.metadata.resolution_x || 256;
    const height = header.metadata.resolution_y || 256;
    const payload = await input.document.readLayerPayload(input.layerIndex);
    const t0 = input.now();

    if (
        payload.kind === "zip-entry" &&
        shouldDecodeSettledZipPngNatively(input.quality, header.format, payload.compressionMethod)
    ) {
        return decodeNativeZipPngPreview({
            compressed: payload.bytes,
            compressionMethod: payload.compressionMethod,
            uncompressedSize: payload.uncompressedSize,
            inflateRaw: input.inflateRaw,
            fallbackWidth: width,
            fallbackHeight: height,
            startedAt: t0,
            now: input.now
        });
    }

    if (payload.kind === "zip-entry") {
        const decoded = await input.decodeZipPreview({
            compressed: payload.bytes,
            compressionMethod: payload.compressionMethod,
            uncompressedSize: payload.uncompressedSize,
            index: input.layerIndex,
            sourceFormat: header.format,
            width,
            height,
            inflate: input.inflateRaw,
            decodeLayerPreview: input.decodeLayerPreview,
            maxPreviewSize: getPreviewMaxSize(input.maxPreviewSize)
        });
        const decodedAt = input.now();
        const bitmap = await rgbaToImageBitmap(decoded.pixels, decoded.width, decoded.height);
        const bitmapAt = input.now();

        return {
            bitmap,
            width: decoded.width,
            height: decoded.height,
            timings: {
                path: "archive-layer-preview",
                zipInflateMs: decoded.timings.zipInflateMs,
                wasmDecodeMs: decoded.timings.wasmPreviewDecodeMs,
                imageBitmapMs: bitmapAt - decodedAt,
                totalMs: bitmapAt - t0
            }
        };
    }

    const decoded = input.decodeLayerPreview(
        payload.bytes,
        input.layerIndex,
        header.format,
        width,
        height,
        header.metadata.anti_alias_level || 1,
        header.metadata.encryption_key || 0,
        getPreviewMaxSize(input.maxPreviewSize)
    );
    const decodedAt = input.now();
    const bitmap = await rgbaToImageBitmap(decoded.pixels, decoded.width, decoded.height);
    const bitmapAt = input.now();

    return {
        bitmap,
        width: decoded.width,
        height: decoded.height,
        timings: {
            path: "encoded-layer-preview",
            wasmDecodeMs: decodedAt - t0,
            imageBitmapMs: bitmapAt - decodedAt,
            totalMs: bitmapAt - t0
        }
    };
}

function shouldDecodeSettledZipPngNatively(
    quality: PreviewQuality,
    sourceFormat: SlicerFormatId,
    compressionMethod: number
): boolean {
    return (
        quality === "settled" &&
        isNativePngPreviewFormat(sourceFormat) &&
        (compressionMethod === 0 || compressionMethod === 8) &&
        typeof createImageBitmap === "function" &&
        typeof Blob !== "undefined"
    );
}

function isNativePngPreviewFormat(format: string): boolean {
    return format === "vdt" || format === "sl1";
}

async function decodeNativeZipPngPreview(input: {
    compressed: Uint8Array;
    compressionMethod: number;
    uncompressedSize: number;
    inflateRaw: (compressed: Uint8Array, expectedUncompressedSize?: number) => Promise<Uint8Array>;
    fallbackWidth: number;
    fallbackHeight: number;
    startedAt: number;
    now: () => number;
}): Promise<SlicerLayerPreviewResult> {
    const pngBytes =
        input.compressionMethod === 0
            ? input.compressed
            : await input.inflateRaw(input.compressed, input.uncompressedSize);
    const inflatedAt = input.now();
    const blob = new Blob([toArrayBufferBlobPart(pngBytes)], { type: "image/png" });
    const bitmap = await createImageBitmap(blob);
    const finishedAt = input.now();

    return {
        bitmap,
        width: bitmap.width || input.fallbackWidth,
        height: bitmap.height || input.fallbackHeight,
        timings: {
            path: "archive-original-png",
            wasmDecodeMs: 0,
            imageBitmapMs: finishedAt - inflatedAt,
            totalMs: finishedAt - input.startedAt,
            zipInflateMs: inflatedAt - input.startedAt
        }
    };
}

async function rgbaToImageBitmap(
    decodedPixels: DecodedPreviewLayer["pixels"],
    width: number,
    height: number
): Promise<ImageBitmap> {
    const clamped = new Uint8ClampedArray(
        decodedPixels.buffer as ArrayBuffer,
        decodedPixels.byteOffset,
        decodedPixels.byteLength
    );
    return createImageBitmap(new ImageData(clamped, width, height));
}

function toArrayBufferBlobPart(bytes: Uint8Array): ArrayBuffer {
    if (bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength && bytes.buffer instanceof ArrayBuffer) {
        return bytes.buffer;
    }

    const copy = new Uint8Array(bytes.byteLength);
    copy.set(bytes);
    return copy.buffer;
}

function getPreviewMaxSize(value: number): number {
    return Math.max(1, Math.floor(value));
}

function now(): number {
    return typeof performance !== "undefined" ? performance.now() : Date.now();
}
