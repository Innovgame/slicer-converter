import { existsSync, mkdirSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { Buffer } from "node:buffer";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);

function run(command, args, options = {}) {
    const result = spawnSync(command, args, {
        stdio: "inherit",
        shell: false,
        ...options
    });
    if (result.error) {
        throw result.error;
    }
    if (result.status !== 0) {
        process.exit(result.status ?? 1);
    }
}

function runCapture(command, args, options = {}) {
    const result = spawnSync(command, args, {
        stdio: ["ignore", "pipe", "pipe"],
        encoding: "utf8",
        shell: false,
        ...options
    });
    if (result.error) {
        throw result.error;
    }
    if (result.status !== 0) {
        throw new Error(result.stderr || result.stdout || `${command} exited with status ${result.status}`);
    }
    return result.stdout;
}

function getChromeDriverPlatform() {
    if (process.platform === "win32") {
        return {
            archivePlatform: "win32",
            archiveName: "chromedriver-win32.zip",
            nestedDir: "chromedriver-win32",
            exeName: "chromedriver.exe"
        };
    }

    if (process.platform === "darwin") {
        return {
            archivePlatform: process.arch === "arm64" ? "mac-arm64" : "mac-x64",
            archiveName: process.arch === "arm64" ? "chromedriver-mac-arm64.zip" : "chromedriver-mac-x64.zip",
            nestedDir: process.arch === "arm64" ? "chromedriver-mac-arm64" : "chromedriver-mac-x64",
            exeName: "chromedriver"
        };
    }

    if (process.platform === "linux") {
        return {
            archivePlatform: "linux64",
            archiveName: "chromedriver-linux64.zip",
            nestedDir: "chromedriver-linux64",
            exeName: "chromedriver"
        };
    }

    return null;
}

function extractArchive(zipPath, installDir) {
    if (process.platform === "win32") {
        const extractCommand = [
            "$ErrorActionPreference='Stop'",
            `$zip='${zipPath.replace(/\\/g, "\\\\")}'`,
            `$dest='${installDir.replace(/\\/g, "\\\\")}'`,
            "Expand-Archive -LiteralPath $zip -DestinationPath $dest -Force"
        ].join("; ");

        run("powershell", ["-NoProfile", "-Command", extractCommand]);
        return;
    }

    if (process.platform === "darwin") {
        run("ditto", ["-x", "-k", zipPath, installDir]);
        return;
    }

    if (process.platform === "linux") {
        run("unzip", ["-o", zipPath, "-d", installDir]);
        return;
    }

    throw new Error(`Unsupported platform for archive extraction: ${process.platform}`);
}

async function fetchJson(url) {
    try {
        const response = await globalThis.fetch(url);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        return await response.json();
    } catch (error) {
        if (process.platform !== "win32") {
            throw error;
        }

        const command = [
            "$ProgressPreference='SilentlyContinue'",
            `$url='${url}'`,
            "Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 60 | Select-Object -ExpandProperty Content"
        ].join("; ");

        const content = runCapture("powershell", ["-NoProfile", "-Command", command]);
        return JSON.parse(content);
    }
}

async function downloadFile(url, outPath) {
    try {
        const response = await globalThis.fetch(url);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const archiveBytes = Buffer.from(await response.arrayBuffer());
        await writeFile(outPath, archiveBytes);
        return;
    } catch (error) {
        if (process.platform === "win32") {
            const command = [
                "$ProgressPreference='SilentlyContinue'",
                `$url='${url}'`,
                `$out='${outPath.replace(/\\/g, "\\\\")}'`,
                "Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -TimeoutSec 120"
            ].join("; ");
            run("powershell", ["-NoProfile", "-Command", command]);
            return;
        }

        if (process.platform === "darwin" || process.platform === "linux") {
            run("curl", ["-L", "--fail", "--output", outPath, url]);
            return;
        }

        throw error;
    }
}

async function ensureChromeDriver() {
    const platformConfig = getChromeDriverPlatform();
    if (!platformConfig) {
        return null;
    }

    const { archivePlatform, archiveName, nestedDir, exeName } = platformConfig;
    const installDir = join(repoRoot, ".cache", "chromedriver", archivePlatform);
    const exePath = join(installDir, nestedDir, exeName);

    if (existsSync(exePath)) {
        return exePath;
    }

    const metadataUrl =
        "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json";
    const metadata = await fetchJson(metadataUrl);
    const downloads = metadata?.channels?.Stable?.downloads?.chromedriver;
    const platformDownload = downloads?.find((item) => item.platform === archivePlatform);

    if (!platformDownload?.url) {
        throw new Error(`No ${archivePlatform} chromedriver download found in Chrome for Testing metadata`);
    }

    mkdirSync(installDir, { recursive: true });
    const zipPath = join(installDir, archiveName);

    await downloadFile(platformDownload.url, zipPath);
    extractArchive(zipPath, installDir);

    if (!existsSync(exePath)) {
        throw new Error(`Expected chromedriver at ${exePath}`);
    }

    return exePath;
}

export function buildWasmPackArgs(extraArgs) {
    const baseArgs = ["test", "--headless", "--chrome"];

    baseArgs.push("rust/crates/wasm");

    if (extraArgs.length > 0) {
        baseArgs.push(...extraArgs);
    }

    return baseArgs;
}

async function main() {
    const extraArgs = process.argv.slice(2);
    const baseArgs = buildWasmPackArgs(extraArgs);
    const exePath = await ensureChromeDriver();
    if (exePath) {
        baseArgs.splice(3, 0, "--chromedriver", exePath);
    }
    run("wasm-pack", baseArgs, { cwd: repoRoot });
}

const invokedPath = process.argv[1] ?? null;

if (invokedPath === fileURLToPath(import.meta.url)) {
    main().catch((error) => {
        console.error(error instanceof Error ? error.message : String(error));
        process.exit(1);
    });
}
