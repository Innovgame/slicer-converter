/// Shared data model, metadata builders, binary helpers, and image utilities.
///
/// These types form the stable core shared by native Rust conversion and the
/// WebAssembly API layer.
///
/// 通用数据模型、metadata builder、二进制辅助工具与图像工具；供 Rust 原生转换和
/// WebAssembly API 层共同使用。
pub mod common;

/// Format implementations and registries.
///
/// This module owns concrete slicer codecs, archive layouts, and the registry
/// used to resolve import/export capabilities.
///
/// 格式实现与 registry；包含具体切片 codec、archive layout，以及用于解析
/// import/export 能力的格式注册表。
pub mod formats;

/// Layer-level transcode orchestration.
///
/// The transcode layer connects source and target format specs while preserving
/// fast-path stats for callers that need to inspect route behavior.
///
/// 单图层转码编排层；连接源格式与目标格式 spec，并保留 fast-path 统计信息，方便调用方检查路线行为。
pub mod transcode;
