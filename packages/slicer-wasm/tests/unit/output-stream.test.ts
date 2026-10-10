import { describe, expect, it } from "vitest";

import { createOrderedLayerQueue } from "../../src/browser/output/stream.js";

describe("ordered layer queue / 有序 layer 队列", () => {
    it("yields layers in index order even if worker results finish out of order / worker 结果乱序完成时仍按 index 顺序 yield", async () => {
        const queue = createOrderedLayerQueue(3);

        queue.push(2, new Uint8Array([3]));
        queue.push(0, new Uint8Array([1]));
        queue.push(1, new Uint8Array([2]));

        const parts: number[][] = [];
        for await (const chunk of queue.consume()) {
            parts.push(Array.from(chunk));
        }

        expect(parts).toEqual([[1], [2], [3]]);
    });

    it("rejects the consumer when the queue fails before all layers arrive / 全部 layers 到达前 fail 会 reject consumer", async () => {
        const queue = createOrderedLayerQueue(2);
        const consumePromise = (async () => {
            const parts: number[][] = [];
            for await (const chunk of queue.consume()) {
                parts.push(Array.from(chunk));
            }
            return parts;
        })();

        queue.push(0, new Uint8Array([1]));
        queue.fail(new Error("conversion failed"));

        await expect(consumePromise).rejects.toThrow("conversion failed");
    });

    it("throws and rejects the consumer for negative, non-integer, and out-of-range indexes / negative、非整数和越界 index 会 throw 并 reject consumer", async () => {
        for (const index of [-1, 0.5, 2]) {
            const queue = createOrderedLayerQueue(2);
            const consumePromise = (async () => {
                for await (const _chunk of queue.consume()) {
                    // Wait for the invalid push to fail the queue.
                }
            })();

            expect(() => queue.push(index, new Uint8Array([1]))).toThrow("Invalid layer index");
            await expect(consumePromise).rejects.toThrow("Invalid layer index");
        }
    });

    it("throws when a pending index is pushed twice / pending index 重复 push 时会 throw", () => {
        const queue = createOrderedLayerQueue(2);

        queue.push(1, new Uint8Array([1]));

        expect(() => queue.push(1, new Uint8Array([2]))).toThrow("already pending");
    });

    it("throws when an already consumed index is pushed again / 已消费 index 再次 push 时会 throw", async () => {
        const queue = createOrderedLayerQueue(2);
        const consumer = queue.consume();

        queue.push(0, new Uint8Array([1]));
        await expect(consumer.next()).resolves.toEqual({
            done: false,
            value: new Uint8Array([1])
        });

        expect(() => queue.push(0, new Uint8Array([2]))).toThrow("already consumed");
        await consumer.return(undefined);
    });
});
