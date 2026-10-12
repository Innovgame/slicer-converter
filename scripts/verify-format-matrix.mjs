import assert from "node:assert/strict";
import { File } from "node:buffer";
import { execFile } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { URL, fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
    assembleWasmBinaryOutputFromLayerStream,
    buildLayerMetadataJson,
    createBlobOutputSink,
    initSlicerConverter,
    inflateRaw,
    openSlicerDocumentFile,
    parseBrowserFileHeader
} from "../packages/slicer-wasm/dist/index.js";
import { decode_layer, encode_layer } from "../packages/slicer-wasm/lib/slicer_converter.js";
import {
    buildFormatValidationCases,
    comparePixels,
    quantizePixel,
    resizeAndQuantizePixels,
    verifyArtifactIdentity
} from "./format-validation.mjs";

const repoRoot = fileURLToPath(new URL("../", import.meta.url));
const outputRoot = path.resolve(process.env.SC_MATRIX_OUTPUT ?? "/tmp/slicer-format-validation");
const mode = process.env.SC_MATRIX_MODE ?? "synthetic";
assert.ok(["synthetic", "real"].includes(mode), "SC_MATRIX_MODE must be synthetic or real");
const cli = path.resolve(
    repoRoot,
    process.env.SC_MATRIX_CLI ?? `rust/target/release/slicer-converter${process.platform === "win32" ? ".exe" : ""}`
);
const runProcess = promisify(execFile);
const api = await initSlicerConverter();
const formats = api.getSupportedFormats();
await mkdir(outputRoot, { recursive: true });

const sourcePath =
    mode === "real"
        ? path.resolve(repoRoot, process.env.SC_MATRIX_SOURCE ?? "rust/data/Untitled_20260413_1125_TI3U.goo")
        : await createGrayscaleSource();
const source = await open(sourcePath);
if (mode === "synthetic") {
    for (let index = 0; index < 2; index++) {
        const pixels = await decode(source, index);
        for (let pixel = 0; pixel < pixels.length; pixel++)
            assert.equal(pixels[pixel], index === 0 ? pixel % 256 : 255 - (pixel % 256), "source gradient");
    }
}
const allCases = buildFormatValidationCases(formats);
const priorities = new Set(["ctb-v4", "ctb-v5", "prz", "chitubox-zip", "anycubic-zip", "cxdlpv4"]);
const cases =
    mode === "real"
        ? allCases.filter(
              (row) => priorities.has(row.format) && (row.format !== "anycubic-zip" || row.extension === "pm7")
          )
        : allCases;
assert.ok(cases.length > 0, "No export cases found in initialized catalog");
const report = {
    mode,
    sourcePath,
    layerCount: source.parsedHeader.layers.length,
    resolution: [source.parsedHeader.metadata.resolution_x, source.parsedHeader.metadata.resolution_y],
    independentUvtoolsValidation: "not-run-by-this-matrix",
    browserExecution: "not-run-by-this-CLI-and-WASM-matrix",
    readOnly: formats
        .filter((info) => !info.capabilities.can_export)
        .map((info) => ({
            format: info.name,
            profileId: info.profile_id,
            reason: "cannot derive by export; external fixture required"
        })),
    rows: []
};
try {
    for (const row of cases) {
        const outputPath = path.join(outputRoot, `${mode}-${row.key}.${row.extension}`);
        const roundtripPath = path.join(outputRoot, `${mode}-${row.key}-roundtrip.goo`);
        try {
            const options = [
                "--profile",
                row.format,
                ...(row.version === null ? [] : ["--version", String(row.version)])
            ];
            const forward = await convert(sourcePath, outputPath, options);
            const reverse = await convert(outputPath, roundtripPath, []);
            const target = await open(outputPath);
            const roundtrip = await open(roundtripPath);
            try {
                const artifactVersion = verifyArtifactIdentity(
                    row,
                    target.parsedHeader.format,
                    new Uint8Array(await target.sourceFile.slice(0, 20).arrayBuffer())
                );
                assert.equal(target.parsedHeader.layers.length, report.layerCount, "export layer count");
                assert.equal(roundtrip.parsedHeader.layers.length, report.layerCount, "roundtrip layer count");
                for (const key of ["resolution_x", "resolution_y"])
                    assert.equal(roundtrip.parsedHeader.metadata[key], target.parsedHeader.metadata[key], key);
                const indices =
                    mode === "real"
                        ? [...new Set([0, Math.floor(report.layerCount / 2), report.layerCount - 1])]
                        : Array.from({ length: report.layerCount }, (_, i) => i);
                const layers = [];
                for (const index of indices) {
                    const original = await decode(source, index);
                    const exported = await decode(target, index);
                    const imported = await decode(roundtrip, index);
                    const policy = grayscalePolicy(row);
                    if (original.length === exported.length) {
                        for (let pixel = 0; pixel < original.length; pixel++)
                            assert.equal(
                                exported[pixel],
                                quantizePixel(original[pixel], policy),
                                `forward ${policy} projection at layer ${index}, pixel ${pixel}`
                            );
                    } else {
                        assert.equal(row.format, "photons", "unexpected resolution projection");
                        const expected = resizeAndQuantizePixels(
                            original,
                            source.parsedHeader.metadata.resolution_x,
                            source.parsedHeader.metadata.resolution_y,
                            target.parsedHeader.metadata.resolution_x,
                            target.parsedHeader.metadata.resolution_y,
                            policy
                        );
                        assert.equal(
                            comparePixels(expected, exported).changedPixels,
                            0,
                            `forward resampling at layer ${index}`
                        );
                    }
                    const reversePixels = comparePixels(exported, imported);
                    assert.equal(reversePixels.changedPixels, 0, `roundtrip pixel differences at layer ${index}`);
                    layers.push({
                        index,
                        sourceToTarget:
                            original.length === exported.length
                                ? comparePixels(original, exported)
                                : { resampled: true, sourcePixels: original.length, targetPixels: exported.length },
                        targetToRoundtrip: reversePixels
                    });
                }
                report.rows.push({
                    ...row,
                    status: "passed",
                    parsedFormat: target.parsedHeader.format,
                    artifactVersion,
                    outputPath,
                    roundtripPath,
                    forward,
                    reverse,
                    metadataDifferences: compareMetadata(source.parsedHeader.metadata, target.parsedHeader.metadata),
                    layers
                });
                console.log(
                    `${mode}: ${row.key} passed${report.rows.at(-1).metadataDifferences.length ? " (metadata differences recorded)" : ""}`
                );
            } finally {
                await Promise.all([target.close(), roundtrip.close()]);
            }
        } catch (error) {
            report.rows.push({
                ...row,
                status: "failed",
                outputPath,
                error: error.message,
                stderr: error.stderr ?? null
            });
            console.log(`${mode}: ${row.key} FAILED: ${error.message}`);
        }
        await writeFile(path.join(outputRoot, `${mode}-report.json`), `${JSON.stringify(report, null, 2)}\n`);
    }
} finally {
    await source.close();
}
const failures = report.rows.filter((row) => row.status === "failed");
console.log(
    `${report.rows.length - failures.length}/${report.rows.length} matrix cases passed; ${report.readOnly.length} read-only profiles need external samples.`
);
if (failures.length) process.exitCode = 1;

