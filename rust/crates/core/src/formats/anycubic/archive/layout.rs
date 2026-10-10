use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::anycubic::specifics::AnycubicArchiveSpec,
    formats::common::{
        archive_entry_descriptor, bytes_archive_entry, encode_png_thumbnail, json_archive_entry,
        sorted_layer_offsets, ArchiveEncodeContext, ArchiveEntry, ArchiveEntryDescriptor,
        ArchiveLayout, FormatSpec, ZipEntryInfo,
    },
};

use super::{
    calc_volumes::build_calc_layer_volumes,
    manifest::{
        build_layers_json, build_lcd_function_json, build_print_info_json, build_settings_json,
        build_software_info_json,
    },
    scene::build_scene_manifest,
};

pub struct AnycubicZipArchiveLayout {
    pub specifics: &'static AnycubicArchiveSpec,
}

fn layer_index(name: &str, specifics: &AnycubicArchiveSpec) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let prefix = specifics.layer_entry_prefix.to_ascii_lowercase();
    let name = lower.strip_prefix(&prefix)?;
    let (index, extension) = name.split_once('.')?;
    if !specifics
        .layer_image_extensions
        .iter()
        .any(|supported| extension.eq_ignore_ascii_case(supported))
    {
        return None;
    }
    index
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| index.parse().ok())
        .flatten()
}

impl ArchiveLayout for AnycubicZipArchiveLayout {
    fn list_layer_entries(
        &self,
        entries: &[ZipEntryInfo],
        _spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError> {
        Ok(sorted_layer_offsets(entries, |name| {
            layer_index(name, self.specifics)
        }))
    }

    fn build_metadata_entries(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        metadata: &FileMetadata,
        total_layers: u32,
        _spec: &FormatSpec,
    ) -> Result<Vec<ArchiveEntry>, ConvertError> {
        let mut entries = Vec::new();

        for (index, thumbnail) in metadata.thumbnails.iter().enumerate() {
            entries.push(bytes_archive_entry(
                format!("preview_images/preview_{index}.png"),
                encode_png_thumbnail(thumbnail)?,
            ));
        }

        entries.push(json_archive_entry(
            self.specifics.settings_manifest_entry,
            build_settings_json(metadata),
        )?);
        entries.push(json_archive_entry(
            self.specifics.layers_manifest_entry,
            build_layers_json(metadata, total_layers),
        )?);
        entries.push(json_archive_entry(
            "lcd_function.json",
            build_lcd_function_json(),
        )?);
        entries.push(json_archive_entry(
            "print_info.json",
            build_print_info_json(metadata, total_layers),
        )?);
        entries.push(json_archive_entry(
            "software_info.conf",
            build_software_info_json(),
        )?);
        entries.push(bytes_archive_entry(
            "calc_layer_volumes.data",
            build_calc_layer_volumes(metadata, total_layers),
        ));
        entries.push(bytes_archive_entry(
            "scene.slice",
            build_scene_manifest(metadata, total_layers),
        ));

        Ok(entries)
    }

    fn describe_layer_entry(
        &self,
        _ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        _spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor {
        archive_entry_descriptor(format!(
            "{}{layer_index}.{}",
            self.specifics.layer_entry_prefix, self.specifics.default_layer_image_extension
        ))
    }
}
