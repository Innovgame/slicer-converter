import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { promisify } from "node:util";

import {
    collectRoundtripCompatReport,
    getRoundtripCompatMarkdownReportPath,
    getRoundtripCompatReportPath
} from "./collect-roundtrip-compat-report.mjs";

const repoRoot = resolve(import.meta.dirname, "..");
const execFileAsync = promisify(execFile);

async function withDownloadRoot(run) {
    const downloadRoot = await mkdtemp(join(tmpdir(), "slicer-roundtrip-"));
    try {
        return await run(downloadRoot);
    } finally {
        await rm(downloadRoot, { recursive: true, force: true });
    }
}

test("collects per-file roundtrip compatibility reports / 汇总单文件 roundtrip compatibility reports", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "chitubox"), { recursive: true });
        await mkdir(join(downloadRoot, "anycubic"), { recursive: true });
        await writeCompat(downloadRoot, "chitubox", "b.compat.json", { label: "b", outputFamily: "chitubox" });
        await writeCompat(downloadRoot, "anycubic", "a.compat.json", { label: "a", outputFamily: "anycubic" });

        const summary = await collectRoundtripCompatReport(downloadRoot);
        const report = JSON.parse(await readFile(getRoundtripCompatReportPath(downloadRoot), "utf8"));

        assert.equal(summary.total, 2);
        assert.equal(report.total, 2);
        assert.deepEqual(
            report.entries.map((entry) => entry.label),
            ["a", "b"]
        );
    });
});

test("writes a markdown roundtrip compatibility table / 写入 roundtrip 兼容性 Markdown 表格", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "anycubic"), { recursive: true });
        await writeCompat(downloadRoot, "anycubic", "sample.compat.json", {
            tag: "@sample",
            label: "sample-goo-to-anycubic-pw0",
            targetLabel: "pw0",
            sourceMachineName: "TestPrinter",
            outputFamily: "anycubic",
            fileSizeBytes: 77540,
            uvtools: {
                canDecode: true,
                canEncode: false,
                usingSquarePixels: false,
                fileExtension: ".pw0",
                resolutionX: 64,
                resolutionY: 64,
                displayWidth: 55.44,
                displayHeight: 98.637,
                layerCount: 10,
                machineName: "Photon Zero",
                warnings: ["UsingSquarePixels: False", "ratio | mismatch"]
            }
        });

        await collectRoundtripCompatReport(downloadRoot);

        const markdown = await readFile(getRoundtripCompatMarkdownReportPath(downloadRoot), "utf8");
        assert.match(markdown, /^# Roundtrip Compatibility Report/u);
        assert.match(markdown, /\*\*Total\*\*: 1/u);
        assert.match(markdown, /\*\*Risk\*\*: fail 1, warning 0, note 0, ok 0/u);
        assert.match(markdown, /\*\*Quality gate\*\*: fail/u);
        assert.match(markdown, /\*\*Family\*\*: anycubic 1/u);
        assert.match(
            markdown,
            /\| risk \| data \| family \| target \| ext \| size \| decode \| encode \| sq \| res \| lcd \| layers \| src \| machine \| notes \|/u
        );
        assert.match(
            markdown,
            /\| fail \| sample \| anycubic \| pw0 \| \.pw0 \| 75\.7 KB \| Y \| N \| N \| 64x64 \| 55\.44x98\.637 \| 10 \| TestPrinter \| Photon Zero \| ratio \\\| mismatch; encode unsupported; sample fixture resolution \|/u
        );
    });
});

test("summarizes sample fixture notes without raising row risk / 汇总 sample fixture notes 但不提升行风险", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "chitubox"), { recursive: true });
        await writeCompat(downloadRoot, "chitubox", "legacy.compat.json", {
            tag: "@sample",
            label: "sample-goo-to-ctb-v1",
            sourceMachineName: "TestPrinter",
            outputFamily: "chitubox",
            fileSizeBytes: 2220,
            uvtools: {
                canDecode: true,
                canEncode: true,
                usingSquarePixels: false,
                fileExtension: ".ctb",
                resolutionX: 64,
                resolutionY: 64,
                displayWidth: 68.04,
                displayHeight: 120.96,
                layerCount: 10,
                machineName: "Unknown",
                warnings: ["UsingSquarePixels: False"]
            }
        });

        await collectRoundtripCompatReport(downloadRoot);

        const markdown = await readFile(getRoundtripCompatMarkdownReportPath(downloadRoot), "utf8");
        assert.match(markdown, /\*\*Risk\*\*: fail 0, warning 0, note 0, ok 1/u);
        assert.match(markdown, /\*\*Sample notes\*\*: 1/u);
        assert.match(
            markdown,
            /\| ok \| sample \| chitubox \| ctb-v1 \| \.ctb \| 2\.2 KB \| Y \| Y \| N \| 64x64 \| 68\.04x120\.96 \| 10 \| TestPrinter \| Unknown \| legacy or generic machine metadata; sample fixture resolution \|/u
        );
    });
});

