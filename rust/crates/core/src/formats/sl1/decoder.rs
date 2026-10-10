use std::io::Cursor;

use zip::ZipArchive;

use super::config::{Sl1Config, Sl1PrinterConfig};
use crate::{
    common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage},
    formats::common::{
        collect_zip_entries, read_optional_zip_text, read_required_zip_text, read_zip_entry_bytes,
        sorted_layer_offsets,
    },
};

/// SL1 ZIP archive decoder.
///
/// SL1 stores metadata in INI files and layers as PNG entries. Because ZIP
/// entries are not cheap byte slices inside the source buffer, layer offsets in
/// this decoder are logical ZIP entry indices.
///
/// SL1 ZIP archive decoder。SL1 将 metadata 存在 INI 文件中，将 layer 存为 PNG entries。
/// 由于 ZIP entry 不能直接用源 buffer byte offset 切片，这里的 layer offsets 是逻辑上的
/// ZIP entry indices。
pub struct Sl1Decoder;

impl Sl1Decoder {
    /// Parse an SL1 ZIP archive header (config.ini + prusaslicer.ini).
    /// Returns metadata and layer offsets (ZIP entry indices as offsets).
    ///
    /// For SL1, "layer offsets" are ZIP entry indices since the ZIP format
    /// doesn't support simple byte-offset slicing. The JS side should pass
    /// the entire ZIP or use a ZIP-aware streaming approach.
    pub fn parse_header(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        let cursor = Cursor::new(data);
        let mut archive =
            ZipArchive::new(cursor).map_err(|e| ConvertError::ZipError(e.to_string()))?;

        let config = Sl1Config::parse(&read_required_zip_text(&mut archive, "config.ini")?);
        let mut metadata = config.to_metadata();

        if let Some(content) = read_optional_zip_text(&mut archive, "prusaslicer.ini")? {
            let printer_config = Sl1PrinterConfig::parse(&content);
            printer_config.apply_to(&mut metadata);
        }

        let entries = collect_zip_entries(&mut archive)?;
        let layer_offsets = sorted_layer_offsets(&entries, sl1_layer_index);

        if metadata.slicing.layer_count == 0 {
            metadata.slicing.layer_count = layer_offsets.len() as u32;
        }

        Ok((metadata, layer_offsets))
    }

    /// Decode a specific layer from an SL1 ZIP archive.
    ///
    /// `entry_index` is the ZIP entry index from the layer offset table.
    ///
    /// 从 SL1 ZIP archive 中按 layer offset table 给出的 ZIP entry index 解码单层 PNG。
    pub fn decode_layer_from_archive(
        data: &[u8],
        entry_index: usize,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, ConvertError> {
        let cursor = Cursor::new(data);
        let mut archive =
            ZipArchive::new(cursor).map_err(|e| ConvertError::ZipError(e.to_string()))?;

        let png_data = read_zip_entry_bytes(&mut archive, entry_index)?;
        decode_png_layer(&png_data, width, height)
    }
}

pub(super) fn sl1_layer_index(name: &str) -> Option<u32> {
    let file_name = name.rsplit('/').next().unwrap_or(name);
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".png") || lower.contains("thumbnail") {
        return None;
    }

    let stem = lower.strip_suffix(".png")?;
    let suffix = stem.get(stem.len().checked_sub(5)?..)?;
    suffix
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| suffix.parse().ok())
        .flatten()
}

/// Decode PNG bytes into a grayscale layer buffer.
///
/// Grayscale PNGs are passed through. Grayscale-alpha, RGB, and RGBA PNGs are
/// converted to luminance so callers always receive one byte per pixel.
///
/// 将 PNG bytes 解码为 grayscale layer buffer。Grayscale PNG 会直接通过；Grayscale-alpha、
/// RGB 与 RGBA 会转为 luminance，保证调用方始终得到每像素 1 字节。
pub fn decode_png_layer(
    png_data: &[u8],
    expected_width: u32,
    expected_height: u32,
) -> Result<Vec<u8>, ConvertError> {
    let mut pixels = Vec::new();
    decode_png_layer_into(png_data, expected_width, expected_height, &mut pixels)?;
    Ok(pixels)
}

