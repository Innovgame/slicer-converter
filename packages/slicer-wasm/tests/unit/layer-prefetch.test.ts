import { describe, expect, it, vi } from "vitest";

import { createSlicerLayerPrefetch } from "../../src/browser/worker/layer-prefetch.js";
import type { SlicerWorkerDispatchPayload } from "../../src/browser/worker/dispatch.js";
const createTestPayload = (index: number): SlicerWorkerDispatchPayload => ({
    request: {
        type: "decode-encode",
        index,
        data: new ArrayBuffer(0),
        sourceFormat: "goo",
        targetFormat: "ctb",
        targetAnycubicVersion: null,
        width: 1024,
        height: 768,
        antiAliasLevel: 1,
        encryptionKey: 0,
        layerMetadataJson: null
    },
    transfer: new ArrayBuffer(index + 1)
});

describe("createSlicerLayerPrefetch / createSlicerLayerPrefetch 用例", () => {
    it("pre-reads layers ahead and returns them in order / pre-reads layers ahead 并且 返回 them in order", async () => {
        const callOrder: number[] = [];
        const buildPayload = vi.fn(async (index: number) => {
            callOrder.push(index);
            return createTestPayload(index);
        });

        const prefetch = createSlicerLayerPrefetch(5, buildPayload, 3, new AbortController().signal);
        expect(callOrder).toEqual([0, 1, 2]);
        const p0 = await prefetch.next();
        expect(p0.transfer.byteLength).toBe(1);
        expect(callOrder).toEqual([0, 1, 2, 3]);
        const p1 = await prefetch.next();
        expect(p1.transfer.byteLength).toBe(2);
        expect(callOrder).toEqual([0, 1, 2, 3, 4]);
        const p2 = await prefetch.next();
        expect(p2.transfer.byteLength).toBe(3);
        const p3 = await prefetch.next();
        expect(p3.transfer.byteLength).toBe(4);
        const p4 = await prefetch.next();
        expect(p4.transfer.byteLength).toBe(5);
        expect(buildPayload).toHaveBeenCalledTimes(5);
    });

    it("throws on underflow when consuming more than totalLayers / 抛出 on underflow 当 consuming more than totalLayers", async () => {
        const prefetch = createSlicerLayerPrefetch(
            1,
            async (index) => createTestPayload(index),
            2,
            new AbortController().signal
        );

        await prefetch.next();
        await expect(prefetch.next()).rejects.toThrow("Prefetch queue underflow");
    });

    it("stops prefetching when signal is aborted / stops prefetching 当 signal is aborted", async () => {
        const controller = new AbortController();
        const buildPayload = vi.fn(async (index: number) => createTestPayload(index));

        const prefetch = createSlicerLayerPrefetch(10, buildPayload, 4, controller.signal);
        expect(buildPayload).toHaveBeenCalledTimes(4);
        controller.abort();

        await prefetch.next(); // consume layer 0
        expect(buildPayload).toHaveBeenCalledTimes(4);
    });

    it("expands window when consumed payload is not ready / expands window 当 consumed payload is not ready", async () => {
        let releaseFirst: (() => void) | undefined;
        const buildPayload = vi.fn((index: number): Promise<SlicerWorkerDispatchPayload> => {
            if (index === 0) {
                return new Promise((resolve) => {
                    releaseFirst = () => resolve(createTestPayload(index));
                });
            }
            return Promise.resolve(createTestPayload(index));
        });

        const prefetch = createSlicerLayerPrefetch(
            6,
            buildPayload,
            { initialAhead: 1, minAhead: 1, maxAhead: 3 },
            new AbortController().signal
        );

        expect(buildPayload).toHaveBeenCalledTimes(1);

        const pending = prefetch.next();
        expect(buildPayload).toHaveBeenCalledTimes(3);

        releaseFirst?.();
        await pending;
    });

    it("shrinks window after sustained ready backlog / shrinks window 在之后 sustained ready backlog", async () => {
        const buildPayload = vi.fn(async (index: number) => createTestPayload(index));

        const prefetch = createSlicerLayerPrefetch(
            10,
            buildPayload,
            { initialAhead: 3, minAhead: 1, maxAhead: 3 },
            new AbortController().signal
        );

        expect(buildPayload).toHaveBeenCalledTimes(3);
        await prefetch.next();
        await prefetch.next();
        await prefetch.next();
        expect(buildPayload).toHaveBeenCalledTimes(5);
    });
});
