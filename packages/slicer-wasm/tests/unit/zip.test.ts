import { describe, expect, it } from "vitest";

import {
    buildCentralHeader,
    buildDataDescriptor,
    buildEocd,
    buildLocalHeader,
    encodeZipEntryName
} from "../../src/index.js";
import { getCompressedData, parseZipDirectory } from "../../../../tests/support/zip-inspection.js";

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

describe("parseZipDirectory / ZIP 目录解析", () => {
    it("parses a minimal ZIP with one stored entry / 解析 a minimal ZIP 带有 one stored entry", () => {
        const zipHex = `
      504b030414000000000092588e5c1d80bc55030000000300000003000000666f6f010203
      504b0102140014000000000092588e5c1d80bc5503000000030000000300000000000000
      00000000800100000000666f6f504b0506000000000100010031000000240000000000
    `;
        const zip = hex(zipHex);
        const dir = parseZipDirectory(zip);
        expect(dir.entries.length).toBe(1);
        const entry = dir.entries[0]!;
        expect(entry.name).toBe("foo");
        expect(entry.compressionMethod).toBe(0);
        expect(entry.compressedSize).toBe(3);
        expect(entry.uncompressedSize).toBe(3);
        expect(entry.localHeaderOffset).toBe(0);
        const payload = getCompressedData(zip, entry);
        expect(Array.from(payload)).toEqual([1, 2, 3]);
    });

    it("exposes descriptor-mode entries and still extracts only compressed payload bytes / 暴露 descriptor-mode entries 并且 still extracts only compressed payload 字节", () => {
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
        const zip = concat([
            localHeader,
            payload,
            dataDescriptor,
            centralHeader,
            buildEocd(1, centralHeader.length, centralOffset)
        ]);

        const dir = parseZipDirectory(zip);
        const entry = dir.entries[0]!;

        expect(entry.usesDataDescriptor).toBe(true);
        expect(Array.from(getCompressedData(zip, entry))).toEqual([1, 2, 3]);
    });

    it("throws on missing EOCD / 抛出 on 缺失 EOCD", () => {
        const badZip = new Uint8Array([0, 1, 2, 3, 4]);
        expect(() => parseZipDirectory(badZip)).toThrow(/EOCD/);
    });
});
