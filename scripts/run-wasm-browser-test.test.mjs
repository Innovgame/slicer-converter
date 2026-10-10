import test from "node:test";
import assert from "node:assert/strict";

import { buildWasmPackArgs } from "./run-wasm-browser-test.mjs";

const WASM_CRATE_PATH = "rust/crates/wasm";

test("defaults to skipping wasm simd bench in browser test runs", () => {
    const args = buildWasmPackArgs([]);

    assert.deepEqual(args, ["test", "--headless", "--chrome", WASM_CRATE_PATH]);
});

test("keeps explicit wasm simd bench runs opt-in", () => {
    const args = buildWasmPackArgs(["--release", "--test", "wasm_simd_bench"]);

    assert.deepEqual(args, [
        "test",
        "--headless",
        "--chrome",
        WASM_CRATE_PATH,
        "--release",
        "--test",
        "wasm_simd_bench"
    ]);
});

test("keeps skipping wasm simd bench for generic browser test flags", () => {
    const args = buildWasmPackArgs(["--release"]);

    assert.deepEqual(args, ["test", "--headless", "--chrome", WASM_CRATE_PATH, "--release"]);
});

test("forwards cargo test args unchanged", () => {
    const args = buildWasmPackArgs(["--", "--nocapture"]);

    assert.deepEqual(args, ["test", "--headless", "--chrome", WASM_CRATE_PATH, "--", "--nocapture"]);
});
