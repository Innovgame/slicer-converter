//! GOO-family facade helpers.
//!
//! This module exposes the historical direct helper API for GOO/PRZ-style
//! binary containers while the registry path uses the same functions through
//! `GooFamilyHeaderLayout`, `GooFamilyLayerCodec`, and `GooFamilyFooterPolicy`.
//!
//! GOO-family facade helpers。本模块保留 GOO/PRZ-style binary container 的历史直接
//! helper API；registry 路径则通过 `GooFamilyHeaderLayout`、`GooFamilyLayerCodec` 与
//! `GooFamilyFooterPolicy` 复用同一组函数。

pub mod adapters;
pub mod decoder;
pub mod encoder;
pub mod header;
pub mod preview;
pub mod rle;
pub mod specs;

#[cfg(test)]
mod rle_tests;

pub use decoder::GooDecoder;
pub use specs::FAMILY;

use super::LayerEncodeContext;
use crate::common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage};

/// GOO-family file magic used by both GOO and PRZ containers.
///
/// GOO 与 PRZ container 共享的 GOO-family file magic。
pub const MAGIC_BYTES: &[u8; 8] = &[0x07, 0x00, 0x00, 0x00, 0x44, 0x4C, 0x50, 0x00];
/// Version marker written by the GOO-family header encoder.
///
/// GOO-family header encoder 写入的版本标记。
pub const VERSION_STRING: &[u8; 4] = b"V3.0";
/// CRLF delimiter used in GOO-family text blocks.
///
/// GOO-family text block 使用的 CRLF delimiter。
pub const DELIMITER: [u8; 2] = [0x0D, 0x0A];
/// Per-layer RLE magic byte.
///
/// 单层 RLE payload 的 magic byte。
pub const LAYER_MAGIC: u8 = 0x55;

/// Detect whether the input looks like a GOO-family container (GOO/PRZ).
///
/// 检测输入数据是否为 GOO-family 容器格式（GOO/PRZ）。
pub fn looks_like_goo_family(data: &[u8]) -> bool {
    data.len() >= 12 && &data[4..12] == MAGIC_BYTES
}

/// Parse a GOO-family header into shared metadata + layer offsets.
///
/// 解析 GOO 容器头部，返回共用元数据和图层偏移表。
pub fn parse_header_data(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let decoder = GooDecoder::parse_header(data)?;
    let result = decoder.to_parse_result();
    Ok((result.metadata, result.layers))
}

/// Decode a single GOO layer's RLE data into grayscale pixels.
///
/// 将单个 GOO 图层的 RLE 字节解码为灰度像素。
pub fn decode_layer_data(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    rle::decode_goo_rle(data, width, height)
}

/// Decode a GOO-family layer directly into an RGBA preview image.
///
/// When the layer already fits the requested preview size, this returns a
/// full-size RGBA buffer. Otherwise it uses the RLE preview decoder to avoid
/// materializing the full grayscale layer.
///
/// 将 GOO-family layer 直接解码为 RGBA preview image。若图层本身已不超过请求尺寸，则返回
/// full-size RGBA buffer；否则走 RLE preview decoder，避免 materialize 完整灰度层。
pub fn decode_layer_preview_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    max_preview_size: u32,
) -> Result<PreviewLayerImage, ConvertError> {
    if width <= max_preview_size && height <= max_preview_size {
        let pixels = rle::decode_goo_rle_rgba(data, width, height)?;
        return Ok(PreviewLayerImage {
            pixels,
            width,
            height,
        });
    }

    let (pixels, dst_width, dst_height) =
        rle::decode_goo_rle_preview_rgba(data, width, height, max_preview_size)?;
    Ok(PreviewLayerImage {
        pixels,
        width: dst_width,
        height: dst_height,
    })
}

