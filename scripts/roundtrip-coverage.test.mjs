import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import initWasm, { list_supported_formats } from "../packages/slicer-wasm/lib/slicer_converter.js";

const repoRoot = resolve(import.meta.dirname, "..");
const roundtripSpecPath = resolve(repoRoot, "apps", "slicer-app", "tests", "e2e", "roundtrip.spec.ts");
const anycubicTargetsPath = resolve(repoRoot, "apps", "slicer-app", "tests", "support", "anycubic-targets.ts");
const wasmPath = resolve(repoRoot, "packages", "slicer-wasm", "lib", "slicer_converter_bg.wasm");

test("roundtrip simple covers every exportable catalog target / roundtrip simple 覆盖所有可导出 catalog target", async () => {
    await initWasm({ module_or_path: await readFile(wasmPath) });
    const formatInfos = JSON.parse(list_supported_formats());
    const specSource = await readFile(roundtripSpecPath, "utf8");
    const anycubicTargetsSource = await readFile(anycubicTargetsPath, "utf8");
    const roundtripTargets = new Set([
        ...extractRoundtripTargetNames(specSource),
        ...extractStringLiterals(anycubicTargetsSource)
    ]);

    const missingTargets = exportableRoundtripTargetNames(formatInfos).filter(
        (targetName) => !roundtripTargets.has(targetName)
    );

    assert.deepEqual(missingTargets, []);
});

function exportableRoundtripTargetNames(formatInfos) {
    return formatInfos
        .filter((info) => info.capabilities?.can_export === true)
        .flatMap((info) => {
            if (info.specifics?.family === "anycubic") {
                return [info.specifics.extension];
            }
            if (info.specifics?.family === "anycubic-archive") {
                return (info.target_names ?? []).map((targetName) =>
                    targetName === info.name ? info.default_extension : targetName
                );
            }
            if (info.specifics?.family === "chitubox") {
                return [info.name];
            }
            return [info.name];
        })
        .filter((targetName, index, targetNames) => targetNames.indexOf(targetName) === index)
        .sort((left, right) => left.localeCompare(right));
}

function extractRoundtripTargetNames(source) {
    const targets = new Set();

    for (const match of source.matchAll(/(?:value|group|version|targetLabel):\s*"([^"]+)"/gu)) {
        targets.add(match[1]);
    }

    for (const match of source.matchAll(/\[\s*((?:"[^"]+"\s*,?\s*)+)\]\.map/gu)) {
        for (const item of match[1].matchAll(/"([^"]+)"/gu)) {
            targets.add(item[1]);
        }
    }

    return targets;
}

function extractStringLiterals(source) {
    return [...source.matchAll(/"([^"]+)"/gu)].map((match) => match[1]);
}
