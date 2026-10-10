//! Voxeldance VDT archive facade helpers.
//!
//! VDT uses a ZIP container with `manifest.json` plus PNG grayscale layer
//! entries. Layer PNG encoding/decoding is shared with SL1; VDT-specific logic
//! lives in manifest and entry naming.
//!
//! Voxeldance VDT archive facade helpers。VDT 使用包含 `manifest.json` 与 PNG grayscale
//! layer entries 的 ZIP container。layer PNG 编解码与 SL1 共享；VDT-specific 逻辑集中在
//! manifest 与 entry naming。

pub mod adapters;
pub mod decoder;
mod entries;
pub mod manifest;
pub mod specs;

pub use specs::{FAMILY, PROFILES, VDT_SPEC};

use crate::common::{ConvertError, FileMetadata, LayerOffset};

// ── Convenience functions ────────────────────────────────────

/// Parse a VDT ZIP header into shared metadata + layer offsets.
///
/// 解析 VDT ZIP 头部，返回共用元数据和图层偏移表。
pub fn parse_header_data(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    decoder::parse_header(data)
}

/// Decode a single VDT layer (PNG) into grayscale pixels.
///
/// 将单个 VDT layer PNG 解码为灰度像素。
pub fn decode_layer_data(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    super::sl1::decode_layer_data(data, width, height)
}

/// Decode a single VDT layer (PNG) into a pre-allocated buffer.
///
/// 将单个 VDT layer PNG 解码到预分配 buffer，复用其已有容量。
pub fn decode_layer_data_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    super::sl1::decode_layer_data_into(data, width, height, output)
}

/// Encode grayscale pixels as PNG bytes (VDT layer format).
///
/// 将灰度像素编码为 VDT layer 使用的 PNG bytes。
pub fn encode_layer_data(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    super::sl1::encode_layer_data(pixels, width, height)
}

/// Build VDT manifest JSON bytes from metadata.
///
/// 从 metadata 构建 VDT manifest JSON bytes。
pub fn build_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let manifest = manifest::VdtManifest::from_metadata(metadata);
    serde_json::to_vec_pretty(&manifest)
        .map_err(|e| ConvertError::IniError(format!("JSON serialization failed: {e}")))
}
