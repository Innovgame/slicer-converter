//! Layer transcode engine and fast-path registry.
//!
//! This module routes one encoded layer from a source `FormatSpec` to a target
//! `FormatSpec`. Registered fast paths may transform encoded payloads directly;
//! otherwise the engine falls back to decode-to-grayscale then encode.
//!
//! 这里是逐层转码引擎与快路径 registry。它把单层 encoded payload 从源 `FormatSpec`
//! 转到目标 `FormatSpec`；已注册快路径可直接转换 encoded payload，否则回退到灰度解码再编码。

pub mod engine;
pub mod execute;
pub mod fallback;
pub mod paths;
pub mod registry;
pub mod request;
pub mod route;
pub mod stats;

pub use engine::{default_engine, TranscodeEngine};
pub use execute::{execute_layer_into, execute_layer_into_with_stats, LayerExecutionBuffers};
pub use registry::{default_registry, TranscodeRegistry};
pub use request::{FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder};
pub use route::{TranscodeRouteDescriptor, TranscodeRouteKey};
pub use stats::{LayerTranscodeStats, RouteTranscodeStats, TranscodeStats};
