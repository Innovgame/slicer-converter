mod gcode;
mod layout;
mod zip_io;

use std::io::Cursor;

use zip::ZipArchive;

use crate::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::common::{
        collect_zip_entries, contains_archive_marker, has_zip_magic, read_required_zip_text,
        ArchiveLayout, ExportTargetOptions, FormatSpec, HeaderLayout,
    },
};

use gcode::build_gcode_metadata;
use gcode::parse_gcode_metadata;
use zip_io::read_thumbnails;

pub use layout::ChituboxZipArchiveLayout;

pub struct ChituboxZipHeaderLayout;

impl HeaderLayout for ChituboxZipHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        has_zip_magic(data) && contains_archive_marker(data, b"run.gcode")
    }

    fn parse_header(
        &self,
        data: &[u8],
        spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        let cursor = Cursor::new(data);
        let mut archive =
            ZipArchive::new(cursor).map_err(|error| ConvertError::ZipError(error.to_string()))?;

        let gcode = read_required_zip_text(&mut archive, "run.gcode")?;
        let mut metadata = parse_gcode_metadata(&gcode);
        metadata.thumbnails = read_thumbnails(&mut archive)?;
        let entries = collect_zip_entries(&mut archive)?;
        let layout = ChituboxZipArchiveLayout;
        let layers = layout.list_layer_entries(&entries, spec)?;
        if metadata.slicing.layer_count == 0 {
            metadata.slicing.layer_count = layers.len() as u32;
        }

        Ok((metadata, layers))
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(build_gcode_metadata(metadata, metadata.slicing.layer_count).into_bytes())
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
