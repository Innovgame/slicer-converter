import type {
    SlicerWorkerErrorMessage,
    SlicerWorkerResponse,
    SlicerWorkerZipLayerPreparedMessage
} from "../protocol.js";
import { toSlicerWorkerErrorMessage } from "../errors.js";
import type { SlicerWorkerHandlerMap, SlicerWorkerRuntimeDeps } from "../runtime.js";

type ConversionHandlerMap = Pick<
    SlicerWorkerHandlerMap,
    "decode-encode" | "decode-encode-prepare" | "zip-decode-encode" | "zip-decode-encode-prepare"
>;

type ZipPreparedEntryWithTimings = Awaited<ReturnType<SlicerWorkerRuntimeDeps["prepareZipEntry"]>>;

/**
 * @zh `createSlicerConversionHandlers` 导出函数。
 * @en Exported function `createSlicerConversionHandlers`.
 */
export function createSlicerConversionHandlers(deps: SlicerWorkerRuntimeDeps): ConversionHandlerMap {
    const handleDecodeEncode: ConversionHandlerMap["decode-encode"] = (message) => {
        const {
            index,
            data,
            sourceFormat,
            targetFormat,
            targetAnycubicVersion,
            width,
            height,
            antiAliasLevel,
            encryptionKey,
            layerMetadataJson
        } = message;

        try {
            const startedAt = deps.now();
            const encoded = deps.decodeEncodeLayer(
                new Uint8Array(data),
                index,
                sourceFormat,
                targetFormat,
                width,
                height,
                antiAliasLevel || 1,
                encryptionKey || 0,
                layerMetadataJson,
                targetAnycubicVersion ?? undefined
            );

            const buffer = toOwnedArrayBuffer(encoded);
            deps.postMessage(
                {
                    type: "layer-done",
                    index,
                    result: buffer,
                    elapsedMs: deps.now() - startedAt
                } satisfies SlicerWorkerResponse,
                [buffer]
            );
        } catch (error: unknown) {
            const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                error,
                code: "E_WORKER_LAYER_CONVERT",
                stage: "worker-convert",
                retryable: false,
                index,
                format: targetFormat,
                layerIndex: index
            });
            deps.postMessage(response);
        }
    };

    const handleZipDecodeEncode: ConversionHandlerMap["zip-decode-encode"] = (message) => {
        void (async () => {
            const {
                index,
                data,
                compressionMethod,
                uncompressedSize,
                sourceFormat,
                targetFormat,
                targetAnycubicVersion,
                width,
                height,
                layerMetadataJson
            } = message;

            try {
                const startedAt = deps.now();
                const encoded = await deps.convertZipLayerForTarget({
                    compressed: new Uint8Array(data),
                    compressionMethod,
                    uncompressedSize,
                    index,
                    sourceFormat,
                    targetFormat,
                    width,
                    height,
                    layerMetadataJson,
                    targetAnycubicVersion: targetAnycubicVersion ?? undefined,
                    inflate: deps.inflateRaw,
                    decodeEncodeLayer: deps.decodeEncodeLayer
                });

                const buffer = toOwnedArrayBuffer(encoded);
                deps.postMessage(
                    {
                        type: "layer-done",
                        index,
                        result: buffer,
                        elapsedMs: deps.now() - startedAt
                    } satisfies SlicerWorkerResponse,
                    [buffer]
                );
            } catch (error: unknown) {
                const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                    error,
                    code: "E_WORKER_ZIP_LAYER_CONVERT",
                    stage: "worker-convert",
                    retryable: false,
                    index,
                    format: targetFormat,
                    layerIndex: index
                });
                deps.postMessage(response);
            }
        })();
    };

    const handleDecodeEncodePrepare: ConversionHandlerMap["decode-encode-prepare"] = (message) => {
        void (async () => {
            const {
                index,
                data,
                sourceFormat,
                targetFormat,
                targetAnycubicVersion,
                width,
                height,
                antiAliasLevel,
                encryptionKey,
                layerMetadataJson,
                entryName,
                entryCompressionMethod
            } = message;

            try {
                const computeStartedAt = deps.now();
                const sourceBytes = new Uint8Array(data);
                const encoded = deps.decodeEncodeLayer(
                    sourceBytes,
                    index,
                    sourceFormat,
                    targetFormat,
                    width,
                    height,
                    antiAliasLevel || 1,
                    encryptionKey || 0,
                    layerMetadataJson,
                    targetAnycubicVersion ?? undefined
                );
                const encodeEndedAt = deps.now();
                const prepared = await deps.prepareZipEntry({
                    name: entryName,
                    data: encoded,
                    compressionMethod: entryCompressionMethod
                });
                const prepareEndedAt = deps.now();
                const { response, transfer } = buildZipLayerPreparedResponse(
                    index,
                    prepared,
                    prepareEndedAt - computeStartedAt,
                    {
                        encodeMs: encodeEndedAt - computeStartedAt,
                        zipPrepareMs: prepareEndedAt - encodeEndedAt,
                        wasmInputBytes: sourceBytes.byteLength,
                        wasmOutputBytes: encoded.byteLength,
                        totalMs: prepareEndedAt - computeStartedAt
                    }
                );
                deps.postMessage(response satisfies SlicerWorkerResponse, [transfer]);
            } catch (error: unknown) {
                const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                    error,
                    code: "E_WORKER_ZIP_PREPARE",
                    stage: "worker-convert",
                    retryable: false,
                    index,
                    format: targetFormat,
                    layerIndex: index
                });
                deps.postMessage(response);
            }
        })();
    };

    const handleZipDecodeEncodePrepare: ConversionHandlerMap["zip-decode-encode-prepare"] = (message) => {
        void (async () => {
            const {
                index,
                data,
                compressionMethod,
                uncompressedSize,
                sourceFormat,
                targetFormat,
                targetAnycubicVersion,
                width,
                height,
                layerMetadataJson,
                entryName,
                entryCompressionMethod
            } = message;

            try {
                const computeStartedAt = deps.now();
                const compressedBytes = new Uint8Array(data);
                const encoded = await deps.convertZipLayerForTarget({
                    compressed: compressedBytes,
                    compressionMethod,
                    uncompressedSize,
                    index,
                    sourceFormat,
                    targetFormat,
                    width,
                    height,
                    layerMetadataJson,
                    targetAnycubicVersion: targetAnycubicVersion ?? undefined,
                    inflate: deps.inflateRaw,
                    decodeEncodeLayer: deps.decodeEncodeLayer
                });
                const encodeEndedAt = deps.now();
                const prepared = await deps.prepareZipEntry({
                    name: entryName,
                    data: encoded,
                    compressionMethod: entryCompressionMethod
                });
                const prepareEndedAt = deps.now();
                const { response, transfer } = buildZipLayerPreparedResponse(
                    index,
                    prepared,
                    prepareEndedAt - computeStartedAt,
                    {
                        encodeMs: encodeEndedAt - computeStartedAt,
                        zipPrepareMs: prepareEndedAt - encodeEndedAt,
                        wasmInputBytes: uncompressedSize,
                        wasmOutputBytes: encoded.byteLength,
                        totalMs: prepareEndedAt - computeStartedAt
                    }
                );
                deps.postMessage(response satisfies SlicerWorkerResponse, [transfer]);
            } catch (error: unknown) {
                const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                    error,
                    code: "E_WORKER_ZIP_TRANSCODE_PREPARE",
                    stage: "worker-convert",
                    retryable: false,
                    index,
                    format: targetFormat,
                    layerIndex: index
                });
                deps.postMessage(response);
            }
        })();
    };

    return {
        "decode-encode": handleDecodeEncode,
        "decode-encode-prepare": handleDecodeEncodePrepare,
        "zip-decode-encode": handleZipDecodeEncode,
        "zip-decode-encode-prepare": handleZipDecodeEncodePrepare
    };
}

