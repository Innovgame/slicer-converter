//! Runtime-selected run scanners for hot RLE encoders.
//!
//! The public crate API never exposes this module. It gives CTB/GOO RLE encoders
//! a small, architecture-aware primitive for finding the end of a repeated byte
//! run while keeping the scalar/SWAR/SIMD implementations behind one wrapper.
//!
//! 这是 RLE encoder 热路径使用的运行期 run scanner。crate 公共 API 不暴露该模块；它为
//! CTB/GOO RLE encoder 提供一个小型、架构感知的重复 byte run 终点查找原语，并把
//! scalar/SWAR/SIMD 实现藏在统一 wrapper 后面。

mod fallback;
mod run_scan;

#[cfg(test)]
mod run_scan_tests;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

pub(crate) use run_scan::RunScanner;
