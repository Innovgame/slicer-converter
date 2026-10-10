use std::io::Cursor;

use zip::ZipArchive;

use crate::{
    common::{ConvertError, Thumbnail},
    formats::common::{decode_png_thumbnail, read_optional_zip_bytes},
};

fn read_optional_thumbnail(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<Thumbnail>, ConvertError> {
    read_optional_zip_bytes(archive, name)?
        .map(|bytes| decode_png_thumbnail(&bytes))
        .transpose()
}

pub(super) fn read_thumbnails(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<Vec<Thumbnail>, ConvertError> {
    let mut thumbnails = Vec::new();
    for name in ["preview.png", "preview_cropping.png"] {
        if let Some(thumbnail) = read_optional_thumbnail(archive, name)? {
            thumbnails.push(thumbnail);
        }
    }
    Ok(thumbnails)
}
