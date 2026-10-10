//! GOO layer RLE codec.
//!
//! GOO layer streams begin with `LAYER_MAGIC` (`0x55`) and end with a checksum.
//! The body supports black/white runs, arbitrary gray runs, and optional
//! color-difference chunks. PRZ reuses this decoder but disables diff chunks
//! when encoding.
//!
//! GOO layer stream 以 `LAYER_MAGIC` (`0x55`) 开头，并以 checksum 结尾。body 支持
//! black/white runs、任意 gray runs，以及可选 color-difference chunks。PRZ 复用该
//! decoder，但 encode 时会禁用 diff chunks。

use crate::common::{simd::RunScanner, ConvertError, PreviewDecodePlan};

/// Decode GOO RLE directly into downsampled RGBA preview using nearest-neighbor sampling.
///
/// The direct path requires actual downsampling; callers should use the normal
/// decode path when the source already fits the requested preview size.
///
/// 直接将 GOO RLE 解码为 downsampled RGBA preview。该路径要求确实发生 downsample；
/// 源图已适配目标 preview size 时，调用方应使用普通 decode 路径。
pub fn decode_goo_rle_preview_rgba(
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

    if data.is_empty() {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "empty GOO RLE data".into(),
        });
    }

    if data[0] != super::LAYER_MAGIC {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: format!("expected magic 0x55, got {:#04x}", data[0]),
        });
    }

    let mut i = 1usize;
    let end = data.len().saturating_sub(1);
    let mut src_pos = 0usize;
    let mut target_i = 0usize;
    let mut prev_pixel = 0u8;

    while i < end && src_pos < src_pixel_count && target_i < target_indices.len() {
        let byte0 = data[i];
        i += 1;

        match (byte0 >> 6) & 0x03 {
            0b00 | 0b11 => {
                let value = if (byte0 >> 6) & 0x03 == 0b00 {
                    0x00
                } else {
                    0xFF
                };
                let stride = read_stride(data, &mut i, byte0)?;
                let actual_count = target_constant_run(
                    &mut rgba,
                    target_indices,
                    &mut target_i,
                    &mut src_pos,
                    stride,
                    src_pixel_count,
                    value,
                );
                if actual_count > 0 {
                    prev_pixel = value;
                }
            }
            0b01 => {
                if i >= end {
                    return Err(ConvertError::RleDecodeError {
                        offset: i,
                        detail: "expected gray value byte".into(),
                    });
                }
                let gray_value = data[i];
                i += 1;
                let stride = read_stride(data, &mut i, byte0)?;
                let actual_count = target_constant_run(
                    &mut rgba,
                    target_indices,
                    &mut target_i,
                    &mut src_pos,
                    stride,
                    src_pixel_count,
                    gray_value,
                );
                if actual_count > 0 {
                    prev_pixel = gray_value;
                }
            }
            0b10 => {
                let diff_mode = (byte0 >> 4) & 0x03;
                let diff_value = (byte0 & 0x0F) as i32;
                let (delta, run_count) = match diff_mode {
                    0b00 => (diff_value, 1usize),
                    0b01 => {
                        if i >= end {
                            return Err(ConvertError::RleDecodeError {
                                offset: i,
                                detail: "expected run count for diff".into(),
                            });
                        }
                        let count = data[i] as usize;
                        i += 1;
                        (diff_value, count)
                    }
                    0b10 => (-diff_value, 1usize),
                    0b11 => {
                        if i >= end {
                            return Err(ConvertError::RleDecodeError {
                                offset: i,
                                detail: "expected run count for diff".into(),
                            });
                        }
                        let count = data[i] as usize;
                        i += 1;
                        (-diff_value, count)
                    }
                    _ => unreachable!(),
                };

                let run_start = src_pos;
                let run_end = (run_start + run_count).min(src_pixel_count);
                let actual_count = run_end.saturating_sub(run_start);
                let start_pixel = prev_pixel;

                if delta == 0 {
                    while target_i < target_indices.len() {
                        let wanted = target_indices[target_i];
                        if wanted >= run_end {
                            break;
                        }
                        if wanted >= run_start {
                            write_rgba_gray(&mut rgba, target_i, start_pixel);
                        }
                        target_i += 1;
                    }
                } else {
                    while target_i < target_indices.len() {
                        let wanted = target_indices[target_i];
                        if wanted >= run_end {
                            break;
                        }
                        if wanted >= run_start {
                            let offset = (wanted - run_start + 1) as i32;
                            let value = clamp_u8(start_pixel as i32 + delta * offset);
                            write_rgba_gray(&mut rgba, target_i, value);
                        }
                        target_i += 1;
                    }
                }

                if actual_count > 0 {
                    prev_pixel = clamp_u8(start_pixel as i32 + delta * actual_count as i32);
                }
                src_pos = run_end;
            }
            _ => unreachable!(),
        }
    }

    Ok((rgba, plan.dst_width, plan.dst_height))
}

