import { describe, expect, it, vi } from "vitest";

import type { PatchableSlicerOutputSink, SlicerOutputSink } from "../../src/index.js";

describe("output diagnostics / output diagnostics", () => {
    it("records output sink write and finish timings / 记录 output sink write 和 finish timings", async () => {
        const { createInstrumentedOutputSink } = await import("../../src/index.js");
        const times = [0, 5, 10, 13, 20, 31];
        const baseSink: SlicerOutputSink = {
            streaming: true,
            write: vi.fn(async () => undefined),
            finish: vi.fn(async () => null),
            abort: vi.fn(async () => undefined)
        };
        const { sink, stats } = createInstrumentedOutputSink(baseSink, () => times.shift() ?? 31);

        await sink.write(new Uint8Array([1, 2, 3]));
        await sink.write(new Uint8Array([4, 5]));
        await sink.finish();

        expect(stats.writeCount).toBe(2);
        expect(stats.writeBytes).toBe(5);
        expect(stats.writeWallMs).toBe(8);
        expect(stats.maxWriteWallMs).toBe(5);
        expect(stats.finishWallMs).toBe(11);
        expect(stats.finishCount).toBe(1);
        expect(baseSink.write).toHaveBeenCalledTimes(2);
        expect(baseSink.finish).toHaveBeenCalledTimes(1);
    });

    it("records patchable sink patch timings / 记录 patchable sink patch timings", async () => {
        const { createInstrumentedPatchableOutputSink } = await import("../../src/index.js");
        const times = [0, 2, 10, 17];
        const baseSink: PatchableSlicerOutputSink = {
            streaming: true,
            write: vi.fn(async () => undefined),
            patchHeader: vi.fn(async () => undefined),
            finish: vi.fn(async () => null),
            abort: vi.fn(async () => undefined)
        };
        const { sink, stats } = createInstrumentedPatchableOutputSink(baseSink, () => times.shift() ?? 17);

        await sink.write(new Uint8Array([1]));
        await sink.patchHeader(new Uint8Array([2, 3]));

        expect(stats.writeCount).toBe(1);
        expect(stats.writeBytes).toBe(1);
        expect(stats.writeWallMs).toBe(2);
        expect(stats.patchCount).toBe(1);
        expect(stats.patchBytes).toBe(2);
        expect(stats.patchWallMs).toBe(7);
        expect(baseSink.patchHeader).toHaveBeenCalledWith(new Uint8Array([2, 3]));
    });

    it("returns a shallow stats snapshot / 返回 shallow stats snapshot", async () => {
        const { snapshotOutputSinkStats } = await import("../../src/index.js");
        const stats = {
            writeCount: 1,
            writeBytes: 2,
            writeWallMs: 3,
            maxWriteWallMs: 3,
            finishCount: 1,
            finishWallMs: 4
        };

        const snapshot = snapshotOutputSinkStats(stats);

        expect(snapshot).toEqual(stats);
        expect(snapshot).not.toBe(stats);
    });
});
