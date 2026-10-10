import test from "node:test";
import assert from "node:assert/strict";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import { cleanRoundtripDownloads, getRoundtripDownloadsPath } from "./clean-roundtrip-downloads.mjs";

const repoRoot = resolve(import.meta.dirname, "..");

test("cleans only roundtrip download history / 只清理 roundtrip 下载历史", async () => {
    const downloadRoot = getRoundtripDownloadsPath();
    const staleFile = join(downloadRoot, "chitubox", "stale.ctb");

    await mkdir(join(downloadRoot, "chitubox"), { recursive: true });
    await writeFile(staleFile, "old");

    await cleanRoundtripDownloads();

    await assert.rejects(stat(staleFile), /ENOENT/u);
    assert.equal((await stat(downloadRoot)).isDirectory(), true);
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