/// Decode GOO RLE data into `width * height` grayscale pixels.
///
/// Format:
///   Byte 0: 0x55 (magic)
///   Subsequent bytes:
///     bits [7:6] = chunk type:
///       0b00 = all black (0x00) + stride
///       0b01 = gray value (next byte) + stride
///       0b10 = diff from previous pixel
///       0b11 = all white (0xFF) + stride
///     For types 00/01/11:
///       bits [5:4] = stride length encoding
///       bits [3:0] = low 4 bits of stride
///     For type 10 (diff):
///       bits [5:4] = diff mode
///       bits [3:0] = diff value
///   Last byte: checksum = (~sum) & 0xFF
///
/// The current decoder validates framing and shape but does not reject checksum
/// mismatches, preserving compatibility with files produced by permissive
/// slicers.
///
/// 将 GOO RLE 编码数据解码为完整灰度像素数组。当前 decoder 校验 framing 与尺寸形状，但不因
/// checksum mismatch 拒绝文件，以兼容宽松 slicer 生成的文件。
pub fn decode_goo_rle(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    let mut pixels = Vec::with_capacity(pixel_count);
    decode_goo_rle_into_vec(data, width, height, &mut pixels)?;
    Ok(pixels)
}

/// Decode GOO RLE data directly into full-size RGBA pixels.
///
/// This is useful for preview paths that need browser-ready pixels but do not
/// downsample.
///
/// 直接将 GOO RLE 解码为 full-size RGBA 像素。适用于需要浏览器可用像素、但不 downsample 的
/// preview 路径。
pub fn decode_goo_rle_rgba(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    decode_goo_rle_into_rgba(data, width, height, &mut rgba)?;
    Ok(rgba)
}

/// Decode GOO RLE data into a pre-allocated full-size RGBA buffer.
///
/// The output buffer is cleared first and then reused.
///
/// 将 GOO RLE 解码到预分配 full-size RGBA buffer；输出 buffer 会先 clear，再复用容量。
pub fn decode_goo_rle_into_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    decode_goo_rle_with_sink(data, width, height, &mut RgbaDecodeSink { output })
}

/// Decode GOO RLE data into a pre-allocated buffer, reusing its capacity.
///
/// 将 GOO RLE 解码到预分配灰度 buffer，并复用已有容量。
pub fn decode_goo_rle_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    decode_goo_rle_into_vec(data, width, height, output)
}

fn decode_goo_rle_into_vec(
    data: &[u8],
    width: u32,
    height: u32,
    pixels: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    decode_goo_rle_with_sink(data, width, height, &mut GrayscaleDecodeSink { pixels })
}

trait GooDecodeSink {
    fn len(&self) -> usize;
    fn resize_constant(&mut self, new_len: usize, value: u8);
    fn push_diff(&mut self, value: u8);
}

struct GrayscaleDecodeSink<'a> {
    pixels: &'a mut Vec<u8>,
}

impl GooDecodeSink for GrayscaleDecodeSink<'_> {
    fn len(&self) -> usize {
        self.pixels.len()
    }

    fn resize_constant(&mut self, new_len: usize, value: u8) {
        self.pixels.resize(new_len, value);
    }

    fn push_diff(&mut self, value: u8) {
        self.pixels.push(value);
    }
}

struct RgbaDecodeSink<'a> {
    output: &'a mut Vec<u8>,
}

impl GooDecodeSink for RgbaDecodeSink<'_> {
    fn len(&self) -> usize {
        self.output.len() / 4
    }

    fn resize_constant(&mut self, new_len: usize, value: u8) {
        let start = self.output.len();
        self.output.resize(new_len * 4, 0);
        for pixel in self.output[start..].as_chunks_mut::<4>().0 {
            pixel[0] = value;
            pixel[1] = value;
            pixel[2] = value;
            pixel[3] = 255;
        }
    }

    fn push_diff(&mut self, value: u8) {
        self.output.extend_from_slice(&[value, value, value, 255]);
    }
}

