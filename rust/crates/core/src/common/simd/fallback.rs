/// Scalar scanner for CTB-style gray7 runs.
///
/// It compares bytes after masking out the low bit, so values that only differ
/// by bit 0 are considered part of the same run. Callers must pass a valid
/// pointer for `len` bytes and a `cur_masked` value that has already been masked
/// with `0xFE`.
///
/// CTB-style gray7 run 的 scalar scanner。它会忽略最低 bit 进行比较，因此只在 bit 0
/// 不同的值仍属于同一个 run。调用方必须保证 `ptr` 指向至少 `len` bytes，且 `cur_masked`
/// 已经用 `0xFE` mask 过。
#[inline(always)]
pub(super) unsafe fn scan_gray7_run_end_scalar(
    ptr: *const u8,
    len: usize,
    mut i: usize,
    cur_masked: u8,
) -> usize {
    while i < len && (unsafe { *ptr.add(i) } & 0xFE) == cur_masked {
        i += 1;
    }
    i
}

/// Scalar scanner for exact `u8` runs.
///
/// Callers must pass a valid pointer for `len` bytes, and `i` must be within or
/// equal to the slice length.
///
/// 精确 `u8` run 的 scalar scanner。调用方必须保证 `ptr` 指向至少 `len` bytes，且 `i`
/// 在 slice 范围内或等于长度。
#[inline(always)]
pub(super) unsafe fn scan_u8_run_end_scalar(
    ptr: *const u8,
    len: usize,
    mut i: usize,
    cur: u8,
) -> usize {
    while i < len && unsafe { *ptr.add(i) } == cur {
        i += 1;
    }
    i
}

/// Portable SWAR scanner for CTB-style gray7 runs.
///
/// This is the non-SIMD fast path used on native targets when x86 SIMD is not
/// available. It scans eight bytes at a time with unaligned `u64` loads, then
/// falls back to scalar code for the tail.
///
/// CTB-style gray7 run 的 portable SWAR scanner。在 native target 上没有 x86 SIMD 时使用；
/// 它通过 unaligned `u64` load 一次扫描 8 bytes，尾部回落到 scalar。
#[cfg(not(target_arch = "wasm32"))]
#[inline(always)]
pub(super) unsafe fn scan_gray7_run_end_swar(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    const MASK8: u64 = 0xFE_FE_FE_FE_FE_FE_FE_FE;
    let cur_masked = cur & 0xFE;
    let broadcast = (cur_masked as u64) * 0x0101_0101_0101_0101;
    let mut i = start + 1;

    while i + 8 <= len {
        let chunk = unsafe { (ptr.add(i) as *const u64).read_unaligned() };
        let diff = (chunk ^ broadcast) & MASK8;
        if diff == 0 {
            i += 8;
            continue;
        }

        #[cfg(target_endian = "little")]
        {
            i += (diff.trailing_zeros() as usize) / 8;
        }
        #[cfg(target_endian = "big")]
        {
            i += (diff.leading_zeros() as usize) / 8;
        }
        return i;
    }

    unsafe { scan_gray7_run_end_scalar(ptr, len, i, cur_masked) }
}

/// Portable SWAR scanner for exact `u8` runs.
///
/// It scans eight bytes at a time with unaligned `u64` loads and falls back to
/// scalar code for the tail.
///
/// 精确 `u8` run 的 portable SWAR scanner。它通过 unaligned `u64` load 一次扫描 8 bytes，
/// 尾部回落到 scalar。
#[cfg(not(target_arch = "wasm32"))]
#[inline(always)]
pub(super) unsafe fn scan_u8_run_end_swar(
    ptr: *const u8,
    len: usize,
    start: usize,
    cur: u8,
) -> usize {
    let broadcast = (cur as u64) * 0x0101_0101_0101_0101;
    let mut i = start + 1;

    while i + 8 <= len {
        let chunk = unsafe { (ptr.add(i) as *const u64).read_unaligned() };
        if chunk == broadcast {
            i += 8;
            continue;
        }

        let diff = chunk ^ broadcast;
        #[cfg(target_endian = "little")]
        {
            i += (diff.trailing_zeros() as usize) / 8;
        }
        #[cfg(target_endian = "big")]
        {
            i += (diff.leading_zeros() as usize) / 8;
        }
        return i;
    }

    unsafe { scan_u8_run_end_scalar(ptr, len, i, cur) }
}
