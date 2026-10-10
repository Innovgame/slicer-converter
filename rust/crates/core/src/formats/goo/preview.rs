//! GOO embedded preview image codec.
//!
//! GOO previews are raw big-endian RGB565 buffers with fixed dimensions in the
//! file header. Unlike CTB previews, these payloads are not RLE-compressed.
//!
//! GOO preview 是嵌在 file header 中的 raw big-endian RGB565 buffer，尺寸固定；
//! 与 CTB preview 不同，这些 payload 不做 RLE 压缩。

use crate::common::ConvertError;

/// Decode GOO RGB565 big-endian raw preview to RGBA pixels.
///
/// Format: each pixel 2 bytes, big-endian:
///   bit [15:11] = R (5 bit)
///   bit [10:5]  = G (6 bit)
///   bit [4:0]   = B (5 bit)
///
/// 将 GOO RGB565 大端原始预览图解码为 RGBA 像素数组。
pub fn decode_rgb565_be(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    let expected_size = pixel_count * 2;
    if data.len() < expected_size {
        return Err(ConvertError::BufferTooSmall {
            need: expected_size,
            got: data.len(),
        });
    }

    let mut rgba = Vec::with_capacity(pixel_count * 4);

    for i in 0..pixel_count {
        let hi = data[i * 2] as u16;
        let lo = data[i * 2 + 1] as u16;
        let val = (hi << 8) | lo; // big-endian

        let r5 = ((val >> 11) & 0x1F) as u8;
        let g6 = ((val >> 5) & 0x3F) as u8;
        let b5 = (val & 0x1F) as u8;

        // Expand to 8-bit
        let r = (r5 << 3) | (r5 >> 2);
        let g = (g6 << 2) | (g6 >> 4);
        let b = (b5 << 3) | (b5 >> 2);

        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(255);
    }

    Ok(rgba)
}

/// Encode RGBA pixels to raw big-endian RGB565 bytes.
///
/// The input must contain at least `width * height * 4` bytes. Alpha is ignored
/// because the GOO preview format only stores RGB.
///
/// 将 RGBA 像素数组编码为 GOO RGB565 big-endian raw format。输入至少需要
/// `width * height * 4` 字节；alpha 会被忽略，因为 GOO preview 只存 RGB。
pub fn encode_rgb565_be(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    if rgba.len() < pixel_count * 4 {
        return Err(ConvertError::BufferTooSmall {
            need: pixel_count * 4,
            got: rgba.len(),
        });
    }

    let mut output = Vec::with_capacity(pixel_count * 2);

    for i in 0..pixel_count {
        let off = i * 4;
        let r5 = (rgba[off] >> 3) as u16;
        let g6 = (rgba[off + 1] >> 2) as u16;
        let b5 = (rgba[off + 2] >> 3) as u16;

        let val = (r5 << 11) | (g6 << 5) | b5;
        output.push((val >> 8) as u8); // big-endian high byte
        output.push(val as u8); // big-endian low byte
    }

    Ok(output)
}

/// Convert RGBA pixels to raw big-endian RGB565 without size validation.
///
/// This helper is used when the caller already owns a correctly sized thumbnail
/// buffer and wants the compact GOO preview representation.
///
/// 将 RGBA pixels 转为 raw big-endian RGB565，不额外做尺寸校验。适用于调用方已经持有正确尺寸
/// thumbnail buffer，并需要紧凑 GOO preview 表示的路径。
pub fn rgba_to_rgb565_be(rgba: &[u8]) -> Vec<u8> {
    let pixel_count = rgba.len() / 4;
    let mut output = Vec::with_capacity(pixel_count * 2);

    for i in 0..pixel_count {
        let off = i * 4;
        let r5 = (rgba[off] >> 3) as u16;
        let g6 = (rgba[off + 1] >> 2) as u16;
        let b5 = (rgba[off + 2] >> 3) as u16;

        let val = (r5 << 11) | (g6 << 5) | b5;
        output.push((val >> 8) as u8);
        output.push(val as u8);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_solid_white() {
        let w = 4;
        let h = 4;
        let rgba = vec![255u8; (w * h * 4) as usize];

        let encoded = encode_rgb565_be(&rgba, w, h).unwrap();
        assert_eq!(encoded.len(), (w * h * 2) as usize);

        let decoded = decode_rgb565_be(&encoded, w, h).unwrap();
        // White (255,255,255) → RGB565 (31,63,31) → expand back (255,255,255)
        assert_eq!(decoded[0], 255); // R: 31 << 3 | 31 >> 2 = 248+7 = 255
        assert_eq!(decoded[1], 255); // G: 63 << 2 | 63 >> 4 = 252+3 = 255
        assert_eq!(decoded[2], 255); // B: 31 << 3 | 31 >> 2 = 248+7 = 255
    }

    #[test]
    fn roundtrip_mixed() {
        let w = 2;
        let h = 2;
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            128, 128, 128, 255, // gray
        ];

        let encoded = encode_rgb565_be(&rgba, w, h).unwrap();
        let decoded = decode_rgb565_be(&encoded, w, h).unwrap();

        // Red pixel
        assert!(decoded[0] > 240);
        assert!(decoded[1] < 8);
        assert!(decoded[2] < 8);
    }
}
