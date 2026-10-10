import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const pipelinePerLayerThreshold = Number(process.env.BENCH_PIPELINE_MAX_MS_PER_LAYER ?? "25");
const benchmarkLayers = Number(process.env.BENCH_RLE_LAYERS ?? "10");

if (!Number.isFinite(pipelinePerLayerThreshold) || pipelinePerLayerThreshold <= 0) {
    throw new Error("BENCH_PIPELINE_MAX_MS_PER_LAYER must be a positive number");
}

if (!Number.isFinite(benchmarkLayers) || benchmarkLayers <= 0) {
    throw new Error("BENCH_RLE_LAYERS must be a positive number");
}

const samplePath = resolve("rust/crates/core/data/Untitled_20260413_1125_TI3U.goo");
if (!existsSync(samplePath)) {
    console.log(`Benchmark threshold skipped: real GOO sample is not present at ${samplePath}`);
    process.exit(0);
}

let output;
try {
    output = execSync(
        "cargo run --manifest-path rust/Cargo.toml -p slicer-converter-core --example bench_rle --release",
        {
            cwd: process.cwd(),
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"],
            env: {
                ...process.env,
                BENCH_RLE_LAYERS: String(Math.trunc(benchmarkLayers))
            }
        }
    );
} catch (error) {
    const stderr = typeof error.stderr === "string" ? error.stderr : "";
    const stdout = typeof error.stdout === "string" ? error.stdout : "";
    throw new Error(`bench_rle failed.\n${stdout}\n${stderr}`, { cause: error });
}

const pipelineMatch = output.match(/Pipeline \(GOO decode \+ CTB encode\):\s+([0-9.]+)ms\s+\(([0-9.]+)ms\/layer\)/);
if (!pipelineMatch) {
    throw new Error(`Failed to parse pipeline timing from benchmark output.\n${output}`);
}

const pipelineTotalMs = Number(pipelineMatch[1]);
const pipelinePerLayerMs = Number(pipelineMatch[2]);

if (!Number.isFinite(pipelineTotalMs) || !Number.isFinite(pipelinePerLayerMs)) {
    throw new Error(`Parsed benchmark values are invalid.\n${output}`);
}

console.log(`Benchmark pipeline total: ${pipelineTotalMs.toFixed(1)}ms`);
console.log(`Benchmark pipeline per layer: ${pipelinePerLayerMs.toFixed(2)}ms/layer`);
console.log(`Threshold: <= ${pipelinePerLayerThreshold.toFixed(2)}ms/layer`);

if (pipelinePerLayerMs > pipelinePerLayerThreshold) {
    throw new Error(
        `Benchmark regression detected: ${pipelinePerLayerMs.toFixed(2)}ms/layer > ${pipelinePerLayerThreshold.toFixed(2)}ms/layer`
    );
}

console.log("Benchmark threshold check passed.");
