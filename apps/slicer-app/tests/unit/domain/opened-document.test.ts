import { existsSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("opened document ownership boundary / opened document 归属边界", () => {
    it("keeps opened document modeling owned by slicer-wasm / 保持 opened document 建模由 slicer-wasm 拥有", () => {
        expect(existsSync(path.join(process.cwd(), "src/domain/document/opened-document.ts"))).toBe(false);
    });
});
