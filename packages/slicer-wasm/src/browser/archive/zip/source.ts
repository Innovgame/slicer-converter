import { parseZipCentralDirectoryEntries, type ZipDirectory, type ZipEntryInfo } from "./directory.js";

const LOCAL_FILE_HEADER_SIGNATURE = 0x04034b50;
const LOCAL_FILE_HEADER_SIZE = 30;
const EOCD_SIGNATURE = 0x06054b50;
const MAX_EOCD_SEARCH = 65557;

/**
 * @zh ZIP byte-range reader；用于浏览器按需读取 ZIP 目录与 entry payload。
 * @en ZIP byte-range reader used by browser openers to lazily read directories and entry payloads.
 */
export interface ZipSource {
    readRange(offset: number, size: number): Promise<Uint8Array>;
    readCompressedEntryPayload(localHeaderOffset: number, compressedSize: number): Promise<Uint8Array>;
}

/**
 * @zh 根据底层 `readRange` 回调创建一个自定义的 ZIP source。
 * @en Creates a custom ZIP source backed by a `readRange` callback.
 */
export function createZipSource(input: { readRange(offset: number, size: number): Promise<Uint8Array> }): ZipSource {
    return {
        readRange(offset, size) {
            return input.readRange(offset, size);
        },
        async readCompressedEntryPayload(localHeaderOffset, compressedSize) {
            const localHeader = await input.readRange(localHeaderOffset, LOCAL_FILE_HEADER_SIZE);
            const view = new DataView(localHeader.buffer, localHeader.byteOffset, localHeader.byteLength);

            if (view.getUint32(0, true) !== LOCAL_FILE_HEADER_SIGNATURE) {
                throw new Error(`Invalid Local File Header signature at offset ${localHeaderOffset}`);
            }

            const localNameLength = view.getUint16(26, true);
            const localExtraLength = view.getUint16(28, true);
            const payloadOffset = localHeaderOffset + LOCAL_FILE_HEADER_SIZE + localNameLength + localExtraLength;

            return input.readRange(payloadOffset, compressedSize);
        }
    };
}

/**
 * @zh 创建一个封装已有 bytes 的内存 ZIP source。
 * @en Creates an in-memory ZIP source wrapping existing bytes.
 */
export function createInMemoryZipSource(data: Uint8Array): ZipSource {
    return createZipSource({
        async readRange(offset, size) {
            return data.subarray(offset, offset + size);
        }
    });
}

/**
 * @zh 创建一个基于浏览器 `File` 的 ZIP source，使用 `File.slice()` 按需读取。
 * @en Creates a browser `File` backed ZIP source using lazy `File.slice()` reads.
 */
export function createFileZipSource(file: File): ZipSource {
    return createZipSource({
        async readRange(offset, size) {
            return new Uint8Array(await file.slice(offset, offset + size).arrayBuffer());
        }
    });
}

/**
 * @zh 从 ZIP source 搜索 EOCD 并解析 Central Directory。
 * @en Locates EOCD in a ZIP source and parses the Central Directory.
 */
export async function parseZipDirectoryFromReader(source: ZipSource, totalSize: number): Promise<ZipDirectory> {
    const tailLength = Math.min(totalSize, MAX_EOCD_SEARCH);
    const tailStart = totalSize - tailLength;
    const tail = await source.readRange(tailStart, tailLength);
    const tailView = new DataView(tail.buffer, tail.byteOffset, tail.byteLength);

    let eocdOffset = -1;
    for (let index = tail.length - 22; index >= 0; index -= 1) {
        if (tailView.getUint32(index, true) === EOCD_SIGNATURE) {
            eocdOffset = index;
            break;
        }
    }

    if (eocdOffset === -1) {
        throw new Error("ZIP EOCD record not found");
    }

    const totalEntries = tailView.getUint16(eocdOffset + 10, true);
    const centralDirectorySize = tailView.getUint32(eocdOffset + 12, true);
    const centralDirectoryOffset = tailView.getUint32(eocdOffset + 16, true);
    const tailEnd = tailStart + tail.length;

    const centralDirectory =
        centralDirectoryOffset >= tailStart && centralDirectoryOffset + centralDirectorySize <= tailEnd
            ? tail.subarray(
                  centralDirectoryOffset - tailStart,
                  centralDirectoryOffset - tailStart + centralDirectorySize
              )
            : await source.readRange(centralDirectoryOffset, centralDirectorySize);

    return {
        entries: parseZipCentralDirectoryEntries(centralDirectory, totalEntries)
    };
}

/**
 * @zh 读取 ZIP entry 解压后的 bytes；支持 stored 与 deflate-raw。
 * @en Reads decompressed ZIP entry bytes; supports stored and deflate-raw entries.
 */
export async function readZipEntryBytes(source: ZipSource, entry: ZipEntryInfo): Promise<Uint8Array> {
    const compressed = await source.readCompressedEntryPayload(entry.localHeaderOffset, entry.compressedSize);

    if (entry.compressionMethod === 0) {
        return compressed;
    }

    if (entry.compressionMethod === 8) {
        return inflateRaw(compressed, entry.uncompressedSize);
    }

    throw new Error(`Unsupported ZIP compression method: ${entry.compressionMethod}`);
}

/**
 * @zh 读取 ZIP entry 解压后的 UTF-8 text。
 * @en Reads a ZIP entry as decompressed UTF-8 text.
 */
export async function readZipEntryText(source: ZipSource, entry: ZipEntryInfo): Promise<string> {
    return new TextDecoder().decode(await readZipEntryBytes(source, entry));
}

/**
 * @zh 使用浏览器 `DecompressionStream` 解压 raw deflate payload。
 * @en Inflates a raw deflate payload using browser `DecompressionStream`.
 */
export async function inflateRaw(data: Uint8Array, expectedSize?: number): Promise<Uint8Array> {
    if (typeof DecompressionStream !== "function") {
        throw new Error("ZIP deflate decompression requires browser DecompressionStream support");
    }

    const stream = new DecompressionStream("deflate-raw" as CompressionFormat);
    const writer = stream.writable.getWriter();
    const reader = stream.readable.getReader();
    const writePromise = writer.write(data as BufferSource).then(() => writer.close());
    if (expectedSize === undefined) {
        const chunks: Uint8Array[] = [];
        let totalLength = 0;

        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) {
                    break;
                }
                chunks.push(value);
                totalLength += value.byteLength;
            }

            await writePromise;
        } catch (error) {
            await reader.cancel(error).catch(() => {});
            await writePromise.catch(() => {});
            throw error;
        }

        if (chunks.length === 1) {
            return chunks[0]!;
        }

        const inflated = new Uint8Array(totalLength);
        let offset = 0;
        for (const chunk of chunks) {
            inflated.set(chunk, offset);
            offset += chunk.byteLength;
        }
        return inflated;
    }

    const inflated = new Uint8Array(expectedSize);
    let offset = 0;

    try {
        while (true) {
            const { done, value } = await reader.read();
            if (done) {
                break;
            }

            if (offset + value.byteLength > inflated.byteLength) {
                throw new Error(`Inflated data exceeded expected uncompressed size: expected ${expectedSize} bytes`);
            }

            inflated.set(value, offset);
            offset += value.byteLength;
        }

        await writePromise;
    } catch (error) {
        await reader.cancel(error).catch(() => {});
        await writePromise.catch(() => {});
        throw error;
    }

    if (offset !== expectedSize) {
        throw new Error(
            `Inflated data did not match expected uncompressed size: expected ${expectedSize} bytes, got ${offset}`
        );
    }

    return inflated;
}
