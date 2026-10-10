import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const SRC_ROOT = path.join(process.cwd(), "src");

const ALLOWED_WASM_VALUE_IMPORT_FILES = [
    "src/app/runtime/use-runtime-controller.ts",
    "src/features/conversion/model/conversion-output-target.ts",
    "src/features/conversion/model/conversion-use-case.ts",
    "src/features/conversion/model/use-conversion-controller.ts",
    "src/features/converter/model/use-converter-controller.ts",
    "src/features/converter/ui/convert-panel.tsx",
    "src/features/converter/ui/file-summary-card.tsx",
    "src/features/converter/ui/preview-panel.tsx",
    "src/features/document-opening/model/use-document-controller.ts",
    "src/features/preview/model/use-preview-controller.ts",
    "src/shared/lib/errors.ts",
    "src/worker/entry.ts"
];

const FORBIDDEN_APP_LOCAL_WASM_BOUNDARY_FILES = [
    "src/infrastructure/archive/archive-output-plan.ts",
    "src/infrastructure/archive/zip-diagnostics.ts",
    "src/worker-protocol.ts",
    "src/worker/runtime.ts",
    "src/worker/entry-runtime.ts",
    "src/domain/document/opened-document.ts",
    "src/infrastructure/document/browser-format-behaviors.ts",
    "src/infrastructure/document/goo-header-parser.ts",
    "src/domain/format/behaviors/behavior.ts",
    "src/infrastructure/output/file-system-output-sink.ts",
    "src/worker/wasm-runtime.ts",
    "src/infrastructure/wasm/slicer-format-id.ts",
    "src/infrastructure/wasm/wasm-format-facts.ts",
    "src/domain/format/format-facts.ts",
    "src/infrastructure/wasm/wasm-facts-sync.ts",
    "src/infrastructure/wasm/slicer-wasm-runtime.ts"
];

describe("wasm direct import boundary / wasm direct import 边界", () => {
    it("keeps app value imports from wasm on the reviewed allowlist / 将 app 的 wasm value imports 保持在已审查白名单", () => {
        const importers = listSourceFiles(SRC_ROOT)
            .filter((file) => extractWasmValueImportBlocks(readFileSync(file, "utf8")).length > 0)
            .map(relativePath)
            .sort();

        expect(importers).toEqual(ALLOWED_WASM_VALUE_IMPORT_FILES);
    });

    it("does not reintroduce app-local wasm boundary modules / 不重新引入 app 本地 wasm 边界模块", () => {
        const existingBoundaryFiles = FORBIDDEN_APP_LOCAL_WASM_BOUNDARY_FILES.filter((relativeFile) =>
            readdirExists(path.join(process.cwd(), relativeFile))
        );

        expect(existingBoundaryFiles).toEqual([]);
    });

    it("does not deep-import slicer-wasm implementation files / 不深导入 slicer-wasm 实现文件", () => {
        const offenders = listSourceFiles(SRC_ROOT).flatMap((file) => {
            const source = readFileSync(file, "utf8");
            const imports = findImportSpecifiers(source).filter(
                (specifier) =>
                    specifier.includes("packages/slicer-wasm/src") ||
                    specifier.includes("@innovgame/slicer-converter/src") ||
                    specifier.includes("@innovgame/slicer-converter/browser")
            );

            return imports.length > 0 ? [{ file: relativePath(file), imports }] : [];
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

function extractWasmValueImportBlocks(text: string): string[] {
    return Array.from(
        text.matchAll(/(?:import|export)\s+(?!type\b)[\s\S]*?from\s+["']@innovgame\/slicer-converter["'];/g),
        (match) => match[0]
    );
}

function findImportSpecifiers(source: string): string[] {
    return Array.from(
        source.matchAll(/\b(?:import|export)\s+(?:type\s+)?(?:[^"']*?\s+from\s+)?["'](?<specifier>[^"']+)["']/g),
        (match) => match.groups?.specifier ?? ""
    ).filter(Boolean);
}

function relativePath(file: string): string {
    return path.relative(process.cwd(), file).replaceAll("\\", "/");
}

function readdirExists(file: string): boolean {
    try {
        readFileSync(file);
        return true;
    } catch {
        return false;
    }
}
