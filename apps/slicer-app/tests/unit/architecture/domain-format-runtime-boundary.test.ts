import { existsSync, readdirSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("domain format runtime boundary / domain format 运行期边界", () => {
    it("detects WASM package value imports and re-exports / 检测 WASM package value imports 和 re-exports", () => {
        expect(extractWasmValueImports('import { getSupportedFormats } from "@innovgame/slicer-converter";')).toEqual([
            "{ getSupportedFormats }"
        ]);
        expect(extractWasmValueImports('export { getSupportedFormats } from "@innovgame/slicer-converter";')).toEqual([
            "{ getSupportedFormats }"
        ]);
        expect(extractWasmValueImports('import type { SupportedFormatInfo } from "@innovgame/slicer-converter";')).toEqual(
            []
        );
        expect(extractWasmValueImports('export type { SupportedFormatInfo } from "@innovgame/slicer-converter";')).toEqual(
            []
        );
    });

    it("does not keep app-local format facts after package sinkdown / package 下沉后不保留 app 本地格式事实", () => {
        expect(listSourceFiles(path.join(process.cwd(), "src/domain/format"))).toEqual([]);
    });
});

function extractWasmValueImports(text: string): string[] {
    return Array.from(
        text.matchAll(/(?:import|export)\s+(?!type\b)([\s\S]*?)\s+from\s+["']@slicer-converter\/wasm["']/g),
        (match) => match[1]!.trim()
    );
}

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
