use std::io::Cursor;

use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crate::{
    common::{ConvertError, Thumbnail},
    formats::common::{decode_png_thumbnail, read_zip_entry_bytes, ZipEntryInfo},
};

pub(super) fn parse_json<T: DeserializeOwned>(
    name: &str,
    content: &str,
) -> Result<T, ConvertError> {
    serde_json::from_str(content)
        .map_err(|error| ConvertError::IniError(format!("{name} parse error: {error}")))
}

fn read_thumbnail(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entry_index: usize,
) -> Result<Thumbnail, ConvertError> {
    let bytes = read_zip_entry_bytes(archive, entry_index)?;
    decode_png_thumbnail(&bytes)
}

pub(super) fn read_thumbnails(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &[ZipEntryInfo],
) -> Result<Vec<Thumbnail>, ConvertError> {
    let mut preview_entries = entries
        .iter()
        .filter(|entry| {
            let name = entry.name.to_ascii_lowercase();
            name.starts_with("preview_images/preview_") && name.ends_with(".png")
        })
        .collect::<Vec<_>>();
    preview_entries.sort_by(|a, b| a.name.cmp(&b.name));

    preview_entries
        .into_iter()
        .map(|entry| read_thumbnail(archive, entry.index))
        .collect()
}
