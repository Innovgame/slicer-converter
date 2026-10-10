use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use super::fallback::{scan_gray7_run_end_swar, scan_u8_run_end_swar};
#[cfg(target_arch = "wasm32")]
use super::wasm::{scan_gray7_run_end_wasm128, scan_u8_run_end_wasm128};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use super::x86::{
    scan_gray7_run_end_avx2, scan_gray7_run_end_sse2, scan_u8_run_end_avx2, scan_u8_run_end_sse2,
};

/// Function pointer type for gray7 run scanners.
///
/// 函数指针类型：gray7 run scanner。
pub(crate) type Gray7RunScannerFn = unsafe fn(*const u8, usize, usize, u8) -> usize;
/// Function pointer type for exact `u8` run scanners.
///
/// 函数指针类型：精确 `u8` run scanner。
pub(crate) type U8RunScannerFn = unsafe fn(*const u8, usize, usize, u8) -> usize;

/// Runtime-selected run scanner pair.
///
/// `detect` picks the best implementation once per process. The scanner methods
/// are unsafe because the hot RLE loops pass raw pointers and lengths to avoid
/// repeated bounds checks.
///
/// 运行期选择的一组 run scanner。`detect` 在进程内只选择一次最佳实现。scanner 方法是 unsafe，
/// 因为 RLE 热路径为了避免重复边界检查会传入 raw pointer 与 length。
#[derive(Clone, Copy)]
pub(crate) struct RunScanner {
    scan_gray7_run_end: Gray7RunScannerFn,
    scan_u8_run_end: U8RunScannerFn,
}

impl RunScanner {
    /// Detects and caches the best scanner implementation for the current target.
    ///
    /// 检测并缓存当前 target 上最佳 scanner 实现。
    #[inline]
    pub(crate) fn detect() -> Self {
        static DETECTED: OnceLock<RunScanner> = OnceLock::new();
        *DETECTED.get_or_init(Self::detect_impl)
    }

    /// Scans a gray7 run and returns the first non-matching index.
    ///
    /// Scans from `start + 1`, assuming `start` already belongs to the run. The
    /// input pointer must be valid for `len` bytes.
    ///
    /// 扫描 gray7 run 并返回第一个不匹配 index。扫描从 `start + 1` 开始，假设 `start`
    /// 已属于该 run；输入 pointer 必须对 `len` bytes 有效。
    #[inline]
    pub(crate) unsafe fn scan_gray7_run_end(
        &self,
        ptr: *const u8,
        len: usize,
        start: usize,
        cur: u8,
    ) -> usize {
        unsafe { (self.scan_gray7_run_end)(ptr, len, start, cur) }
    }

    /// Scans an exact `u8` run and returns the first non-matching index.
    ///
    /// Scans from `start + 1`, assuming `start` already belongs to the run. The
    /// input pointer must be valid for `len` bytes.
    ///
    /// 扫描精确 `u8` run 并返回第一个不匹配 index。扫描从 `start + 1` 开始，假设 `start`
    /// 已属于该 run；输入 pointer 必须对 `len` bytes 有效。
    #[inline]
    pub(crate) unsafe fn scan_u8_run_end(
        &self,
        ptr: *const u8,
        len: usize,
        start: usize,
        cur: u8,
    ) -> usize {
        unsafe { (self.scan_u8_run_end)(ptr, len, start, cur) }
    }

    #[cfg(target_arch = "wasm32")]
    fn detect_impl() -> Self {
        Self {
            scan_gray7_run_end: scan_gray7_run_end_wasm128,
            scan_u8_run_end: scan_u8_run_end_wasm128,
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn detect_impl() -> Self {
        let (scan_gray7_run_end, scan_u8_run_end): (Gray7RunScannerFn, U8RunScannerFn) =
            if std::is_x86_feature_detected!("avx2") {
                (scan_gray7_run_end_avx2, scan_u8_run_end_avx2)
            } else if std::is_x86_feature_detected!("sse2") {
                (scan_gray7_run_end_sse2, scan_u8_run_end_sse2)
            } else {
                (scan_gray7_run_end_swar, scan_u8_run_end_swar)
            };
        Self {
            scan_gray7_run_end,
            scan_u8_run_end,
        }
    }

    #[cfg(not(any(target_arch = "wasm32", target_arch = "x86", target_arch = "x86_64")))]
    fn detect_impl() -> Self {
        Self {
            scan_gray7_run_end: scan_gray7_run_end_swar,
            scan_u8_run_end: scan_u8_run_end_swar,
        }
    }
}
