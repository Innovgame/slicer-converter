import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const WORKER_WASM_FACADE = path.normalize(path.join(process.cwd(), "src/worker/wasm-runtime.ts"));

describe("worker WASM runtime boundary / worker WASM runtime 边界", () => {
    it("does not keep a worker-local WASM facade / 不保留 worker 本地 WASM facade", () => {
        expect(existsSync(WORKER_WASM_FACADE)).toBe(false);
    });

    it("composes worker runtime directly from package APIs / 直接用 package API 组合 worker runtime", () => {
        const entrySource = readFileSync(path.join(process.cwd(), "src/worker/entry.ts"), "utf8");

        expect(entrySource).toContain("slicerLayerCodecRuntime");
        expect(entrySource).toContain("slicerLayerCodecRuntime.decodeEncodeLayer");
        expect(entrySource).toContain("slicerLayerCodecRuntime.decodeLayerPreview");
        expect(entrySource).toContain("convertZipLayerForTarget");
        expect(entrySource).toContain("decodeZipPreview");
        expect(entrySource).toContain("initSlicerConverter");
        expect(entrySource).not.toContain("./wasm-runtime");
        expect(entrySource).not.toContain("decodeEncodeLayerStats");
        expect(entrySource).not.toMatch(/\bdecode_encode_layer\b/);
        expect(entrySource).not.toMatch(/\bdecode_layer_preview\b/);
    });

    it("keeps worker runtime types in the package runtime / 保持 worker runtime types in the package runtime", () => {
        const packageRuntimeSource = readFileSync(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/runtime.ts"),
            "utf8"
        );

        expect(existsSync(path.join(process.cwd(), "src/worker/runtime.ts"))).toBe(false);
        expect(packageRuntimeSource).toContain("LayerCodecRuntime");
        expect(packageRuntimeSource).not.toContain("import type { decode_encode_layer, decode_layer_preview }");
        expect(packageRuntimeSource).toContain('LayerCodecRuntime["decodeEncodeLayer"]');
        expect(packageRuntimeSource).toContain('LayerCodecRuntime["decodeLayerPreview"]');
        expect(packageRuntimeSource).not.toContain('LayerCodecRuntime["decode_encode_layer"]');
        expect(packageRuntimeSource).not.toContain('LayerCodecRuntime["decode_layer_preview"]');
    });
});
