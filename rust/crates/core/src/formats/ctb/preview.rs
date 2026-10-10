//! CTB preview image codec.
//!
//! CTB preview payloads use little-endian RGB15 words with bit 5 overloaded as
//! the repeat marker. Alpha is not stored on disk; decoded RGBA always uses an
//! opaque alpha for emitted pixels and transparent padding for malformed short
//! streams.
//!
//! CTB preview payload 使用 little-endian RGB15 words，并把 bit 5 复用为 repeat marker。
//! 文件中不存 alpha；正常 decoded pixels 使用 opaque alpha，异常短流的 padding 使用透明像素。

use crate::common::ConvertError;

/// Decode CTB RGB15 RLE preview image to RGBA pixels.
///
/// UVtools/Chitubox preview RLE uses RGB15-like packing with a run marker in bit 5.
/// The payload is little-endian u16 words:
///   bits [15:11] = R (5 bit)
///   bits [10:6]  = G (5 bit)
///   bit  [5]     = run marker
///   bits [4:0]   = B (5 bit)
///
/// If the run marker is set, the next u16 contributes low 12 bits as `repeat - 1`.
///
/// 将 CTB RGB15 RLE 编码的预览图解码为 RGBA 像素数组。
pub fn decode_rgb15_rle(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    const REPEAT_RGB15_MASK: u16 = 0x20;

    let pixel_count = (width as usize) * (height as usize);
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    let mut i = 0;

    while i + 1 < data.len() && rgba.len() < pixel_count * 4 {
        let lo = data[i] as u16;
        let hi = data[i + 1] as u16;
        let val = lo | (hi << 8); // little-endian
        i += 2;

        let r5 = ((val >> 11) & 0x1F) as u8;
        let g5 = ((val >> 6) & 0x1F) as u8;
        let b5 = (val & 0x1F) as u8;
        let is_run = (val & REPEAT_RGB15_MASK) == REPEAT_RGB15_MASK;

        // Expand 5-bit to 8-bit: (v << 3) | (v >> 2)
        let r = (r5 << 3) | (r5 >> 2);
        let g = (g5 << 3) | (g5 >> 2);
        let b = (b5 << 3) | (b5 >> 2);

        let count = if is_run {
            if i + 1 >= data.len() {
                return Err(ConvertError::RleDecodeError {
                    offset: i,
                    detail: "unexpected end reading RGB15 run length".into(),
                });
            }
            let rlo = data[i] as u16;
            let rhi = data[i + 1] as u16;
            i += 2;
            1 + (((rlo | (rhi << 8)) & 0x0FFF) as usize)
        } else {
            1
        };

        let remaining = (pixel_count * 4 - rgba.len()) / 4;
        let actual = count.min(remaining);
        for _ in 0..actual {
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255);
        }
    }

    // Pad remaining pixels as transparent
    rgba.resize(pixel_count * 4, 0);
    Ok(rgba)
}

