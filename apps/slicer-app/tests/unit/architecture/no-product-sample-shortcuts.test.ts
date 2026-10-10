import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const BANNED_PRODUCT_SAMPLE_SHORTCUTS = [
    "SampleButtons",
    "loadSample",
    "onSampleLoad",
    "public/samples",
    "samples/",
    "Quick test with sample files"
];

describe("product sample shortcuts / 产品 sample shortcut", () => {
    it("keeps built-in sample loading out of product source / 保持内置 sample loading 不进入产品源码", () => {
        const offenders = listSourceFiles(path.join(process.cwd(), "src")).flatMap((file) => {
            const text = readFileSync(file, "utf8");
            const bannedTokens = BANNED_PRODUCT_SAMPLE_SHORTCUTS.filter((token) => text.includes(token));
            if (bannedTokens.length === 0) {
                return [];
            }

            return [
                {
                    file: path.relative(process.cwd(), file).replaceAll("\\", "/"),
                    bannedTokens
                }
            ];
        });

        expect(offenders).toEqual([]);
    });
});

function listSourceFiles(root: string): string[] {
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
