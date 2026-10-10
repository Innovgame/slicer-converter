import { existsSync, readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
    BufferedSlicerOutputSink,
    createBlobOutputSink,
    createBufferedOutputSink,
    type PatchableSlicerOutputSink
} from "../../src/browser/output/sink.js";

describe("createBlobOutputSink / Blob 输出 sink", () => {
    it("writes and patches synchronously while still producing a Blob / 同步写入和修补后仍生成 Blob", async () => {
        const sink = createBlobOutputSink();

        const writeResult = sink.write(new Uint8Array([1, 2]));
        const patchResult = sink.patchHeader(new Uint8Array([7, 8]));
        const blob = await sink.finish();

        expect(writeResult).toBeUndefined();
        expect(patchResult).toBeUndefined();
        expect(blob).toBeInstanceOf(Blob);
        expect(Array.from(new Uint8Array(await blob!.arrayBuffer()))).toEqual([7, 8]);
    });

    it("rejects writes, patches, and second finish after finishing / finish 后拒绝继续写入、修补和再次 finish", async () => {
        const sink = createBlobOutputSink();

        sink.write(new Uint8Array([1]));
        await sink.finish();

        expect(() => sink.write(new Uint8Array([2]))).toThrow("already finished");
        expect(() => sink.patchHeader(new Uint8Array([3]))).toThrow("already finished");
        await expect(sink.finish()).rejects.toThrow("already finished");
        await expect(sink.abort("late cancel")).resolves.toBeUndefined();
    });

    it("clears buffered parts and rejects writes, patches, and finish after aborting / abort 后清空缓存并拒绝继续写入、修补和 finish", async () => {
        const sink = createBlobOutputSink();

        sink.write(new Uint8Array([1, 2]));
        await sink.abort("cancelled");

        expect(() => sink.write(new Uint8Array([3]))).toThrow("aborted");
        expect(() => sink.patchHeader(new Uint8Array([4]))).toThrow("aborted");
        await expect(sink.finish()).rejects.toThrow("aborted");
    });

    it("keeps File System Access concerns out of the generic sink module / 泛用 sink 模块不包含 File System Access 逻辑", () => {
        const source = readFileSync("src/browser/output/sink.ts", "utf8");

        expect(source).not.toContain("showSaveFilePicker");
        expect(source).not.toContain("FileSystem");
    });

    it("keeps header patching out of the append-only sink contract / 追加写入契约不包含 header 修补", () => {
        const source = readFileSync("src/browser/output/sink.ts", "utf8");
        const outputSinkBody =
            source.match(/export interface SlicerOutputSink \{(?<body>[\s\S]*?)\n\}/)?.groups?.body ?? "";

        expect(outputSinkBody).not.toContain("patchHeader");
        expect(source).toContain("export interface PatchableSlicerOutputSink extends SlicerOutputSink");
    });

    it("does not keep an unused response-backed sink fallback / 不保留未使用的 response-backed sink fallback", () => {
        expect(existsSync("src/browser/streaming-response-sink.ts")).toBe(false);
    });
});