/// Decode a PNG layer into a pre-allocated grayscale buffer.
///
/// `expected_width` / `expected_height` may be zero to skip dimension checking.
/// The buffer is cleared and reused so archive decode loops can avoid repeated
/// allocations.
///
/// 将 PNG layer 解码到预分配 grayscale buffer。`expected_width` / `expected_height`
/// 可为 0 以跳过尺寸校验；buffer 会被 clear 并复用 capacity，减少 archive decode 循环分配。
pub fn decode_png_layer_into(
    png_data: &[u8],
    expected_width: u32,
    expected_height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder
        .read_info()
        .map_err(|e| ConvertError::PngError(e.to_string()))?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;

    if expected_width != 0
        && expected_height != 0
        && (width != expected_width || height != expected_height)
    {
        return Err(ConvertError::PngError(format!(
            "PNG dimensions {width}×{height} don't match expected {expected_width}×{expected_height}"
        )));
    }

    let buf_size = reader.output_buffer_size().unwrap_or(0);
    output.clear();
    output.resize(buf_size, 0u8);
    let frame_info = reader
        .next_frame(output)
        .map_err(|e| ConvertError::PngError(e.to_string()))?;

    let used = frame_info.buffer_size();

    match frame_info.color_type {
        png::ColorType::Grayscale => {
            output.truncate(used);
        }
        png::ColorType::GrayscaleAlpha => {
            let pixel_count = used / 2;
            for i in 0..pixel_count {
                output[i] = output[i * 2];
            }
            output.truncate(pixel_count);
        }
        png::ColorType::Rgb => {
            let pixel_count = used / 3;
            for i in 0..pixel_count {
                let r = output[i * 3] as u32;
                let g = output[i * 3 + 1] as u32;
                let b = output[i * 3 + 2] as u32;
                output[i] = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
            }
            output.truncate(pixel_count);
        }
        png::ColorType::Rgba => {
            let pixel_count = used / 4;
            for i in 0..pixel_count {
                let r = output[i * 4] as u32;
                let g = output[i * 4 + 1] as u32;
                let b = output[i * 4 + 2] as u32;
                output[i] = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
            }
            output.truncate(pixel_count);
        }
        _ => {
            return Err(ConvertError::PngError(format!(
                "Unsupported PNG color type: {:?}",
                frame_info.color_type
            )))
        }
    }

    Ok(())
}

/// Decode PNG bytes directly into a bounded RGBA preview.
///
/// Downsampled previews use row-by-row PNG decoding so we avoid materializing a
/// full-resolution frame before sampling. Non-downsampled previews keep the
/// full-frame path because callers are asking for every pixel anyway.
///
/// 将 PNG bytes 直接解码为 bounded RGBA preview。downsample preview 会按行解 PNG，
/// 避免先 materialize 全尺寸 frame 再采样；不需要 downsample 时仍走整帧路径。
pub fn decode_png_layer_preview_rgba(
    png_data: &[u8],
    expected_width: u32,
    expected_height: u32,
    max_preview_size: u32,
) -> Result<PreviewLayerImage, ConvertError> {
    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder
        .read_info()
        .map_err(|e| ConvertError::PngError(e.to_string()))?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;

    if expected_width != 0
        && expected_height != 0
        && (width != expected_width || height != expected_height)
    {
        return Err(ConvertError::PngError(format!(
            "PNG dimensions {width}×{height} don't match expected {expected_width}×{expected_height}"
        )));
    }

    let plan = PngPreviewDecodePlan::for_downsample(width, height, max_preview_size);
    if !info.interlaced {
        if let Some(plan) = plan {
            return decode_png_layer_preview_rgba_rows(reader, plan);
        }
    }

    let mut frame = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let frame_info = reader
        .next_frame(&mut frame)
        .map_err(|e| ConvertError::PngError(e.to_string()))?;
    frame.truncate(frame_info.buffer_size());

    let (dst_width, dst_height) = if let Some(plan) = &plan {
        (plan.dst_width, plan.dst_height)
    } else {
        (width, height)
    };

    let mut pixels = Vec::with_capacity((dst_width as usize) * (dst_height as usize) * 4);
    if let Some(plan) = &plan {
        for &source_y in &plan.source_rows {
            sample_png_frame_row_to_rgba(
                &frame,
                frame_info.color_type,
                width,
                source_y,
                plan.source_cols.iter().copied(),
                &mut pixels,
            )?;
        }
    } else {
        for source_y in 0..height {
            let source_cols = 0..width;
            sample_png_frame_row_to_rgba(
                &frame,
                frame_info.color_type,
                width,
                source_y,
                source_cols,
                &mut pixels,
            )?;
        }
    }

    Ok(PreviewLayerImage {
        pixels,
        width: dst_width,
        height: dst_height,
    })
}

struct PngPreviewDecodePlan {
    source_rows: Vec<u32>,
    source_cols: Vec<u32>,
    dst_width: u32,
    dst_height: u32,
}

impl PngPreviewDecodePlan {
    fn for_downsample(src_width: u32, src_height: u32, max_preview_size: u32) -> Option<Self> {
        if max_preview_size == 0
            || (src_width <= max_preview_size && src_height <= max_preview_size)
        {
            return None;
        }

        let scale = f64::min(
            max_preview_size as f64 / src_width as f64,
            max_preview_size as f64 / src_height as f64,
        );
        let dst_width = ((src_width as f64) * scale).round().max(1.0) as u32;
        let dst_height = ((src_height as f64) * scale).round().max(1.0) as u32;

        let mut source_rows = Vec::with_capacity(dst_height as usize);
        for y in 0..dst_height {
            source_rows.push(((y as f64) / scale).floor().min((src_height - 1) as f64) as u32);
        }

        let mut source_cols = Vec::with_capacity(dst_width as usize);
        for x in 0..dst_width {
            source_cols.push(((x as f64) / scale).floor().min((src_width - 1) as f64) as u32);
        }

        Some(Self {
            source_rows,
            source_cols,
            dst_width,
            dst_height,
        })
    }

    fn rgba_capacity(&self) -> usize {
        (self.dst_width as usize) * (self.dst_height as usize) * 4
    }
}