fn decode_goo_rle_with_sink(
    data: &[u8],
    width: u32,
    height: u32,
    sink: &mut impl GooDecodeSink,
) -> Result<(), ConvertError> {
    let pixel_count = (width as usize) * (height as usize);

    if data.is_empty() {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: "empty GOO RLE data".into(),
        });
    }

    // Verify magic byte
    if data[0] != super::LAYER_MAGIC {
        return Err(ConvertError::RleDecodeError {
            offset: 0,
            detail: format!("expected magic 0x55, got {:#04x}", data[0]),
        });
    }

    let mut i = 1; // skip magic
    let end = if data.len() > 1 { data.len() - 1 } else { 1 }; // exclude checksum
    let mut prev_pixel: u8 = 0;

    while i < end && sink.len() < pixel_count {
        let byte0 = data[i];
        i += 1;

        let chunk_type = (byte0 >> 6) & 0x03;

        match chunk_type {
            // 0b00 = all black, 0b11 = all white
            0b00 | 0b11 => {
                let value = if chunk_type == 0b00 { 0x00u8 } else { 0xFFu8 };
                let stride = read_stride(data, &mut i, byte0)?;
                let count = stride.min(pixel_count - sink.len());
                // Let the output sink choose its efficient constant-run fill strategy.
                let new_len = sink.len() + count;
                sink.resize_constant(new_len, value);
                if count > 0 {
                    prev_pixel = value;
                }
            }
            // 0b01 = gray value follows
            0b01 => {
                if i >= end {
                    return Err(ConvertError::RleDecodeError {
                        offset: i,
                        detail: "expected gray value byte".into(),
                    });
                }
                let gray_value = data[i];
                i += 1;
                let stride = read_stride(data, &mut i, byte0)?;
                let count = stride.min(pixel_count - sink.len());
                let new_len = sink.len() + count;
                sink.resize_constant(new_len, gray_value);
                if count > 0 {
                    prev_pixel = gray_value;
                }
            }
            // 0b10 = diff from previous
            0b10 => {
                let diff_mode = (byte0 >> 4) & 0x03;
                let diff_value = (byte0 & 0x0F) as i16;

                let (delta, run_count) = match diff_mode {
                    0b00 => (diff_value, 1usize), // positive, single
                    0b01 => {
                        // positive, with run
                        if i >= end {
                            return Err(ConvertError::RleDecodeError {
                                offset: i,
                                detail: "expected run count for diff".into(),
                            });
                        }
                        let count = data[i] as usize;
                        i += 1;
                        (diff_value, count)
                    }
                    0b10 => (-diff_value, 1usize), // negative, single
                    0b11 => {
                        // negative, with run
                        if i >= end {
                            return Err(ConvertError::RleDecodeError {
                                offset: i,
                                detail: "expected run count for diff".into(),
                            });
                        }
                        let count = data[i] as usize;
                        i += 1;
                        (-diff_value, count)
                    }
                    _ => unreachable!(),
                };

                let actual_count = run_count.min(pixel_count - sink.len());
                if delta == 0 {
                    let new_len = sink.len() + actual_count;
                    sink.resize_constant(new_len, prev_pixel);
                } else {
                    for _ in 0..actual_count {
                        let new_val = (prev_pixel as i16 + delta).clamp(0, 255) as u8;
                        sink.push_diff(new_val);
                        prev_pixel = new_val;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    sink.resize_constant(pixel_count, 0);
    Ok(())
}

/// Read variable-length stride from GOO RLE data.
#[inline]
fn read_stride(data: &[u8], i: &mut usize, byte0: u8) -> Result<usize, ConvertError> {
    let length_type = (byte0 >> 4) & 0x03;
    let low4 = (byte0 & 0x0F) as usize;

    match length_type {
        0b00 => Ok(low4), // 4-bit stride
        0b01 => {
            // 12-bit stride
            if *i >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: *i,
                    detail: "unexpected end reading 12-bit stride".into(),
                });
            }
            let b1 = data[*i] as usize;
            *i += 1;
            Ok((b1 << 4) | low4)
        }
        0b10 => {
            // 20-bit stride
            if *i + 1 >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: *i,
                    detail: "unexpected end reading 20-bit stride".into(),
                });
            }
            let b1 = data[*i] as usize;
            let b2 = data[*i + 1] as usize;
            *i += 2;
            Ok((b1 << 12) | (b2 << 4) | low4)
        }
        0b11 => {
            // 28-bit stride
            if *i + 2 >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: *i,
                    detail: "unexpected end reading 28-bit stride".into(),
                });
            }
            let b1 = data[*i] as usize;
            let b2 = data[*i + 1] as usize;
            let b3 = data[*i + 2] as usize;
            *i += 3;
            Ok((b1 << 20) | (b2 << 12) | (b3 << 4) | low4)
        }
        _ => unreachable!(),
    }
}

