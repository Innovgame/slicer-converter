import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const FORMAT_ID_FACADE = path.normalize(path.join(process.cwd(), "src/infrastructure/wasm/slicer-format-id.ts"));

describe("slicer format id boundary / slicer format id 边界", () => {
    it("does not keep an app-local format id validation facade / 不保留 app 本地 format id validation facade", () => {
        expect(existsSync(FORMAT_ID_FACADE)).toBe(false);
    });

    it("uses package format and target matrix APIs directly where needed / 在需要处直接使用 package format 和 target matrix API", () => {
        const converterControllerSource = readFileSync(
            path.join(process.cwd(), "src/features/converter/model/use-converter-controller.ts"),
            "utf8"
        );

        expect(converterControllerSource).toContain("findSlicerExportTargetMatrixSelection");
        expect(converterControllerSource).toContain("@slicer-converter/wasm");
        expect(converterControllerSource).not.toContain("asAppSlicerFormatId");
        expect(converterControllerSource).not.toContain("slicer-format-id");
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/wasm/wasm-format-facts.ts"))).toBe(false);
    });
});
