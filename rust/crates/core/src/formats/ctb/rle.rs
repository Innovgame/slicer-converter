//! CTB and CBDDLP layer RLE codecs.
//!
//! CTB stores grayscale layers as a 7-bit grayscale value plus variable-length
//! run length. Older CBDDLP stores binary layers as one byte per black/white run
//! and relies on separate bitplanes for anti-aliasing.
//!
//! CTB 使用 7-bit grayscale value 加 variable-length run length 存储灰度层。
//! 较旧的 CBDDLP 使用单字节 black/white run，并通过多个 bitplanes 表达 anti-aliasing。

use crate::common::{simd::RunScanner, ConvertError, PreviewDecodePlan};

/// Decode CTB RLE directly into downsampled RGBA preview using nearest-neighbor sampling.
///
/// This path is only valid when downsampling is required; callers that request a
/// preview at or above source size should use the normal grayscale decode path.
/// The decoder walks source runs and only writes sampled target pixels, avoiding
/// a full `width * height` grayscale allocation.
///
/// 仅在需要 downsample 时使用。decoder 会扫描 source runs，只写入目标 preview 需要的
/// sampled pixels，避免分配完整的 `width * height` 灰度层。
pub fn decode_ctb_rle_preview_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    max_preview_size: u32,
) -> Result<(Vec<u8>, u32, u32), ConvertError> {
    if max_preview_size == 0 {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "max_preview_size must be > 0".to_string(),
        });
    }

    if width <= max_preview_size && height <= max_preview_size {
        // No downsample needed; caller should use normal decode path.
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "direct preview path requires downsample".to_string(),
        });
    }

    let plan = PreviewDecodePlan::for_downsample(width, height, max_preview_size).ok_or(
        ConvertError::RleDecodeError {
            offset: 0,
            detail: "direct preview path requires downsample".to_string(),
        },
    )?;
    let mut rgba = plan.new_rgba_buffer();

    let src_pixel_count = (width as usize) * (height as usize);
    let mut stream_i = 0usize;
    let mut src_pos = 0usize;
    let mut target_i = 0usize;
    let target_indices = plan.target_indices();

    while stream_i < data.len() && src_pos < src_pixel_count && target_i < target_indices.len() {
        let byte0 = data[stream_i];
        stream_i += 1;

        let is_run = (byte0 & 0x80) != 0;
        let gray7 = byte0 & 0x7F;
        let gray8 = (gray7 << 1) | (gray7 >> 6);

        let stride = if is_run {
            if stream_i >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: stream_i,
                    detail: "unexpected end of data reading stride".into(),
                });
            }
            let b1 = data[stream_i];
            stream_i += 1;

            if b1 < 0x80 {
                b1 as usize
            } else if b1 < 0xC0 {
                if stream_i >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: stream_i,
                        detail: "unexpected end of data reading 2-byte stride".into(),
                    });
                }
                let b2 = data[stream_i];
                stream_i += 1;
                (((b1 & 0x3F) as usize) << 8) | (b2 as usize)
            } else if b1 < 0xE0 {
                if stream_i + 1 >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: stream_i,
                        detail: "unexpected end of data reading 3-byte stride".into(),
                    });
                }
                let b2 = data[stream_i];
                let b3 = data[stream_i + 1];
                stream_i += 2;
                (((b1 & 0x1F) as usize) << 16) | ((b2 as usize) << 8) | (b3 as usize)
            } else {
                if stream_i + 2 >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: stream_i,
                        detail: "unexpected end of data reading 4-byte stride".into(),
                    });
                }
                let b2 = data[stream_i];
                let b3 = data[stream_i + 1];
                let b4 = data[stream_i + 2];
                stream_i += 3;
                (((b1 & 0x0F) as usize) << 24)
                    | ((b2 as usize) << 16)
                    | ((b3 as usize) << 8)
                    | (b4 as usize)
            }
        } else {
            1
        };

        let run_end = (src_pos + stride).min(src_pixel_count);
        while target_i < target_indices.len() {
            let wanted = target_indices[target_i];
            if wanted >= run_end {
                break;
            }
            if wanted >= src_pos {
                let out = target_i * 4;
                rgba[out] = gray8;
                rgba[out + 1] = gray8;
                rgba[out + 2] = gray8;
            }
            target_i += 1;
        }
        src_pos = run_end;
    }

    Ok((rgba, plan.dst_width, plan.dst_height))
}

