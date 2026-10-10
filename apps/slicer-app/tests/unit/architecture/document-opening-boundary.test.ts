import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const PACKAGE_DOCUMENT_OPENING_FILES = [
    "packages/slicer-wasm/src/browser/document/document.ts",
    "packages/slicer-wasm/src/browser/document/source.ts",
    "packages/slicer-wasm/src/browser/format/browser-format-behaviors.ts",
    "packages/slicer-wasm/src/browser/format/goo-header-parser.ts",
    "packages/slicer-wasm/src/browser/archive/zip/directory.ts",
    "packages/slicer-wasm/src/browser/archive/zip/source.ts",
    "packages/slicer-wasm/src/browser/archive/zip/header-parser.ts"
];

describe("document opening package boundary / 文档打开 package 边界", () => {
    it("keeps wasm package document modules free of app imports / wasm package document modules 不导入 app", () => {
        const offenders = PACKAGE_DOCUMENT_OPENING_FILES.flatMap((relativePath) => {
            const source = readFileSync(path.join(process.cwd(), "..", "..", relativePath), "utf8");
            const forbiddenFragments = ["apps/slicer-app", "../../../apps", "@slicer-converter/slicer-app"];
            const fragments = forbiddenFragments.filter((fragment) => source.includes(fragment));

            return fragments.length > 0 ? [{ file: relativePath, fragments }] : [];
        });

        expect(offenders).toEqual([]);
    });
});