fn decode_png_layer_preview_rgba_rows<R: std::io::BufRead + std::io::Seek>(
    mut reader: png::Reader<R>,
    plan: PngPreviewDecodePlan,
) -> Result<PreviewLayerImage, ConvertError> {
    let (color_type, bit_depth) = reader.output_color_type();
    if bit_depth != png::BitDepth::Eight {
        return Err(ConvertError::PngError(format!(
            "Unsupported PNG bit depth for preview: {:?}",
            bit_depth
        )));
    }

    let mut output = Vec::with_capacity(plan.rgba_capacity());
    let mut dst_y = 0usize;
    let mut wanted_row = plan.source_rows.get(dst_y).copied();
    let mut source_y = 0u32;

    while let Some(row) = reader
        .next_row()
        .map_err(|e| ConvertError::PngError(e.to_string()))?
    {
        if Some(source_y) == wanted_row {
            sample_png_row_to_rgba(row.data(), color_type, &plan, &mut output)?;
            dst_y += 1;
            wanted_row = plan.source_rows.get(dst_y).copied();
        }
        source_y += 1;
    }

    let expected_len = plan.rgba_capacity();
    if output.len() != expected_len {
        return Err(ConvertError::PngError(format!(
            "PNG preview produced {} bytes, expected {}",
            output.len(),
            expected_len
        )));
    }

    Ok(PreviewLayerImage {
        pixels: output,
        width: plan.dst_width,
        height: plan.dst_height,
    })
}

fn sample_png_row_to_rgba(
    row: &[u8],
    color_type: png::ColorType,
    plan: &PngPreviewDecodePlan,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    for &source_x in &plan.source_cols {
        let source_x = source_x as usize;
        let v = match color_type {
            png::ColorType::Grayscale => *row.get(source_x).ok_or_else(|| {
                ConvertError::PngError("PNG grayscale row is shorter than expected".into())
            })?,
            png::ColorType::GrayscaleAlpha => *row.get(source_x * 2).ok_or_else(|| {
                ConvertError::PngError("PNG grayscale-alpha row is shorter than expected".into())
            })?,
            png::ColorType::Rgb => {
                let offset = source_x * 3;
                luminance(
                    *row.get(offset).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB row is shorter than expected".into())
                    })?,
                    *row.get(offset + 1).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB row is shorter than expected".into())
                    })?,
                    *row.get(offset + 2).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB row is shorter than expected".into())
                    })?,
                )
            }
            png::ColorType::Rgba => {
                let offset = source_x * 4;
                luminance(
                    *row.get(offset).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA row is shorter than expected".into())
                    })?,
                    *row.get(offset + 1).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA row is shorter than expected".into())
                    })?,
                    *row.get(offset + 2).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA row is shorter than expected".into())
                    })?,
                )
            }
            _ => {
                return Err(ConvertError::PngError(format!(
                    "Unsupported PNG color type: {:?}",
                    color_type
                )))
            }
        };
        output.extend_from_slice(&[v, v, v, 255]);
    }

    Ok(())
}

fn sample_png_frame_row_to_rgba<I>(
    frame: &[u8],
    color_type: png::ColorType,
    src_width: u32,
    source_y: u32,
    source_cols: I,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError>
where
    I: IntoIterator<Item = u32>,
{
    let source_row = (source_y as usize) * (src_width as usize);
    for source_x in source_cols {
        let source_index = source_row + source_x as usize;
        let v = match color_type {
            png::ColorType::Grayscale => *frame.get(source_index).ok_or_else(|| {
                ConvertError::PngError("PNG grayscale frame is shorter than expected".into())
            })?,
            png::ColorType::GrayscaleAlpha => *frame.get(source_index * 2).ok_or_else(|| {
                ConvertError::PngError("PNG grayscale-alpha frame is shorter than expected".into())
            })?,
            png::ColorType::Rgb => {
                let offset = source_index * 3;
                luminance(
                    *frame.get(offset).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB frame is shorter than expected".into())
                    })?,
                    *frame.get(offset + 1).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB frame is shorter than expected".into())
                    })?,
                    *frame.get(offset + 2).ok_or_else(|| {
                        ConvertError::PngError("PNG RGB frame is shorter than expected".into())
                    })?,
                )
            }
            png::ColorType::Rgba => {
                let offset = source_index * 4;
                luminance(
                    *frame.get(offset).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA frame is shorter than expected".into())
                    })?,
                    *frame.get(offset + 1).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA frame is shorter than expected".into())
                    })?,
                    *frame.get(offset + 2).ok_or_else(|| {
                        ConvertError::PngError("PNG RGBA frame is shorter than expected".into())
                    })?,
                )
            }
            _ => {
                return Err(ConvertError::PngError(format!(
                    "Unsupported PNG color type: {:?}",
                    color_type
                )))
            }
        };
        output.extend_from_slice(&[v, v, v, 255]);
    }

    Ok(())
}

fn luminance(r: u8, g: u8, b: u8) -> u8 {
    ((77 * r as u32 + 150 * g as u32 + 29 * b as u32) >> 8) as u8
}
