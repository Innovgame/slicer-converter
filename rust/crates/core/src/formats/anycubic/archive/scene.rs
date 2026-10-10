use crate::{
    common::{ConvertError, FileMetadata},
    formats::common::binary::{BinaryReader, BinaryWriter},
};

use super::manifest::{metadata_layer_position_z, positive_or, round_positive_4};

/// Parsed subset of Anycubic ZIP `scene.slice`.
///
/// The binary scene file carries model and layer geometry. Import currently only
/// needs per-layer heights, which are used to refine `PerLayerSettings`.
///
/// Anycubic ZIP `scene.slice` 的已解析字段子集。该二进制 scene 文件携带模型与层几何信息；
/// 当前导入只需要 per-layer heights，用于修正 `PerLayerSettings`。
#[derive(Debug, Clone, Default)]
pub(super) struct AnycubicZipSceneManifest {
    pub(super) layer_defs: Vec<AnycubicZipSceneLayerDef>,
}

/// One layer geometry entry from `scene.slice`.
///
/// `scene.slice` 中的单层几何条目。
#[derive(Debug, Clone, Default)]
pub(super) struct AnycubicZipSceneLayerDef {
    pub(super) height: f32,
}

fn zip_binary_parse_error(entry: &str, offset: usize, detail: impl Into<String>) -> ConvertError {
    ConvertError::ZipError(format!(
        "{entry} parse error at byte {offset}: {}",
        detail.into()
    ))
}

fn read_manifest_u32(reader: &mut BinaryReader<'_>, entry: &str) -> Result<u32, ConvertError> {
    let offset = reader.position();
    reader
        .read_le_u32()
        .map_err(|_| zip_binary_parse_error(entry, offset, "truncated u32"))
}

fn read_manifest_f32(reader: &mut BinaryReader<'_>, entry: &str) -> Result<f32, ConvertError> {
    let offset = reader.position();
    reader
        .read_le_f32()
        .map_err(|_| zip_binary_parse_error(entry, offset, "truncated f32"))
}

fn skip_manifest_bytes(
    reader: &mut BinaryReader<'_>,
    count: usize,
    entry: &str,
) -> Result<(), ConvertError> {
    let offset = reader.position();
    reader
        .skip(count)
        .map_err(|_| zip_binary_parse_error(entry, offset, "truncated bytes"))
}

fn read_fixed_marker(
    reader: &mut BinaryReader<'_>,
    marker: &[u8],
    entry: &str,
) -> Result<(), ConvertError> {
    let offset = reader.position();
    let value = reader
        .read_range(marker.len())
        .map_err(|_| zip_binary_parse_error(entry, offset, "truncated marker"))?;
    if value != marker {
        return Err(zip_binary_parse_error(entry, offset, "invalid marker"));
    }
    Ok(())
}

/// Parse Anycubic ZIP `scene.slice` binary metadata.
///
/// The parser validates the `<---` / `--->` layer-definition markers and reads
/// each layer height while skipping geometry fields that are not yet needed by
/// metadata import.
///
/// 解析 Anycubic ZIP `scene.slice` 二进制 metadata。parser 校验 `<---` / `--->`
/// layer-definition markers，并读取每层 height；暂未需要的 geometry 字段会被跳过。
pub(super) fn parse_scene_manifest(data: &[u8]) -> Result<AnycubicZipSceneManifest, ConvertError> {
    const ENTRY: &str = "scene.slice";
    let mut reader = BinaryReader::new(data);
    skip_manifest_bytes(&mut reader, 16, ENTRY)?;
    skip_manifest_bytes(&mut reader, 64, ENTRY)?;
    for _ in 0..4 {
        let _ = read_manifest_u32(&mut reader, ENTRY)?;
    }
    let _point_ratio = read_manifest_f32(&mut reader, ENTRY)?;
    let _layer_count = read_manifest_u32(&mut reader, ENTRY)?;
    for _ in 0..6 {
        let _ = read_manifest_f32(&mut reader, ENTRY)?;
    }
    let _model_stats = read_manifest_u32(&mut reader, ENTRY)?;
    skip_manifest_bytes(&mut reader, 64 * 4, ENTRY)?;
    read_fixed_marker(&mut reader, b"<---", ENTRY)?;
    let layer_def_count = read_manifest_u32(&mut reader, ENTRY)?;
    let mut layer_defs = Vec::with_capacity(layer_def_count as usize);
    for _ in 0..layer_def_count {
        let height = read_manifest_f32(&mut reader, ENTRY)?;
        for _ in 0..5 {
            let _ = read_manifest_f32(&mut reader, ENTRY)?;
        }
        let _object_count = read_manifest_u32(&mut reader, ENTRY)?;
        let _max_contour_area = read_manifest_f32(&mut reader, ENTRY)?;
        skip_manifest_bytes(&mut reader, 8 * 4, ENTRY)?;
        layer_defs.push(AnycubicZipSceneLayerDef { height });
    }
    read_fixed_marker(&mut reader, b"--->", ENTRY)?;

    Ok(AnycubicZipSceneManifest { layer_defs })
}

