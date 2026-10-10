use std::io::Cursor;

use zip::ZipArchive;

use super::{entries::layer_entry_index, manifest::VdtManifest};
use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::common::{collect_zip_entries, read_required_zip_text, sorted_layer_offsets},
};

/// Parse a VDT ZIP archive header.
///
/// The parser reads `manifest.json`, normalizes it into shared metadata, and
/// enumerates PNG layer entries. As with other ZIP-backed formats, layer offsets
/// are logical ZIP entry indices instead of byte offsets.
///
/// 解析 VDT ZIP archive header：读取 `manifest.json`，归一为共享 metadata，并枚举 PNG
/// layer entries。与其他 ZIP-backed formats 一样，layer offsets 是逻辑 ZIP entry indices，
/// 不是源文件 byte offsets。
pub fn parse_header(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| ConvertError::ZipError(e.to_string()))?;

    let manifest_text = read_required_zip_text(&mut archive, "manifest.json")?;
    let manifest: VdtManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| ConvertError::IniError(format!("manifest.json parse error: {e}")))?;
    let mut metadata = manifest.to_metadata();

    let entries = collect_zip_entries(&mut archive)?;
    let layer_offsets = sorted_layer_offsets(&entries, layer_entry_index);

    // Update layer count
    if metadata.slicing.layer_count == 0
        || metadata.slicing.layer_count != layer_offsets.len() as u32
    {
        metadata.slicing.layer_count = layer_offsets.len() as u32;
    }

    Ok((metadata, layer_offsets))
}
