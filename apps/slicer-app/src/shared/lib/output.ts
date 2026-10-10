import type { ExportTargetFormat } from "@innovgame/slicer-converter";

/**
 * @zh 根据源文件名和目标格式构建下载文件名。
 * @en Builds a download filename from source filename and target format.
 *
 * @param sourceFileName - @zh 原始文件名（含扩展名），`null/undefined` 时使用 `"converted"` @en Original filename with extension; falls back to `"converted"` when null/undefined
 * @param targetFormat - @zh 请求的目标格式 id @en Requested target format id
 * @param resolvedExtension - @zh 用于文件名的真实扩展名 @en Concrete filename extension
 */
export function buildDownloadFilename(
    sourceFileName: string | null | undefined,
    targetFormat: ExportTargetFormat,
    resolvedExtension?: string
): string {
    const baseName = sourceFileName ? sourceFileName.replace(/\.[^.]+$/, "") : "converted";
    const extension = resolvedExtension ?? targetFormat;
    return `${baseName}.${extension}`;
}

/**
 * @zh 将 `Uint8Array` 转换为属于该缓冲区的 `ArrayBuffer`，必要时复制以避免共享缓冲区问题。
 * @en Converts a `Uint8Array` to an owned `ArrayBuffer`, copying when necessary
 * to avoid shared-buffer issues.
 *
 * @param data - @zh 要转换的字节数组 @en Byte array to convert
 */
export function toOwnedArrayBuffer(data: Uint8Array): ArrayBuffer {
    if (data.byteOffset === 0 && data.byteLength === data.buffer.byteLength) {
        return data.buffer as ArrayBuffer;
    }
    const result = new Uint8Array(data.byteLength);
    result.set(data);
    return result.buffer as ArrayBuffer;
}

/**
 * @zh 将头部、层块、尾部押平为 `ArrayBuffer` 数组，将中性点为空的尾部制读。
 * @en Flattens header, layer chunks, and footer into an `ArrayBuffer` array,
 * omitting a zero-length footer.
 *
 * @param headerBytes - @zh 文件头字节 @en File header bytes
 * @param layerChunks - @zh 每个图层的编码内容 @en Encoded bytes for each layer
 * @param footerBytes - @zh 文件尾字节，长度为 0 时将被忽略 @en File footer bytes; ignored when length is 0
 */
export function buildBinaryParts(
    headerBytes: Uint8Array,
    layerChunks: Uint8Array[],
    footerBytes: Uint8Array
): ArrayBuffer[] {
    const parts = [toOwnedArrayBuffer(headerBytes), ...layerChunks.map(toOwnedArrayBuffer)];

    if (footerBytes.length > 0) {
        parts.push(toOwnedArrayBuffer(footerBytes));
    }

    return parts;
}
