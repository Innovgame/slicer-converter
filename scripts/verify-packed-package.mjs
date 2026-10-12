import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { URL, fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../", import.meta.url));
const outputRoot = path.resolve(process.env.SC_PACK_SMOKE_OUTPUT ?? path.join(tmpdir(), "slicer-package-verification"));
const packageRoot = path.join(repoRoot, "packages/slicer-wasm");
mkdirSync(outputRoot, { recursive: true });
const consumerRoot = mkdtempSync(path.join(outputRoot, "consumer-"));
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const npmOptions = { encoding: "utf8", shell: false, maxBuffer: 4 * 1024 * 1024 };
const packed = JSON.parse(
    execFileSync(
        npm,
        [
            "pack",
            "--ignore-scripts",
            "--json",
            "--pack-destination",
            outputRoot,
            "--cache",
            path.join(outputRoot, "cache")
        ],
        { ...npmOptions, cwd: packageRoot }
    )
)[0];
const artifact = path.join(outputRoot, packed.filename);
const packageName = JSON.parse(readFileSync(path.join(packageRoot, "package.json"), "utf8")).name;
writeFileSync(
    path.join(consumerRoot, "package.json"),
    JSON.stringify({ name: "slicer-external-smoke", private: true, type: "module" })
);
execFileSync(
    npm,
    [
        "install",
        "--offline",
        "--ignore-scripts",
        "--no-audit",
        "--no-fund",
        "--cache",
        path.join(outputRoot, "cache"),
        artifact
    ],
    { ...npmOptions, cwd: consumerRoot }
);
const consumer = `
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { File } from "node:buffer";
import { initSlicerConverter, initSlicerConverterWithProgress, openSlicerDocumentFile, parseBrowserFileHeader } from ${JSON.stringify(packageName)};
const resolved = import.meta.resolve(${JSON.stringify(packageName)});
assert.ok(resolved.includes("/node_modules/"), "must load installed tarball exports");
if (process.argv[2] === "progress") await initSlicerConverterWithProgress(() => undefined);
else await initSlicerConverter();
const bytes = await readFile(process.argv[3]);
const document = await openSlicerDocumentFile({ file: new File([bytes], "fixture.ctb"), parseHeader: parseBrowserFileHeader });
assert.equal(document.parsedHeader.layers.length, 10);
assert.equal(document.parsedHeader.metadata.resolution_x, 64);
assert.ok((await document.readLayerPayload(0)).bytes.length > 0);
await document.close();
const table = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(64, true);
await assert.rejects(openSlicerDocumentFile({ file: new File([bytes.subarray(0, table)], "bad.ctb"), parseHeader: parseBrowserFileHeader }), /Buffer too small/);
console.log(JSON.stringify({ mode: process.argv[2], resolved, layers: 10, status: "passed" }));
`;
const consumerPath = path.join(consumerRoot, "smoke.mjs");
writeFileSync(consumerPath, consumer);
const runs = ["default", "progress"].map((mode) =>
    JSON.parse(
        execFileSync(
            process.execPath,
            [consumerPath, mode, path.join(repoRoot, "rust/crates/core/tests/fixtures/sample.ctb")],
            { ...npmOptions, cwd: consumerRoot }
        )
    )
);
const report = { artifact, integrity: packed.integrity, bytes: packed.size, version: packed.version, runs };
writeFileSync(path.join(outputRoot, "report.json"), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
