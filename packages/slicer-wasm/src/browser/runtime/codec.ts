import {
    decode_encode_layer as decodeEncodeLayerRaw,
    decode_encode_layer_stats as decodeEncodeLayerStatsRaw,
    decode_layer as decodeLayerRaw,
    decode_layer_preview as decodeLayerPreviewRaw,
    encode_layer as encodeLayerRaw
} from "../../../lib/slicer_converter.js";

import type { DecodedPreviewLayer } from "../types.js";

/**
 * @zh 解包 WASM preview 解码结果，返回尺寸和像素视图。
 * @en Unpacks the WASM preview decode result into dimensions and a pixel view.
 */
export function decodeLayerPreview(
    data: Uint8Array,
    layerIndex: number,
    format: string,
    width: number,
    height: number,
    antiAliasLevel: number,
    encryptionKey: number,
    maxPreviewSize: number
): DecodedPreviewLayer {
    const packed = decodeLayerPreviewRaw(
        data,
        layerIndex,
        format,
        width,
        height,
        antiAliasLevel,
        encryptionKey,
        maxPreviewSize
    );
    const view = new DataView(packed.buffer, packed.byteOffset, packed.byteLength);
    const previewWidth = view.getUint32(0, true);
    const previewHeight = view.getUint32(4, true);
    const pixels = packed.subarray(8);
    return { pixels, width: previewWidth, height: previewHeight };
}

/**
 * @zh 浏览器侧 layer codec 的最小 WASM 函数集合。
 * @en Minimal WASM function set for the browser-side layer codec.
 */
export interface LayerCodecRuntime {
    decodeEncodeLayer(
        data: Uint8Array,
        layerIndex: number,
        sourceFormat: string,
        targetFormat: string,
        width: number,
        height: number,
        antiAliasLevel: number,
        encryptionKey: number,
        layerMetadataJson?: string | null,
        targetAnycubicVersion?: number
    ): Uint8Array;
    decodeEncodeLayerStats(
        data: Uint8Array,
        layerIndex: number,
        sourceFormat: string,
        targetFormat: string,
        width: number,
        height: number,
        antiAliasLevel: number,
        encryptionKey: number,
        layerMetadataJson?: string | null,
        targetAnycubicVersion?: number
    ): string;
    decodeLayer(
        data: Uint8Array,
        layerIndex: number,
        format: string,
        width: number,
        height: number,
        antiAliasLevel: number,
        encryptionKey: number
    ): Uint8Array;
    decodeLayerPreview: typeof decodeLayerPreview;
    encodeLayer(
        pixels: Uint8Array,
        layerIndex: number,
        targetFormat: string,
        width: number,
        height: number,
        layerMetadataJson?: string | null,
        targetAnycubicVersion?: number
    ): Uint8Array;
}

/**
 * @zh package 公开的 layer codec adapter，避免 app/worker 直接依赖完整 raw runtime。
 * @en Public package layer-codec adapter so app/worker code does not depend on the full raw runtime.
 */
export const slicerLayerCodecRuntime: LayerCodecRuntime = {
    decodeEncodeLayer: decodeEncodeLayerRaw,
    decodeEncodeLayerStats: decodeEncodeLayerStatsRaw,
    decodeLayer: decodeLayerRaw,
    decodeLayerPreview,
    encodeLayer: encodeLayerRaw
};
