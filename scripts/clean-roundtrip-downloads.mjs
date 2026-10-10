import { mkdir, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const downloadRoot = join(repoRoot, "apps", "slicer-app", "test-results", "roundtrip-downloads");

export function getRoundtripDownloadsPath() {
    return downloadRoot;
}

export function assertRoundtripDownloadsPath(root = downloadRoot) {
    const resolved = resolve(root);
    const relativePath = relative(repoRoot, resolved).replaceAll("\\", "/");
    const tempRelative = relative(resolve(tmpdir()), resolved);
    const underTemp = tempRelative !== "" && !tempRelative.startsWith("..") && !isAbsolute(tempRelative);

    if (relativePath !== "apps/slicer-app/test-results/roundtrip-downloads" && !underTemp) {
        throw new Error(`Refusing to use unexpected roundtrip downloads path: ${resolved}`);
    }

    return resolved;
}

export async function cleanRoundtripDownloads(root = downloadRoot) {
    const target = assertRoundtripDownloadsPath(root);

    await mkdir(target, { recursive: true });
    const entries = await readdir(target);

    await Promise.all(entries.map((entry) => rm(join(target, entry), { recursive: true, force: true })));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
    await cleanRoundtripDownloads();
}
