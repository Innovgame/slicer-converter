use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::common::{
        archive_entry_descriptor, bytes_archive_entry, encode_png_thumbnail, sorted_layer_offsets,
        text_archive_entry, ArchiveEncodeContext, ArchiveEntry, ArchiveEntryDescriptor,
        ArchiveLayout, FormatSpec, ZipEntryInfo,
    },
};

use super::gcode::build_gcode_metadata;

pub struct ChituboxZipArchiveLayout;

fn layer_index(name: &str) -> Option<u32> {
    let file_name = name.rsplit('/').next().unwrap_or(name);
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".png") || lower.contains("preview") {
        return None;
    }

    let stem = lower.strip_suffix(".png")?;
    let layer_number = stem
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| stem.parse::<u32>().ok())
        .flatten()?;
    layer_number.checked_sub(1)
}

impl ArchiveLayout for ChituboxZipArchiveLayout {
    fn list_layer_entries(
        &self,
        entries: &[ZipEntryInfo],
        _spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError> {
        Ok(sorted_layer_offsets(entries, layer_index))
    }

    fn build_metadata_entries(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        metadata: &FileMetadata,
        total_layers: u32,
        _spec: &FormatSpec,
    ) -> Result<Vec<ArchiveEntry>, ConvertError> {
        let mut entries = vec![text_archive_entry(
            "run.gcode",
            build_gcode_metadata(metadata, total_layers),
        )];

        for (name, thumbnail) in ["preview.png", "preview_cropping.png"]
            .into_iter()
            .zip(metadata.thumbnails.iter())
        {
            entries.push(bytes_archive_entry(name, encode_png_thumbnail(thumbnail)?));
        }

        Ok(entries)
    }

    fn describe_layer_entry(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        _spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor {
        archive_entry_descriptor(format!("{}.png", layer_index + 1))
    }
}
