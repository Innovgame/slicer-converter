import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const FORBIDDEN_CORE_WASM_VALUE_IMPORTS = [
    "build_archive_metadata_entries",
    "build_cxdlpv4_document_header",
    "build_cxdlpv4_layer_def",
    "build_document_from_layers",
    "build_footer",
    "build_header",
    "crc32_bytes",
    "cxdlp_crc32_update",
    "describe_archive_layer_entries",
    "finalize_header",
    "initSlicerConverterRuntime"
];

describe("core WASM runtime boundary / core WASM runtime 边界", () => {
    it("keeps app imports on typed package APIs instead of raw WASM exports / app 只导入 package typed APIs 而不是 raw WASM exports", () => {
        const offenders = listSourceFiles(path.join(process.cwd(), "src")).flatMap((file) => {
            const text = readFileSync(file, "utf8");
            const importBlocks = extractWasmValueImportBlocks(text);
            const forbiddenNames = FORBIDDEN_CORE_WASM_VALUE_IMPORTS.filter((name) =>
                importBlocks.some((block) => block.includes(name))
            );
            if (forbiddenNames.length === 0) {
                return [];
            }

            return [
                {
                    file: path.relative(process.cwd(), file).replaceAll("\\", "/"),
                    forbiddenNames
                }
            ];
        });

        expect(offenders).toEqual([]);
    });

    it("does not keep an app-local WASM runtime facade / 不保留 app 本地 WASM runtime facade", () => {
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/wasm/slicer-wasm-runtime.ts"))).toBe(false);
    });

    it("does not keep app-local archive output plan tests / 不保留 app-local archive output plan tests", () => {
        expect(existsSync(path.join(process.cwd(), "tests/unit/infrastructure/archive-output-plan.test.ts"))).toBe(
            false
        );
    });

    it("keeps raw output assembly helper names behind package tests / raw output assembly helper 名称保留在 package tests 后面", () => {
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/output/wasm-binary-output-adapter.ts"))).toBe(
            false
        );

        const packageTestPath = path.join(
            process.cwd(),
            "..",
            "..",
            "packages/slicer-wasm/tests/unit/wasm-binary-output.test.ts"
        );
        const text = readFileSync(packageTestPath, "utf8");
        const forbiddenFragments = ["build_header:", "finalize_header:", "build_footer:"];
        const fragments = forbiddenFragments.filter((fragment) => text.includes(fragment));

        expect(fragments).toEqual([]);
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

function extractWasmValueImportBlocks(text: string): string[] {
    return Array.from(
        text.matchAll(/import\s+(?!type\b)[\s\S]*?from\s+["']@innovgame\/slicer-converter["'];/g),
        (match) => match[0]
    );
}