function buildZipLayerPreparedResponse(
    index: number,
    prepared: ZipPreparedEntryWithTimings,
    computeMs: number,
    timings?: SlicerWorkerZipLayerPreparedMessage["timings"]
): { response: SlicerWorkerZipLayerPreparedMessage; transfer: ArrayBuffer } {
    const entry = prepared.entry;
    const buffer = toOwnedArrayBuffer(entry.compressedData);
    return {
        response: {
            type: "zip-layer-prepared",
            index,
            name: entry.name,
            data: buffer,
            compressionMethod: entry.compressionMethod,
            crc32: entry.crc32,
            uncompressedSize: entry.uncompressedSize,
            computeMs,
            timings: timings
                ? {
                      ...timings,
                      crcMs: prepared.timings.crcMs,
                      deflateMs: prepared.timings.deflateMs,
                      zipPackMs: prepared.timings.packMs
                  }
                : undefined
        },
        transfer: buffer
    };
}

function toOwnedArrayBuffer(data: Uint8Array): ArrayBuffer {
    if (data.byteOffset === 0 && data.byteLength === data.buffer.byteLength) {
        return data.buffer as ArrayBuffer;
    }

    const result = new Uint8Array(data.byteLength);
    result.set(data);
    return result.buffer as ArrayBuffer;
}