/// Encode grayscale pixels into GOO RLE format.
///
/// The public GOO helper enables color-difference compression, matching the GOO
/// profile. PRZ uses `encode_layer_data_with_options` with this option disabled.
///
/// 将灰度像素编码为 GOO RLE。公开 GOO helper 会启用 color-difference compression，
/// 与 GOO profile 保持一致；PRZ 通过 `encode_layer_data_with_options` 关闭该选项。
pub fn encode_layer_data(
    pixels: &[u8],
    _width: u32,
    _height: u32,
) -> Result<Vec<u8>, ConvertError> {
    encode_layer_data_with_options(pixels, _width, _height, true)
}

/// Encode grayscale pixels into a GOO-family layer with configurable diff compression.
///
/// This is the shared entrypoint for GOO and PRZ profiles.
///
/// 将灰度像素编码为可配置 diff compression 的 GOO-family layer；这是 GOO 与 PRZ profile
/// 共享的入口。
pub fn encode_layer_data_with_options(
    pixels: &[u8],
    _width: u32,
    _height: u32,
    use_color_difference_compression: bool,
) -> Result<Vec<u8>, ConvertError> {
    rle::encode_goo_rle_with_options(pixels, use_color_difference_compression)
}

/// Encode a GOO-family layer into a caller-owned buffer.
///
/// When `ctx.layer_metadata` is present, the output includes the GOO layer
/// definition wrapper and checksum. Otherwise it contains only the raw RLE
/// payload. This lets binary file assembly and direct layer helpers share the
/// same codec implementation.
///
/// 将 GOO-family layer 编码到调用方持有的 buffer。存在 `ctx.layer_metadata` 时，输出包含
/// GOO layer definition wrapper 与 checksum；否则只包含 raw RLE payload。这样二进制文件组装与
/// 直接 layer helper 可以共享同一 codec 实现。
pub(crate) fn encode_goo_layer_into(
    pixels: &[u8],
    ctx: &LayerEncodeContext<'_>,
    use_color_difference_compression: bool,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    if let Some(metadata) = ctx.layer_metadata {
        let rle_start =
            encoder::begin_goo_layer_with_metadata_into(metadata, ctx.layer_index, output)?;
        rle::encode_goo_rle_with_options_append(pixels, use_color_difference_compression, output)?;
        encoder::finish_goo_layer_with_metadata_in_place(output, rle_start)
    } else {
        rle::encode_goo_rle_with_options_into(pixels, use_color_difference_compression, output)
    }
}

/// Build GOO file header bytes from metadata.
///
/// 根据元数据构建 GOO 文件头字节。
pub fn build_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    encoder::build_goo_header(metadata)
}

/// Build GOO-family footer bytes.
///
/// The footer is three zero padding bytes followed by the shared family magic.
///
/// 构建 GOO-family footer：三个零 padding bytes 后接共享 family magic。
pub fn build_footer_bytes() -> Vec<u8> {
    let mut footer = vec![0u8; 3];
    footer.extend_from_slice(MAGIC_BYTES);
    footer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::downsample_to_rgba_nearest;
    use crate::formats::LayerDecodeContext;

    fn make_test_layer(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height) as usize];
        for y in 0..height as usize {
            for x in 0..width as usize {
                pixels[y * width as usize + x] = ((x * 7 + y * 11) % 256) as u8;
            }
        }
        pixels
    }

    #[test]
    fn preview_fast_path_matches_generic_downsample() {
        let width = 16;
        let height = 12;
        let max_preview_size = 6;
        let original = make_test_layer(width, height);
        let encoded = encode_layer_data(&original, width, height).unwrap();
        let ctx = LayerDecodeContext::new(0, width, height);

        let preview = specs::GOO_SPEC
            .decode_layer_preview_rgba(&encoded, &ctx, max_preview_size)
            .unwrap()
            .expect("GOO preview fast path should be implemented");

        let decoded = decode_layer_data(&encoded, width, height).unwrap();
        let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
        assert_eq!(preview.width, expected.width);
        assert_eq!(preview.height, expected.height);
        assert_eq!(preview.pixels, expected.pixels);
    }
}
