import { readFile } from "node:fs/promises";
import { describe, expect, it, vi } from "vitest";

import { createSlicerWorkerRuntime } from "../../src/browser/worker/runtime.js";

describe("createSlicerWorkerRuntime / createSlicerWorkerRuntime 用例", () => {
    it("composes preview and conversion handlers into one dispatch table / composes preview 并且 转换 handlers 到 one dispatch table", () => {
        const runtime = createSlicerWorkerRuntime({
            postMessage: vi.fn(),
            decodeEncodeLayer: vi.fn(),
            decodeLayerPreview: vi.fn(),
            inflateRaw: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview: vi.fn(),
            now: () => 0
        });

        expect(runtime).toBeDefined();
        expect(runtime.handlers["decode-preview"]).toBeTypeOf("function");
        expect(runtime.handlers).not.toHaveProperty("zip-decode-preview");
        expect(runtime.handlers["decode-encode"]).toBeTypeOf("function");
        expect(runtime.handlers["decode-encode-prepare"]).toBeTypeOf("function");
        expect(runtime.handlers["zip-decode-encode"]).toBeTypeOf("function");
        expect(runtime.handlers["zip-decode-encode-prepare"]).toBeTypeOf("function");
        expect(runtime.handlers["decode-preview"]).not.toBe(runtime.handlers["decode-encode"]);
    });

    it("reuses core ZIP archive entry types for prepareZipEntry dependencies / reuses core ZIP archive entry types for prepareZipEntry dependencies 用例", async () => {
        const source = await readFile("src/browser/worker/runtime.ts", "utf8");

        expect(source).toContain('import type { ZipArchiveEntry, ZipPreparedEntryWithTimings } from "../archive/zip/archive.js";');
        expect(source).not.toContain("type ZipPreparedEntry = {");
        expect(source).not.toContain("type ZipStreamPreparedEntry = {");
        expect(source).not.toContain("prepareZipEntryStream");
    });
});
