import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const FORBIDDEN_PATTERNS = [
    ["decode", "encode", "layers"].join("_"),
    ["decode", "Encode", "Layers"].join(""),
    ["worker", "Decode", "Encode", "Layers"].join(""),
    ["decode", "encode", "prepare", "batch"].join("-"),
    ["zip", "layer", "prepared", "batch"].join("-"),
    ["BENCH", "ZIP", "ENCODE", "BATCH", "SIZE"].join("_"),
    ["__SLICER", "ZIP", "ENCODE", "BATCH", "SIZE__"].join("_")
];

const SCAN_ROOTS = [
    "src",
    "tests",
    "../../docs",
    "../../packages/slicer-wasm/src",
    "../../packages/slicer-wasm/tests",
    "../../packages/slicer-wasm/lib",
    "../../rust/crates"
];

describe("plural batch encode removal / plural batch encode removal 用例", () => {
    it("does not keep batch encode APIs, protocol, or tests / 不 keep batch encode APIs, protocol, 或 tests", () => {
        const offenders = SCAN_ROOTS.flatMap((root) => scanFiles(path.resolve(process.cwd(), root)))
            .map((file) => ({
                file: path.relative(process.cwd(), file).replaceAll("\\", "/"),
                text: readFileSync(file, "utf8")
            }))
            .flatMap(({ file, text }) =>
                FORBIDDEN_PATTERNS.filter((pattern) => text.includes(pattern)).map((pattern) => ({ file, pattern }))
            );

        expect(offenders).toEqual([]);
    });
});

function scanFiles(root: string): string[] {
    if (!existsSync(root)) {
        return [];
    }
    if (!statSync(root).isDirectory()) {
        return [root];
    }

    const result: string[] = [];
    for (const entry of readdirSync(root, { withFileTypes: true })) {
        const fullPath = path.join(root, entry.name);
        if (entry.isDirectory()) {
            result.push(...scanFiles(fullPath));
        } else if (/\.(d\.ts|js|md|rs|ts|tsx)$/.test(entry.name)) {
            result.push(fullPath);
        }
    }
    return result;
}