/// Decode CTB variable-length RLE into `width * height` grayscale pixels.
///
/// Byte format:
///   bit 7     = run marker (1 = run with stride, 0 = single pixel)
///   bit [6:1] = 7-bit grayscale (original 8-bit >> 1)
///
/// If run marker is set, subsequent bytes encode stride:
///   0x00..0x7F:      1-byte stride (7 bits, max 127)
///   0x80..0xBF:      2-byte stride (14 bits, max 16,383)
///   0xC0..0xDF:      3-byte stride (20 bits, max 1,048,575)
///   0xE0..0xFF:      4-byte stride (28 bits, max 268,435,455)
///
/// Short streams are padded with black pixels so callers always receive a full
/// image-sized buffer.
///
/// 将 CTB 可变长度 RLE 解码为完整灰度像素数组。短流会用黑色像素补齐，保证输出长度为
/// `width * height`。
pub fn decode_ctb_rle(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let mut output = Vec::new();
    decode_ctb_rle_into(data, width, height, &mut output)?;
    Ok(output)
}

/// Decode CTB variable-length RLE, writing pixels into a pre-allocated buffer.
///
/// Reuses the buffer's existing capacity to avoid repeated heap allocations.
///
/// 将 CTB RLE 数据解码到预分配缓冲区，保留 capacity 以避免重复堆分配。
pub fn decode_ctb_rle_into(
    data: &[u8],
    width: u32,
    height: u32,
    pixels: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    pixels.clear();
    if pixels.capacity() < pixel_count {
        pixels.reserve(pixel_count - pixels.capacity());
    }
    let mut i = 0;

    while i < data.len() && pixels.len() < pixel_count {
        let byte0 = data[i];
        i += 1;

        let is_run = (byte0 & 0x80) != 0;
        let gray7 = byte0 & 0x7F;
        // Expand 7-bit (0-127) to 8-bit (0-255)
        let gray8 = (gray7 << 1) | (gray7 >> 6);

        let stride = if is_run {
            if i >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: i,
                    detail: "unexpected end of data reading stride".into(),
                });
            }
            let b1 = data[i];
            i += 1;

            if b1 < 0x80 {
                // 1-byte stride: 7 bits
                b1 as usize
            } else if b1 < 0xC0 {
                // 2-byte stride: 14 bits
                if i >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: i,
                        detail: "unexpected end of data reading 2-byte stride".into(),
                    });
                }
                let b2 = data[i];
                i += 1;
                (((b1 & 0x3F) as usize) << 8) | (b2 as usize)
            } else if b1 < 0xE0 {
                // 3-byte stride: 20 bits
                if i + 1 >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: i,
                        detail: "unexpected end of data reading 3-byte stride".into(),
                    });
                }
                let b2 = data[i];
                let b3 = data[i + 1];
                i += 2;
                (((b1 & 0x1F) as usize) << 16) | ((b2 as usize) << 8) | (b3 as usize)
            } else {
                // 4-byte stride: 28 bits
                if i + 2 >= data.len() {
                    return Err(ConvertError::RleDecodeError {
                        offset: i,
                        detail: "unexpected end of data reading 4-byte stride".into(),
                    });
                }
                let b2 = data[i];
                let b3 = data[i + 1];
                let b4 = data[i + 2];
                i += 3;
                (((b1 & 0x0F) as usize) << 24)
                    | ((b2 as usize) << 16)
                    | ((b3 as usize) << 8)
                    | (b4 as usize)
            }
        } else {
            1
        };

        let count = stride.min(pixel_count - pixels.len());
        // resize() uses memset internally for u8 — faster than iterator extend for large runs
        let new_len = pixels.len() + count;
        pixels.resize(new_len, gray8);
    }

    // Pad if we ran out of RLE data before filling the image
    pixels.resize(pixel_count, 0);

    Ok(())
}

