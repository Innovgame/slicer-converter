use super::fallback::{scan_gray7_run_end_scalar, scan_u8_run_end_scalar};

/// wasm128 scanner for CTB-style gray7 runs.
///
/// Mismatches and tails fall back to the scalar scanner so the SIMD and scalar
/// implementations share exactly the same end-index semantics.
///
/// CTB-style gray7 run 的 wasm128 scanner。遇到 mismatch 或 tail 时回落到 scalar scanner，
/// 确保 SIMD 与 scalar 实现共享完全一致的 end-index 语义。
#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub(super) unsafe fn scan_gray7_run_end_wasm128(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::wasm32::*;

    let cur_masked = cur & 0xFE;
    let mask_v = u8x16_splat(0xFE);
    let broadcast_v = u8x16_splat(cur_masked);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { v128_load(ptr.add(i) as *const v128) };
        let masked = v128_and(chunk, mask_v);
        let eq = u8x16_eq(masked, broadcast_v);
        let bitmask = u8x16_bitmask(eq);
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_gray7_run_end_scalar(ptr, len, i, cur_masked) };
        }
    }

    unsafe { scan_gray7_run_end_scalar(ptr, len, i, cur_masked) }
}

/// wasm128 scanner for exact `u8` runs.
///
/// Mismatches and tails fall back to the scalar scanner so the SIMD and scalar
/// implementations share exactly the same end-index semantics.
///
/// 精确 `u8` run 的 wasm128 scanner。遇到 mismatch 或 tail 时回落到 scalar scanner，
/// 确保 SIMD 与 scalar 实现共享完全一致的 end-index 语义。
#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub(super) unsafe fn scan_u8_run_end_wasm128(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::wasm32::*;

    let broadcast_v = u8x16_splat(cur);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { v128_load(ptr.add(i) as *const v128) };
        let eq = u8x16_eq(chunk, broadcast_v);
        let bitmask = u8x16_bitmask(eq);
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_u8_run_end_scalar(ptr, len, i, cur) };
        }
    }

    unsafe { scan_u8_run_end_scalar(ptr, len, i, cur) }
}
