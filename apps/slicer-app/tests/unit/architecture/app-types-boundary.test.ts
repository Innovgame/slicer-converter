import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("app type boundary / app 类型边界", () => {
    it("does not keep a generic app types barrel / 不 keep a 通用 app types barrel", () => {
        expect(existsSync(path.join(process.cwd(), "src/types.ts"))).toBe(false);
    });

    it("does not import metadata types through the removed app barrel / 不 import metadata types through the removed app barrel", () => {
        const srcRoot = path.join(process.cwd(), "src");
        const testRoot = path.join(process.cwd(), "tests");
        const files = listSourceFiles([srcRoot, testRoot]);
        const staleImports = files.filter((file) => {
            const text = readFileSync(file, "utf8");
            return /from\s+["'][^"']*\/types["']/.test(text) || /from\s+["']\.\/types["']/.test(text);
        });

        expect(staleImports).toEqual([]);
    });
});

function listSourceFiles(roots: string[]): string[] {
    const result: string[] = [];
    for (const root of roots) {
        collectSourceFiles(root, result);
    }
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
