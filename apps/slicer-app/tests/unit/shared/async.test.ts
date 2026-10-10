import { describe, expect, it } from "vitest";

import { mapWithConcurrency } from "../../../src/shared/lib/async";

describe("mapWithConcurrency / 有界并发映射", () => {
    it("maps work with bounded concurrency while preserving order / 映射 work 带有 bounded concurrency 同时 preserving order", async () => {
        let inFlight = 0;
        let maxInFlight = 0;

        const results = await mapWithConcurrency([3, 1, 2, 0], 2, async (value) => {
            inFlight += 1;
            maxInFlight = Math.max(maxInFlight, inFlight);
            await new Promise((resolve) => setTimeout(resolve, value));
            inFlight -= 1;
            return value * 10;
        });

        expect(results).toEqual([30, 10, 20, 0]);
        expect(maxInFlight).toBeLessThanOrEqual(2);
    });

    it("does not call the mapper for empty input / 空输入不调用 mapper", async () => {
        const results = await mapWithConcurrency([], 2, async (value: number) => value * 10);

        expect(results).toEqual([]);
    });
});
