import type {
    DecodePreviewRequest,
    PreviewLayerPayload,
    SlicerWorkerErrorMessage,
    SlicerWorkerResponse,
    ZipPreviewLayerPayload
} from "../protocol.js";
import { toSlicerWorkerErrorMessage } from "../errors.js";
import type { SlicerWorkerHandlerMap, SlicerWorkerRuntimeDeps } from "../runtime.js";

const PREVIEW_CANVAS_POOL_LIMIT = 2;
const previewCanvasPool = new Map<string, { canvas: OffscreenCanvas; ctx: OffscreenCanvasRenderingContext2D }>();

type PreviewHandlerMap = Pick<SlicerWorkerHandlerMap, "decode-preview">;

/**
 * @zh `createSlicerPreviewHandlers` 导出函数。
 * @en Exported function `createSlicerPreviewHandlers`.
 */
export function createSlicerPreviewHandlers(deps: SlicerWorkerRuntimeDeps): PreviewHandlerMap {
    const handleDecodePreview: PreviewHandlerMap["decode-preview"] = (message) => {
        const { index, requestId, quality, payload } = message;

        void (async () => {
            try {
                const t0 = deps.now();
                if (shouldDecodeSettledZipPngNatively(message, payload)) {
                    const nativeZipPng = await decodeNativeZipPngPreview(deps, payload, t0);
                    logPreviewPerf({
                        index,
                        quality,
                        sourceFormat: previewPayloadFormat(payload),
                        path: nativeZipPng.timings.path,
                        zipInflateMs: nativeZipPng.timings.zipInflateMs,
                        wasmDecodeMs: nativeZipPng.timings.wasmDecodeMs,
                        imageBitmapMs: nativeZipPng.timings.imageBitmapMs,
                        totalMs: nativeZipPng.timings.totalMs
                    });
                    deps.postMessage(
                        {
                            type: "preview-done",
                            requestId,
                            quality,
                            index,
                            result: nativeZipPng.bitmap,
                            width: nativeZipPng.width,
                            height: nativeZipPng.height,
                            timings: nativeZipPng.timings
                        } satisfies SlicerWorkerResponse,
                        [nativeZipPng.bitmap]
                    );
                    return;
                }

                const decoded = await decodePreviewPayload(deps, message, payload);
                const t1 = deps.now();
                const bitmap = await rgbaToImageBitmap(decoded.pixels, decoded.width, decoded.height);
                const t2 = deps.now();
                const timings = buildPreviewTimings({
                    payload,
                    decoded,
                    decodeMs: t1 - t0,
                    bitmapMs: t2 - t1,
                    totalMs: t2 - t0
                });
                logPreviewPerf({
                    index,
                    quality,
                    sourceFormat: previewPayloadFormat(payload),
                    path: timings.path,
                    zipInflateMs: timings.zipInflateMs,
                    wasmDecodeMs: timings.wasmDecodeMs,
                    imageBitmapMs: timings.imageBitmapMs,
                    totalMs: timings.totalMs
                });
                deps.postMessage(
                    {
                        type: "preview-done",
                        requestId,
                        quality,
                        index,
                        result: bitmap,
                        width: decoded.width,
                        height: decoded.height,
                        timings
                    } satisfies SlicerWorkerResponse,
                    [bitmap]
                );
            } catch (error: unknown) {
                const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                    error,
                    code: payload.kind === "zip-entry" ? "E_WORKER_ZIP_PREVIEW" : "E_WORKER_PREVIEW",
                    stage: "worker-preview",
                    retryable: false,
                    index,
                    format: previewPayloadFormat(payload),
                    layerIndex: index
                });
                deps.postMessage(response);
            }
        })();
    };

    return {
        "decode-preview": handleDecodePreview
    };
}

function getPreviewMaxSize(message: DecodePreviewRequest): number {
    return Math.max(1, Math.floor(message.maxPreviewSize));
}

type DecodedPreviewPayload = {
    pixels: Uint8Array;
    width: number;
    height: number;
    timings?: {
        zipInflateMs: number;
        wasmPreviewDecodeMs: number;
        totalMs: number;
    };
};

type NativeZipPngPreview = {
    bitmap: ImageBitmap;
    width: number;
    height: number;
    timings: {
        path: "archive-original-png";
        wasmDecodeMs: number;
        imageBitmapMs: number;
        totalMs: number;
        zipInflateMs: number;
    };
};

function shouldDecodeSettledZipPngNatively(
    message: DecodePreviewRequest,
    payload: PreviewLayerPayload
): payload is ZipPreviewLayerPayload {
    // Fixed preview policy: SL1/VDT ZIP PNG layers use bounded WASM while interactive,
    // then native full-size PNG decode once settled. Other payloads stay on WASM.
    return (
        message.quality === "settled" &&
        payload.kind === "zip-entry" &&
        isNativePngPreviewFormat(payload.sourceFormat) &&
        (payload.compressionMethod === 0 || payload.compressionMethod === 8) &&
        typeof createImageBitmap === "function" &&
        typeof Blob !== "undefined"
    );
}

async function decodeNativeZipPngPreview(
    deps: SlicerWorkerRuntimeDeps,
    payload: ZipPreviewLayerPayload,
    startedAt: number
): Promise<NativeZipPngPreview> {
    const pngBytes =
        payload.compressionMethod === 0
            ? new Uint8Array(payload.data)
            : await deps.inflateRaw(new Uint8Array(payload.data), payload.uncompressedSize);
    const inflatedAt = deps.now();
    const bitmap = await createNativePngImageBitmap(pngBytes);
    const finishedAt = deps.now();

    return {
        bitmap,
        width: bitmap.width || payload.width,
        height: bitmap.height || payload.height,
        timings: {
            path: "archive-original-png",
            wasmDecodeMs: 0,
            imageBitmapMs: finishedAt - inflatedAt,
            totalMs: finishedAt - startedAt,
            zipInflateMs: inflatedAt - startedAt
        }
    };
}

