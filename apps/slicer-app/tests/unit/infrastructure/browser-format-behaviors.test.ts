import { existsSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("browser format behavior ownership boundary / browser format behavior 归属边界", () => {
    it("keeps browser format behavior adapters owned by slicer-wasm / 保持 browser format behavior adapters 由 slicer-wasm 拥有", () => {
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/document/browser-format-behaviors.ts"))).toBe(
            false
        );
    });
});
