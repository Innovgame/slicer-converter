//! PWS RLE1 layer codec.
//!
//! PWS stores binary black/white runs in one byte: bit 7 is the color and the
//! low 7 bits are the run length. Export binarizes grayscale pixels at `>= 128`.
//!
//! PWS 使用单字节保存 binary black/white runs：bit 7 是颜色，低 7 bit 是 run length。
//! 导出时 grayscale pixels 以 `>= 128` 作为白色阈值。

use crate::common::{ConvertError, PreviewDecodePlan};

/// Decode PWS RLE1 directly into full-size RGBA pixels.
///
/// 直接将 PWS RLE1 解码为 full-size RGBA pixels，跳过中间 grayscale buffer。
pub fn decode_pws_rle1_rgba(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut rgba = Vec::with_capacity(data.len() * 128);
    decode_pws_rle1_rgba_into(data, &mut rgba)?;
    Ok(rgba)
}

/// Decode PWS RLE1 into a caller-provided full-size RGBA buffer.
///
/// The buffer is cleared first and capacity is reused.
///
/// 将 PWS RLE1 解码到调用方提供的 full-size RGBA buffer。buffer 会先 clear 并复用 capacity。
pub fn decode_pws_rle1_rgba_into(data: &[u8], rgba: &mut Vec<u8>) -> Result<(), ConvertError> {
    rgba.clear();

    for (offset, &b) in data.iter().enumerate() {
        let color: u8 = if b & 0x80 != 0 { 0xFF } else { 0x00 };
        let run = (b & 0x7F) as usize;
        if run == 0 {
            continue;
        }
        append_rgba_run(rgba, color, run, offset)?;
    }

    Ok(())
}