function createNativePngImageBitmap(pngBytes: Uint8Array): Promise<ImageBitmap> {
    const blob = new Blob([toArrayBufferBlobPart(pngBytes)], { type: "image/png" });
    return createImageBitmap(blob);
}

function isNativePngPreviewFormat(format: string): boolean {
    return format === "vdt" || format === "sl1";
}

function toArrayBufferBlobPart(bytes: Uint8Array): ArrayBuffer {
    if (bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength && bytes.buffer instanceof ArrayBuffer) {
        return bytes.buffer;
    }

    const copy = new Uint8Array(bytes.byteLength);
    copy.set(bytes);
    return copy.buffer;
}

async function decodePreviewPayload(
    deps: SlicerWorkerRuntimeDeps,
    message: DecodePreviewRequest,
    payload: PreviewLayerPayload
): Promise<DecodedPreviewPayload> {
    if (payload.kind === "zip-entry") {
        return deps.decodeZipPreview({
            compressed: new Uint8Array(payload.data),
            compressionMethod: payload.compressionMethod,
            uncompressedSize: payload.uncompressedSize,
            index: message.index,
            sourceFormat: payload.sourceFormat,
            width: payload.width,
            height: payload.height,
            inflate: deps.inflateRaw,
            decodeLayerPreview: deps.decodeLayerPreview,
            maxPreviewSize: getPreviewMaxSize(message)
        });
    }

    return deps.decodeLayerPreview(
        new Uint8Array(payload.data),
        message.index,
        payload.format,
        payload.width,
        payload.height,
        payload.antiAliasLevel || 1,
        payload.encryptionKey || 0,
        getPreviewMaxSize(message)
    );
}

function buildPreviewTimings(input: {
    payload: PreviewLayerPayload;
    decoded: DecodedPreviewPayload;
    decodeMs: number;
    bitmapMs: number;
    totalMs: number;
}) {
    if (input.payload.kind === "zip-entry") {
        const zipTimings = input.decoded.timings;
        return {
            path: "archive-layer-preview",
            wasmDecodeMs: zipTimings?.wasmPreviewDecodeMs ?? input.decodeMs,
            imageBitmapMs: input.bitmapMs,
            totalMs: input.totalMs,
            zipInflateMs: zipTimings?.zipInflateMs ?? 0
        } as const;
    }

    return {
        path: "encoded-layer-preview",
        wasmDecodeMs: input.decodeMs,
        imageBitmapMs: input.bitmapMs,
        totalMs: input.totalMs
    } as const;
}

function previewPayloadFormat(payload: PreviewLayerPayload): string {
    return payload.kind === "zip-entry" ? payload.sourceFormat : payload.format;
}

function logPreviewPerf(input: {
    index: number;
    quality: string;
    sourceFormat: string;
    path: string;
    zipInflateMs?: number;
    wasmDecodeMs: number;
    imageBitmapMs: number;
    totalMs: number;
}): void {
    console.info(
        `[preview-perf] worker layer=${input.index} quality=${input.quality} format=${input.sourceFormat} path=${input.path} inflate=${formatMs(input.zipInflateMs ?? 0)} decode=${formatMs(input.wasmDecodeMs)} bitmap=${formatMs(input.imageBitmapMs)} total=${formatMs(input.totalMs)}`
    );
}

function formatMs(value: number): string {
    return `${value.toFixed(1)}ms`;
}

function acquirePreviewCanvas(
    width: number,
    height: number
): {
    canvas: OffscreenCanvas;
    ctx: OffscreenCanvasRenderingContext2D;
} | null {
    if (typeof OffscreenCanvas === "undefined") {
        return null;
    }

    const key = `${width}x${height}`;
    const cached = previewCanvasPool.get(key);
    if (cached) {
        previewCanvasPool.delete(key);
        previewCanvasPool.set(key, cached);
        return cached;
    }

    let canvas: OffscreenCanvas;
    try {
        canvas = new OffscreenCanvas(width, height);
    } catch {
        return null;
    }

    const ctx = canvas.getContext("2d", { willReadFrequently: false, alpha: true });
    if (!ctx) {
        return null;
    }

    const entry = { canvas, ctx };
    previewCanvasPool.set(key, entry);
    while (previewCanvasPool.size > PREVIEW_CANVAS_POOL_LIMIT) {
        const oldestKey = previewCanvasPool.keys().next().value;
        if (oldestKey === undefined) {
            break;
        }
        previewCanvasPool.delete(oldestKey);
    }

    return entry;
}

async function rgbaToImageBitmap(rgbaPixels: Uint8Array, width: number, height: number): Promise<ImageBitmap> {
    const clamped = new Uint8ClampedArray(
        rgbaPixels.buffer as ArrayBuffer,
        rgbaPixels.byteOffset,
        rgbaPixels.byteLength
    );
    const imageData = new ImageData(clamped, width, height);
    const pooled = acquirePreviewCanvas(width, height);

    if (pooled) {
        pooled.ctx.putImageData(imageData, 0, 0);
        return pooled.canvas.transferToImageBitmap();
    }

    return createImageBitmap(imageData);
}
