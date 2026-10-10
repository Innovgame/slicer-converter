import { describe, expect, it } from "vitest";

import { buildCentralHeader, encodeZipEntryName, parseZipCentralDirectoryEntries } from "../../src/index.js";

describe("parseZipCentralDirectoryEntries / ZIP central directory entry parser", () => {
    it("parses central directory entries from a standalone directory buffer / 从 standalone directory buffer 解析 entries", () => {
        const nameBytes = encodeZipEntryName("foo");
        const centralDirectory = buildCentralHeader({
            nameBytes,
            compressionMethod: 8,
            compressedSize: 3,
            uncompressedSize: 5,
            crc32: 0x1d80bc55,
            localOffset: 12,
            usesDataDescriptor: true
        });

        expect(parseZipCentralDirectoryEntries(centralDirectory, 1)).toEqual([
            {
                index: 0,
                name: "foo",
                compressionMethod: 8,
                compressedSize: 3,
                uncompressedSize: 5,
                localHeaderOffset: 12,
                usesDataDescriptor: true
            }
        ]);
    });
});
