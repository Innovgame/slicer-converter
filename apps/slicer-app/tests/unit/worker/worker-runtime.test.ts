import { existsSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("worker runtime ownership boundary / worker runtime 归属边界", () => {
    it("keeps worker runtime orchestration owned by slicer-wasm / 保持 worker runtime 编排由 slicer-wasm 拥有", () => {
        expect(existsSync(path.join(process.cwd(), "src/worker/runtime.ts"))).toBe(false);
    });
});
