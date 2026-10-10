import {
    parseZipCentralDirectoryEntries,
    type ZipDirectory,
    type ZipEntryInfo
} from "../../packages/slicer-wasm/src/browser/archive/zip/directory.js";

const EOCD_SIGNATURE = 0x06054b50;
const LOCAL_FILE_HEADER_SIGNATURE = 0x04034b50;

/**
 * @zh 测试用 ZIP directory inspector；避免把 ZIP inspection helper 暴露为 package root API。
 * @en Test-only ZIP directory inspector; avoids exposing ZIP inspection helpers as package root API.
 */
export function parseZipDirectory(data: Uint8Array): ZipDirectory {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const eocdOffset = findEocdOffset(view);
    const entryCount = view.getUint16(eocdOffset + 10, true);
    const centralDirectorySize = view.getUint32(eocdOffset + 12, true);
    const centralDirectoryOffset = view.getUint32(eocdOffset + 16, true);
    return {
        entries: parseZipCentralDirectoryEntries(
            data.subarray(centralDirectoryOffset, centralDirectoryOffset + centralDirectorySize),
            entryCount
        )
    };
}

/**
 * @zh 返回测试 ZIP entry 的 compressed payload bytes。
 * @en Returns compressed payload bytes for a test ZIP entry.
 */
export function getCompressedData(data: Uint8Array, entry: ZipEntryInfo): Uint8Array {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    if (view.getUint32(entry.localHeaderOffset, true) !== LOCAL_FILE_HEADER_SIGNATURE) {
        throw new Error("Invalid local file header signature");
    }
    const fileNameLength = view.getUint16(entry.localHeaderOffset + 26, true);
    const extraLength = view.getUint16(entry.localHeaderOffset + 28, true);
    const start = entry.localHeaderOffset + 30 + fileNameLength + extraLength;
    return data.subarray(start, start + entry.compressedSize);
}

function findEocdOffset(view: DataView): number {
    const minOffset = Math.max(0, view.byteLength - 65_557);
    for (let offset = view.byteLength - 22; offset >= minOffset; offset -= 1) {
        if (view.getUint32(offset, true) === EOCD_SIGNATURE) {
            return offset;
        }
    }
    throw new Error("EOCD not found");
}
