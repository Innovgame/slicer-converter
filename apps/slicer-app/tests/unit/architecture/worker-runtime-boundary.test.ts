import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const PACKAGE_WORKER_FILES = [
    "packages/slicer-wasm/src/browser/worker/protocol.ts",
    "packages/slicer-wasm/src/browser/worker/errors.ts",
    "packages/slicer-wasm/src/browser/worker/runtime.ts",
    "packages/slicer-wasm/src/browser/worker/entry-runtime.ts",
    "packages/slicer-wasm/src/browser/worker/handlers/conversion.ts",
    "packages/slicer-wasm/src/browser/worker/handlers/preview.ts",
    "packages/slicer-wasm/src/browser/worker/dispatch.ts",
    "packages/slicer-wasm/src/browser/worker/layer-prefetch.ts",
    "packages/slicer-wasm/src/browser/worker/layer-worker-scheduler.ts",
    "packages/slicer-wasm/src/browser/worker/pool.ts"
];

describe("worker runtime package boundary / worker runtime package 边界", () => {
    it("keeps wasm package worker modules free of app imports / wasm package worker modules 不导入 app", () => {
        const offenders = PACKAGE_WORKER_FILES.flatMap((relativePath) => {
            const source = readFileSync(path.join(process.cwd(), "..", "..", relativePath), "utf8");
            const forbiddenFragments = ["apps/slicer-app", "../../../apps", "@slicer-converter/slicer-app"];
            const fragments = forbiddenFragments.filter((fragment) => source.includes(fragment));

            return fragments.length > 0 ? [{ file: relativePath, fragments }] : [];
        });

        expect(offenders).toEqual([]);
    });

    it("keeps concrete worker entry in app during this phase / 本阶段 concrete worker entry 留在 app", () => {
        const packageJson = JSON.parse(
            readFileSync(path.join(process.cwd(), "..", "..", "packages/slicer-wasm/package.json"), "utf8")
        ) as { exports?: Record<string, unknown> };
        const appEntry = readFileSync(path.join(process.cwd(), "src/worker/entry.ts"), "utf8");

        expect(packageJson.exports).not.toHaveProperty("./browser-worker");
        expect(appEntry).toContain("startSlicerWorkerEntryRuntime");
        expect(appEntry).toContain("@innovgame/slicer-converter");
    });
});
