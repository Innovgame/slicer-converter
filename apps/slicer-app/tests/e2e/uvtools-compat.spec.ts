import { execFile } from "node:child_process";
import path from "node:path";
import { promisify } from "node:util";

import { test } from "@playwright/test";

import { fixtureFilePath, repoFilePath } from "../support/fixture-files";
import { ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS } from "../support/anycubic-targets";
import { expectUvtoolsCanOpen } from "../support/uvtools";

const execFileAsync = promisify(execFile);

interface UvtoolsCompatTarget {
    label: string;
    outputName: string;
}

test.describe("external UVTools compatibility smoke / 外部 UVTools 兼容 smoke", () => {
    for (const target of uvtoolsCompatTargets()) {
        test(`${target.label} @uvtools / ${target.label} 可被 UVTools 解析`, async (_fixtures, testInfo) => {
            test.setTimeout(180_000);

            const outputPath = testInfo.outputPath(target.outputName);

            await test.step("CLI exports sample GOO / CLI 导出 sample GOO", async () => {
                await runCliConversion(fixtureFilePath("sample.goo"), outputPath);
            });

            await test.step("UVTools can open CLI output / UVTools 可打开 CLI 输出", async () => {
                await expectUvtoolsCanOpen(outputPath);
            });
        });
    }
});

async function runCliConversion(inputPath: string, outputPath: string): Promise<void> {
    const args = [
        "run",
        "--manifest-path",
        repoFilePath("rust", "Cargo.toml"),
        "-p",
        "slicer-converter-cli",
        "--",
        "--input",
        inputPath,
        "--output",
        outputPath,
        "--threads",
        "1"
    ];

    try {
        await execFileAsync("cargo", args, {
            cwd: repoFilePath(),
            timeout: 120_000,
            windowsHide: true,
            maxBuffer: 8 * 1024 * 1024
        });
    } catch (error) {
        throw new Error(`CLI conversion failed for ${path.basename(outputPath)}:\n${outputFromExecFileError(error)}`, {
            cause: error
        });
    }
}

function uvtoolsCompatTargets(): UvtoolsCompatTarget[] {
    return [
        { label: "ctb-v1", outputName: "sample.v1.ctb" },
        { label: "ctb-v2", outputName: "sample.v2.ctb" },
        { label: "ctb-v3", outputName: "sample.v3.ctb" },
        { label: "ctb-v4", outputName: "sample.v4.ctb" },
        { label: "ctb-v5", outputName: "sample.v5.ctb" },
        { label: "cbddlp-v1", outputName: "sample.v1.cbddlp" },
        { label: "cbddlp", outputName: "sample.cbddlp" },
        { label: "gktwo-ctb-v1", outputName: "sample.gktwo-v1.ctb" },
        { label: "gktwo-ctb-v2", outputName: "sample.gktwo-v2.ctb" },
        { label: "gktwo-ctb-v3", outputName: "sample.gktwo-v3.ctb" },
        { label: "gktwo-ctb-v4", outputName: "sample.gktwo.ctb" },
        { label: "gktwo-ctb-v5", outputName: "sample.gktwo-v5.ctb" },
        { label: "encrypted-ctb-v4", outputName: "sample.v4.encrypted.ctb" },
        { label: "encrypted-ctb-v5", outputName: "sample.v5.encrypted.ctb" },
        { label: "phz", outputName: "sample.phz" },
        { label: "fdg", outputName: "sample.fdg" },
        { label: "photons", outputName: "sample.photons" },
        { label: "chitubox-zip", outputName: "sample.zip" },
        ...ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS.map((extension) => ({
            label: `anycubic-${extension}`,
            outputName: `sample.${extension}`
        }))
    ];
}

function outputFromExecFileError(error: unknown): string {
    if (typeof error === "object" && error !== null) {
        const maybeOutput = error as { stdout?: unknown; stderr?: unknown; message?: unknown };
        return [maybeOutput.stdout, maybeOutput.stderr, maybeOutput.message]
            .filter((part) => part !== undefined && part !== null)
            .map((part) => (Buffer.isBuffer(part) ? part.toString("utf8") : String(part)))
            .join("\n");
    }
    return String(error);
}