#[inline]
fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

#[inline]
fn write_rgba_gray(rgba: &mut [u8], target_i: usize, value: u8) {
    let out = target_i * 4;
    rgba[out] = value;
    rgba[out + 1] = value;
    rgba[out + 2] = value;
}

#[inline]
fn target_constant_run(
    rgba: &mut [u8],
    target_indices: &[usize],
    target_i: &mut usize,
    src_pos: &mut usize,
    run_count: usize,
    src_pixel_count: usize,
    value: u8,
) -> usize {
    let run_start = *src_pos;
    let run_end = (run_start + run_count).min(src_pixel_count);
    while *target_i < target_indices.len() {
        let wanted = target_indices[*target_i];
        if wanted >= run_end {
            break;
        }
        if wanted >= run_start {
            write_rgba_gray(rgba, *target_i, value);
        }
        *target_i += 1;
    }
    *src_pos = run_end;
    run_end.saturating_sub(run_start)
}

/// Encode grayscale pixels into GOO RLE format.
///
/// Color-difference compression is enabled for standard GOO output.
///
/// 将灰度像素数组编码为 GOO RLE 格式。标准 GOO 输出启用 color-difference compression。
pub fn encode_goo_rle(pixels: &[u8]) -> Result<Vec<u8>, ConvertError> {
    encode_goo_rle_with_options(pixels, true)
}

/// Encode grayscale pixels into a GOO-family RLE stream.
///
/// PRZ files reuse the same container and decoder but disable color-diff chunks.
///
/// 将灰度像素编码为 GOO 系列 RLE 流，可选是否启用差分压缩；PRZ 复用同一 container 与
/// decoder，但禁用 color-diff chunks。
pub fn encode_goo_rle_with_options(
    pixels: &[u8],
    use_color_difference_compression: bool,
) -> Result<Vec<u8>, ConvertError> {
    let mut output = Vec::with_capacity(pixels.len() / 2);
    encode_goo_rle_with_options_into(pixels, use_color_difference_compression, &mut output)?;
    Ok(output)
}

/// In-place variant of [`encode_goo_rle`] writing to a caller-owned buffer.
///
/// The buffer is cleared first and capacity is preserved.
///
/// [`encode_goo_rle`] 的就地写入版本。buffer 会先 clear，并保留容量。
pub fn encode_goo_rle_into(pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
    encode_goo_rle_with_options_into(pixels, true, output)
}

/// In-place variant of [`encode_goo_rle_with_options`] writing to a caller-owned
/// buffer. The buffer is `clear()`-ed first; capacity is preserved so a
/// thread_local pool (`ENCODE_BUF`) can amortise the allocation across layers.
///
/// [`encode_goo_rle_with_options`] 的就地写入版本。buffer 会先 `clear()` 并保留 capacity，
/// 便于 thread_local pool (`ENCODE_BUF`) 在多层编码间摊销分配。
pub fn encode_goo_rle_with_options_into(
    pixels: &[u8],
    use_color_difference_compression: bool,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    encode_goo_rle_with_options_append(pixels, use_color_difference_compression, output)
}

