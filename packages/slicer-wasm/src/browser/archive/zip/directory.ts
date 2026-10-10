/**
 * @zh ZIP Central Directory 的共享 entry facts 与 parser。
 * @en Shared ZIP Central Directory entry facts and parser.
 */

/**
 * @zh ZIP Central Directory 中记录的单个 entry facts。
 * @en Facts for one ZIP entry recorded in the Central Directory.
 */
export interface ZipEntryInfo {
    readonly index: number;
    readonly name: string;
    readonly compressionMethod: number;
    readonly compressedSize: number;
    readonly uncompressedSize: number;
    readonly localHeaderOffset: number;
    readonly usesDataDescriptor: boolean;
}

/**
 * @zh ZIP Central Directory 的解析结果。
 * @en Parsed ZIP Central Directory.
 */
export interface ZipDirectory {
    readonly entries: readonly ZipEntryInfo[];
}

const CENTRAL_DIRECTORY_SIGNATURE = 0x02014b50;
const DATA_DESCRIPTOR_FLAG = 0x0008;

/**
 * @zh 从 Central Directory 字节解析 entry facts，不读取 EOCD，也不读取 payload。
 * @en Parses entry facts from Central Directory bytes; it does not read EOCD or payloads.
 */
export function parseZipCentralDirectoryEntries(data: Uint8Array, totalEntries: number): ZipEntryInfo[] {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const entries: ZipEntryInfo[] = [];
    let offset = 0;

    for (let index = 0; index < totalEntries; index += 1) {
        if (offset + 46 > data.length) {
            throw new Error(`ZIP Central Directory truncated at entry ${index}`);
        }

        if (view.getUint32(offset, true) !== CENTRAL_DIRECTORY_SIGNATURE) {
            throw new Error(`Invalid Central Directory signature at offset ${offset}`);
        }

        const generalPurposeBitFlag = view.getUint16(offset + 8, true);
        const compressionMethod = view.getUint16(offset + 10, true);
        const compressedSize = view.getUint32(offset + 20, true);
        const uncompressedSize = view.getUint32(offset + 24, true);
        const nameLength = view.getUint16(offset + 28, true);
        const extraLength = view.getUint16(offset + 30, true);
        const commentLength = view.getUint16(offset + 32, true);
        const localHeaderOffset = view.getUint32(offset + 42, true);
        const recordLength = 46 + nameLength + extraLength + commentLength;

        if (offset + recordLength > data.length) {
            throw new Error(`ZIP Central Directory truncated at entry ${index}`);
        }

        const nameBytes = data.subarray(offset + 46, offset + 46 + nameLength);
        const name = new TextDecoder().decode(nameBytes);

        entries.push({
            index,
            name,
            compressionMethod,
            compressedSize,
            uncompressedSize,
            localHeaderOffset,
            usesDataDescriptor: (generalPurposeBitFlag & DATA_DESCRIPTOR_FLAG) !== 0
        });

        offset += recordLength;
    }

    return entries;
}
