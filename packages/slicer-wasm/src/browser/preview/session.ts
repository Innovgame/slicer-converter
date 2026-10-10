import type { SlicerDocument } from "../document/document.js";
import type { DecodePreviewRequest, PreviewQuality } from "../worker/protocol.js";

/**
 * @zh 单次 preview dispatch 的请求参数。
 * @en Request parameters for one preview dispatch.
 */
export interface SlicerPreviewDispatchRequest {
    layerIndex: number;
    requestId: number;
    quality: PreviewQuality;
    maxPreviewSize: number;
}

/**
 * @zh 可直接传给 worker 的 preview 请求与 transferable buffer。
 * @en Preview request and transferable buffer ready for worker dispatch.
 */
export interface SlicerPreviewDispatchPayload {
    request: DecodePreviewRequest;
    transfer: ArrayBuffer;
}

/**
 * @zh 已打开文档上的 preview 读取会话。
 * @en Preview read session for an opened slicer document.
 */
export interface SlicerPreviewSession {
    buildDispatchPayload(request: SlicerPreviewDispatchRequest): Promise<SlicerPreviewDispatchPayload>;
}

/**
 * @zh 创建 preview 读取会话，将 raw/ZIP layer payload 映射成 worker protocol 请求。
 * @en Creates a preview read session that maps raw/ZIP layer payloads into worker protocol requests.
 */
export function createSlicerPreviewSession(input: { document: SlicerDocument }): SlicerPreviewSession {
    const header = input.document.parsedHeader;
    const width = header.metadata.resolution_x || 256;
    const height = header.metadata.resolution_y || 256;

    return {
        async buildDispatchPayload(request) {
            const layerPayload = await input.document.readLayerPayload(request.layerIndex);
            const transfer = toTransferBuffer(layerPayload.bytes);

            if (layerPayload.kind === "zip-entry") {
                return {
                    transfer,
                    request: {
                        type: "decode-preview",
                        requestId: request.requestId,
                        quality: request.quality,
                        index: request.layerIndex,
                        maxPreviewSize: request.maxPreviewSize,
                        payload: {
                            kind: "zip-entry",
                            data: transfer,
                            compressionMethod: layerPayload.compressionMethod,
                            uncompressedSize: layerPayload.uncompressedSize,
                            sourceFormat: header.format,
                            width,
                            height
                        }
                    }
                };
            }

            return {
                transfer,
                request: {
                    type: "decode-preview",
                    requestId: request.requestId,
                    quality: request.quality,
                    index: request.layerIndex,
                    maxPreviewSize: request.maxPreviewSize,
                    payload: {
                        kind: "encoded-layer",
                        data: transfer,
                        format: header.format,
                        width,
                        height,
                        antiAliasLevel: header.metadata.anti_alias_level || 1,
                        encryptionKey: header.metadata.encryption_key || 0
                    }
                }
            };
        }
    };
}

function toTransferBuffer(data: Uint8Array): ArrayBuffer {
    if (data.byteOffset === 0 && data.byteLength === data.buffer.byteLength && data.buffer instanceof ArrayBuffer) {
        return data.buffer;
    }

    const copy = new Uint8Array(data.byteLength);
    copy.set(data);
    return copy.buffer;
}
