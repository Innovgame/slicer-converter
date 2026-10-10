//! PW0 RLE4 layer codec.
//!
//! PW0 stores one 4-bit grayscale code per run. Codes `0x0` and `0xF` use an
//! extended 12-bit run length, while intermediate grayscale codes store a 4-bit
//! run length in the low nibble.
//!
//! PW0 使用每个 run 一个 4-bit grayscale code。`0x0` 与 `0xF` 使用扩展 12-bit run
//! length；中间灰度 code 使用低 4-bit 存储 run length。

use crate::common::{ConvertError, PreviewDecodePlan};

/// Decode PW0 RLE4 directly into full-size RGBA pixels.
///
/// This skips the intermediate grayscale buffer for callers that need
/// browser-ready pixels at source resolution.
///
/// 直接将 PW0 RLE4 解码为 full-size RGBA pixels，适用于需要源分辨率浏览器像素且不想先构造
/// grayscale buffer 的路径。
pub fn decode_pw0_rle4_rgba(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut rgba = Vec::with_capacity(data.len() * 16);
    decode_pw0_rle4_rgba_into(data, &mut rgba)?;
    Ok(rgba)
}

/// Decode PW0 RLE4 into a caller-provided full-size RGBA buffer.
///
/// The buffer is cleared first and its capacity is reused.
///
/// 将 PW0 RLE4 解码到调用方提供的 full-size RGBA buffer。buffer 会先 clear 并复用 capacity。
pub fn decode_pw0_rle4_rgba_into(data: &[u8], rgba: &mut Vec<u8>) -> Result<(), ConvertError> {
    rgba.clear();
    let mut i = 0;

    while i < data.len() {
        let b = data[i];
        i += 1;
        let code = b >> 4;
        let mut repeat = (b & 0x0F) as usize;
        let color = match code {
            0x0 => {
                if i < data.len() {
                    repeat = (repeat << 8) | (data[i] as usize);
                    i += 1;
                }
                0
            }
            0xF => {
                if i < data.len() {
                    repeat = (repeat << 8) | (data[i] as usize);
                    i += 1;
                }
                255
            }
            _ => (code << 4) | code,
        };

        append_rgba_run(rgba, color, repeat, i)?;
    }

    Ok(())
}

/// Decode PW0 RLE4 directly into a downsampled RGBA preview.
///
/// The decoder walks RLE runs and only materializes sampled preview pixels,
/// avoiding a full grayscale layer allocation for large source images.
///
/// 直接将 PW0 RLE4 解码为 downsampled RGBA preview。decoder 扫描 RLE runs，只 materialize
/// 目标 preview 采样像素，避免为大图分配完整 grayscale layer。
pub fn decode_pw0_rle4_preview_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    max_preview_size: u32,
) -> Result<(Vec<u8>, u32, u32), ConvertError> {
    if max_preview_size == 0 {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "max_preview_size must be > 0".into(),
        });
    }

    let plan = PreviewDecodePlan::for_downsample(width, height, max_preview_size).ok_or(
        ConvertError::RleDecodeError {
            offset: 0,
            detail: "direct preview path requires downsample".into(),
        },
    )?;
    let src_pixel_count = (width as usize) * (height as usize);
    let mut rgba = plan.new_rgba_buffer();
    let target_indices = plan.target_indices();
    let mut target_i = 0usize;
    let mut src_pos = 0usize;
    let mut i = 0usize;

    while i < data.len() && src_pos < src_pixel_count && target_i < target_indices.len() {
        let b = data[i];
        i += 1;
        let code = b >> 4;
        let mut repeat = (b & 0x0F) as usize;
        let color = match code {
            0x0 => {
                if i < data.len() {
                    repeat = (repeat << 8) | data[i] as usize;
                    i += 1;
                }
                0
            }
            0xF => {
                if i < data.len() {
                    repeat = (repeat << 8) | data[i] as usize;
                    i += 1;
                }
                255
            }
            _ => (code << 4) | code,
        };

        let run_start = src_pos;
        let run_end = (run_start + repeat).min(src_pixel_count);
        while target_i < target_indices.len() {
            let wanted = target_indices[target_i];
            if wanted >= run_end {
                break;
            }
            if wanted >= run_start {
                let out = target_i * 4;
                rgba[out] = color;
                rgba[out + 1] = color;
                rgba[out + 2] = color;
            }
            target_i += 1;
        }
        src_pos = run_end;
    }

    Ok((rgba, plan.dst_width, plan.dst_height))
}

/// Decode PW0 RLE4 encoded layer data into grayscale pixels.
///
/// Decoded grayscale values are the expanded nibble values (`0x7` becomes
/// `0x77`), matching the quantization performed by the encoder.
///
/// 将 PW0 RLE4 encoded layer data 解码为 grayscale pixels。解码后的灰度值为扩展后的
/// nibble 值（例如 `0x7` 变为 `0x77`），与 encoder 的量化行为一致。
pub fn decode_pw0_rle4(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut pixels = Vec::with_capacity(data.len() * 4);
    decode_pw0_rle4_into(data, &mut pixels)?;
    Ok(pixels)
}

