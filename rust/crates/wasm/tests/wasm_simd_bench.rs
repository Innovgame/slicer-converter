//! T3.5 / R5 — `wasm-bindgen-test` micro-benchmarks for the WASM SIMD hot paths.
//!
//! @zh 在浏览器内通过 `performance.now()` 实测 CTB / GOO RLE 的 WASM 吞吐，
//!     用于 R5 的 SIMD 收口验证（criterion 仅覆盖 native，无法替代）。
//! @en In-browser micro-benchmarks measured via `performance.now()` for the
//!     CTB / GOO RLE hot paths. Required by R5: native criterion does not
//!     reflect actual WASM SIMD throughput.
//!
//! 运行 / Run:
//!     `node scripts/run-wasm-browser-test.mjs --test wasm_simd_bench`
//!
//! 输出格式：每个 bench 打印一行 `[wasm-simd-bench] <name> <iters> iters / <total_ms> ms = <per_op_ms> ms/op (<MiB/s>)`
//! 结果应回填 `findings.md` 的 R5 段落。

#![cfg(target_arch = "wasm32")]

use slicer_converter_core::formats::{ctb, goo};
use wasm_bindgen_test::*;
use web_sys::window;

wasm_bindgen_test_configure!(run_in_browser);

/// Resolution used for SIMD micro-bench. 1024×1024 keeps each iteration well under
/// the chromedriver test timeout while remaining large enough to dominate setup
/// overhead and exercise the v128 inner loops.
const W: u32 = 1024;
const H: u32 = 1024;
/// Iteration count per bench run. Picked to land the total bench under ~3s on
/// modern hardware so the harness stays well within `wasm-bindgen-test` timeouts.
const ITERS: u32 = 30;

fn now_ms() -> f64 {
    window()
        .expect("window")
        .performance()
        .expect("performance")
        .now()
}

fn make_pixels(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let x = (index % width) as u8;
        let y = (index / width) as u8;
        *pixel = x.wrapping_mul(3) ^ y.wrapping_mul(5);
    }
    pixels
}

fn report(name: &str, iters: u32, total_ms: f64, bytes_per_op: usize) {
    let per_op_ms = total_ms / f64::from(iters);
    let mib_per_s = (bytes_per_op as f64) / (per_op_ms / 1000.0) / (1024.0 * 1024.0);
    // wasm-bindgen-test only surfaces stdout on FAILURE, so we deliberately
    // panic with the bench numbers. This file is a bench harness, not a
    // regression test — the "failures" ARE the result. Run via:
    //     `node scripts/run-wasm-browser-test.mjs --release --test wasm_simd_bench`
    panic!(
        "[wasm-simd-bench] {name} {iters} iters / {total_ms:.2} ms = {per_op_ms:.3} ms/op ({mib_per_s:.1} MiB/s)"
    );
}

#[wasm_bindgen_test]
fn bench_ctb_rle_encode_simd() {
    let pixels = make_pixels(W as usize, H as usize);
    // Warmup
    let _ = ctb::encode_layer_data(&pixels, 0, W, H).expect("ctb encode warmup");

    let start = now_ms();
    let mut sink = 0usize;
    for _ in 0..ITERS {
        let encoded = ctb::encode_layer_data(&pixels, 0, W, H).expect("ctb encode");
        sink = sink.wrapping_add(encoded.len());
    }
    let total = now_ms() - start;
    // Prevent the optimizer from eliding the loop.
    assert!(sink > 0);
    report("ctb_rle_encode_1024x1024", ITERS, total, pixels.len());
}

#[wasm_bindgen_test]
fn bench_ctb_rle_decode_simd() {
    let pixels = make_pixels(W as usize, H as usize);
    let encoded = ctb::encode_layer_data(&pixels, 0, W, H).expect("ctb encode for decode bench");
    let mut buf = Vec::with_capacity(pixels.len());

    // Warmup
    buf.clear();
    ctb::decode_layer_data_into(&encoded, 0, W, H, 1, 0, &mut buf).expect("ctb decode warmup");

    let start = now_ms();
    let mut sink = 0usize;
    for _ in 0..ITERS {
        buf.clear();
        ctb::decode_layer_data_into(&encoded, 0, W, H, 1, 0, &mut buf).expect("ctb decode");
        sink = sink.wrapping_add(buf.len());
    }
    let total = now_ms() - start;
    assert!(sink > 0);
    report("ctb_rle_decode_1024x1024", ITERS, total, pixels.len());
}

#[wasm_bindgen_test]
fn bench_goo_rle_encode_simd() {
    let pixels = make_pixels(W as usize, H as usize);
    let _ = goo::encode_layer_data(&pixels, W, H).expect("goo encode warmup");

    let start = now_ms();
    let mut sink = 0usize;
    for _ in 0..ITERS {
        let encoded = goo::encode_layer_data(&pixels, W, H).expect("goo encode");
        sink = sink.wrapping_add(encoded.len());
    }
    let total = now_ms() - start;
    assert!(sink > 0);
    report("goo_rle_encode_1024x1024", ITERS, total, pixels.len());
}
