import { crc32Bytes } from "../../crc.js";

/**
 * @zh ZIP 压缩算法编号：0 = 存储（不压缩），8 = Deflate。
 * @en ZIP compression method code: 0 = stored (no compression), 8 = Deflate.
 */
export type ZipArchiveCompressionMethod = 0 | 8;

/**
 * @zh 要写入 ZIP archive 的单个条目。
 * @en A single entry to write into a ZIP archive.
 */
export interface ZipArchiveEntry {
    name: string;
    data: Uint8Array;
    compressionMethod: ZipArchiveCompressionMethod;
}

/**
 * @zh 已解析 CRC 与大小的 ZIP 条目，可直接写入 local header、payload 与 central directory。
 * @en A ZIP entry with CRC and sizes resolved, ready for local header, payload, and central directory writes.
 */
export interface ZipPreparedEntry {
    name: string;
    compressionMethod: ZipArchiveCompressionMethod;
    compressedData: Uint8Array;
    uncompressedSize: number;
    crc32: number;
}

/**
 * @zh 单个 ZIP 条目 prepare 阶段的耗时明细。
 * @en Timing breakdown for preparing one ZIP entry.
 */
export interface ZipPrepareTimings {
    crcMs: number;
    deflateMs: number;
    packMs: number;
    totalMs: number;
}

/**
 * @zh 带 prepare 耗时明细的 ZIP 条目结果。
 * @en Prepared ZIP entry result with prepare timing diagnostics.
 */
export interface ZipPreparedEntryWithTimings {
    entry: ZipPreparedEntry;
    timings: ZipPrepareTimings;
}

/**
 * @zh 构造 central directory header 所需的条目描述。
 * @en Entry descriptor required to build a central directory header.
 */
export interface ZipCentralDescriptor {
    nameBytes: Uint8Array;
    compressionMethod: ZipArchiveCompressionMethod;
    compressedSize: number;
    uncompressedSize: number;
    crc32: number;
    localOffset: number;
    usesDataDescriptor?: boolean;
}

/**
 * @zh 构造 local header 时的 ZIP 写入选项。
 * @en ZIP write options used when building a local header.
 */
export interface ZipLocalHeaderOptions {
    useDataDescriptor?: boolean;
}

/**
 * @zh 写在 payload 之后的 ZIP data descriptor 字段。
 * @en ZIP data descriptor fields emitted after the payload.
 */
export interface ZipDataDescriptor {
    crc32: number;
    compressedSize: number;
    uncompressedSize: number;
}

/**
 * @zh 已解析的 ZIP 条目元数据，可用于 descriptor-mode local header。
 * @en Resolved ZIP entry metadata that can be used for descriptor-mode local headers.
 */
export interface ZipResolvedEntryMetadata extends ZipDataDescriptor {
    compressionMethod: ZipArchiveCompressionMethod;
}

/**
 * @zh 可用于构造 local header 的 ZIP 条目形态。
 * @en ZIP entry shape accepted by local header construction.
 */
export type ZipLocalHeaderEntry = ZipPreparedEntry | ZipResolvedEntryMetadata;

const textEncoder = new TextEncoder();
const ZIP_DATA_DESCRIPTOR_FLAG = 0x0008;

/**
 * @zh 构造 ZIP local file header 字节（30 字节定长 + 文件名）。
 * @en Builds ZIP local file header bytes (30 fixed bytes plus filename).
 */
export function buildLocalHeader(
    entry: ZipLocalHeaderEntry,
    nameBytes: Uint8Array,
    options: ZipLocalHeaderOptions = {}
): Uint8Array {
    const localHeader = new Uint8Array(30 + nameBytes.length);
    const localView = new DataView(localHeader.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, 20, true);
    localView.setUint16(6, options.useDataDescriptor === true ? ZIP_DATA_DESCRIPTOR_FLAG : 0, true);
    localView.setUint16(8, entry.compressionMethod, true);
    localView.setUint16(10, 0, true);
    localView.setUint16(12, 0, true);
    localView.setUint32(14, options.useDataDescriptor === true ? 0 : entry.crc32, true);
    localView.setUint32(18, options.useDataDescriptor === true ? 0 : getCompressedSize(entry), true);
    localView.setUint32(22, options.useDataDescriptor === true ? 0 : entry.uncompressedSize, true);
    localView.setUint16(26, nameBytes.length, true);
    localView.setUint16(28, 0, true);
    localHeader.set(nameBytes, 30);
    return localHeader;
}

/**
 * @zh 构造 ZIP data descriptor 字节（含签名）。
 * @en Builds ZIP data descriptor bytes with signature.
 */
export function buildDataDescriptor(descriptor: ZipDataDescriptor): Uint8Array {
    const dataDescriptor = new Uint8Array(16);
    const descriptorView = new DataView(dataDescriptor.buffer);
    descriptorView.setUint32(0, 0x08074b50, true);
    descriptorView.setUint32(4, descriptor.crc32, true);
    descriptorView.setUint32(8, descriptor.compressedSize, true);
    descriptorView.setUint32(12, descriptor.uncompressedSize, true);
    return dataDescriptor;
}

/**
 * @zh 构造 ZIP central directory header 字节（46 字节定长 + 文件名）。
 * @en Builds ZIP central directory header bytes (46 fixed bytes plus filename).
 */
