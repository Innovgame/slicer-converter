import type { SlicerFormatId } from "../types.js";

/**
 * @zh slicer worker 可接收的 request 类型哨兵。
 * @en Request type sentinels accepted by a slicer worker.
 */
export const SLICER_WORKER_REQUEST_TYPES = [
    "decode-encode",
    "decode-encode-prepare",
    "decode-preview",
    "zip-decode-encode",
    "zip-decode-encode-prepare",
    "cancel"
] as const;

/**
 * @zh slicer worker 可发送的 response 类型哨兵。
 * @en Response type sentinels emitted by a slicer worker.
 */
export const SLICER_WORKER_RESPONSE_TYPES = ["ready", "layer-done", "preview-done", "zip-layer-prepared", "error"] as const;

/**
 * @zh 普通 encoded layer 的 decode/encode 请求。
 * @en Decode/encode request for a regular encoded layer.
 */
export interface DecodeEncodeRequest {
    type: "decode-encode";
    index: number;
    data: ArrayBuffer;
    sourceFormat: SlicerFormatId;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion: number | null;
    width: number;
    height: number;
    antiAliasLevel: number;
    encryptionKey: number;
    layerMetadataJson: string | null;
}

/**
 * @zh 普通 encoded layer 的 decode/encode 并准备 ZIP entry 请求。
 * @en Decode/encode and ZIP-entry preparation request for a regular encoded layer.
 */
export interface DecodeEncodePrepareRequest {
    type: "decode-encode-prepare";
    index: number;
    data: ArrayBuffer;
    sourceFormat: SlicerFormatId;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion: number | null;
    width: number;
    height: number;
    antiAliasLevel: number;
    encryptionKey: number;
    layerMetadataJson: string | null;
    entryName: string;
    entryCompressionMethod: 0 | 8;
}

/**
 * @zh preview 请求质量；interactive 用于滑动中反馈，settled 用于停止后的高清图。
 * @en Preview quality; interactive is feedback while sliding, settled is the idle high-quality image.
 */
export type PreviewQuality = "interactive" | "settled";

/**
 * @zh 发送到 worker 的 preview 请求。
 * @en Preview request dispatched to a worker.
 */
export interface DecodePreviewRequest {
    type: "decode-preview";
    requestId: number;
    quality: PreviewQuality;
    index: number;
    maxPreviewSize: number;
    payload: PreviewLayerPayload;
}

/**
 * @zh preview worker 可解码的统一 layer payload。
 * @en Unified layer payload decodable by the preview worker.
 */
export type PreviewLayerPayload = EncodedPreviewLayerPayload | ZipPreviewLayerPayload;

/**
 * @zh 普通 encoded layer 的 preview payload，例如 GOO/CTB RLE。
 * @en Preview payload for regular encoded layers such as GOO/CTB RLE.
 */
export interface EncodedPreviewLayerPayload {
    kind: "encoded-layer";
    data: ArrayBuffer;
    format: SlicerFormatId;
    width: number;
    height: number;
    antiAliasLevel: number;
    encryptionKey: number;
}

/**
 * @zh ZIP entry layer 的 preview payload，例如 VDT/SL1 PNG layer。
 * @en Preview payload for ZIP entry layers such as VDT/SL1 PNG layers.
 */
export interface ZipPreviewLayerPayload {
    kind: "zip-entry";
    data: ArrayBuffer;
    compressionMethod: number;
    uncompressedSize: number;
    sourceFormat: SlicerFormatId;
    width: number;
    height: number;
}

/**
 * @zh ZIP entry layer 的 decode/encode 请求。
 * @en Decode/encode request for a ZIP entry layer.
 */
export interface ZipDecodeEncodeRequest {
    type: "zip-decode-encode";
    index: number;
    /**
     * @zh ZIP entry 的原始 payload，可能是 deflate 压缩或 stored。
     * @en Raw ZIP entry payload, either deflated or stored.
     */
    data: ArrayBuffer;
    /**
     * @zh ZIP 压缩方式，0 表示 stored，8 表示 deflate。
     * @en ZIP compression method; 0 means stored and 8 means deflate.
     */
    compressionMethod: number;
    /**
     * @zh ZIP 解压后的预期字节数。
     * @en Expected byte length after ZIP decompression.
     */
    uncompressedSize: number;
    sourceFormat: SlicerFormatId;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion: number | null;
    width: number;
    height: number;
    layerMetadataJson: string | null;
}

/**
 * @zh ZIP entry layer 的 decode/encode 并准备 ZIP entry 请求。
 * @en Decode/encode and ZIP-entry preparation request for a ZIP entry layer.
 */
