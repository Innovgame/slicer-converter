import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const SRC_ROOT = path.join(process.cwd(), "src");
const WORKER_ENTRY = path.join(SRC_ROOT, "worker.ts");
const WORKER_RUNTIME_ENTRY = path.join(SRC_ROOT, "worker/entry.ts");
const WORKER_RUNTIME_ROOT = path.join(SRC_ROOT, "worker");
const WORKER_INFRASTRUCTURE_ROOT = path.join(SRC_ROOT, "infrastructure/worker");
const FORBIDDEN_WORKER_RUNTIME_ROOTS = ["app", "application", "benchmark", "domain"].map((segment) =>
    path.join(SRC_ROOT, segment)
);

describe("worker runtime structure / worker runtime 结构", () => {
    it("keeps main-thread worker infrastructure independent from worker runtime / 主线程 worker infrastructure 不依赖 worker runtime", () => {
        const offenders = listSourceFiles(WORKER_INFRASTRUCTURE_ROOT).flatMap((file) =>
            resolvedLocalImports(file)
                .filter(
                    (target) =>
                        isSameOrInside(target, WORKER_RUNTIME_ROOT) ||
                        isSameFile(target, WORKER_ENTRY) ||
                        isSameFile(withTsExtension(target), WORKER_ENTRY)
                )
                .map((target) => ({
                    file: relativePath(file),
                    importTarget: relativePath(target)
                }))
        );

        expect(offenders).toEqual([]);
    });

    it("keeps worker runtime away from UI and application layers / worker runtime 不依赖 UI 与 application layers", () => {
        const workerRuntimeFiles = [WORKER_ENTRY, ...listSourceFiles(WORKER_RUNTIME_ROOT)];
        const offenders = workerRuntimeFiles.flatMap((file) =>
            resolvedLocalImports(file)
                .filter((target) => FORBIDDEN_WORKER_RUNTIME_ROOTS.some((root) => isSameOrInside(target, root)))
                .map((target) => ({
                    file: relativePath(file),
                    importTarget: relativePath(target)
                }))
        );

        expect(offenders).toEqual([]);
    });

    it("keeps the Vite worker URL entry thin / 保持 Vite worker URL entry 精简", () => {
        const text = readFileSync(WORKER_ENTRY, "utf8");

        expect(text).toContain('import "./worker/entry";');
        expect(resolvedLocalImports(WORKER_ENTRY).map(withTsExtension)).toEqual([WORKER_RUNTIME_ENTRY]);
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

function resolvedLocalImports(file: string): string[] {
    const text = readFileSync(file, "utf8");
    return [...extractFromImports(text), ...extractSideEffectImports(text)]
        .filter((specifier) => specifier.startsWith("."))
        .map((specifier) => path.normalize(path.resolve(path.dirname(file), specifier)));
}

function extractFromImports(text: string): string[] {
    return Array.from(
        text.matchAll(/\b(?:import|export)\s+(?!\()[\s\S]*?\s+from\s+["']([^"']+)["'];/g),
        (match) => match[1]!
    );
}

function extractSideEffectImports(text: string): string[] {
    return Array.from(text.matchAll(/\bimport\s+["']([^"']+)["'];/g), (match) => match[1]!);
}

function isSameOrInside(target: string, root: string): boolean {
    const relative = path.relative(root, target);
    return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function isSameFile(left: string, right: string): boolean {
    return path.normalize(left) === path.normalize(right);
}

function withTsExtension(target: string): string {
    return /\.[cm]?[tj]sx?$/.test(target) ? target : `${target}.ts`;
}

function relativePath(file: string): string {
    return path.relative(process.cwd(), file).replaceAll("\\", "/");
}
