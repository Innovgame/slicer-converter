import { existsSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("GOO header parser ownership boundary / GOO header parser 归属边界", () => {
    it("keeps GOO parsing owned by slicer-wasm / 保持 GOO 解析由 slicer-wasm 拥有", () => {
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/document/goo-header-parser.ts"))).toBe(false);
    });
});