/// Encode RGBA pixels to CTB RGB15 RLE preview bytes.
///
/// The input must contain at least `width * height * 4` bytes. Alpha is ignored,
/// and RGB is quantized to 5 bits per channel before adjacent equal colors are
/// collapsed into repeat records.
///
/// 将 RGBA 像素数组编码为 CTB RGB15 RLE 预览图格式。输入至少需要
/// `width * height * 4` 字节；alpha 会被忽略，RGB 会量化为 5-bit channel 后再做 run 合并。
pub fn encode_rgb15_rle(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    const REPEAT_RGB15_MASK: u16 = 0x20;
    const RLE16_ENCODING_LIMIT: usize = 0x0FFF;

    let pixel_count = (width as usize) * (height as usize);
    if rgba.len() < pixel_count * 4 {
        return Err(ConvertError::BufferTooSmall {
            need: pixel_count * 4,
            got: rgba.len(),
        });
    }

    let mut output = Vec::with_capacity(pixel_count * 2);

    let mut color15 = 0u16;
    let mut run = 0usize;

    let flush_run = |output: &mut Vec<u8>, color15: u16, run: usize| match run {
        0 => {}
        1 | 2 => {
            for _ in 0..run {
                let val = color15 & !REPEAT_RGB15_MASK;
                output.extend_from_slice(&val.to_le_bytes());
            }
        }
        _ => {
            let mut remaining = run;
            while remaining > 0 {
                let chunk = remaining.min(RLE16_ENCODING_LIMIT + 1);
                if chunk <= 2 {
                    for _ in 0..chunk {
                        let val = color15 & !REPEAT_RGB15_MASK;
                        output.extend_from_slice(&val.to_le_bytes());
                    }
                } else {
                    let val = color15 | REPEAT_RGB15_MASK;
                    let repeat_word = ((chunk - 1) as u16) | 0x3000;
                    output.extend_from_slice(&val.to_le_bytes());
                    output.extend_from_slice(&repeat_word.to_le_bytes());
                }
                remaining -= chunk;
            }
        }
    };

    for idx in 0..pixel_count {
        let off = idx * 4;
        let b = rgba[off + 2] as u16;
        let g = rgba[off + 1] as u16;
        let r = rgba[off] as u16;
        let next_color15 = (b >> 3) | ((g >> 2) << 5) | ((r >> 3) << 11);

        if run == 0 {
            color15 = next_color15;
            run = 1;
            continue;
        }

        if next_color15 == color15 {
            run += 1;
            if run == RLE16_ENCODING_LIMIT + 1 {
                flush_run(&mut output, color15, run);
                run = 0;
            }
        } else {
            flush_run(&mut output, color15, run);
            color15 = next_color15;
            run = 1;
        }
    }

    flush_run(&mut output, color15, run);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_solid_red() {
        let w = 8;
        let h = 8;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for i in 0..(w * h) as usize {
            rgba[i * 4] = 255; // R
            rgba[i * 4 + 1] = 0; // G
            rgba[i * 4 + 2] = 0; // B
            rgba[i * 4 + 3] = 255; // A
        }

        let encoded = encode_rgb15_rle(&rgba, w, h).unwrap();
        let decoded = decode_rgb15_rle(&encoded, w, h).unwrap();

        // Check first pixel
        assert_eq!(decoded[0], 255); // R: 31 << 3 | 31 >> 2 = 248+7 = 255
        assert_eq!(decoded[1], 0);
        assert_eq!(decoded[2], 0);
        assert_eq!(decoded[3], 255);
    }

    #[test]
    fn roundtrip_gradient() {
        let w = 16;
        let h = 4;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for i in 0..(w * h) as usize {
            let gray = (i * 4) as u8;
            rgba[i * 4] = gray;
            rgba[i * 4 + 1] = gray;
            rgba[i * 4 + 2] = gray;
            rgba[i * 4 + 3] = 255;
        }

        let encoded = encode_rgb15_rle(&rgba, w, h).unwrap();
        let decoded = decode_rgb15_rle(&encoded, w, h).unwrap();

        // Due to 5-bit quantization, values won't match exactly
        // but should be close (within 8)
        for i in 0..(w * h) as usize {
            let orig = rgba[i * 4];
            let dec = decoded[i * 4];
            assert!(
                (orig as i16 - dec as i16).unsigned_abs() <= 8,
                "pixel {i}: orig={orig}, decoded={dec}"
            );
        }
    }

    #[test]
    fn encode_uses_uvtools_repeat_marker_layout() {
        let w = 8;
        let h = 1;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for i in 0..w as usize {
            rgba[i * 4] = 255;
            rgba[i * 4 + 1] = 0;
            rgba[i * 4 + 2] = 0;
            rgba[i * 4 + 3] = 255;
        }

        let encoded = encode_rgb15_rle(&rgba, w, h).unwrap();
        assert_eq!(encoded.len(), 4);

        let color_word = u16::from_le_bytes([encoded[0], encoded[1]]);
        let repeat_word = u16::from_le_bytes([encoded[2], encoded[3]]);

        assert_eq!(color_word & 0x20, 0x20);
        assert_eq!(repeat_word & 0x0FFF, 7);
        assert_eq!(
            decode_rgb15_rle(&encoded, w, h).unwrap().len(),
            (w * h * 4) as usize
        );
    }
}
