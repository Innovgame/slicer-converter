import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { cleanRoundtripDownloads } from "./clean-roundtrip-downloads.mjs";

const repoRoot = resolve(import.meta.dirname, "..");

test("cleans only roundtrip download history / 只清理 roundtrip 下载历史", async () => {
    const downloadRoot = await mkdtemp(join(tmpdir(), "slicer-roundtrip-"));
    const staleFile = join(downloadRoot, "chitubox", "stale.ctb");

    try {
        await mkdir(join(downloadRoot, "chitubox"), { recursive: true });
        await writeFile(staleFile, "old");

        await cleanRoundtripDownloads(downloadRoot);

        await assert.rejects(stat(staleFile), /ENOENT/u);
        assert.equal((await stat(downloadRoot)).isDirectory(), true);
    } finally {
        await rm(downloadRoot, { recursive: true, force: true });
    }
});

test("roundtrip scripts clean downloads before Playwright starts / roundtrip 脚本在 Playwright 启动前清理下载目录", async () => {
    const packageJson = JSON.parse(await readFile(join(repoRoot, "apps", "slicer-app", "package.json"), "utf8"));

    assert.match(
        packageJson.scripts["test:e2e:roundtrip"],
        /^node \.\.\/\.\.\/scripts\/clean-roundtrip-downloads\.mjs && playwright test/u
    );
    assert.match(
        packageJson.scripts["test:e2e:roundtrip:real"],
        /^node \.\.\/\.\.\/scripts\/clean-roundtrip-downloads\.mjs && playwright test/u
    );
    assert.match(
        packageJson.scripts["test:e2e:roundtrip:real:headed"],
        /^node \.\.\/\.\.\/scripts\/clean-roundtrip-downloads\.mjs && playwright test/u
    );
});
