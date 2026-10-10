import type { SlicerFormatId } from "../types.js";
import type { SlicerDocument } from "../document/document.js";
import type { SlicerWorkerDispatchPayload } from "../worker/dispatch.js";
import type { SlicerRuntimeMetrics } from "../runtime/metrics.js";

/**
 * @zh 输出 ZIP entry 的目标描述，由 archive plan 决定名称和压缩方式。
 * @en Target descriptor for an output ZIP entry; the archive plan decides the entry name and compression method.
 */
export interface SlicerZipLayerDescriptor {
    readonly name: string;
    readonly compressionMethod: 0 | 8;
}

/**
 * @zh 当前 conversion session 选择的 ZIP worker 调度策略摘要。
 * @en Summary of the ZIP worker dispatch strategy selected by this conversion session.
 */
export interface SlicerZipStrategyPlan {
    readonly modeLabel: string;
}

/**
 * @zh 将已打开文档的 layer payload 转换为 conversion worker 请求。
 * @en Converts opened-document layer payloads into conversion worker requests.
 */
export interface SlicerConversionSession {
    buildLayerDispatchPayload(index: number): Promise<SlicerWorkerDispatchPayload>;
    buildPreparedZipLayerPayload(
        index: number,
        descriptor: SlicerZipLayerDescriptor
    ): Promise<SlicerWorkerDispatchPayload>;
    planZipStrategy(): SlicerZipStrategyPlan;
    close(): Promise<void>;
}

/**
 * @zh 创建 conversion 读取会话，集中处理 raw/ZIP payload 到 worker protocol 的映射和 transfer 统计。
 * @en Creates a conversion read session that centralizes raw/ZIP payload to worker protocol mapping and transfer metrics.
 */
export function createSlicerConversionSession(input: {
    readonly document: SlicerDocument;
    readonly targetFormat: SlicerFormatId;
    readonly targetAnycubicVersion?: number;
    readonly width: number;
    readonly height: number;
    readonly layerMetadataJson: string | null;
    readonly workerCount: number;
    readonly metrics: SlicerRuntimeMetrics;
}): SlicerConversionSession {
    const sourceFormat = input.document.parsedHeader.format;
    const antiAliasLevel = input.document.parsedHeader.metadata.anti_alias_level ?? 1;
    const encryptionKey = input.document.parsedHeader.metadata.encryption_key ?? 0;
    const workerCount = Math.max(1, input.workerCount);

    async function buildLayerDispatchPayload(index: number): Promise<SlicerWorkerDispatchPayload> {
        const layerPayload = await input.document.readLayerPayload(index);
        const transfer = toTransferBuffer(layerPayload.bytes, input.metrics);

        if (layerPayload.kind === "zip-entry") {
            return {
                transfer,
                request: {
                    type: "zip-decode-encode",
                    index,
                    data: transfer,
                    compressionMethod: layerPayload.compressionMethod,
                    uncompressedSize: layerPayload.uncompressedSize,
                    sourceFormat,
                    targetFormat: input.targetFormat,
                    targetAnycubicVersion: input.targetAnycubicVersion ?? null,
                    width: input.width,
                    height: input.height,
                    layerMetadataJson: input.layerMetadataJson
                }
            };
        }

        return {
            transfer,
            request: {
                type: "decode-encode",
                index,
                data: transfer,
                sourceFormat,
                targetFormat: input.targetFormat,
                targetAnycubicVersion: input.targetAnycubicVersion ?? null,
                width: input.width,
                height: input.height,
                antiAliasLevel,
                encryptionKey,
                layerMetadataJson: input.layerMetadataJson
            }
        };
    }

    async function buildPreparedZipLayerPayload(
        index: number,
        descriptor: SlicerZipLayerDescriptor
    ): Promise<SlicerWorkerDispatchPayload> {
        const layerPayload = await input.document.readLayerPayload(index);
        const transfer = toTransferBuffer(layerPayload.bytes, input.metrics);

        if (layerPayload.kind === "zip-entry") {
            return {
                transfer,
                request: {
                    type: "zip-decode-encode-prepare",
                    index,
                    data: transfer,
                    compressionMethod: layerPayload.compressionMethod,
                    uncompressedSize: layerPayload.uncompressedSize,
                    sourceFormat,
                    targetFormat: input.targetFormat,
                    targetAnycubicVersion: input.targetAnycubicVersion ?? null,
                    width: input.width,
                    height: input.height,
                    layerMetadataJson: input.layerMetadataJson,
                    entryName: descriptor.name,
                    entryCompressionMethod: descriptor.compressionMethod
                }
            };
        }

        return {
            transfer,
            request: {
                type: "decode-encode-prepare",
                index,
                data: transfer,
                sourceFormat,
                targetFormat: input.targetFormat,
                targetAnycubicVersion: input.targetAnycubicVersion ?? null,
                width: input.width,
                height: input.height,
                antiAliasLevel,
                encryptionKey,
                layerMetadataJson: input.layerMetadataJson,
                entryName: descriptor.name,
                entryCompressionMethod: descriptor.compressionMethod
            }
        };
    }

    function planZipStrategy(): SlicerZipStrategyPlan {
        const modeLabel = `workers x${workerCount}`;
        input.metrics.recordConversionMode(modeLabel);
        return { modeLabel };
    }

    return {
        buildLayerDispatchPayload,
        buildPreparedZipLayerPayload,
        planZipStrategy,
        async close() {}
    };
}

function toTransferBuffer(data: Uint8Array, metrics: SlicerRuntimeMetrics): ArrayBuffer {
    if (data.byteOffset === 0 && data.byteLength === data.buffer.byteLength) {
        metrics.recordTransferBuffer({ byteLength: data.byteLength, copied: false });
        return data.buffer as ArrayBuffer;
    }

    const copy = new Uint8Array(data.byteLength);
    copy.set(data);
    metrics.recordTransferBuffer({ byteLength: data.byteLength, copied: true });
    return copy.buffer as ArrayBuffer;
}