test("renders smoke cases as real data / smoke 用例按 real 数据展示", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "archive"), { recursive: true });
        await writeCompat(downloadRoot, "archive", "smoke.compat.json", {
            tag: "@smoke",
            label: "real-ball-to-sl1",
            targetLabel: "sl1",
            outputFamily: "archive",
            fileSizeBytes: 1024,
            uvtools: {
                canDecode: true,
                canEncode: true,
                usingSquarePixels: true,
                fileExtension: ".sl1",
                resolutionX: 8520,
                resolutionY: 4320,
                displayWidth: 153.36,
                displayHeight: 77.76,
                layerCount: 1380,
                machineName: "SL1"
            }
        });

        await collectRoundtripCompatReport(downloadRoot);

        const markdown = await readFile(getRoundtripCompatMarkdownReportPath(downloadRoot), "utf8");
        assert.match(
            markdown,
            /\| ok \| real \| archive \| sl1 \| \.sl1 \| 1\.0 KB \| Y \| Y \| Y \| 8520x4320 \| 153\.36x77\.76 \| 1380 \| - \| SL1 \| - \|/u
        );
    });
});

test("fails the quality gate for sample compatibility risks / sample 兼容性风险会触发质量门禁", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "anycubic"), { recursive: true });
        await writeCompat(downloadRoot, "anycubic", "sample-risk.compat.json", {
            tag: "@sample",
            label: "sample-goo-to-anycubic-pw0",
            outputFamily: "anycubic",
            uvtools: {
                canDecode: true,
                canEncode: true,
                usingSquarePixels: true,
                fileExtension: ".pw0",
                resolutionX: 64,
                resolutionY: 64,
                displayWidth: 55.44,
                displayHeight: 98.637,
                warnings: ["It looks like this file was sliced with an incorrect image ratio."]
            }
        });

        const summary = await collectRoundtripCompatReport(downloadRoot);

        assert.equal(summary.qualityGate.ok, false);
        assert.deepEqual(summary.qualityGate.violations, [
            {
                label: "sample-goo-to-anycubic-pw0",
                risk: "warning",
                reason: "sample compatibility warning"
            }
        ]);

        await assert.rejects(
            execFileAsync(process.execPath, [join(repoRoot, "scripts", "collect-roundtrip-compat-report.mjs")], {
                cwd: repoRoot,
                windowsHide: true,
                env: { ...process.env, SC_ROUNDTRIP_DOWNLOADS: downloadRoot }
            }),
            /Roundtrip compatibility gate failed/u
        );
    });
});

test("allows real image ratio warnings in the quality gate / real 图像比例 warning 只记录不阻断", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "anycubic"), { recursive: true });
        await writeCompat(downloadRoot, "anycubic", "real-warning.compat.json", {
            tag: "@real",
            label: "real-ball-to-anycubic-pw0",
            outputFamily: "anycubic",
            uvtools: {
                canDecode: true,
                canEncode: true,
                usingSquarePixels: false,
                fileExtension: ".pw0",
                resolutionX: 8520,
                resolutionY: 4320,
                displayWidth: 55.44,
                displayHeight: 98.637,
                warnings: ["It looks like this file was sliced with an incorrect image ratio."]
            }
        });

        const summary = await collectRoundtripCompatReport(downloadRoot);

        assert.equal(summary.qualityGate.ok, true);
        assert.deepEqual(summary.qualityGate.violations, []);
    });
});

test("roundtrip scripts collect compatibility reports after Playwright / roundtrip 脚本在 Playwright 后汇总兼容报告", async () => {
    const packageJson = JSON.parse(await readFile(join(repoRoot, "apps", "slicer-app", "package.json"), "utf8"));

    assert.match(
        packageJson.scripts["test:e2e:roundtrip"],
        /playwright test .+ && node \.\.\/\.\.\/scripts\/collect-roundtrip-compat-report\.mjs/u
    );
    assert.match(
        packageJson.scripts["test:e2e:roundtrip:real"],
        /playwright test .+ && node \.\.\/\.\.\/scripts\/collect-roundtrip-compat-report\.mjs/u
    );
    assert.match(packageJson.scripts["test:e2e:roundtrip:real:smoke"], /--grep @smoke/u);
    assert.match(packageJson.scripts["test:e2e:roundtrip:real:smoke"], /--workers=2/u);
    assert.match(packageJson.scripts["test:e2e:roundtrip:real:smoke"], /collect-roundtrip-compat-report\.mjs/u);
    assert.match(
        packageJson.scripts["test:e2e:roundtrip:real:headed"],
        /playwright test .+ && node \.\.\/\.\.\/scripts\/collect-roundtrip-compat-report\.mjs/u
    );
});

test("collect script runs from node cli on Windows paths / collect 脚本可通过 node cli 执行 Windows 路径", async () => {
    await withDownloadRoot(async (downloadRoot) => {
        await mkdir(join(downloadRoot, "chitubox"), { recursive: true });
        await writeCompat(downloadRoot, "chitubox", "cli.compat.json", { label: "cli", outputFamily: "chitubox" });

        await execFileAsync(process.execPath, [join(repoRoot, "scripts", "collect-roundtrip-compat-report.mjs")], {
            cwd: repoRoot,
            windowsHide: true,
            env: { ...process.env, SC_ROUNDTRIP_DOWNLOADS: downloadRoot }
        });

        const report = JSON.parse(await readFile(getRoundtripCompatReportPath(downloadRoot), "utf8"));
        assert.equal(report.total, 1);
        assert.equal(report.entries[0].label, "cli");
    });
});

async function writeCompat(downloadRoot, family, fileName, report) {
    await writeFile(join(downloadRoot, family, fileName), `${JSON.stringify(report)}\n`, "utf8");
    assert.equal((await stat(join(downloadRoot, family, fileName))).isFile(), true);
}