/// Append a GOO RLE payload to an existing buffer without clearing it first.
///
/// This is used by file builders that prefix layer metadata before appending the
/// RLE stream. The checksum covers only the appended RLE payload.
///
/// 将 GOO RLE payload 追加到已有 buffer，不先 clear。文件构建器可先写 layer metadata，
/// 再追加 RLE stream；checksum 只覆盖追加的 RLE payload。
pub fn encode_goo_rle_with_options_append(
    pixels: &[u8],
    use_color_difference_compression: bool,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let hint = pixels.len() / 2;
    let needed = output.len().saturating_add(hint);
    if output.capacity() < needed {
        output.reserve(needed - output.capacity());
    }
    let rle_start = output.len();
    output.push(super::LAYER_MAGIC); // 0x55

    if pixels.is_empty() {
        output.push(0); // checksum
        return Ok(());
    }

    let mut i = 0;
    let mut prev_pixel: u8 = 0;
    let len = pixels.len();
    let ptr = pixels.as_ptr();
    let scanner = RunScanner::detect();

    while i < len {
        let current = pixels[i];
        let j = unsafe { scanner.scan_u8_run_end(ptr, len, i, current) };
        let run = j - i;

        if current == 0x00 {
            // All black
            encode_solid_run(output, 0b00, run);
        } else if current == 0xFF {
            // All white
            encode_solid_run(output, 0b11, run);
        } else {
            // Check if diff encoding is more efficient
            let diff = current as i16 - prev_pixel as i16;
            // Diff with run>1 only works when diff==0 (constant value).
            // When diff!=0, the decoder applies it cumulatively, producing
            // an arithmetic sequence — not a constant run.
            let use_diff = use_color_difference_compression
                && diff.unsigned_abs() <= 15
                && (run == 1 || (diff == 0 && run <= 255));
            if use_diff {
                // Diff encoding
                let (sign_bits, abs_diff) = if diff >= 0 {
                    (0b00u8, diff as u8)
                } else {
                    (0b10u8, (-diff) as u8)
                };

                if run == 1 {
                    output.push(0b1000_0000 | (sign_bits << 4) | (abs_diff & 0x0F));
                } else {
                    output.push(0b1000_0000 | ((sign_bits | 0b01) << 4) | (abs_diff & 0x0F));
                    output.push(run as u8);
                }
            } else {
                // Gray value encoding
                encode_gray_run(output, current, run);
            }
        }

        prev_pixel = current;
        i += run;
    }

    // Checksum: (~sum) & 0xFF
    let sum: u8 = output[rle_start + 1..]
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b));
    output.push(!sum);

    Ok(())
}

/// Encode a solid (black or white) run with variable stride.
#[inline]
fn encode_solid_run(output: &mut Vec<u8>, chunk_type: u8, total_stride: usize) {
    let max_stride = 0x0FFF_FFFF; // 28-bit max
    let mut remaining = total_stride;
    while remaining > 0 {
        let stride = remaining.min(max_stride);
        let byte0 = (chunk_type << 6) | encode_stride_header(stride);
        output.push(byte0);
        encode_stride_extra(output, stride);
        remaining -= stride;
    }
}

/// Encode a gray-value run with variable stride.
#[inline]
fn encode_gray_run(output: &mut Vec<u8>, gray: u8, total_stride: usize) {
    let max_stride = 0x0FFF_FFFF;
    let mut remaining = total_stride;
    while remaining > 0 {
        let stride = remaining.min(max_stride);
        let byte0 = (0b01 << 6) | encode_stride_header(stride);
        output.push(byte0);
        output.push(gray);
        encode_stride_extra(output, stride);
        remaining -= stride;
    }
}

/// Encode stride length type + low 4 bits into byte0 bits [5:0].
#[inline]
fn encode_stride_header(stride: usize) -> u8 {
    let low4 = (stride & 0x0F) as u8;
    if stride <= 0x0F {
        low4
    } else if stride <= 0x0FFF {
        (0b01 << 4) | low4
    } else if stride <= 0x0F_FFFF {
        (0b10 << 4) | low4
    } else {
        (0b11 << 4) | low4
    }
}

/// Write extra stride bytes (after byte0).
#[inline]
fn encode_stride_extra(output: &mut Vec<u8>, stride: usize) {
    if stride <= 0x0F {
        // No extra bytes
    } else if stride <= 0x0FFF {
        output.push((stride >> 4) as u8);
    } else if stride <= 0x0F_FFFF {
        output.push((stride >> 12) as u8);
        output.push((stride >> 4) as u8);
    } else {
        output.push((stride >> 20) as u8);
        output.push((stride >> 12) as u8);
        output.push((stride >> 4) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::read_stride;

    #[test]
    fn read_stride_decodes_all_supported_widths() {
        let mut index = 0usize;
        let data = [0x00, 0xAB, 0xCD, 0xEF];

        assert_eq!(read_stride(&data, &mut index, 0x0F).unwrap(), 0x0F);
        assert_eq!(index, 0);

        assert_eq!(read_stride(&data, &mut index, 0x1F).unwrap(), 0x0F);
        assert_eq!(index, 1);

        assert_eq!(read_stride(&data, &mut index, 0x2F).unwrap(), 0xABCDF);
        assert_eq!(index, 3);
    }
}
