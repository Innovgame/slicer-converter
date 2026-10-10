use super::fallback::{scan_gray7_run_end_swar, scan_u8_run_end_swar};

/// SSE2 scanner for CTB-style gray7 runs on 32-bit x86.
///
/// The SIMD loop handles aligned semantics with unaligned loads; mismatches and
/// tails are handed back to SWAR so the end-index contract stays shared.
///
/// 32-bit x86 上的 CTB-style gray7 SSE2 scanner。SIMD loop 使用 unaligned loads；
/// mismatch 与 tail 回到 SWAR，以保持共享的 end-index 契约。
#[cfg(target_arch = "x86")]
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scan_gray7_run_end_sse2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86::*;

    let cur_masked = cur & 0xFE;
    let mask_v = _mm_set1_epi8(0xFEu8 as i8);
    let broadcast_v = _mm_set1_epi8(cur_masked as i8);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { _mm_loadu_si128(ptr.add(i) as *const __m128i) };
        let masked = _mm_and_si128(chunk, mask_v);
        let eq = _mm_cmpeq_epi8(masked, broadcast_v);
        let bitmask = _mm_movemask_epi8(eq) as u32;
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) };
        }
    }

    unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) }
}

/// SSE2 scanner for exact `u8` runs on 32-bit x86.
///
/// mismatch 与 tail 回到 SWAR，以保持 SIMD 与 fallback 语义一致。
#[cfg(target_arch = "x86")]
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scan_u8_run_end_sse2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86::*;

    let broadcast_v = _mm_set1_epi8(cur as i8);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { _mm_loadu_si128(ptr.add(i) as *const __m128i) };
        let eq = _mm_cmpeq_epi8(chunk, broadcast_v);
        let bitmask = _mm_movemask_epi8(eq) as u32;
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) };
        }
    }

    unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) }
}

/// SSE2 scanner for CTB-style gray7 runs on x86_64.
///
/// The runtime dispatcher only selects this function after confirming SSE2
/// availability.
///
/// x86_64 上的 CTB-style gray7 SSE2 scanner。runtime dispatcher 只会在确认 SSE2 可用后选择它。
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scan_gray7_run_end_sse2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86_64::*;

    let cur_masked = cur & 0xFE;
    let mask_v = _mm_set1_epi8(0xFEu8 as i8);
    let broadcast_v = _mm_set1_epi8(cur_masked as i8);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { _mm_loadu_si128(ptr.add(i) as *const __m128i) };
        let masked = _mm_and_si128(chunk, mask_v);
        let eq = _mm_cmpeq_epi8(masked, broadcast_v);
        let bitmask = _mm_movemask_epi8(eq) as u32;
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) };
        }
    }

    unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) }
}

/// SSE2 scanner for exact `u8` runs on x86_64.
///
/// x86_64 上的精确 `u8` SSE2 scanner。
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scan_u8_run_end_sse2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86_64::*;

    let broadcast_v = _mm_set1_epi8(cur as i8);
    let mut i = start + 1;

    while i + 16 <= len {
        let chunk = unsafe { _mm_loadu_si128(ptr.add(i) as *const __m128i) };
        let eq = _mm_cmpeq_epi8(chunk, broadcast_v);
        let bitmask = _mm_movemask_epi8(eq) as u32;
        if bitmask == 0xFFFF {
            i += 16;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) };
        }
    }

    unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) }
}

/// AVX2 scanner for CTB-style gray7 runs on 32-bit x86.
///
/// Scans 32 bytes per iteration, then delegates mismatch/tail handling to SWAR.
///
/// 32-bit x86 上的 CTB-style gray7 AVX2 scanner。每轮扫描 32 bytes，mismatch/tail
/// 交给 SWAR 处理。
#[cfg(target_arch = "x86")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scan_gray7_run_end_avx2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86::*;

    let cur_masked = cur & 0xFE;
    let mask_v = _mm256_set1_epi8(0xFEu8 as i8);
    let broadcast_v = _mm256_set1_epi8(cur_masked as i8);
    let mut i = start + 1;

    while i + 32 <= len {
        let chunk = unsafe { _mm256_loadu_si256(ptr.add(i) as *const __m256i) };
        let masked = _mm256_and_si256(chunk, mask_v);
        let eq = _mm256_cmpeq_epi8(masked, broadcast_v);
        let bitmask = _mm256_movemask_epi8(eq) as u32;
        if bitmask == u32::MAX {
            i += 32;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) };
        }
    }

    unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) }
}

/// AVX2 scanner for exact `u8` runs on 32-bit x86.
///
/// 32-bit x86 上的精确 `u8` AVX2 scanner。
#[cfg(target_arch = "x86")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scan_u8_run_end_avx2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86::*;

    let broadcast_v = _mm256_set1_epi8(cur as i8);
    let mut i = start + 1;

    while i + 32 <= len {
        let chunk = unsafe { _mm256_loadu_si256(ptr.add(i) as *const __m256i) };
        let eq = _mm256_cmpeq_epi8(chunk, broadcast_v);
        let bitmask = _mm256_movemask_epi8(eq) as u32;
        if bitmask == u32::MAX {
            i += 32;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) };
        }
    }

    unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) }
}

/// AVX2 scanner for CTB-style gray7 runs on x86_64.
///
/// Scans 32 bytes per iteration, then delegates mismatch/tail handling to SWAR.
///
/// x86_64 上的 CTB-style gray7 AVX2 scanner。每轮扫描 32 bytes，mismatch/tail
/// 交给 SWAR 处理。
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scan_gray7_run_end_avx2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86_64::*;

    let cur_masked = cur & 0xFE;
    let mask_v = _mm256_set1_epi8(0xFEu8 as i8);
    let broadcast_v = _mm256_set1_epi8(cur_masked as i8);
    let mut i = start + 1;

    while i + 32 <= len {
        let chunk = unsafe { _mm256_loadu_si256(ptr.add(i) as *const __m256i) };
        let masked = _mm256_and_si256(chunk, mask_v);
        let eq = _mm256_cmpeq_epi8(masked, broadcast_v);
        let bitmask = _mm256_movemask_epi8(eq) as u32;
        if bitmask == u32::MAX {
            i += 32;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) };
        }
    }

    unsafe { scan_gray7_run_end_swar(ptr, len, i.saturating_sub(1), cur_masked) }
}

/// AVX2 scanner for exact `u8` runs on x86_64.
///
/// x86_64 上的精确 `u8` AVX2 scanner。
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scan_u8_run_end_avx2(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    use std::arch::x86_64::*;

    let broadcast_v = _mm256_set1_epi8(cur as i8);
    let mut i = start + 1;

    while i + 32 <= len {
        let chunk = unsafe { _mm256_loadu_si256(ptr.add(i) as *const __m256i) };
        let eq = _mm256_cmpeq_epi8(chunk, broadcast_v);
        let bitmask = _mm256_movemask_epi8(eq) as u32;
        if bitmask == u32::MAX {
            i += 32;
        } else {
            i += bitmask.trailing_ones() as usize;
            return unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) };
        }
    }

    unsafe { scan_u8_run_end_swar(ptr, len, i.saturating_sub(1), cur) }
}