export function buildCentralHeader(descriptor: ZipCentralDescriptor): Uint8Array {
    const centralHeader = new Uint8Array(46 + descriptor.nameBytes.length);
    const centralView = new DataView(centralHeader.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, 20, true);
    centralView.setUint16(6, 20, true);
    centralView.setUint16(8, descriptor.usesDataDescriptor === true ? ZIP_DATA_DESCRIPTOR_FLAG : 0, true);
    centralView.setUint16(10, descriptor.compressionMethod, true);
    centralView.setUint16(12, 0, true);
    centralView.setUint16(14, 0, true);
    centralView.setUint32(16, descriptor.crc32, true);
    centralView.setUint32(20, descriptor.compressedSize, true);
    centralView.setUint32(24, descriptor.uncompressedSize, true);
    centralView.setUint16(28, descriptor.nameBytes.length, true);
    centralView.setUint16(30, 0, true);
    centralView.setUint16(32, 0, true);
    centralView.setUint16(34, 0, true);
    centralView.setUint16(36, 0, true);
    centralView.setUint32(38, 0, true);
    centralView.setUint32(42, descriptor.localOffset, true);
    centralHeader.set(descriptor.nameBytes, 46);
    return centralHeader;
}

/**
 * @zh 构造 ZIP End-Of-Central-Directory 记录字节。
 * @en Builds ZIP End-Of-Central-Directory record bytes.
 */
export function buildEocd(entryCount: number, centralSize: number, localOffset: number): Uint8Array {
    const eocd = new Uint8Array(22);
    const eocdView = new DataView(eocd.buffer);
    eocdView.setUint32(0, 0x06054b50, true);
    eocdView.setUint16(4, 0, true);
    eocdView.setUint16(6, 0, true);
    eocdView.setUint16(8, entryCount, true);
    eocdView.setUint16(10, entryCount, true);
    eocdView.setUint32(12, centralSize, true);
    eocdView.setUint32(16, localOffset, true);
    eocdView.setUint16(20, 0, true);
    return eocd;
}

/**
 * @zh 将 ZIP 条目名编码为 UTF-8 字节。
 * @en Encodes a ZIP entry name as UTF-8 bytes.
 */
export function encodeZipEntryName(name: string): Uint8Array {
    return textEncoder.encode(name);
}

/**
 * @zh 准备单个 ZIP 条目，解析 payload、未压缩大小与 CRC。
 * @en Prepares a single ZIP entry by resolving payload, uncompressed size, and CRC.
 */
export async function prepareZipEntry(entry: ZipArchiveEntry): Promise<ZipPreparedEntry> {
    return (await prepareZipEntryWithTimings(entry)).entry;
}

/**
 * @zh 准备单个 ZIP 条目并返回耗时诊断。
 * @en Prepares a single ZIP entry and returns timing diagnostics.
 */
export async function prepareZipEntryWithTimings(
    entry: ZipArchiveEntry,
    now: () => number = () => performance.now()
): Promise<ZipPreparedEntryWithTimings> {
    const startedAt = now();
    let payloadMs = 0;
    const compressedPromise = buildEntryPayload(entry, now, (elapsedMs) => {
        payloadMs = elapsedMs;
    });
    const crcStartedAt = now();
    const crc32 = crc32Bytes(entry.data);
    const crcEndedAt = now();
    const compressedData = await compressedPromise;
    const packStartedAt = now();
    const prepared = {
        name: entry.name,
        compressionMethod: entry.compressionMethod,
        compressedData,
        uncompressedSize: entry.data.length,
        crc32
    };
    const finishedAt = now();

    return {
        entry: prepared,
        timings: {
            crcMs: crcEndedAt - crcStartedAt,
            deflateMs: payloadMs,
            packMs: finishedAt - packStartedAt,
            totalMs: finishedAt - startedAt
        }
    };
}

function getCompressedSize(entry: ZipLocalHeaderEntry): number {
    if ("compressedData" in entry) {
        return entry.compressedData.length;
    }
    return entry.compressedSize;
}

async function buildEntryPayload(
    entry: ZipArchiveEntry,
    now: () => number,
    onTiming: (elapsedMs: number) => void
): Promise<Uint8Array> {
    const startedAt = now();
    let payloadPromise: Promise<Uint8Array>;
    if (entry.compressionMethod === 0) {
        payloadPromise = Promise.resolve(entry.data);
    } else if (entry.compressionMethod === 8) {
        payloadPromise = deflateRaw(entry.data);
    } else {
        throw new Error(`Unsupported ZIP compression method: ${entry.compressionMethod}`);
    }

    return payloadPromise.finally(() => {
        onTiming(now() - startedAt);
    });
}

async function deflateRaw(data: Uint8Array): Promise<Uint8Array> {
    if (typeof CompressionStream === "undefined") {
        throw new Error("CompressionStream is required for ZIP archive assembly");
    }

    const stream = new CompressionStream("deflate-raw");
    const writer = stream.writable.getWriter();
    const reader = stream.readable.getReader();
    const writePromise = writer.write(data as unknown as BufferSource).then(() => writer.close());

    const chunks: Uint8Array[] = [];
    let totalLength = 0;
    while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        totalLength += value.byteLength;
    }

    await writePromise;
    return concatUint8Arrays(chunks, totalLength);
}

function concatUint8Arrays(parts: Uint8Array[], knownLength?: number): Uint8Array {
    if (parts.length === 1) {
        return parts[0];
    }

    const totalLength = knownLength ?? parts.reduce((sum, part) => sum + part.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const part of parts) {
        result.set(part, offset);
        offset += part.length;
    }
    return result;
}
