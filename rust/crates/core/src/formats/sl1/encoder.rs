use std::io::Cursor;

use crate::common::ConvertError;

/// Encode grayscale pixels as an 8-bit grayscale PNG layer.
///
/// SL1 layer images are ZIP entries, so the encoder returns the complete PNG
/// byte stream for the archive writer to insert.
///
/// 将灰度像素数组编码为 8-bit grayscale PNG layer。SL1 layer image 是 ZIP entry，
/// 因此这里返回完整 PNG byte stream 供 archive writer 写入。
pub fn encode_png_layer(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ConvertError> {
    let mut buf = Vec::new();
    encode_png_layer_into(pixels, width, height, &mut buf)?;
    Ok(buf)
}

/// Encode grayscale pixels as PNG bytes into a caller-owned buffer.
///
/// The buffer is cleared first and capacity is preserved, matching the hot-path
/// pattern used by archive layer encoders.
///
/// 将灰度像素编码为 PNG bytes 并写入调用方提供的 buffer。buffer 会先 clear 并保留 capacity，
/// 与 archive layer encoder 的热路径复用模式一致。
pub fn encode_png_layer_into(
    pixels: &[u8],
    width: u32,
    height: u32,
    buf: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    buf.clear();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut *buf), width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::Filter::Sub);

        let mut writer = encoder
            .write_header()
            .map_err(|e| ConvertError::PngError(e.to_string()))?;
        writer
            .write_image_data(pixels)
            .map_err(|e| ConvertError::PngError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip() {
        let width = 64u32;
        let height = 48u32;
        let mut pixels = vec![0u8; (width * height) as usize];
        // Create a simple gradient pattern
        for y in 0..height {
            for x in 0..width {
                pixels[(y * width + x) as usize] = ((x * 255) / width) as u8;
            }
        }

        let encoded = encode_png_layer(&pixels, width, height).unwrap();
        assert!(!encoded.is_empty());
        assert_eq!(&encoded[..4], &[0x89, 0x50, 0x4E, 0x47]); // PNG magic

        let decoded = super::super::decoder::decode_png_layer(&encoded, width, height).unwrap();
        assert_eq!(decoded.len(), pixels.len());
        assert_eq!(decoded, pixels); // Lossless for grayscale PNG
    }
}
