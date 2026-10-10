import type { ParsedHeader } from "../types.js";

/**
 * @zh 切片文档的 layer 存储来源；raw 文件按 header offset 延迟切片，ZIP 文件通过 archive adapter 保持压缩 payload 透传。
 * @en Layer storage source for a slicer document; raw files are lazily sliced by
 * header offsets, while ZIP files pass compressed payloads through an archive adapter.
 */
export type SlicerLayerSource =
    | { kind: "raw-file" }
    | {
          kind: "zip";
          readCompressedLayerPayload(index: number): Promise<CompressedZipLayerPayload>;
      };

/**
 * @zh ZIP-backed layer 的压缩 payload；解压由 preview/conversion worker 负责。
 * @en Compressed payload for a ZIP-backed layer; preview/conversion workers handle inflation.
 */
export interface CompressedZipLayerPayload {
    readonly bytes: Uint8Array;
    readonly compressionMethod: number;
    readonly uncompressedSize: number;
}

/**
 * @zh 单层原始输入 payload；调用方通过 `kind` 显式区分 encoded layer 与 ZIP entry。
 * @en Raw input payload for one layer; callers branch explicitly on `kind` for encoded layers and ZIP entries.
 */
export type SlicerLayerPayload =
    | {
          readonly kind: "encoded-layer";
          readonly bytes: Uint8Array;
      }
    | {
          readonly kind: "zip-entry";
          readonly bytes: Uint8Array;
          readonly compressionMethod: number;
          readonly uncompressedSize: number;
      };

/**
 * @zh 浏览器端已打开切片文档的稳定读取边界；隐藏文件切片策略和 ZIP entry table。
 * @en Stable browser read boundary for an opened slicer document; hides file slicing strategy and ZIP entry tables.
 */
export interface SlicerDocument {
    readonly sourceFile: File;
    readonly sourceName: string;
    readonly sourceSize: number;
    readonly parsedHeader: ParsedHeader;
    readLayerPayload(index: number): Promise<SlicerLayerPayload>;
    close(): Promise<void>;
}

/**
 * @zh 把 opener 返回的 source 信息封装成统一的 layer payload reader。
 * @en Wraps source information returned by an opener into a unified layer payload reader.
 */
export function createSlicerDocument(input: {
    sourceFile: File;
    parsedHeader: ParsedHeader;
    layerSource: SlicerLayerSource;
}): SlicerDocument {
    return {
        sourceFile: input.sourceFile,
        sourceName: input.sourceFile.name,
        sourceSize: input.sourceFile.size,
        parsedHeader: input.parsedHeader,
        async readLayerPayload(index) {
            return readLayerPayloadFromSource(input.layerSource, input.sourceFile, input.parsedHeader, index);
        },
        async close() {}
    };
}

async function readLayerPayloadFromSource(
    layerSource: SlicerLayerSource,
    sourceFile: File,
    parsedHeader: ParsedHeader,
    index: number
): Promise<SlicerLayerPayload> {
    if (layerSource.kind === "zip") {
        return readZipLayerPayloadFromSource(layerSource, index);
    }

    return {
        kind: "encoded-layer",
        bytes: await readEncodedLayerBytesFromSource(sourceFile, parsedHeader, index)
    };
}

async function readEncodedLayerBytesFromSource(
    sourceFile: File,
    parsedHeader: ParsedHeader,
    index: number
): Promise<Uint8Array> {
    const layer = parsedHeader.layers[index];
    if (!layer) {
        throw new Error(`Layer ${index} not found`);
    }

    return new Uint8Array(await sourceFile.slice(layer.offset, layer.offset + layer.size).arrayBuffer());
}

function readZipLayerPayloadFromSource(
    layerSource: Extract<SlicerLayerSource, { kind: "zip" }>,
    index: number
): Promise<Extract<SlicerLayerPayload, { kind: "zip-entry" }>> {
    return layerSource.readCompressedLayerPayload(index).then((payload) => ({
        kind: "zip-entry",
        bytes: payload.bytes,
        compressionMethod: payload.compressionMethod,
        uncompressedSize: payload.uncompressedSize
    }));
}
