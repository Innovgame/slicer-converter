import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("domain dependency direction / domain 依赖方向", () => {
    it("keeps domain modules independent from infrastructure modules / 保持 domain modules 不依赖 infrastructure modules", () => {
        const sourceRoot = path.join(process.cwd(), "src");
        const domainRoot = path.join(sourceRoot, "domain");
        const offenders = listSourceFiles(domainRoot).flatMap((file) => {
            const imports = extractRelativeImports(readFileSync(file, "utf8"));
            const infrastructureImports = imports.filter((specifier) => {
                const target = resolveImport(file, specifier);
                return target !== null && path.relative(sourceRoot, target).replaceAll("\\", "/").startsWith("infrastructure/");
            });

            if (infrastructureImports.length === 0) {
                return [];
            }

            return [
                {
                    file: path.relative(process.cwd(), file).replaceAll("\\", "/"),
                    infrastructureImports
                }
            ];
        });

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

function extractRelativeImports(text: string): string[] {
    return Array.from(
        text.matchAll(/(?:import|export)\s+(?:type\s+)?(?:[\s\S]*?\s+from\s+)?["'](\.[^"']+)["']/g),
        (match) => match[1]!
    );
}

function resolveImport(file: string, specifier: string): string | null {
    const base = path.resolve(path.dirname(file), specifier);
    const candidates = [base, `${base}.ts`, `${base}.tsx`, path.join(base, "index.ts"), path.join(base, "index.tsx")];
    return candidates.find((candidate) => isSourceFile(candidate)) ?? null;
}

function isSourceFile(file: string): boolean {
    try {
        return /\.(ts|tsx)$/.test(file) && readdirSync(path.dirname(file)).includes(path.basename(file));
    } catch {
        return false;
    }
}
