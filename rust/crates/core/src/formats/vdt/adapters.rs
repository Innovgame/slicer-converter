use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::common::{
        archive_entry_descriptor, bytes_archive_entry, contains_archive_marker, has_zip_magic,
        sorted_layer_offsets, ArchiveEncodeContext, ArchiveEntry, ArchiveEntryDescriptor,
        ArchiveLayout, ExportTargetOptions, FormatSpec, HeaderLayout, ZipEntryInfo,
    },
};

use super::{
    build_header_bytes,
    entries::{layer_entry_index, layer_entry_name},
    manifest::VdtManifest,
    parse_header_data,
};

/// Header layout adapter for VDT ZIP archives.
///
/// VDT detection is scoped to ZIP files containing `manifest.json`.
///
/// VDT ZIP archive 的 header layout adapter。VDT detection 限定为包含 `manifest.json`
/// 的 ZIP 文件。
pub struct VdtArchiveHeaderLayout;

/// Archive layout adapter for VDT manifest and layer entries.
///
/// VDT layer entry naming is centralized in `entries` so parsing and writing
/// stay symmetric.
///
/// VDT manifest 与 layer entries 的 archive layout adapter。VDT layer entry naming
/// 集中在 `entries` 中，确保 parse 与 write 对称。
pub struct VdtArchiveLayout;

impl HeaderLayout for VdtArchiveHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        has_zip_magic(data) && contains_archive_marker(data, b"manifest.json")
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_header_data(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        build_header_bytes(metadata)
    }

    fn finalize_header(
        &self,
        header: &[u8],
        _layer_sizes: &[u32],
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(header.to_vec())
    }
}

impl ArchiveLayout for VdtArchiveLayout {
    fn list_layer_entries(
        &self,
        entries: &[ZipEntryInfo],
        _spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError> {
        Ok(sorted_layer_offsets(entries, layer_entry_index))
    }

    fn build_metadata_entries(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        metadata: &FileMetadata,
        total_layers: u32,
        _spec: &FormatSpec,
    ) -> Result<Vec<ArchiveEntry>, ConvertError> {
        let mut metadata = metadata.clone();
        metadata.slicing.layer_count = total_layers;
        let manifest = VdtManifest::from_metadata_with_layers(&metadata, total_layers);
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| ConvertError::ZipError(error.to_string()))?;

        Ok(vec![bytes_archive_entry("manifest.json", manifest_bytes)])
    }

    fn describe_layer_entry(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        _spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor {
        archive_entry_descriptor(layer_entry_name(layer_index))
    }
}
