import { mkdir, readdir, rm } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const downloadRoot = join(repoRoot, "apps", "slicer-app", "test-results", "roundtrip-downloads");

export function getRoundtripDownloadsPath() {
    return downloadRoot;
}

export async function cleanRoundtripDownloads() {
    const relativePath = relative(repoRoot, downloadRoot).replaceAll("\\", "/");

    if (relativePath !== "apps/slicer-app/test-results/roundtrip-downloads") {
        throw new Error(`Refusing to clean unexpected path: ${downloadRoot}`);
    }

    await mkdir(downloadRoot, { recursive: true });
    const entries = await readdir(downloadRoot);

    await Promise.all(entries.map((entry) => rm(join(downloadRoot, entry), { recursive: true, force: true })));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
    await cleanRoundtripDownloads();
}
