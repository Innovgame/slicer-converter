import { describe, expect, it, vi } from "vitest";

import {
    buildCentralHeader,
    buildDataDescriptor,
    buildEocd,
    buildLocalHeader,
    createZipSource,
    encodeZipEntryName,
    parseZipDirectoryFromReader,
    readZipEntryText
} from "../../src/index.js";

function hex(str: string): Uint8Array {
    const clean = str.replace(/[^0-9a-f]/gi, "");
    const arr = new Uint8Array(clean.length / 2);
    for (let i = 0; i < arr.length; i += 1) {
        arr[i] = Number.parseInt(clean.slice(i * 2, i * 2 + 2), 16);
    }
    return arr;
}

function concat(parts: Uint8Array[]): Uint8Array {
    const result = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
    let offset = 0;
    for (const part of parts) {
        result.set(part, offset);
        offset += part.length;
    }
    return result;
}

describe("zip source / ZIP source", () => {
    it("reads compressed payloads on demand from a range reader / reads compressed payloads on demand 来自 a range reader", async () => {
        const header = new Uint8Array(31);
        const view = new DataView(header.buffer);
        view.setUint32(0, 0x04034b50, true);
        view.setUint16(26, 1, true);
        view.setUint16(28, 0, true);
        header[30] = "a".charCodeAt(0);

        const readRange = vi.fn(async (offset: number, size: number) => {
            if (offset === 100 && size === 30) {
                return header.subarray(0, 30);
            }

            if (offset === 131 && size === 3) {
                return new Uint8Array([1, 2, 3]);
            }

            throw new Error(`Unexpected range request ${offset}:${size}`);
        });

        const source = createZipSource({ readRange });
        const result = await source.readCompressedEntryPayload(100, 3);

        expect(Array.from(result)).toEqual([1, 2, 3]);
        expect(readRange).toHaveBeenNthCalledWith(1, 100, 30);
        expect(readRange).toHaveBeenNthCalledWith(2, 131, 3);
    });

    it("parses the central directory from a reader without needing a full archive buffer / 解析 the central directory 来自 a reader 不通过 needing a 完整 archive buffer", async () => {
        const zipBytes = hex(`
          504b030414000000000092588e5c1d80bc55030000000300000003000000666f6f010203
          504b0102140014000000000092588e5c1d80bc5503000000030000000300000000000000
          00000000800100000000666f6f504b0506000000000100010031000000240000000000
        `);
        const source = createZipSource({
            readRange: vi.fn(async (offset: number, size: number) => zipBytes.subarray(offset, offset + size))
        });

        const directory = await parseZipDirectoryFromReader(source, zipBytes.length);

        expect(directory.entries).toEqual([
            {
                index: 0,
                name: "foo",
                compressionMethod: 0,
                compressedSize: 3,
                uncompressedSize: 3,
                localHeaderOffset: 0,
                usesDataDescriptor: false
            }
        ]);
    });

    it("exposes data descriptor mode while parsing from a range reader / 暴露 data descriptor mode 同时 parsing 来自 a range reader", async () => {
        const payload = new Uint8Array([1, 2, 3]);
        const nameBytes = encodeZipEntryName("foo");
        const prepared = {
            name: "foo",
            compressionMethod: 0 as const,
            compressedData: payload,
            uncompressedSize: payload.length,
            crc32: 0x1d80bc55
        };
        const localHeader = buildLocalHeader(prepared, nameBytes, { useDataDescriptor: true });
        const dataDescriptor = buildDataDescriptor({
            crc32: prepared.crc32,
            compressedSize: prepared.compressedData.length,
            uncompressedSize: prepared.uncompressedSize
        });
        const centralOffset = localHeader.length + payload.length + dataDescriptor.length;
        const centralHeader = buildCentralHeader({
            nameBytes,
            compressionMethod: prepared.compressionMethod,
            compressedSize: prepared.compressedData.length,
            uncompressedSize: prepared.uncompressedSize,
            crc32: prepared.crc32,
            localOffset: 0,
            usesDataDescriptor: true
        });
        const zipBytes = concat([
            localHeader,
            payload,
            dataDescriptor,
            centralHeader,
            buildEocd(1, centralHeader.length, centralOffset)
        ]);
        const source = createZipSource({
            readRange: vi.fn(async (offset: number, size: number) => zipBytes.subarray(offset, offset + size))
        });

        const directory = await parseZipDirectoryFromReader(source, zipBytes.length);

        expect(directory.entries[0]?.usesDataDescriptor).toBe(true);
    });

    it("reads text entries through the shared zip source helpers / reads text entries through the shared zip source helpers 用例", async () => {
        const header = new Uint8Array(31);
        const view = new DataView(header.buffer);
        view.setUint32(0, 0x04034b50, true);
        view.setUint16(26, 1, true);
        view.setUint16(28, 0, true);
        header[30] = "a".charCodeAt(0);

        const source = createZipSource({
            readRange: vi.fn(async (offset: number, size: number) => {
                if (offset === 200 && size === 30) {
                    return header.subarray(0, 30);
                }

                if (offset === 231 && size === 4) {
                    return new TextEncoder().encode("test");
                }

                throw new Error(`Unexpected range request ${offset}:${size}`);
            })
        });

        const text = await readZipEntryText(source, {
            index: 0,
            name: "a",
            compressionMethod: 0,
            compressedSize: 4,
            uncompressedSize: 4,
            localHeaderOffset: 200,
            usesDataDescriptor: false
        });

        expect(text).toBe("test");
    });
});