/// Decode PWS RLE1 directly into a downsampled RGBA preview.
///
/// This walks runs and writes only sampled target pixels, which is cheaper than
/// decoding the full binary layer for large previews.
///
/// 直接将 PWS RLE1 解码为 downsampled RGBA preview。该路径扫描 runs 且只写目标采样像素，
/// 对大图 preview 比完整解码 binary layer 更省内存。
pub fn decode_pws_rle1_preview_rgba(
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

    for &b in data {
        if src_pos >= src_pixel_count || target_i >= target_indices.len() {
            break;
        }

        let color: u8 = if b & 0x80 != 0 { 0xFF } else { 0x00 };
        let run = (b & 0x7F) as usize;
        if run == 0 {
            continue;
        }

        let run_start = src_pos;
        let run_end = (run_start + run).min(src_pixel_count);
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

/// Decode PWS RLE1 encoded layer data into grayscale pixels.
///
/// Output pixels are binary `0x00` or `0xFF`.
///
/// 将 PWS RLE1 encoded layer data 解码为 grayscale pixels。输出像素为 binary `0x00` 或
/// `0xFF`。
pub fn decode_pws_rle1(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut pixels = Vec::with_capacity(data.len() * 32);
    decode_pws_rle1_into(data, &mut pixels)?;
    Ok(pixels)
}

/// Decode PWS RLE1 into a pre-allocated grayscale buffer.
///
/// 将 PWS RLE1 解码到预分配 grayscale buffer，并复用已有 capacity。
pub fn decode_pws_rle1_into(data: &[u8], pixels: &mut Vec<u8>) -> Result<(), ConvertError> {
    pixels.clear();

    for &b in data {
        let color: u8 = if b & 0x80 != 0 { 0xFF } else { 0x00 };
        let run = (b & 0x7F) as usize;
        if run == 0 {
            continue;
        }
        let new_len = pixels.len() + run;
        pixels.resize(new_len, color);
    }

    Ok(())
}

fn append_rgba_run(
    rgba: &mut Vec<u8>,
    color: u8,
    run: usize,
    offset: usize,
) -> Result<(), ConvertError> {
    let start = rgba.len();
    let end = start
        .checked_add(run.checked_mul(4).ok_or(ConvertError::RleDecodeError {
            offset,
            detail: "PWS RGBA run length overflow".to_string(),
        })?)
        .ok_or(ConvertError::RleDecodeError {
            offset,
            detail: "PWS RGBA output length overflow".to_string(),
        })?;
    rgba.resize(end, 0);
    for pixel in rgba[start..end].as_chunks_mut::<4>().0 {
        pixel[0] = color;
        pixel[1] = color;
        pixel[2] = color;
        pixel[3] = 255;
    }
    Ok(())
}

/// Encode grayscale pixels into PWS RLE1 format.
///
/// Pixels `>= 128` are encoded as white; all lower values are encoded as black.
///
/// 将 grayscale pixels 编码为 PWS RLE1 format。像素 `>= 128` 编码为 white，其余编码为 black。
pub fn encode_pws_rle1(pixels: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(pixels.len() / 32);
    encode_pws_rle1_into(pixels, &mut output);
    output
}

/// Encode PWS RLE1 into a caller-provided buffer.
///
/// Runs are capped at 125 bytes for compatibility with Anycubic readers.
///
/// 将 PWS RLE1 编码到调用方提供的 buffer。run length 限制为 125，以兼容 Anycubic readers。
pub fn encode_pws_rle1_into(pixels: &[u8], output: &mut Vec<u8>) {
    output.clear();
    let len = pixels.len();
    if len == 0 {
        return;
    }
    let hint = pixels.len() / 32;
    if output.capacity() < hint {
        output.reserve(hint - output.capacity());
    }

    let mut i = 0;
    while i < len {
        let color_bit: u8 = if pixels[i] >= 128 { 0x80 } else { 0x00 };
        let is_white = color_bit != 0;
        let run_start = i;
        i += 1;

        while i < len && (pixels[i] >= 128) == is_white {
            i += 1;
        }
        let mut remaining = i - run_start;

        while remaining > 0 {
            let run = remaining.min(125);
            output.push(color_bit | run as u8);
            remaining -= run;
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
        let encoded = encode_pws_rle1(&pixels);
        let decoded = decode_pws_rle1(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_all_white() {
        let pixels = vec![0xFFu8; 1000];
        let encoded = encode_pws_rle1(&pixels);
        let decoded = decode_pws_rle1(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_alternating() {
        let pixels: Vec<u8> = (0..1000)
            .map(|i| if i % 2 == 0 { 0x00 } else { 0xFF })
            .collect();
        let encoded = encode_pws_rle1(&pixels);
        let decoded = decode_pws_rle1(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn roundtrip_large_run() {
        let pixels = vec![0u8; 100_000];
        let encoded = encode_pws_rle1(&pixels);
        let decoded = decode_pws_rle1(&encoded).unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn max_run_capped_at_125() {
        let pixels = vec![0xFFu8; 300];
        let encoded = encode_pws_rle1(&pixels);
        assert_eq!(encoded.len(), 3);
        assert_eq!(encoded[0], 0x80 | 125);
        assert_eq!(encoded[1], 0x80 | 125);
        assert_eq!(encoded[2], 0x80 | 50);
    }

    #[test]
    fn binarization_threshold() {
        let pixels = vec![0, 50, 127, 128, 200, 255];
        let encoded = encode_pws_rle1(&pixels);
        let decoded = decode_pws_rle1(&encoded).unwrap();
        assert_eq!(decoded, vec![0, 0, 0, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn preview_fast_path_matches_generic_downsample() {
        let width = 16;
        let height = 12;
        let max_preview_size = 6;
        let pixels: Vec<u8> = (0..(width * height) as usize)
            .map(|index| if index % 3 == 0 { 0xFF } else { 0x00 })
            .collect();

        let encoded = encode_pws_rle1(&pixels);
        let (rgba_fast, w_fast, h_fast) =
            decode_pws_rle1_preview_rgba(&encoded, width, height, max_preview_size).unwrap();
        let decoded = decode_pws_rle1(&encoded).unwrap();
        let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
        assert_eq!(w_fast, expected.width);
        assert_eq!(h_fast, expected.height);
        assert_eq!(rgba_fast, expected.pixels);
    }
}
