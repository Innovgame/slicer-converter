import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const SRC_ROOT = path.join(process.cwd(), "src");
const DOMAIN_BEHAVIOR_ROOT = path.join(SRC_ROOT, "domain/format/behaviors");
const BEHAVIOR_ROOT = path.join(SRC_ROOT, "infrastructure/document/behaviors");
const BEHAVIOR_AGGREGATE = path.join(BEHAVIOR_ROOT, "index.ts");

describe("format behavior structure / format behavior 结构", () => {
    it("keeps built-in behavior consumers on the aggregate module / 内置 behavior 消费方只依赖 aggregate module", () => {
        const singleBehaviorImportPattern =
            /from\s+["'][^"']*infrastructure\/document\/behaviors\/(?:chitubox-zip|goo|sl1|vdt)-behavior["']/;
        const offenders = listSourceFiles(SRC_ROOT)
            .filter((file) => path.normalize(file) !== path.normalize(BEHAVIOR_AGGREGATE))
            .filter((file) => singleBehaviorImportPattern.test(readFileSync(file, "utf8")))
            .map(relativePath);

        expect(offenders).toEqual([]);
    });

    it("keeps app document behavior adapters out of slicer-app / 保持 app document behavior adapters 不回流到 slicer-app", () => {
        const behaviorFiles = existsSync(BEHAVIOR_ROOT)
            ? readdirSync(BEHAVIOR_ROOT)
                  .filter((name) => /-behavior\.ts$/.test(name))
                  .sort()
            : [];

        expect(behaviorFiles).toEqual([]);
        expect(existsSync(path.join(SRC_ROOT, "infrastructure/document/browser-format-behaviors.ts"))).toBe(false);
        expect(existsSync(path.join(SRC_ROOT, "domain/format/behaviors/behavior.ts"))).toBe(false);
    });

    it("keeps browser behavior adapters outside the domain layer / 保持 browser behavior adapters outside domain layer", () => {
        const domainBehaviorFiles = existsSync(DOMAIN_BEHAVIOR_ROOT)
            ? readdirSync(DOMAIN_BEHAVIOR_ROOT)
                  .filter((name) => /-behavior\.ts$/.test(name))
                  .sort()
            : [];

        expect(domainBehaviorFiles).toEqual([]);
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

function relativePath(file: string): string {
    return path.relative(process.cwd(), file).replaceAll("\\", "/");
}
