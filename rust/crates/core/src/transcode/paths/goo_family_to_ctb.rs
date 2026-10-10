//! Shared GOO-family to CTB RLE fast path.
//!
//! GOO and PRZ share the same layer RLE payload shape for this route, so the
//! direct transcoder lives here and profile-specific modules only expose thin
//! strategy wrappers.
//!
//! GOO 与 PRZ 在这条 route 上共享相同的 layer RLE payload 形态，因此直接转码逻辑放在这里，
//! 具体 profile 模块只暴露很薄的策略 wrapper。

use crate::{
    common::ConvertError,
    formats::goo,
    transcode::{FastPathOutcome, LayerTranscodeRequest, LayerTranscodeScratch, LayerTranscoder},
};

/// Shared strategy object for GOO-family RLE to CTB gray7 RLE.
///
/// GOO-family RLE 到 CTB gray7 RLE 的共享策略对象。
pub(crate) struct GooFamilyToCtbTranscoder;

/// Process-wide singleton used by the route registry.
///
/// route registry 使用的进程级 singleton。
pub(crate) static GOO_FAMILY_TO_CTB_TRANSCODER: GooFamilyToCtbTranscoder = GooFamilyToCtbTranscoder;

impl LayerTranscoder for GooFamilyToCtbTranscoder {
    fn try_transcode_layer_into(
        &self,
        src: &[u8],
        req: &LayerTranscodeRequest<'_>,
        scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError> {
        try_transcode_layer_into(src, req, scratch, out)
    }
}

/// Attempts direct GOO-family RLE to CTB RLE conversion.
///
/// The function returns `Fallback` for malformed or unsupported payloads instead
/// of surfacing an error, allowing the engine to use the authoritative
/// decode/encode path.
///
/// 尝试将 GOO-family RLE 直接转换为 CTB RLE。遇到 malformed 或不支持的 payload 时返回
/// `Fallback` 而不是错误，让 engine 继续使用权威的 decode/encode 路径。
pub(crate) fn try_transcode_layer_into(
    src: &[u8],
    req: &LayerTranscodeRequest<'_>,
    _scratch: &mut LayerTranscodeScratch,
    out: &mut Vec<u8>,
) -> Result<FastPathOutcome, ConvertError> {
    if src.len() < 2 || src[0] != goo::LAYER_MAGIC {
        return Ok(FastPathOutcome::Fallback);
    }

    let pixel_count = (req.width as usize).saturating_mul(req.height as usize);
    let end = src.len() - 1; // exclude checksum, matching GOO-family fallback decoder
    let mut i = 1usize;
    let mut src_pos = 0usize;
    let mut prev_pixel = 0u8;
    let mut pending_gray7 = None;
    let mut pending_len = 0usize;

    out.clear();

    while i < end && src_pos < pixel_count {
        let byte0 = src[i];
        i += 1;

        match (byte0 >> 6) & 0x03 {
            0b00 | 0b11 => {
                let value = if ((byte0 >> 6) & 0x03) == 0b00 {
                    0x00
                } else {
                    0xFF
                };
                let Some(stride) = read_goo_stride(src, &mut i, end, byte0) else {
                    return fallback(out);
                };
                let actual_count = stride.min(pixel_count - src_pos);
                queue_constant_run(
                    out,
                    &mut pending_gray7,
                    &mut pending_len,
                    value,
                    actual_count,
                );
                if actual_count > 0 {
                    prev_pixel = value;
                    src_pos += actual_count;
                }
            }
            0b01 => {
                if i >= end {
                    return fallback(out);
                }
                let gray_value = src[i];
                i += 1;
                let Some(stride) = read_goo_stride(src, &mut i, end, byte0) else {
                    return fallback(out);
                };
                let actual_count = stride.min(pixel_count - src_pos);
                queue_constant_run(
                    out,
                    &mut pending_gray7,
                    &mut pending_len,
                    gray_value,
                    actual_count,
                );
                if actual_count > 0 {
                    prev_pixel = gray_value;
                    src_pos += actual_count;
                }
            }
            0b10 => {
                let Some((delta, run_count)) = read_goo_diff(src, &mut i, end, byte0) else {
                    return fallback(out);
                };
                let actual_count = run_count.min(pixel_count - src_pos);
                if actual_count == 0 {
                    continue;
                }
                queue_diff_run(
                    out,
                    &mut pending_gray7,
                    &mut pending_len,
                    &mut prev_pixel,
                    delta,
                    actual_count,
                );
                src_pos += actual_count;
            }
            _ => unreachable!(),
        }
    }

    if src_pos != pixel_count {
        return fallback(out);
    }

    flush_pending_run(out, &mut pending_gray7, &mut pending_len);
    Ok(FastPathOutcome::Applied)
}

fn fallback(out: &mut Vec<u8>) -> Result<FastPathOutcome, ConvertError> {
    out.clear();
    Ok(FastPathOutcome::Fallback)
}

fn read_goo_stride(data: &[u8], index: &mut usize, end: usize, byte0: u8) -> Option<usize> {
    let length_type = (byte0 >> 4) & 0x03;
    let low4 = (byte0 & 0x0F) as usize;

    match length_type {
        0b00 => Some(low4),
        0b01 => {
            if *index >= end {
                return None;
            }
            let b1 = data[*index] as usize;
            *index += 1;
            Some((b1 << 4) | low4)
        }
        0b10 => {
            if *index + 1 >= end {
                return None;
            }
            let b1 = data[*index] as usize;
            let b2 = data[*index + 1] as usize;
            *index += 2;
            Some((b1 << 12) | (b2 << 4) | low4)
        }
        0b11 => {
            if *index + 2 >= end {
                return None;
            }
            let b1 = data[*index] as usize;
            let b2 = data[*index + 1] as usize;
            let b3 = data[*index + 2] as usize;
            *index += 3;
            Some((b1 << 20) | (b2 << 12) | (b3 << 4) | low4)
        }
        _ => unreachable!(),
    }
}

fn read_goo_diff(data: &[u8], index: &mut usize, end: usize, byte0: u8) -> Option<(i16, usize)> {
    let diff_mode = (byte0 >> 4) & 0x03;
    let diff_value = (byte0 & 0x0F) as i16;

    match diff_mode {
        0b00 => Some((diff_value, 1)),
        0b01 => {
            if *index >= end {
                return None;
            }
            let count = data[*index] as usize;
            *index += 1;
            Some((diff_value, count))
        }
        0b10 => Some((-diff_value, 1)),
        0b11 => {
            if *index >= end {
                return None;
            }
            let count = data[*index] as usize;
            *index += 1;
            Some((-diff_value, count))
        }
        _ => unreachable!(),
    }
}

fn queue_constant_run(
    output: &mut Vec<u8>,
    pending_gray7: &mut Option<u8>,
    pending_len: &mut usize,
    value: u8,
    count: usize,
) {
    if count == 0 {
        return;
    }

    let gray7 = value >> 1;
    match pending_gray7 {
        Some(current) if *current == gray7 => {
            *pending_len += count;
        }
        Some(_) => {
            flush_pending_run(output, pending_gray7, pending_len);
            *pending_gray7 = Some(gray7);
            *pending_len = count;
        }
        None => {
            *pending_gray7 = Some(gray7);
            *pending_len = count;
        }
    }
}

fn flush_pending_run(
    output: &mut Vec<u8>,
    pending_gray7: &mut Option<u8>,
    pending_len: &mut usize,
) {
    if let Some(gray7) = pending_gray7.take() {
        emit_ctb_run(output, gray7, *pending_len);
    }
    *pending_len = 0;
}

fn queue_diff_run(
    output: &mut Vec<u8>,
    pending_gray7: &mut Option<u8>,
    pending_len: &mut usize,
    prev_pixel: &mut u8,
    delta: i16,
    count: usize,
) {
    if count == 0 {
        return;
    }

    if delta == 0 {
        queue_constant_run(output, pending_gray7, pending_len, *prev_pixel, count);
        return;
    }

    let mut current = *prev_pixel;
    for _ in 0..count {
        current = clamp_u8(current as i16 + delta);
        queue_constant_run(output, pending_gray7, pending_len, current, 1);
    }
    *prev_pixel = current;
}

fn emit_ctb_run(output: &mut Vec<u8>, gray7: u8, mut stride: usize) {
    while stride > 0 {
        let chunk = stride.min(0x0FFF_FFFF);
        if chunk == 1 {
            output.push(gray7);
        } else {
            output.push(gray7 | 0x80);
            encode_ctb_stride(output, chunk);
        }
        stride -= chunk;
    }
}

fn encode_ctb_stride(output: &mut Vec<u8>, stride: usize) {
    if stride <= 0x7F {
        output.push(stride as u8);
    } else if stride <= 0x3FFF {
        output.push(((stride >> 8) as u8) | 0x80);
        output.push(stride as u8);
    } else if stride <= 0x1F_FFFF {
        output.push(((stride >> 16) as u8) | 0xC0);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    } else {
        output.push(((stride >> 24) as u8) | 0xE0);
        output.push((stride >> 16) as u8);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    }
}

fn clamp_u8(value: i16) -> u8 {
    value.clamp(0, 255) as u8
}
