import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const SOURCE_ROOTS = ["src/app", "src/features"];
const REMOVED_APP_LOCAL_FACTS_REFERENCES = [
    "domain/format/format-facts",
    "infrastructure/wasm/wasm-format-facts",
    "infrastructure/wasm/wasm-facts-sync",
    "infrastructure/wasm/slicer-wasm-runtime"
];

describe("package format facts boundary / package format facts 边界", () => {
    it("reads format facts from the wasm package instead of app-local facades / 从 wasm package 而不是 app 本地 facade 读取 format facts", () => {
        const offenders = SOURCE_ROOTS.flatMap((root) => listSourceFiles(path.join(process.cwd(), root))).flatMap(
            (file) => {
                const text = readFileSync(file, "utf8");
                const references = REMOVED_APP_LOCAL_FACTS_REFERENCES.filter((reference) => text.includes(reference));
                if (references.length === 0) {
                    return [];
                }

                return [{ file: path.relative(process.cwd(), file).replaceAll("\\", "/"), references }];
            }
        );

        expect(offenders).toEqual([]);
    });
});

function listSourceFiles(root: string): string[] {
    if (!existsSync(root)) {
        return [];
    }

    const result: string[] = [];
    collectSourceFiles(root, result);
    return result;
}

function collectSourceFiles(dir: string, result: string[]): void {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            collectSourceFiles(fullPath, result);
            continue;
        }
        if (/\.(ts|tsx)$/.test(entry.name)) {
            result.push(fullPath);
        }
    }
}
