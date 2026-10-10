//! Prusa SL1 archive facade helpers.
//!
//! SL1 stores metadata INI files and PNG grayscale layer entries inside a ZIP.
//! The direct helpers here are reused by `Sl1ArchiveHeaderLayout`,
//! `Sl1ArchiveLayout`, and the shared PNG `LayerCodec`.
//!
//! Prusa SL1 archive facade helpers。SL1 在 ZIP 中存储 metadata INI files 与 PNG grayscale
//! layer entries；这里的 direct helpers 由 `Sl1ArchiveHeaderLayout`、`Sl1ArchiveLayout`
//! 与共享 PNG `LayerCodec` 复用。

pub mod adapters;
pub mod config;
pub mod decoder;
pub mod encoder;
pub mod specs;

pub use decoder::Sl1Decoder;
pub use specs::{FAMILY, PROFILES, SL1_SPEC};

use crate::common::{ConvertError, FileMetadata};

/// Decode a single layer PNG entry to grayscale pixels.
///
/// 将单个 SL1 图层的 PNG 字节解码为灰度像素。
pub fn decode_layer_data(
    png_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, ConvertError> {
    decoder::decode_png_layer(png_data, width, height)
}

/// Decode a single layer PNG entry into a pre-allocated buffer.
///
/// 将单个 layer PNG entry 解码到预分配 buffer，复用其已有容量。
pub fn decode_layer_data_into(
    png_data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    decoder::decode_png_layer_into(png_data, width, height, output)
}

/// Encode grayscale pixels as PNG bytes (for SL1 ZIP entry).
///
/// 将灰度像素编码为 PNG 字节（SL1 ZIP 条目格式）。
pub fn encode_layer_data(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    encoder::encode_png_layer(pixels, width, height)
}

/// Build SL1 config.ini content from metadata.
///
/// 根据元数据构建 SL1 格式的 config.ini 内容。
pub fn build_config_ini(metadata: &FileMetadata) -> String {
    config::build_config_ini(metadata)
}
