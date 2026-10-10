//! CTB/CBDDLP layer and document facade helpers.
//!
//! The Chitubox family now routes concrete profile behavior through
//! `formats::chitubox`, but these helpers remain the shared codec/header
//! building blocks for CTB-compatible profiles and tests.
//!
//! CTB/CBDDLP layer 与 document facade helpers。Chitubox family 的具体 profile 行为现在通过
//! `formats::chitubox` 路由，但这些 helper 仍是 CTB-compatible profiles 与测试共享的
//! codec/header building blocks。

pub mod crypto;
pub mod decoder;
pub mod encoder;
pub mod header;
pub mod preview;
pub mod rle;

#[cfg(test)]
mod rle_tests;

pub use decoder::CtbDecoder;

use crate::common::{downsample_to_rgba_nearest, ConvertError, FileMetadata, PreviewLayerImage};

/// CBDDLP/Photon legacy bitplane magic.
///
/// CBDDLP/Photon legacy bitplane magic。
pub const MAGIC_CBDDLP: u32 = 0x12FD_0019;
/// Generic CTB v1-v3 magic.
///
/// 通用 CTB v1-v3 magic。
pub const MAGIC_CTB: u32 = 0x12FD_0086;
/// Generic CTB v4/v5 magic.
///
/// 通用 CTB v4/v5 magic。
pub const MAGIC_CTB_V4: u32 = 0x12FD_0106;
/// UniFormation GKtwo CTB v4/v5 magic.
///
/// UniFormation GKtwo CTB v4/v5 magic。
pub const MAGIC_CTB_V4_GKTWO: u32 = 0xFF22_0810;

/// Decode a single layer's RLE data into grayscale pixels.
/// For CBDDLP format with anti_alias_level > 1, pass each bitplane separately
/// and use `merge_bitplanes` to combine them.
/// If `encryption_key` is non-zero, data is decrypted before RLE decode.
///
/// 将单个图层的 RLE 字节解码为灰度像素。
pub fn decode_layer_data(
    data: &[u8],
    layer_index: u32,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
) -> Result<Vec<u8>, ConvertError> {
    let rle_data;
    let slice = if encryption_key != 0 {
        rle_data = {
            let mut buf = data.to_vec();
            crypto::crypt_layer_data(&mut buf, encryption_key, layer_index);
            buf
        };
        &rle_data[..]
    } else {
        data
    };

    if anti_alias_level > 1 {
        // CBDDLP 7-bit RLE (1-bit black/white per bitplane)
        rle::decode_cbddlp_rle(slice, width, height, anti_alias_level)
    } else {
        // CTB variable-length RLE decode (grayscale)
        rle::decode_ctb_rle(slice, width, height)
    }
}

/// Decode a single layer's RLE data into a pre-allocated buffer.
/// Reuses the buffer's existing capacity to avoid heap allocations.
///
/// 将单个图层的 RLE 字节解码并写入预分配缓冲区，避免重复堆分配。
pub fn decode_layer_data_into(
    data: &[u8],
    layer_index: u32,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let rle_data;
    let slice = if encryption_key != 0 {
        rle_data = {
            let mut buf = data.to_vec();
            crypto::crypt_layer_data(&mut buf, encryption_key, layer_index);
            buf
        };
        &rle_data[..]
    } else {
        data
    };

    if anti_alias_level > 1 {
        rle::decode_cbddlp_rle_into(slice, width, height, anti_alias_level, output)
    } else {
        rle::decode_ctb_rle_into(slice, width, height, output)
    }
}

/// Decode one CTB/CBDDLP layer directly into an RGBA preview image.
///
/// Plain CTB grayscale RLE can use a direct downsample decoder for large
/// previews. CBDDLP bitplanes and encrypted layers first go through the normal
/// grayscale decode path to preserve bitplane merge and crypto behavior.
///
/// 将单个 CTB/CBDDLP layer 直接解码为 RGBA preview image。普通 CTB grayscale RLE 在大图预览时
/// 可使用 direct downsample decoder；CBDDLP bitplanes 与加密层先走常规灰度解码路径，以保留
/// bitplane merge 与 crypto 行为。
pub fn decode_layer_preview_rgba(
    data: &[u8],
    layer_index: u32,
    width: u32,
    height: u32,
    anti_alias_level: u32,
    encryption_key: u32,
    max_preview_size: u32,
) -> Result<PreviewLayerImage, ConvertError> {
    let rle_data;
    let slice = if encryption_key != 0 {
        rle_data = {
            let mut buf = data.to_vec();
            crypto::crypt_layer_data(&mut buf, encryption_key, layer_index);
            buf
        };
        &rle_data[..]
    } else {
        data
    };

    // For CBDDLP bitplane mode, keep generic decode path to preserve behavior.
    if anti_alias_level > 1 {
        let pixels = decode_layer_data(slice, layer_index, width, height, anti_alias_level, 0)?;
        return Ok(downsample_to_rgba_nearest(
            &pixels,
            width,
            height,
            max_preview_size,
        ));
    }

    if width <= max_preview_size && height <= max_preview_size {
        let pixels = decode_layer_data(slice, layer_index, width, height, 1, 0)?;
        return Ok(downsample_to_rgba_nearest(
            &pixels,
            width,
            height,
            max_preview_size,
        ));
    }

    let (pixels, dst_width, dst_height) =
        rle::decode_ctb_rle_preview_rgba(slice, width, height, max_preview_size)?;
    Ok(PreviewLayerImage {
        pixels,
        width: dst_width,
        height: dst_height,
    })
}

