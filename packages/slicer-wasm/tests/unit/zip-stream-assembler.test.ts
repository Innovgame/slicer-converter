import { readFile } from "node:fs/promises";

import { describe, expect, it, vi } from "vitest";

import { createBlobOutputSink } from "../../src/index.js";
import { prepareZipEntry } from "../../src/browser/archive/zip/archive.js";
import { createInMemoryZipSource, parseZipDirectoryFromReader, readZipEntryBytes } from "../../src/browser/archive/zip/source.js";
import { assembleZipArchiveForTest, createZipStreamAssemblerForTest } from "./support/zip-test-archive.js";
import type { PatchableSlicerOutputSink } from "../../src/index.js";

vi.mock("../../src/browser/crc.js", () => ({
    crc32Bytes(data: Uint8Array): number {
        let crc = 0;
        for (const byte of data) {
            crc = (crc + byte) >>> 0;
        }
        return crc;
    }
}));

describe("ZipStreamAssembler / ZIP 流式组装器", () => {
    it("exposes only prepared-entry write operations / 只暴露 prepared-entry 写入操作", async () => {
        const source = await readFile("src/browser/archive/zip/stream-assembler.ts", "utf8");
        const publicMethods = Array.from(source.matchAll(/^\s{4}(?:async\s+)?([a-z]\w*)\(/gm), (match) => match[1]!)
            .filter((name) => name !== "constructor")
            .sort();

        expect(publicMethods).toEqual(["finalize", "pushLayerEntry", "writePreparedEntry"]);
    });

    it("produces a byte-identical archive to the test archive builder when entries arrive in order / entries 顺序到达时生成与测试 builder 字节一致的 archive", async () => {
        const prepared = await Promise.all([
            prepareZipEntry({ name: "header.bin", data: new Uint8Array([1, 2, 3, 4]), compressionMethod: 0 }),
            prepareZipEntry({
                name: "0.png",
                data: new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]),
                compressionMethod: 0
            }),
            prepareZipEntry({
                name: "1.png",
                data: new Uint8Array([0x10, 0x20, 0x30, 0x40, 0x50]),
                compressionMethod: 0
            })
        ]);

        const reference = await assembleZipArchiveForTest(prepared, createBlobOutputSink());
        const referenceBytes = new Uint8Array(await reference.blob!.arrayBuffer());

        const streamAssembler = createZipStreamAssemblerForTest(createBlobOutputSink());
        await streamAssembler.writePreparedEntry(prepared[0]);
        await streamAssembler.pushLayerEntry(0, prepared[1]);
        await streamAssembler.pushLayerEntry(1, prepared[2]);
        const streamed = await streamAssembler.finalize();
        const streamedBytes = new Uint8Array(await streamed.blob!.arrayBuffer());

        expect(streamedBytes).toEqual(referenceBytes);
        expect(streamed.timings.finalizeOnlyWallMs).toBeGreaterThanOrEqual(0);
    });

    it("flushes layers in ascending index order even when pushed out of order / 即使乱序 push 也按 layer index 升序 flush", async () => {
        const prepared = await Promise.all([
            prepareZipEntry({ name: "0.png", data: new Uint8Array([0xaa]), compressionMethod: 0 }),
            prepareZipEntry({ name: "1.png", data: new Uint8Array([0xbb, 0xbb]), compressionMethod: 0 }),
            prepareZipEntry({ name: "2.png", data: new Uint8Array([0xcc, 0xcc, 0xcc]), compressionMethod: 0 })
        ]);

        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());
        await assembler.pushLayerEntry(2, prepared[2]);
        expect(assembler.pendingCount).toBe(1);
        await assembler.pushLayerEntry(0, prepared[0]);
        expect(assembler.pendingCount).toBe(1);
        await assembler.pushLayerEntry(1, prepared[1]);
        expect(assembler.pendingCount).toBe(0);
        const result = await assembler.finalize();
        const archiveBytes = new Uint8Array(await result.blob!.arrayBuffer());
        const source = createInMemoryZipSource(archiveBytes);
        const dir = await parseZipDirectoryFromReader(source, archiveBytes.length);

        expect(dir.entries.map((entry) => entry.name)).toEqual(["0.png", "1.png", "2.png"]);
    });

    it("flushes metadata entries before layer entries and leaves a recoverable archive / 先写 metadata 再写 layer 且 archive 可恢复读取", async () => {
        const meta = await prepareZipEntry({
            name: "config.ini",
            data: new TextEncoder().encode("abc"),
            compressionMethod: 0
        });
        const layer0 = await prepareZipEntry({
            name: "0.png",
            data: new Uint8Array([0x11, 0x22, 0x33, 0x44]),
            compressionMethod: 0
        });

        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());
        await assembler.writePreparedEntry(meta);
        await assembler.pushLayerEntry(0, layer0);
        const result = await assembler.finalize();
        const archiveBytes = new Uint8Array(await result.blob!.arrayBuffer());
        const source = createInMemoryZipSource(archiveBytes);
        const dir = await parseZipDirectoryFromReader(source, archiveBytes.length);

        expect(dir.entries.map((entry) => entry.name)).toEqual(["config.ini", "0.png"]);
        await expect(readZipEntryBytes(source, dir.entries[0]!)).resolves.toEqual(new TextEncoder().encode("abc"));
        await expect(readZipEntryBytes(source, dir.entries[1]!)).resolves.toEqual(
            new Uint8Array([0x11, 0x22, 0x33, 0x44])
        );
    });

    it("throws on duplicate layer index / 重复 layer index 时抛错", async () => {
        const entry = await prepareZipEntry({ name: "0.png", data: new Uint8Array([1]), compressionMethod: 0 });
        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());

        await assembler.pushLayerEntry(1, entry);
        await expect(assembler.pushLayerEntry(1, entry)).rejects.toThrow(/duplicate layer index 1/);
    });

    it("throws on replaying an already-flushed layer / 重放已 flush layer 时抛错", async () => {
        const a = await prepareZipEntry({ name: "0.png", data: new Uint8Array([1]), compressionMethod: 0 });
        const b = await prepareZipEntry({ name: "1.png", data: new Uint8Array([2]), compressionMethod: 0 });
        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());

        await assembler.pushLayerEntry(0, a);
        await assembler.pushLayerEntry(1, b);
        await expect(assembler.pushLayerEntry(0, a)).rejects.toThrow(/already flushed/);
    });

    it("throws on finalize with pending layers / finalize 遇到 pending layers 时抛错", async () => {
        const entry = await prepareZipEntry({ name: "1.png", data: new Uint8Array([1]), compressionMethod: 0 });
        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());

        await assembler.pushLayerEntry(1, entry);
        await expect(assembler.finalize()).rejects.toThrow(/still pending: 1/);
    });

    it("records finalize timing diagnostics / 记录 finalize timing 诊断", async () => {
        const entry = await prepareZipEntry({
            name: "0.png",
            data: new Uint8Array(1024),
            compressionMethod: 0
        });

        const assembler = createZipStreamAssemblerForTest(createBlobOutputSink());
        await assembler.pushLayerEntry(0, entry);
        await new Promise((resolve) => setTimeout(resolve, 20));
        const { timings } = await assembler.finalize();

        expect(timings.finalizeOnlyWallMs).toBeGreaterThanOrEqual(0);
        expect(timings.finalizeOnlyWallMs).toBeLessThan(timings.totalWallMs);
        expect(timings.centralDirectoryBytes).toBe(46 + "0.png".length + 22);
        expect(timings.centralDirectoryEntryCount).toBe(1);
        expect(timings.sinkFinishWallMs).toBeGreaterThanOrEqual(0);
    });

    it("writes the central directory as one contiguous block / central directory 作为单个连续 block 写入", async () => {
        const entries = await Promise.all([
            prepareZipEntry({ name: "0.png", data: new Uint8Array([1]), compressionMethod: 0 }),
            prepareZipEntry({ name: "1.png", data: new Uint8Array([2]), compressionMethod: 0 })
        ]);
        const writes: Uint8Array[] = [];
        const sink: PatchableSlicerOutputSink = {
            streaming: false,
            write(data: Uint8Array): void {
                writes.push(data);
            },
            async finish(): Promise<Blob> {
                return new Blob(writes.map((data) => data as unknown as BlobPart));
            },
            async abort(): Promise<void> {},
            patchHeader(data: Uint8Array): void {
                writes[0] = data;
            }
        };

        const assembler = createZipStreamAssemblerForTest(sink);
        await assembler.pushLayerEntry(0, entries[0]);
        await assembler.pushLayerEntry(1, entries[1]);
        const writeCountBeforeFinalize = writes.length;
        const { timings } = await assembler.finalize();

        expect(writes.length - writeCountBeforeFinalize).toBe(1);
        expect(writes.at(-1)?.byteLength).toBe(timings.centralDirectoryBytes);
    });
});
