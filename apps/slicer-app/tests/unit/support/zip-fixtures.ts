const STORED_SINGLE_ENTRY_ZIP_HEX = `
  504b030414000000000092588e5c1d80bc55030000000300000003000000666f6f010203
  504b0102140014000000000092588e5c1d80bc5503000000030000000300000000000000
  00000000800100000000666f6f504b0506000000000100010031000000240000000000
`;

/**
 * @zh 创建一个包含单个 stored entry 的最小 ZIP fixture。
 * @en Creates a minimal ZIP fixture with one stored entry.
 */
export function createStoredSingleEntryZipBytes(): Uint8Array {
    return hexToBytes(STORED_SINGLE_ENTRY_ZIP_HEX);
}

/**
 * @zh 将字节视图转换为紧凑的 `ArrayBuffer`。
 * @en Converts a byte view to a compact `ArrayBuffer`.
 */
export function bytesToArrayBuffer(bytes: Uint8Array): ArrayBuffer {
    return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

function hexToBytes(str: string): Uint8Array {
    const clean = str.replace(/[^0-9a-f]/gi, "");
    const arr = new Uint8Array(clean.length / 2);
    for (let index = 0; index < arr.length; index += 1) {
        arr[index] = Number.parseInt(clean.slice(index * 2, index * 2 + 2), 16);
    }
    return arr;
}
