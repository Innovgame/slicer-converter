import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
    crc32Bytes: vi.fn()
}));

vi.mock("../../src/browser/crc.js", () => ({
    crc32Bytes: mocks.crc32Bytes
}));

describe("ZIP archive browser helpers / ZIP archive 浏览器辅助函数", () => {
    it("does not export full archive assembly / 不导出完整 archive 组装", async () => {
        const mod = await import("../../src/index.js");

        expect(mod).not.toHaveProperty("assembleZipArchive");
    });

    it("prepares stored entries with original payload and mocked CRC / 准备 stored 条目并保留原始 payload 与 mocked CRC", async () => {
        const { prepareZipEntry } = await import("../../src/browser/archive/zip/archive.js");
        const data = new Uint8Array([0x7b, 0x7d]);
        mocks.crc32Bytes.mockReturnValueOnce(0x12345678);

        const prepared = await prepareZipEntry({ name: "manifest.json", data, compressionMethod: 0 });

        expect(mocks.crc32Bytes).toHaveBeenCalledWith(data);
        expect(prepared).toEqual({
            name: "manifest.json",
            compressionMethod: 0,
            compressedData: data,
            uncompressedSize: data.length,
            crc32: 0x12345678
        });
    });

    it("reports prepare timings without changing entry shape / 返回 prepare timings 且不改变 entry shape", async () => {
        const { prepareZipEntryWithTimings } = await import("../../src/browser/archive/zip/archive.js");
        const data = new Uint8Array([0x01, 0x02, 0x03]);
        const times = [0, 2, 5, 9, 11];
        mocks.crc32Bytes.mockReturnValueOnce(0x87654321);

        const timed = await prepareZipEntryWithTimings(
            { name: "layers/0.png", data, compressionMethod: 0 },
            () => times.shift() ?? 11
        );

        expect(timed.entry).toEqual({
            name: "layers/0.png",
            compressionMethod: 0,
            compressedData: data,
            uncompressedSize: data.length,
            crc32: 0x87654321
        });
        expect(timed.timings).toEqual({
            crcMs: expect.any(Number),
            deflateMs: expect.any(Number),
            packMs: expect.any(Number),
            totalMs: expect.any(Number)
        });
    });

    it("throws for unsupported compression method / 遇到 unsupported compression method 时抛错", async () => {
        const { prepareZipEntry } = await import("../../src/browser/archive/zip/archive.js");

        await expect(
            prepareZipEntry({
                name: "bad.bin",
                data: new Uint8Array([1]),
                compressionMethod: 99 as never
            })
        ).rejects.toThrow("Unsupported ZIP compression method: 99");
    });

    it("encodes descriptor-mode local header with deferred CRC and sizes / 编码 descriptor-mode local header 并延后 CRC 与 size", async () => {
        const { buildLocalHeader, encodeZipEntryName } = await import("../../src/browser/archive/zip/archive.js");
        const entry = {
            name: "0.png",
            compressionMethod: 8 as const,
            compressedData: new Uint8Array([0xde, 0xad, 0xbe, 0xef]),
            uncompressedSize: 9,
            crc32: 0x12345678
        };
        const localHeader = buildLocalHeader(entry, encodeZipEntryName(entry.name), { useDataDescriptor: true });
        const view = new DataView(localHeader.buffer, localHeader.byteOffset, localHeader.byteLength);

        expect(view.getUint32(0, true)).toBe(0x04034b50);
        expect(view.getUint16(6, true) & 0x0008).toBe(0x0008);
        expect(view.getUint16(8, true)).toBe(8);
        expect(view.getUint32(14, true)).toBe(0);
        expect(view.getUint32(18, true)).toBe(0);
        expect(view.getUint32(22, true)).toBe(0);
    });

    it("encodes central header and data descriptor fields / 编码 central header 与 data descriptor 字段", async () => {
        const { buildCentralHeader, buildDataDescriptor, encodeZipEntryName } = await import(
            "../../src/browser/archive/zip/archive.js"
        );
        const nameBytes = encodeZipEntryName("0.png");
        const centralHeader = buildCentralHeader({
            nameBytes,
            compressionMethod: 8,
            compressedSize: 4,
            uncompressedSize: 9,
            crc32: 0x12345678,
            localOffset: 42,
            usesDataDescriptor: true
        });
        const centralView = new DataView(centralHeader.buffer, centralHeader.byteOffset, centralHeader.byteLength);

        expect(centralView.getUint32(0, true)).toBe(0x02014b50);
        expect(centralView.getUint16(8, true) & 0x0008).toBe(0x0008);
        expect(centralView.getUint16(10, true)).toBe(8);
        expect(centralView.getUint32(16, true)).toBe(0x12345678);
        expect(centralView.getUint32(20, true)).toBe(4);
        expect(centralView.getUint32(24, true)).toBe(9);
        expect(centralView.getUint32(42, true)).toBe(42);
        expect(Array.from(centralHeader.slice(46))).toEqual(Array.from(nameBytes));

        const dataDescriptor = buildDataDescriptor({
            crc32: 0x12345678,
            compressedSize: 4,
            uncompressedSize: 9
        });
        const descriptorView = new DataView(
            dataDescriptor.buffer,
            dataDescriptor.byteOffset,
            dataDescriptor.byteLength
        );

        expect(dataDescriptor).toHaveLength(16);
        expect(descriptorView.getUint32(0, true)).toBe(0x08074b50);
        expect(descriptorView.getUint32(4, true)).toBe(0x12345678);
        expect(descriptorView.getUint32(8, true)).toBe(4);
        expect(descriptorView.getUint32(12, true)).toBe(9);
    });

    it("encodes entry names as UTF-8 bytes / 将 entry name 编码为 UTF-8 bytes", async () => {
        const { encodeZipEntryName } = await import("../../src/browser/archive/zip/archive.js");

        expect(Array.from(encodeZipEntryName("层/0.png"))).toEqual(
            Array.from(new TextEncoder().encode("层/0.png"))
        );
    });
});
