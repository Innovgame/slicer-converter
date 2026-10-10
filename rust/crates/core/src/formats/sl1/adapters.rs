use crate::{
    common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage},
    formats::{
        common::{
            archive_entry_descriptor, contains_archive_marker, has_zip_magic, sorted_layer_offsets,
            text_archive_entry, ArchiveEncodeContext, ArchiveEntry, ArchiveEntryDescriptor,
            ArchiveLayout, ExportTargetOptions, FormatSpec, HeaderLayout, LayerCodec, ZipEntryInfo,
        },
        LayerDecodeContext, LayerEncodeContext,
    },
};

use super::{config, decoder, encoder};

/// Header layout adapter for SL1 ZIP archives.
///
/// Detection requires both `config.ini` and `prusaslicer.ini`, matching the
/// metadata pair this writer emits.
///
/// SL1 ZIP archive 的 header layout adapter。检测要求同时存在 `config.ini` 与
/// `prusaslicer.ini`，与 writer 输出的 metadata pair 保持一致。
pub struct Sl1ArchiveHeaderLayout;

/// Archive layout adapter for SL1 metadata and layer entries.
///
/// It writes INI metadata first and names layer PNG entries as
/// `{job_name}_{layer_index:05}.png`.
///
/// SL1 metadata 与 layer entries 的 archive layout adapter。它先写入 INI metadata，
/// 并将 layer PNG entry 命名为 `{job_name}_{layer_index:05}.png`。
pub struct Sl1ArchiveLayout;

/// Shared PNG grayscale layer codec used by SL1 and VDT archive profiles.
///
/// SL1 与 VDT archive profiles 共享的 PNG grayscale layer codec。
pub struct PngLayerCodec;

impl HeaderLayout for Sl1ArchiveHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        has_zip_magic(data)
            && contains_archive_marker(data, b"config.ini")
            && contains_archive_marker(data, b"prusaslicer.ini")
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        decoder::Sl1Decoder::parse_header(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(config::build_config_ini(metadata).into_bytes())
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

impl ArchiveLayout for Sl1ArchiveLayout {
    fn list_layer_entries(
        &self,
        entries: &[ZipEntryInfo],
        _spec: &FormatSpec,
    ) -> Result<Vec<LayerOffset>, ConvertError> {
        Ok(sorted_layer_offsets(entries, decoder::sl1_layer_index))
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

        Ok(vec![
            text_archive_entry("config.ini", config::build_config_ini(&metadata)),
            text_archive_entry("prusaslicer.ini", config::build_prusaslicer_ini(&metadata)),
        ])
    }

    fn describe_layer_entry(
        &self,
        ctx: &ArchiveEncodeContext<'_>,
        layer_index: u32,
        _spec: &FormatSpec,
    ) -> ArchiveEntryDescriptor {
        archive_entry_descriptor(format!("{}_{:05}.png", ctx.job_name, layer_index))
    }
}

impl LayerCodec for PngLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        decoder::decode_png_layer_into(data, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        encoder::encode_png_layer_into(pixels, ctx.width, ctx.height, output)
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        decoder::decode_png_layer_preview_rgba(data, ctx.width, ctx.height, max_preview_size)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::downsample_to_rgba_nearest,
        formats::{sl1::encoder, LayerOptions},
    };

    #[test]
    fn png_layer_codec_decodes_bounded_preview_rgba() {
        let width = 8u32;
        let height = 4u32;
        let max_preview_size = 4u32;
        let pixels: Vec<u8> = (0..width * height).map(|index| (index * 7) as u8).collect();
        let encoded = encoder::encode_png_layer(&pixels, width, height).expect("encode PNG layer");
        let ctx = LayerDecodeContext::new(0, width, height).with_options(LayerOptions::default());

        let preview = PngLayerCodec
            .decode_preview_rgba(&encoded, &ctx, max_preview_size)
            .expect("decode preview")
            .expect("PNG codec should provide preview fast path");

        let expected = downsample_to_rgba_nearest(&pixels, width, height, max_preview_size);
        assert_eq!(preview.width, expected.width);
        assert_eq!(preview.height, expected.height);
        assert_eq!(preview.pixels, expected.pixels);
    }

    #[test]
    fn sl1_archive_layer_names_separate_numeric_job_names_from_layer_indices() {
        let ctx = ArchiveEncodeContext::new("job_202604141902");

        let entry =
            Sl1ArchiveLayout.describe_layer_entry(&ctx, 1379, &crate::formats::sl1::SL1_SPEC);

        assert_eq!(entry.name, "job_202604141902_01379.png");
    }
}
