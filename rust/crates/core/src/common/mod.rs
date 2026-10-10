//! Shared data model, image helpers, and error types.
//!
//! This module is the format-agnostic layer used by codecs, transcode routes,
//! CLI, and WASM bindings. Concrete format modules should translate their native
//! headers/manifests into these normalized structs before crossing boundaries.
//!
//! 这里是格式无关的共享层，包含通用数据模型、图像 helper 与错误类型。具体格式模块应先把
//! native header/manifest 转换成这些归一化结构，再跨越 codec、transcode、CLI 或 WASM 边界。

pub mod error;
pub mod layer;
pub mod metadata;
pub(crate) mod simd;

pub use error::ConvertError;
pub use layer::{
    downsample_grayscale_nearest, downsample_to_rgba_nearest, thumbnail_rgba_for_size, LayerImage,
    PreviewDecodePlan, PreviewLayerImage,
};
pub use metadata::{
    ExposurePlan, FileMetadata, FileMetadataBuilder, LayerEncodeMetadata, LayerOffset, MotionPlan,
    OwnedLayerEncodeMetadata, ParseResult, PerLayerSettings, PrintStatistics, PrinterSpec,
    SlicingSpec, Thumbnail, WaitTimes,
};
