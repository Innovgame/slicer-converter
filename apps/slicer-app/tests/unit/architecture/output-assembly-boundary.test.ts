import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const PACKAGE_OUTPUT_FILES = [
    ...listPackageTypeScriptFiles("packages/slicer-wasm/src/browser/output"),
    "packages/slicer-wasm/src/browser/archive/zip/archive.ts",
    "packages/slicer-wasm/src/browser/archive/zip/stream-assembler.ts"
];

describe("output assembly package boundary / 输出组装 package 边界", () => {
    it("keeps wasm package output modules free of app imports / wasm package output modules 不导入 app", () => {
        const offenders = PACKAGE_OUTPUT_FILES.flatMap((relativePath) => {
            const source = readFileSync(path.join(process.cwd(), "..", "..", relativePath), "utf8");
            const forbiddenFragments = ["apps/slicer-app", "../../../apps", "@slicer-converter/slicer-app"];
            const fragments = forbiddenFragments.filter((fragment) => source.includes(fragment));

            return fragments.length > 0 ? [{ file: relativePath, fragments }] : [];
        });

        expect(offenders).toEqual([]);
    });

    it("owns File System Access sink implementation in the wasm package / 在 wasm package 拥有 File System Access sink 实现", () => {
        const packageSources = PACKAGE_OUTPUT_FILES.map((relativePath) =>
            readFileSync(path.join(process.cwd(), "..", "..", relativePath), "utf8")
        ).join("\n");

        expect(packageSources).toContain("showSaveFilePicker");
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/output/file-system-output-sink.ts"))).toBe(false);
    });
});

function listPackageTypeScriptFiles(relativeDirectory: string): string[] {
    const directory = path.join(process.cwd(), "..", "..", relativeDirectory);

    return readdirSync(directory)
        .filter((entry) => entry.endsWith(".ts") && entry !== "index.ts")
        .map((entry) => `${relativeDirectory}/${entry}`);
}
