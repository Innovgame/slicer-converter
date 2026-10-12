import { URL } from "node:url";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { test } from "node:test";

test("fails when a required benchmark fixture is missing / 必需 benchmark 样本缺失时失败", () => {
    const directory = mkdtempSync(join(tmpdir(), "slicer-bench-test-"));
    try {
        const result = spawnSync(
            process.execPath,
            [fileURLToPath(new URL("./check-benchmark-threshold.mjs", import.meta.url))],
            {
                encoding: "utf8",
                env: { ...process.env, BENCH_RLE_INPUT: join(directory, "missing.goo") }
            }
        );
        assert.notEqual(result.status, 0);
        assert.match(result.stderr, /required.*sample|fixture.*missing/i);
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
