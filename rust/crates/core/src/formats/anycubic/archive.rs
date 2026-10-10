mod calc_volumes;
mod codec;
mod layout;
mod manifest;
mod pwsz;
mod scene;
mod zip_io;

use std::io::Cursor;

use zip::ZipArchive;

use crate::formats::common::ExportTargetOptions;
use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::{
        anycubic::specifics::AnycubicArchiveSpec,
        common::{
            collect_zip_entries, contains_archive_markers, has_zip_magic, read_optional_zip_bytes,
            read_optional_zip_text, read_required_zip_text, ArchiveLayout, FormatSpec,
            HeaderLayout,
        },
    },
};

use calc_volumes::parse_calc_layer_volumes;
use manifest::{
    metadata_from_manifests, AnycubicZipLayersController, AnycubicZipPrintInfo,
    AnycubicZipSettingsManifest,
};
use scene::parse_scene_manifest;
use zip_io::{parse_json, read_thumbnails};

pub use codec::AnycubicZipLayerCodec;
pub use layout::AnycubicZipArchiveLayout;

pub struct AnycubicZipHeaderLayout {
    pub specifics: &'static AnycubicArchiveSpec,
}

fn has_required_manifest_markers(data: &[u8], specifics: &AnycubicArchiveSpec) -> bool {
    contains_archive_markers(data, specifics.required_manifest_entries.iter().copied())
}

impl HeaderLayout for AnycubicZipHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        has_zip_magic(data) && has_required_manifest_markers(data, self.specifics)
    }

    fn parse_header(
        &self,
        data: &[u8],
        spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        let specifics = self.specifics;
        let cursor = Cursor::new(data);
        let mut archive =
            ZipArchive::new(cursor).map_err(|error| ConvertError::ZipError(error.to_string()))?;

        let settings = read_required_zip_text(&mut archive, specifics.settings_manifest_entry)?;
        let settings = parse_json::<AnycubicZipSettingsManifest>(
            specifics.settings_manifest_entry,
            &settings,
        )?;
        let layers = read_required_zip_text(&mut archive, specifics.layers_manifest_entry)?;
        let layers =
            parse_json::<AnycubicZipLayersController>(specifics.layers_manifest_entry, &layers)?;
        if layers.layers.is_empty() {
            return Err(ConvertError::ZipError(format!(
                "{} contains no layer entries",
                specifics.layers_manifest_entry
            )));
        }
        let print_info = read_optional_zip_text(&mut archive, "print_info.json")?
            .map(|content| parse_json::<AnycubicZipPrintInfo>("print_info.json", &content))
            .transpose()?;
        let scene = read_optional_zip_bytes(&mut archive, "scene.slice")?
            .map(|content| parse_scene_manifest(&content))
            .transpose()?;
        let calc_volumes = read_optional_zip_bytes(&mut archive, "calc_layer_volumes.data")?
            .map(|content| parse_calc_layer_volumes(&content))
            .transpose()?;

        let entries = collect_zip_entries(&mut archive)?;
        let thumbnails = read_thumbnails(&mut archive, &entries)?;
        let layout = AnycubicZipArchiveLayout {
            specifics: self.specifics,
        };
        let layer_offsets = layout.list_layer_entries(&entries, spec)?;
        let mut metadata = metadata_from_manifests(
            &settings,
            &layers,
            print_info.as_ref(),
            scene.as_ref(),
            calc_volumes,
        );
        metadata.thumbnails = thumbnails;
        if metadata.slicing.layer_count == 0 {
            metadata.slicing.layer_count = layer_offsets.len() as u32;
        }

        Ok((metadata, layer_offsets))
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(Vec::new())
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