/// Encode grayscale pixels into CTB variable-length RLE.
///
/// 将灰度像素编码为 CTB variable-length RLE。
pub fn encode_layer_data(
    pixels: &[u8],
    _layer_index: u32,
    _width: u32,
    _height: u32,
) -> Result<Vec<u8>, ConvertError> {
    rle::encode_ctb_rle(pixels)
}

/// Build CTB file header bytes from universal metadata.
///
/// 从通用 metadata 构建 CTB header bytes。
pub fn build_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    encoder::build_ctb_header(metadata)
}

/// Patch CTB header layer table with actual layer data offsets and sizes.
///
/// 使用实际 layer data offsets 与 sizes patch CTB header layer table。
pub fn finalize_header_bytes(header: &[u8], layer_sizes: &[u32]) -> Result<Vec<u8>, ConvertError> {
    encoder::finalize_ctb_header(header, layer_sizes)
}

/// Merge multiple CBDDLP anti-alias bitplanes into a single grayscale image.
///
/// Each bitplane is a 1-bit (black=0 / white=255) pixel buffer of the same size.
/// The final pixel value = sum_of_bitplanes * (256 / aa_level), clamped to 255.
///
/// For aa_level=1, just returns the first plane unchanged.
///
/// 将多个 CBDDLP anti-alias bitplanes 合并为单张灰度图像。每个 bitplane 是同尺寸的
/// 1-bit 黑白像素 buffer；最终像素值按 bitplane 命中数量加权并夹到 255。`aa_level = 1`
/// 时直接返回第一张 plane。
pub fn merge_bitplanes(
    planes: &[Vec<u8>],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, ConvertError> {
    let borrowed = planes.iter().map(Vec::as_slice).collect::<Vec<_>>();
    merge_bitplane_slices(&borrowed, width, height)
}

/// Merge borrowed CBDDLP anti-alias bitplane slices into a grayscale image.
///
/// This avoids allocating a `Vec<Vec<u8>>` wrapper when callers already own
/// borrowed bitplane slices, such as the WASM merge helper.
///
/// 将 borrowed CBDDLP anti-alias bitplane slices 合并为灰度图像。调用方已经持有 borrowed
/// bitplane slices 时可避免额外构造 `Vec<Vec<u8>>`，例如 WASM merge helper。
pub fn merge_bitplane_slices(
    planes: &[&[u8]],
    _width: u32,
    _height: u32,
) -> Result<Vec<u8>, ConvertError> {
    if planes.is_empty() {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "no bitplanes to merge".into(),
        });
    }

    if planes.len() == 1 {
        return Ok(planes[0].to_vec());
    }

    let pixel_count = planes[0].len();
    let aa_level = planes.len() as u32;
    let weight = (256u32 / aa_level) as u8;
    let mut output = vec![0u8; pixel_count];

    #[cfg(target_arch = "wasm32")]
    {
        use std::arch::wasm32::*;
        let weight_v = u8x16_splat(weight);
        let zero = u8x16_splat(0);
        let simd_len = pixel_count & !15; // round down to 16

        for i in (0..simd_len).step_by(16) {
            let mut acc = zero;
            for plane in planes.iter() {
                let px = unsafe { v128_load(plane[i..].as_ptr() as *const v128) };
                let mask = u8x16_ne(px, zero);
                let weighted = v128_and(mask, weight_v);
                acc = u8x16_add_sat(acc, weighted);
            }
            unsafe { v128_store(output[i..].as_mut_ptr() as *mut v128, acc) };
        }

        // Handle remaining pixels
        for i in simd_len..pixel_count {
            let mut sum: u16 = 0;
            for plane in planes.iter() {
                if i < plane.len() && plane[i] > 0 {
                    sum += weight as u16;
                }
            }
            output[i] = sum.min(255) as u8;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        for (i, out) in output.iter_mut().enumerate() {
            let mut sum: u16 = 0;
            for plane in planes {
                if i < plane.len() && plane[i] > 0 {
                    sum += weight as u16;
                }
            }
            *out = sum.min(255) as u8;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_bitplanes_single() {
        let plane = vec![0, 255, 128, 0];
        let result = merge_bitplanes(std::slice::from_ref(&plane), 2, 2).unwrap();
        assert_eq!(result, plane);
    }

    #[test]
    fn test_merge_bitplanes_two() {
        // aa_level=2, weight=128 per plane
        let plane0 = vec![0, 255, 255, 0];
        let plane1 = vec![0, 0, 255, 255];
        let result = merge_bitplanes(&[plane0, plane1], 2, 2).unwrap();
        // pixel 0: 0, pixel 1: 128, pixel 2: 255 (clamped), pixel 3: 128
        assert_eq!(result, vec![0, 128, 255, 128]);
    }

    #[test]
    fn test_merge_bitplanes_four() {
        // aa_level=4, weight=64
        let p0 = vec![255, 255, 0, 0];
        let p1 = vec![255, 0, 255, 0];
        let p2 = vec![255, 0, 0, 0];
        let p3 = vec![255, 255, 255, 0];
        let result = merge_bitplanes(&[p0, p1, p2, p3], 2, 2).unwrap();
        // pixel 0: 4*64=256→255, pixel 1: 2*64=128, pixel 2: 2*64=128, pixel 3: 0
        assert_eq!(result, vec![255, 128, 128, 0]);
    }

    #[test]
    fn test_merge_bitplanes_empty() {
        let result = merge_bitplanes(&[], 2, 2);
        assert!(result.is_err());
    }
}
