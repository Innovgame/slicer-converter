pub mod specs;

use crate::common::{ConvertError, FileMetadata, LayerOffset};

use super::goo;

pub use specs::FAMILY;

/// Parse a PRZ header into shared metadata + layer offsets.
///
/// 解析 PRZ 头部，返回共用元数据和图层偏移表。
pub fn parse_header_data(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    goo::parse_header_data(data)
}

/// Decode a single PRZ layer.
/// 解码单个 PRZ 图层的 RLE 数据（复用 GOO 解码器）。
pub fn decode_layer_data(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    goo::decode_layer_data(data, width, height)
}

/// Encode grayscale pixels into PRZ RLE.
/// 将灰度像素编码为 PRZ RLE 格式（GOO 容器，禁用差分压缩）。
///
/// UVtools treats PRZ as a GOO-family container with color-diff compression disabled.
pub fn encode_layer_data(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    goo::encode_layer_data_with_options(pixels, width, height, false)
}

/// Build PRZ file header bytes.
/// 构建 PRZ 文件头部字节流（与 GOO 格式共用）。
pub fn build_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    goo::build_header_bytes(metadata)
}

/// Build PRZ footer bytes.
/// 构建 PRZ 文件尾部字节流（与 GOO 格式共用）。
pub fn build_footer_bytes() -> Vec<u8> {
    goo::build_footer_bytes()
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
                pixels[y * width as usize + x] = ((x * 5 + y * 13) % 256) as u8;
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

        let preview = specs::PRZ_SPEC
            .decode_layer_preview_rgba(&encoded, &ctx, max_preview_size)
            .unwrap()
            .expect("PRZ preview fast path should be implemented");

        let decoded = decode_layer_data(&encoded, width, height).unwrap();
        let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
        assert_eq!(preview.width, expected.width);
        assert_eq!(preview.height, expected.height);
        assert_eq!(preview.pixels, expected.pixels);
    }
}