/// Encode grayscale pixels into CTB variable-length RLE.
///
/// The encoder stores 8-bit grayscale values as gray7 (`value >> 1`) and groups
/// adjacent pixels that share the same gray7 value.
///
/// 将灰度像素数组编码为 CTB 可变长度 RLE 格式。encoder 会把 8-bit 灰度值压成 gray7
/// (`value >> 1`)，并合并相邻 gray7 相同的像素。
pub fn encode_ctb_rle(pixels: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut output = Vec::with_capacity(pixels.len() / 2);
    encode_ctb_rle_into(pixels, &mut output)?;
    Ok(output)
}

/// Encode CTB variable-length RLE into a caller-provided buffer.
///
/// 将灰度像素编码到调用方提供的 `output` 中（先 `clear()`，保留容量）。
/// 用于 thread_local `ENCODE_BUF` 摊销分配开销。
pub fn encode_ctb_rle_into(pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
    output.clear();
    if pixels.is_empty() {
        return Ok(());
    }
    let hint = pixels.len() / 2;
    if output.capacity() < hint {
        output.reserve(hint - output.capacity());
    }

    let len = pixels.len();
    let ptr = pixels.as_ptr();
    let scanner = RunScanner::detect();

    let mut i = 0;
    while i < len {
        // SAFETY: i < len, so ptr.add(i) is valid
        let cur = unsafe { *ptr.add(i) };
        let cur_gray7 = cur >> 1;
        let run_start = i;
        i = unsafe { scanner.scan_gray7_run_end(ptr, len, run_start, cur) };

        let stride = i - run_start;
        emit_run_inline(output, cur_gray7, stride);
    }

    Ok(())
}

/// Emit a single color + stride run to the output buffer (inlined for performance).
#[inline(always)]
fn emit_run_inline(output: &mut Vec<u8>, gray7: u8, stride: usize) {
    if stride == 1 {
        // Single pixel: gray7 in bits [6:0], bit 7 clear
        output.push(gray7);
    } else {
        // Run: gray7 in bits [6:0], bit 7 set as run marker
        output.push(gray7 | 0x80);
        encode_stride_inline(output, stride);
    }
}

/// Encode stride as variable-length bytes (inlined for performance).
#[inline(always)]
fn encode_stride_inline(output: &mut Vec<u8>, stride: usize) {
    if stride <= 0x7F {
        output.push(stride as u8);
    } else if stride <= 0x3FFF {
        output.push(((stride >> 8) as u8) | 0x80);
        output.push(stride as u8);
    } else if stride <= 0x1F_FFFF {
        output.push(((stride >> 16) as u8) | 0xC0);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    } else if stride <= 0x0FFF_FFFF {
        output.push(((stride >> 24) as u8) | 0xE0);
        output.push((stride >> 16) as u8);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    } else {
        // Stride too large — split into multiple runs (should be extremely rare)
        output.push(0xEF); // max 4-byte prefix
        output.push(0xFF);
        output.push(0xFF);
        output.push(0xFF);
        let remaining = stride - 0x0FFF_FFFF;
        if remaining > 0 {
            encode_stride_inline(output, remaining);
        }
    }
}

/// Decode CBDDLP 7-bit simple RLE (for older format files).
///
/// Each byte:
///   bit 7 = color (0 = black, 1 = white)
///   bit [6:0] = run length (1-125)
///
///
/// `anti_alias_level` is accepted for facade symmetry; CBDDLP anti-aliasing is
/// represented by separate bitplanes that are merged by the caller.
///
/// 解码 CBDDLP 7-bit simple RLE（旧版本 CBDDLP 格式）。`anti_alias_level` 仅用于保持
/// facade 参数对齐；CBDDLP anti-aliasing 由调用方合并多个 bitplanes 完成。
pub fn decode_cbddlp_rle(
    data: &[u8],
    width: u32,
    height: u32,
    _anti_alias_level: u32,
) -> Result<Vec<u8>, ConvertError> {
    let mut output = Vec::new();
    decode_cbddlp_rle_into(data, width, height, _anti_alias_level, &mut output)?;
    Ok(output)
}

