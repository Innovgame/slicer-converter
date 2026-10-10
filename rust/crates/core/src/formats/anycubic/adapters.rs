use crate::{
    common::{ConvertError, FileMetadata, LayerOffset, PreviewLayerImage},
    formats::{
        anycubic::{
            layouts::{V1Layout, V515Layout, V516Layout, V517Layout, V518Layout},
            AnycubicLayoutStrategy, AnycubicSpec, AnycubicVersion,
        },
        common::{ExportTargetOptions, FormatSpec, HeaderLayout, LayerCodec},
        LayerDecodeContext, LayerEncodeContext,
    },
};

pub struct AnycubicHeaderLayoutAdapter {
    pub specifics: &'static AnycubicSpec,
}

pub struct AnycubicLayerCodecAdapter {
    pub specifics: &'static AnycubicSpec,
}

impl HeaderLayout for AnycubicHeaderLayoutAdapter {
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool {
        if let Some(hint) = hint {
            let hint = hint.trim_start_matches('.');
            if !hint.eq_ignore_ascii_case(self.specifics.extension) {
                return false;
            }
        }
        self.specifics.layout.parse_header(data).is_ok()
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        self.specifics.layout.parse_header(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        requested_layout(self.specifics, options)?.build_header(
            metadata,
            self.specifics,
            self.specifics.codec,
        )
    }

    fn finalize_header(
        &self,
        header: &[u8],
        layer_sizes: &[u32],
        _spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        requested_layout(self.specifics, options)?.finalize_header(header.to_vec(), layer_sizes)
    }

    fn finalize_header_owned(
        &self,
        header: Vec<u8>,
        layer_sizes: &[u32],
        _spec: &FormatSpec,
        options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        requested_layout(self.specifics, options)?.finalize_header(header, layer_sizes)
    }
}

fn requested_layout(
    specifics: &'static AnycubicSpec,
    options: ExportTargetOptions<'_>,
) -> Result<&'static dyn AnycubicLayoutStrategy, ConvertError> {
    let Some(version) = options.format_version else {
        return Ok(specifics.layout);
    };
    let Some(version) = AnycubicVersion::from_u32(version) else {
        return Err(unsupported_anycubic_version(specifics, version));
    };
    if !specifics.supports_version(version) {
        return Err(unsupported_anycubic_version(specifics, version.as_u32()));
    }
    Ok(match version {
        AnycubicVersion::V1 => &V1Layout,
        AnycubicVersion::V515 => &V515Layout,
        AnycubicVersion::V516 => &V516Layout,
        AnycubicVersion::V517 => &V517Layout,
        AnycubicVersion::V518 => &V518Layout,
    })
}

fn unsupported_anycubic_version(specifics: &AnycubicSpec, version: u32) -> ConvertError {
    let supported = specifics
        .supported_versions
        .iter()
        .map(|version| version.label())
        .collect::<Vec<_>>()
        .join(", ");
    ConvertError::UnsupportedFormatVersion {
        target: specifics.extension.to_string(),
        version,
        supported,
    }
}

impl LayerCodec for AnycubicLayerCodecAdapter {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let _ = ctx.layer_index;
        let _ = ctx.options.anti_alias_level;
        self.specifics.codec.decode_layer_into(data, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let _ = ctx.layer_index;
        let _ = (ctx.width, ctx.height, ctx.layer_metadata);
        self.specifics.codec.encode_layer_into(pixels, output)
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        self.specifics
            .codec
            .decode_layer_preview_rgba(data, ctx.width, ctx.height, max_preview_size)
            .map(Some)
    }
}