async function convert(input, output, options) {
    const args = ["--input", input, "--output", output, "--threads", "4", "--timing", ...options];
    const started = performance.now();
    const { stderr } = await runProcess(cli, args, { cwd: repoRoot, timeout: 300_000, maxBuffer: 1024 * 1024 });
    const timing = {};
    for (const match of stderr.matchAll(/\b(read|layer_cpu|transcode|assemble|write|total)=([\d.]+)ms/g))
        timing[`${match[1]}Ms`] = Number(match[2]);
    for (const match of stderr.matchAll(/\b(fast_path_hits|fallback_hits|strategy_attempts|strategy_declines)=(\d+)/g))
        timing[match[1]] = Number(match[2]);
    return { command: [cli, ...args], processWallMs: performance.now() - started, timing, stderr };
}

async function open(filePath) {
    return openSlicerDocumentFile({
        file: new File([await readFile(filePath)], path.basename(filePath)),
        parseHeader: parseBrowserFileHeader
    });
}

async function decode(document, index) {
    const payload = await document.readLayerPayload(index);
    const bytes =
        payload.kind === "zip-entry" && payload.compressionMethod === 8
            ? await inflateRaw(payload.bytes, payload.uncompressedSize)
            : payload.bytes;
    const header = document.parsedHeader;
    return decode_layer(
        bytes,
        index,
        header.format,
        header.metadata.resolution_x,
        header.metadata.resolution_y,
        header.metadata.anti_alias_level ?? 1,
        header.metadata.encryption_key ?? 0
    );
}

function compareMetadata(before, after) {
    return [
        "resolution_x",
        "resolution_y",
        "layer_height_mm",
        "exposure_time",
        "bottom_exposure_time",
        "bottom_layer_count",
        "lift_height",
        "lift_speed",
        "display_width_mm",
        "display_height_mm"
    ].flatMap((key) => {
        if (
            before[key] === after[key] ||
            (typeof before[key] === "number" &&
                typeof after[key] === "number" &&
                Math.abs(before[key] - after[key]) < 1e-4)
        )
            return [];
        return [{ key, source: before[key] ?? null, target: after[key] ?? null }];
    });
}

async function createGrayscaleSource() {
    const fixture = new Uint8Array(await readFile(path.join(repoRoot, "rust/crates/core/tests/fixtures/sample.goo")));
    const metadata = {
        ...parseBrowserFileHeader(fixture, "goo").metadata,
        layer_count: 2,
        bottom_layer_count: 1,
        encryption_key: 0,
        anti_alias_level: 1
    };
    async function* layers() {
        for (let index = 0; index < 2; index++) {
            const pixels = Uint8Array.from({ length: metadata.resolution_x * metadata.resolution_y }, (_, pixel) =>
                index === 0 ? pixel % 256 : 255 - (pixel % 256)
            );
            yield encode_layer(
                pixels,
                index,
                "goo",
                metadata.resolution_x,
                metadata.resolution_y,
                buildLayerMetadataJson(metadata),
                undefined
            );
        }
    }
    const blob = await assembleWasmBinaryOutputFromLayerStream({
        metadata,
        targetFormat: "goo",
        layerCount: 2,
        layerStream: layers(),
        sink: createBlobOutputSink()
    });
    const filePath = path.join(outputRoot, "grayscale-source.goo");
    await writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
    return filePath;
}

function grayscalePolicy(row) {
    if (["goo", "prz", "sl1", "vdt", "chitubox-zip"].includes(row.format)) return "lossless";
    if (["pws", "photon", "cbddlp", "cbddlp-v1", "photons"].includes(row.format)) return "binary";
    if (["phz", "fdg"].includes(row.format)) return "phz";
    if (row.format === "cxdlpv4") return "cxdlp";
    if (row.format.includes("ctb")) return "gray7";
    if (row.codecFamily === "anycubic" || row.codecFamily === "anycubic-archive") return "gray4";
    throw new Error(`No independent grayscale policy for ${row.format}`);
}