describe("createBufferedOutputSink / 缓冲输出 sink", () => {
    function createRecordingSink(): PatchableSlicerOutputSink & {
        writes: Uint8Array[];
        patches: Uint8Array[];
        abortReasons: (string | undefined)[];
    } {
        const writes: Uint8Array[] = [];
        const patches: Uint8Array[] = [];
        const abortReasons: (string | undefined)[] = [];
        return {
            writes,
            patches,
            abortReasons,
            streaming: true,
            async write(data: Uint8Array) {
                writes.push(data);
            },
            async patchHeader(data: Uint8Array) {
                patches.push(data);
            },
            async finish() {
                return null;
            },
            async abort(reason?: string) {
                abortReasons.push(reason);
            }
        };
    }

    it("coalesces small writes before forwarding to the inner sink / 转发到内部 sink 前合并小块写入", async () => {
        const inner = createRecordingSink();
        const sink = createBufferedOutputSink(inner, { flushBytes: 4 });

        await sink.write(new Uint8Array([1]));
        await sink.write(new Uint8Array([2]));
        expect(inner.writes).toEqual([]);

        await sink.write(new Uint8Array([3, 4]));
        expect(inner.writes.map((write) => Array.from(write))).toEqual([[1, 2, 3, 4]]);

        await sink.write(new Uint8Array([5]));
        await sink.finish();
        expect(inner.writes.map((write) => Array.from(write))).toEqual([[1, 2, 3, 4], [5]]);
    });

    it("flushes pending bytes before patching the header / 修补 header 前先 flush pending bytes", async () => {
        const inner = createRecordingSink();
        const sink = createBufferedOutputSink(inner, { flushBytes: 8 });

        await sink.write(new Uint8Array([1, 2]));
        await sink.patchHeader(new Uint8Array([9]));

        expect(inner.writes.map((write) => Array.from(write))).toEqual([[1, 2]]);
        expect(inner.patches.map((patch) => Array.from(patch))).toEqual([[9]]);
    });

    it("passes through large writes without copying them into the pending buffer / 大块写入不复制进 pending buffer", async () => {
        const inner = createRecordingSink();
        const sink = createBufferedOutputSink(inner, { flushBytes: 4 });
        const large = new Uint8Array([1, 2, 3, 4, 5]);

        await sink.write(new Uint8Array([0]));
        await sink.write(large);
        await sink.finish();

        expect(inner.writes[1]).toBe(large);
        expect(inner.writes.map((write) => Array.from(write))).toEqual([[0], [1, 2, 3, 4, 5]]);
    });

    it("reports buffered flush time separately from inner finish time / 分开报告 buffered flush 与 inner finish 耗时", async () => {
        const inner = createRecordingSink();
        const times = [0, 3, 10, 17, 20, 42, 49];
        const sink = new BufferedSlicerOutputSink(inner, { flushBytes: 4, now: () => times.shift() ?? 47 });

        await sink.write(new Uint8Array([1, 2]));
        await sink.write(new Uint8Array([3, 4]));
        await sink.write(new Uint8Array([5]));
        await sink.finish();

        expect(sink.snapshotStats()).toEqual({
            flushCount: 2,
            flushBytes: 5,
            flushWallMs: 6,
            maxFlushWallMs: 3,
            finishCount: 1,
            finishWallMs: 39,
            innerFinishWallMs: 7
        });
    });

    it("clears pending bytes, forwards inner abort, and rejects terminal operations after aborting / abort 后清空 pending、转发内部 abort 并拒绝终态操作", async () => {
        const inner = createRecordingSink();
        const sink = createBufferedOutputSink(inner, { flushBytes: 8 });

        await sink.write(new Uint8Array([1, 2]));
        await sink.abort("cancelled");

        expect(inner.writes).toEqual([]);
        expect(inner.abortReasons).toEqual(["cancelled"]);
        await expect(sink.write(new Uint8Array([3]))).rejects.toThrow("aborted");
        await expect(sink.patchHeader(new Uint8Array([4]))).rejects.toThrow("aborted");
        await expect(sink.finish()).rejects.toThrow("aborted");
    });

    it("rejects writes, patches, and second finish after finishing / finish 后拒绝继续写入、修补和再次 finish", async () => {
        const inner = createRecordingSink();
        const sink = createBufferedOutputSink(inner, { flushBytes: 8 });

        await sink.write(new Uint8Array([1]));
        await sink.finish();

        await expect(sink.write(new Uint8Array([2]))).rejects.toThrow("already finished");
        await expect(sink.patchHeader(new Uint8Array([3]))).rejects.toThrow("already finished");
        await expect(sink.finish()).rejects.toThrow("already finished");
        await expect(sink.abort("late cancel")).resolves.toBeUndefined();
        expect(inner.abortReasons).toEqual([]);
    });
});