fn write_fixed_string(bytes: &mut Vec<u8>, value: &str, len: usize) {
    let raw = value.as_bytes();
    let copy_len = raw.len().min(len.saturating_sub(1));
    bytes.extend_from_slice(&raw[..copy_len]);
    bytes.resize(bytes.len() + len - copy_len, 0);
}

/// Append a little-endian `u32` to a binary manifest buffer.
///
/// 向二进制 manifest buffer 追加 little-endian `u32`。
pub(super) fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    BinaryWriter::new(bytes).write_le_u32(value);
}

/// Append a little-endian `f32` to a binary manifest buffer.
///
/// 向二进制 manifest buffer 追加 little-endian `f32`。
pub(super) fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    BinaryWriter::new(bytes).write_le_f32(value);
}

/// Build Anycubic ZIP `scene.slice` binary metadata.
///
/// The generated scene contains a minimal model envelope and one layer
/// definition per layer. It is intentionally conservative but includes the layer
/// heights needed for import roundtrips.
///
/// 构建 Anycubic ZIP `scene.slice` 二进制 metadata。生成内容包含最小 model envelope 与每层
/// layer definition；它保持保守，但包含 import roundtrip 所需的 layer heights。
pub(super) fn build_scene_manifest(metadata: &FileMetadata, total_layers: u32) -> Vec<u8> {
    let half_width = positive_or(metadata.printer.display_width_mm, 1.0) / 2.0;
    let half_height = positive_or(metadata.printer.display_height_mm, 1.0) / 2.0;
    let z_max = metadata_layer_position_z(metadata, total_layers.saturating_sub(1));
    let mut bytes = Vec::new();

    write_fixed_string(&mut bytes, "ANYCUBIC-PWSZ", 16);
    write_fixed_string(&mut bytes, "slicer-converter", 64);
    for value in [3u32, 1, 0, 0] {
        push_u32(&mut bytes, value);
    }
    push_f32(&mut bytes, 1.0);
    push_u32(&mut bytes, total_layers);
    for value in [
        -half_width,
        -half_height,
        0.0,
        half_width,
        half_height,
        z_max,
    ] {
        push_f32(&mut bytes, round_positive_4(value));
    }
    push_u32(&mut bytes, 0);
    for _ in 0..64 {
        push_u32(&mut bytes, 0);
    }
    bytes.extend_from_slice(b"<---");
    push_u32(&mut bytes, total_layers);
    for layer_index in 0..total_layers {
        push_f32(
            &mut bytes,
            round_positive_4(metadata_layer_position_z(metadata, layer_index)),
        );
        for value in [0.0f32, -half_width, -half_height, half_width, half_height] {
            push_f32(&mut bytes, round_positive_4(value));
        }
        push_u32(&mut bytes, 0);
        push_f32(&mut bytes, 0.0);
        for _ in 0..8 {
            push_u32(&mut bytes, 0);
        }
    }
    bytes.extend_from_slice(b"--->");
    bytes
}

pub(super) fn read_manifest_u32_for_entry(
    reader: &mut BinaryReader<'_>,
    entry: &str,
) -> Result<u32, ConvertError> {
    read_manifest_u32(reader, entry)
}

pub(super) fn read_manifest_f32_for_entry(
    reader: &mut BinaryReader<'_>,
    entry: &str,
) -> Result<f32, ConvertError> {
    read_manifest_f32(reader, entry)
}

pub(super) fn skip_manifest_bytes_for_entry(
    reader: &mut BinaryReader<'_>,
    count: usize,
    entry: &str,
) -> Result<(), ConvertError> {
    skip_manifest_bytes(reader, count, entry)
}

pub(super) fn zip_binary_parse_error_for_entry(
    entry: &str,
    offset: usize,
    detail: impl Into<String>,
) -> ConvertError {
    zip_binary_parse_error(entry, offset, detail)
}