/// Decode PW0 RLE4 into a pre-allocated grayscale buffer.
///
/// 将 PW0 RLE4 解码到预分配 grayscale buffer，并复用已有 capacity。
pub fn decode_pw0_rle4_into(data: &[u8], pixels: &mut Vec<u8>) -> Result<(), ConvertError> {
    pixels.clear();
    let mut i = 0;

    while i < data.len() {
        let b = data[i];
        i += 1;
        let code = b >> 4;
        let mut repeat = (b & 0x0F) as usize;
        let color: u8;

        match code {
            0x0 => {
                color = 0;
                if i < data.len() {
                    repeat = (repeat << 8) | (data[i] as usize);
                    i += 1;
                }
            }
            0xF => {
                color = 255;
                if i < data.len() {
                    repeat = (repeat << 8) | (data[i] as usize);
                    i += 1;
                }
            }
            _ => {
                color = (code << 4) | code;
            }
        }

        let new_len = pixels.len() + repeat;
        pixels.resize(new_len, color);
    }

    Ok(())
}

fn append_rgba_run(
    rgba: &mut Vec<u8>,
    color: u8,
    repeat: usize,
    offset: usize,
) -> Result<(), ConvertError> {
    let start = rgba.len();
    let end = start
        .checked_add(repeat.checked_mul(4).ok_or(ConvertError::RleDecodeError {
            offset,
            detail: "PW0 RGBA run length overflow".to_string(),
        })?)
        .ok_or(ConvertError::RleDecodeError {
            offset,
            detail: "PW0 RGBA output length overflow".to_string(),
        })?;
    rgba.resize(end, 0);
    for pixel in rgba[start..end].chunks_exact_mut(4) {
        pixel[0] = color;
        pixel[1] = color;
        pixel[2] = color;
        pixel[3] = 255;
    }
    Ok(())
}

/// Encode grayscale pixels into PW0 RLE4 format.
///
/// Input pixels are quantized to their high nibble before run grouping.
///
/// 将 grayscale pixels 编码为 PW0 RLE4 format。输入像素会先量化到 high nibble，再按 run 分组。
pub fn encode_pw0_rle4(pixels: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(pixels.len() / 4);
    encode_pw0_rle4_into(pixels, &mut output);
    output
}

/// Encode PW0 RLE4 into a caller-provided buffer.
///
/// The output buffer is cleared first and its capacity is reused across layers.
///
/// 将 PW0 RLE4 编码到调用方提供的 buffer。output 会先 clear，并在多层之间复用 capacity。
pub fn encode_pw0_rle4_into(pixels: &[u8], output: &mut Vec<u8>) {
    output.clear();
    let len = pixels.len();
    if len == 0 {
        return;
    }
    let hint = pixels.len() / 4;
    if output.capacity() < hint {
        output.reserve(hint - output.capacity());
    }

    let mut i = 0;
    while i < len {
        let color = pixels[i] >> 4;
        let run_start = i;
        i += 1;

        while i < len && (pixels[i] >> 4) == color {
            i += 1;
        }
        let mut remaining = i - run_start;

        match color {
            0x0 | 0xF => {
                while remaining > 0 {
                    let run = remaining.min(4095);
                    let val = ((color as u16) << 12) | (run as u16);
                    output.push((val >> 8) as u8);
                    output.push(val as u8);
                    remaining -= run;
                }
            }
            _ => {
                while remaining > 0 {
                    let run = remaining.min(15);
                    output.push((color << 4) | (run as u8));
                    remaining -= run;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::downsample_to_rgba_nearest;

    #[test]
    fn roundtrip_all_black() {
        let pixels = vec![0u8; 1000];
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_all_white() {
        let pixels = vec![0xFFu8; 1000];
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_gray_pattern() {
        let mut pixels = Vec::new();
        for g in 0..16u8 {
            let expanded = (g << 4) | g;
            for _ in 0..100 {
                pixels.push(expanded);
            }
        }
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_large_run() {
        let pixels = vec![0u8; 100_000];
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_alternating() {
        let pixels: Vec<u8> = (0..1000)
            .map(|i| if i % 2 == 0 { 0x00 } else { 0xFF })
            .collect();
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn quantization_roundtrip() {
        let pixels: Vec<u8> = (0..=255).collect();
        let encoded = encode_pw0_rle4(&pixels);
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        for (orig, dec) in pixels.iter().zip(decoded.iter()) {
            let code = orig >> 4;
            let expected = (code << 4) | code;
            assert_eq!(*dec, expected);
        }
    }

    #[test]
    fn preview_fast_path_matches_generic_downsample() {
        let width = 16;
        let height = 12;
        let max_preview_size = 6;
        let pixels: Vec<u8> = (0..(width * height) as usize)
            .map(|index| {
                let value = (index % 16) as u8;
                (value << 4) | value
            })
            .collect();

        let encoded = encode_pw0_rle4(&pixels);
        let (rgba_fast, w_fast, h_fast) =
            decode_pw0_rle4_preview_rgba(&encoded, width, height, max_preview_size).unwrap();
        let decoded = decode_pw0_rle4(&encoded).unwrap();
        let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
        assert_eq!(w_fast, expected.width);
        assert_eq!(h_fast, expected.height);
        assert_eq!(rgba_fast, expected.pixels);
    }
}