/// Decode CBDDLP 7-bit simple RLE into a caller-provided buffer.
///
/// As with CTB grayscale decode, malformed short streams are padded with black
/// pixels to keep the output image-sized.
///
/// 将 CBDDLP RLE 数据解码到调用方提供的 buffer。异常短流会用黑色像素补齐到完整图像尺寸。
pub fn decode_cbddlp_rle_into(
    data: &[u8],
    width: u32,
    height: u32,
    _anti_alias_level: u32,
    pixels: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    pixels.clear();
    if pixels.capacity() < pixel_count {
        pixels.reserve(pixel_count - pixels.capacity());
    }

    for &byte in data {
        if pixels.len() >= pixel_count {
            break;
        }
        let is_white = (byte & 0x80) != 0;
        let count = (byte & 0x7F) as usize;
        let value = if is_white { 255u8 } else { 0u8 };
        let actual_count = count.min(pixel_count - pixels.len());
        let new_len = pixels.len() + actual_count;
        pixels.resize(new_len, value);
    }

    pixels.resize(pixel_count, 0);
    Ok(())
}

/// Encode binary pixels as CBDDLP 7-bit simple RLE.
///
/// Each output byte stores a single black/white run:
///   bit 7 = color (0 = black, 1 = white)
///   bit [6:0] = run length (1-127)
///
/// 将二值像素编码为 CBDDLP 7-bit simple RLE。输入灰度值 >= 128 视为 white。
pub fn encode_cbddlp_rle_into(pixels: &[u8], output: &mut Vec<u8>) {
    output.clear();
    if pixels.is_empty() {
        return;
    }

    let mut index = 0usize;
    while index < pixels.len() {
        let is_white = pixels[index] >= 128;
        let mut run_len = 1usize;
        while index + run_len < pixels.len()
            && run_len < 0x7F
            && (pixels[index + run_len] >= 128) == is_white
        {
            run_len += 1;
        }

        let color = if is_white { 0x80 } else { 0x00 };
        output.push(color | (run_len as u8));
        index += run_len;
    }
}

/// Allocating variant of [`encode_cbddlp_rle_into`].
///
/// [`encode_cbddlp_rle_into`] 的分配式 convenience wrapper。
pub fn encode_cbddlp_rle(pixels: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    encode_cbddlp_rle_into(pixels, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use crate::common::simd::RunScanner;

    #[test]
    fn scan_gray7_run_end_groups_adjacent_values_with_same_gray7() {
        let pixels = [4u8, 5, 4, 5, 6, 7, 8];
        let scanner = RunScanner::detect();

        assert_eq!(
            unsafe { scanner.scan_gray7_run_end(pixels.as_ptr(), pixels.len(), 0, pixels[0]) },
            4
        );
        assert_eq!(
            unsafe { scanner.scan_gray7_run_end(pixels.as_ptr(), pixels.len(), 4, pixels[4]) },
            6
        );
        assert_eq!(
            unsafe { scanner.scan_gray7_run_end(pixels.as_ptr(), pixels.len(), 6, pixels[6]) },
            7
        );
    }

    #[test]
    fn scan_gray7_run_end_handles_misaligned_long_runs() {
        let mut pixels = vec![11u8, 13, 15];
        pixels.extend((0..65).map(|i| if i % 2 == 0 { 200u8 } else { 201u8 }));
        pixels.push(202);
        let scanner = RunScanner::detect();

        assert_eq!(
            unsafe { scanner.scan_gray7_run_end(pixels.as_ptr(), pixels.len(), 3, pixels[3]) },
            68
        );
        assert_eq!(
            unsafe { scanner.scan_gray7_run_end(pixels.as_ptr(), pixels.len(), 68, pixels[68]) },
            69
        );
    }
}
