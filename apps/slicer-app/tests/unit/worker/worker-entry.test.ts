import { readFile } from "node:fs/promises";

import { describe, expect, it } from "vitest";

describe("worker entry package composition / worker entry package 组合", () => {
    it("uses package worker runtime directly / 直接使用 package worker runtime", async () => {
        const source = await readFile("src/worker/entry.ts", "utf8");

        expect(source).toContain("createSlicerWorkerRuntime");
        expect(source).toContain("startSlicerWorkerEntryRuntime");
        expect(source).not.toContain("./runtime");
        expect(source).not.toContain("./entry-runtime");
        expect(source).not.toContain("./wasm-runtime");
    });
});