export interface ZipDecodeEncodePrepareRequest {
    type: "zip-decode-encode-prepare";
    index: number;
    /**
     * @zh ZIP entry 的原始 payload，可能是 deflate 压缩或 stored。
     * @en Raw ZIP entry payload, either deflated or stored.
     */
    data: ArrayBuffer;
    /**
     * @zh ZIP 压缩方式，0 表示 stored，8 表示 deflate。
     * @en ZIP compression method; 0 means stored and 8 means deflate.
     */
    compressionMethod: number;
    /**
     * @zh ZIP 解压后的预期字节数。
     * @en Expected byte length after ZIP decompression.
     */
    uncompressedSize: number;
    sourceFormat: SlicerFormatId;
    targetFormat: SlicerFormatId;
    targetAnycubicVersion: number | null;
    width: number;
    height: number;
    layerMetadataJson: string | null;
    entryName: string;
    entryCompressionMethod: 0 | 8;
}

/**
 * @zh 取消请求；当前 worker entry 会忽略它，由上层 scheduler/AbortSignal 负责停止派发。
 * @en Cancel request; the current worker entry ignores it and scheduling is stopped by AbortSignal.
 */
export interface CancelRequest {
    type: "cancel";
}

/**
 * @zh slicer worker request union。
 * @en Slicer worker request union.
 */
export type SlicerWorkerRequest =
    | DecodeEncodeRequest
    | DecodeEncodePrepareRequest
    | DecodePreviewRequest
    | ZipDecodeEncodeRequest
    | ZipDecodeEncodePrepareRequest
    | CancelRequest;

/**
 * @zh worker 初始化完成消息。
 * @en Worker ready message.
 */
export interface SlicerWorkerReadyMessage {
    type: "ready";
}

/**
 * @zh 单层转换完成消息。
 * @en Single-layer conversion completion message.
 */
export interface SlicerWorkerLayerDoneMessage {
    type: "layer-done";
    index: number;
    result: ArrayBuffer;
    elapsedMs: number;
}

/**
 * @zh preview 渲染计时信息。
 * @en Preview rendering timing information.
 */
export interface PreviewTimings {
    /**
     * @zh preview 渲染路线，用于区分普通 encoded layer、archive layer preview 与 archive 原始 PNG 显示。
     * @en Preview rendering route, distinguishing encoded-layer previews, archive-layer previews, and archive original-PNG display.
     */
    path: "encoded-layer-preview" | "archive-layer-preview" | "archive-original-png";
    /**
     * @zh WASM `decodeLayerPreview` 耗时，单位毫秒。
     * @en Time spent in WASM `decodeLayerPreview`, in milliseconds.
     */
    wasmDecodeMs: number;
    /**
     * @zh JS 灰度转 RGBA 和 `createImageBitmap` 耗时，单位毫秒。
     * @en Time spent on JS grayscale-to-RGBA conversion plus `createImageBitmap`, in milliseconds.
     */
    imageBitmapMs: number;
    /**
     * @zh preview 请求端到端总耗时，单位毫秒。
     * @en Total end-to-end preview request time, in milliseconds.
     */
    totalMs: number;
    /**
     * @zh ZIP deflate 解压耗时，单位毫秒；stored entry 时为 0。
     * @en Time spent inflating ZIP deflate data, in milliseconds; 0 for stored entries.
     */
    zipInflateMs?: number;
}

/**
 * @zh preview 完成消息。
 * @en Preview completion message.
 */
export interface SlicerWorkerPreviewDoneMessage {
    type: "preview-done";
    requestId: number;
    quality: PreviewQuality;
    index: number;
    result: ImageBitmap;
    width: number;
    height: number;
    timings?: PreviewTimings;
}

/**
 * @zh ZIP layer 已准备完成消息。
 * @en ZIP layer prepared message.
 */
export interface SlicerWorkerZipLayerPreparedMessage {
    type: "zip-layer-prepared";
    index: number;
    name: string;
    data: ArrayBuffer;
    compressionMethod: 0 | 8;
    crc32: number;
    uncompressedSize: number;
    computeMs: number;
    timings?: {
        encodeMs: number;
        zipPrepareMs: number;
        wasmInputBytes?: number;
        wasmOutputBytes?: number;
        crcMs?: number;
        deflateMs?: number;
        zipPackMs?: number;
        totalMs: number;
    };
}

/**
 * @zh worker 结构化错误。
 * @en Structured worker error.
 */
export interface SlicerWorkerError {
    code: string;
    message: string;
    stage: string;
    retryable: boolean;
    timestamp: number;
    format?: SlicerFormatId | string;
    layerIndex?: number;
    detail?: string;
}

/**
 * @zh worker 错误消息。
 * @en Worker error message.
 */
export interface SlicerWorkerErrorMessage {
    type: "error";
    error: SlicerWorkerError;
    index?: number;
}

/**
 * @zh slicer worker response union。
 * @en Slicer worker response union.
 */
export type SlicerWorkerResponse =
    | SlicerWorkerReadyMessage
    | SlicerWorkerLayerDoneMessage
    | SlicerWorkerPreviewDoneMessage
    | SlicerWorkerZipLayerPreparedMessage
    | SlicerWorkerErrorMessage;
