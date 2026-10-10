use std::borrow::Cow;

use crate::{
    common::{
        downsample_to_rgba_nearest, ConvertError, FileMetadata, LayerOffset, PreviewDecodePlan,
        PreviewLayerImage,
    },
    formats::{
        common::{
            binary::{
                read_be_u16, read_be_u32, read_le_f32, read_le_u16, read_le_u32, BinaryWriter,
            },
            ArchiveEncodeContext, DocumentEncodeStats, ExportTargetOptions, FormatPipeline,
            FormatSpec, HeaderLayout, LayerCodec, LayerSource, OutputSink, ParsedDocument,
        },
        LayerDecodeContext, LayerEncodeContext,
    },
};

use super::specifics::CrealityCxdLpSpec;

const MAGIC_SIZE: usize = 9;
const MAGIC_VALUE: &str = "CXSW3DV2";
const HEADER_VALUE_GENERIC: &str = "CXSW3D";
const LEGACY_THUMBNAIL_SIZES: &[(usize, usize)] = &[(116, 116), (290, 290), (290, 290)];
const LEGACY_LAYER_HEADER_SIZE: usize = 8;
const LEGACY_LAYER_LINE_SIZE: usize = 6;
const LAYER_DEF_SIZE: usize = 40;
const PRINT_PARAMETERS_V4_SIZE: usize = 68;
const SLICER_INFO_V4_SIZE: usize = 76;

pub struct CrealityCxdLpLegacyHeaderLayout {
    pub specifics: &'static CrealityCxdLpSpec,
}

pub struct CrealityCxdLpV1HeaderLayout {
    pub specifics: &'static CrealityCxdLpSpec,
}

pub struct CrealityCxdLpV4HeaderLayout {
    pub specifics: &'static CrealityCxdLpSpec,
}

pub struct CrealityCxdLpLegacyLayerCodec {
    pub specifics: &'static CrealityCxdLpSpec,
}

pub struct CrealityCxdLpLayerCodec {
    pub specifics: &'static CrealityCxdLpSpec,
}

pub struct CrealityCxdLpV4Pipeline;

#[derive(Debug, Clone)]
struct HeaderLegacy {
    magic_size: u32,
    magic: String,
    version: u16,
    printer_model: String,
    layer_count: u16,
    resolution_x: u16,
    resolution_y: u16,
}

#[derive(Debug, Clone)]
struct HeaderV1 {
    magic_size: u32,
    magic: String,
    layer_count: u16,
    resolution_x: u16,
    resolution_y: u16,
}

#[derive(Debug, Clone)]
struct SlicerInfoLegacy {
    display_width: f32,
    display_height: f32,
    layer_height: f32,
    exposure_time: f32,
    wait_time_before_cure: f32,
    bottom_exposure_time: f32,
    bottom_layer_count: u16,
    bottom_lift_height: f32,
    bottom_lift_speed: f32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    bottom_light_pwm: u16,
    light_pwm: u16,
}

#[derive(Debug, Clone)]
struct HeaderV4 {
    magic_size: u32,
    magic: String,
    version: u16,
    printer_model: String,
    resolution_x: u16,
    resolution_y: u16,
    bed_size_x: f32,
    bed_size_y: f32,
    bed_size_z: f32,
    layer_height: f32,
    bottom_layer_count: u32,
    layers_definition_offset: u32,
    layer_count: u32,
    print_time: u32,
    projector_type: u32,
    print_parameters_offset: u32,
    print_parameters_size: u32,
    anti_alias_level: u32,
    light_pwm: u16,
    bottom_light_pwm: u16,
    encryption_key: u32,
    slicer_offset: u32,
    slicer_size: u32,
}

#[derive(Debug, Clone)]
struct PrintParametersV4 {
    bottom_lift_height: f32,
    bottom_lift_speed: f32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    volume_ml: f32,
    weight_g: f32,
    cost: f32,
    bottom_light_off_delay: f32,
    light_off_delay: f32,
    exposure_time: f32,
    bottom_exposure_time: f32,
}

#[derive(Debug, Clone)]
struct SlicerInfoV4 {
    lift_height2: f32,
    lift_speed2: f32,
    retract_height2: f32,
    retract_speed2: f32,
    rest_time_after_lift: f32,
    rest_time_after_retract: f32,
    rest_time_before_lift: f32,
    transition_layer_count: u32,
}

impl HeaderLayout for CrealityCxdLpLegacyHeaderLayout {
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool {
        let hint_matches = hint
            .map(|hint| {
                hint.trim_start_matches('.')
                    .eq_ignore_ascii_case(self.specifics.extension)
            })
            .unwrap_or(true);
        hint_matches && looks_like_cxdlp_legacy(data, self.specifics).unwrap_or(false)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_cxdlp_legacy(data, self.specifics)
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }

    fn finalize_header(
        &self,
        _header: &[u8],
        _layer_sizes: &[u32],
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }
}

impl HeaderLayout for CrealityCxdLpV1HeaderLayout {
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool {
        let hint_matches = hint
            .map(|hint| {
                hint.trim_start_matches('.')
                    .eq_ignore_ascii_case(self.specifics.extension)
            })
            .unwrap_or(true);
        hint_matches && looks_like_cxdlp_v1(data, self.specifics).unwrap_or(false)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_cxdlp_v1(data, self.specifics)
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }

    fn finalize_header(
        &self,
        _header: &[u8],
        _layer_sizes: &[u32],
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }
}

impl HeaderLayout for CrealityCxdLpV4HeaderLayout {
    fn try_detect(&self, data: &[u8], hint: Option<&str>) -> bool {
        let hint_matches = hint
            .map(|hint| {
                hint.trim_start_matches('.')
                    .eq_ignore_ascii_case(self.specifics.extension)
            })
            .unwrap_or(true);
        hint_matches && looks_like_cxdlp_v4(data).unwrap_or(false)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_cxdlp_v4(data, self.specifics)
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }

    fn finalize_header(
        &self,
        _header: &[u8],
        _layer_sizes: &[u32],
        spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::BinrwError(format!(
            "{} encoding is not implemented",
            spec.identity.name
        )))
    }
}

impl FormatPipeline for CrealityCxdLpV4Pipeline {
    fn parse_document(
        &self,
        spec: &FormatSpec,
        data: &[u8],
    ) -> Result<ParsedDocument, ConvertError> {
        let (metadata, layers) = spec.header.parse_header(data, spec)?;
        Ok(ParsedDocument { metadata, layers })
    }

    fn encode_document_into(
        &self,
        _spec: &FormatSpec,
        metadata: &FileMetadata,
        layers: LayerSource<'_>,
        sink: &mut dyn OutputSink,
        _ctx: &ArchiveEncodeContext<'_>,
    ) -> Result<DocumentEncodeStats, ConvertError> {
        match layers {
            LayerSource::Encoded(encoded_layers) => {
                let bytes_written = write_cxdlp_v4_document(metadata, encoded_layers, sink)?;

                Ok(DocumentEncodeStats {
                    bytes_written,
                    layers_written: encoded_layers.len(),
                })
            }
            LayerSource::Streaming {
                layer_count,
                stream,
            } => write_cxdlp_v4_streaming_document(metadata, layer_count, stream, sink),
        }
    }
}

impl LayerCodec for CrealityCxdLpLegacyLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        decode_cxdlp_line_layer_into(data, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        _pixels: &[u8],
        _ctx: &LayerEncodeContext<'_>,
        _output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        Err(ConvertError::RleEncodeError(
            "Creality CXDLP line encoding is not implemented".to_string(),
        ))
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        let mut pixels = Vec::new();
        decode_cxdlp_line_layer_into(data, ctx.width, ctx.height, &mut pixels)?;
        Ok(Some(downsample_to_rgba_nearest(
            &pixels,
            ctx.width,
            ctx.height,
            max_preview_size,
        )))
    }
}

impl LayerCodec for CrealityCxdLpLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        if data.len() < self.specifics.layer_parameter_size as usize {
            return Err(ConvertError::BufferTooSmall {
                need: self.specifics.layer_parameter_size as usize,
                got: data.len(),
            });
        }

        let payload_start = self.specifics.layer_parameter_size as usize;
        let mut payload = if ctx.options.encryption_key == 0 {
            Cow::Borrowed(&data[payload_start..])
        } else {
            Cow::Owned(data[payload_start..].to_vec())
        };
        if ctx.options.encryption_key != 0 {
            crypt_layer_rle(
                ctx.options.encryption_key,
                ctx.layer_index,
                payload.to_mut(),
            );
        }
        decode_cxdlp_rle_into(&payload, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        encode_cxdlp_v4_layer_into(pixels, ctx, output)
    }

    fn decode_preview_rgba(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        max_preview_size: u32,
    ) -> Result<Option<PreviewLayerImage>, ConvertError> {
        if data.len() < self.specifics.layer_parameter_size as usize {
            return Err(ConvertError::BufferTooSmall {
                need: self.specifics.layer_parameter_size as usize,
                got: data.len(),
            });
        }

        let payload_start = self.specifics.layer_parameter_size as usize;
        let mut payload = if ctx.options.encryption_key == 0 {
            Cow::Borrowed(&data[payload_start..])
        } else {
            Cow::Owned(data[payload_start..].to_vec())
        };
        if ctx.options.encryption_key != 0 {
            crypt_layer_rle(
                ctx.options.encryption_key,
                ctx.layer_index,
                payload.to_mut(),
            );
        }
        decode_cxdlp_rle_preview_rgba(&payload, ctx.width, ctx.height, max_preview_size).map(Some)
    }
}

fn parse_cxdlp_legacy(
    data: &[u8],
    specifics: &CrealityCxdLpSpec,
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let (header, mut index) = parse_header_legacy(data)?;
    validate_header_legacy(&header, specifics)?;
    validate_legacy_checksum(data, header.version)?;

    skip_legacy_previews(data, &mut index)?;
    let slicer_info = read_slicer_info_legacy(data, &mut index)?;
    skip_bytes(
        data,
        &mut index,
        header.layer_count as usize * 4 + 2,
        "pre layer table",
    )?;
    if header.version >= 3 {
        skip_slicer_info_v3_legacy(data, &mut index)?;
    }
    let layers = read_legacy_layer_offsets(data, index, header.layer_count as usize)?;

    let metadata = FileMetadata::builder()
        .machine_name(header.printer_model)
        .resolution(header.resolution_x as u32, header.resolution_y as u32)
        .display_size_mm(slicer_info.display_width, slicer_info.display_height)
        .layer_height_mm(slicer_info.layer_height)
        .layer_count(header.layer_count as u32)
        .bottom_layer_count(slicer_info.bottom_layer_count as u32)
        .anti_alias_level(1)
        .encryption_key(0)
        .exposure(slicer_info.exposure_time, slicer_info.bottom_exposure_time)
        .light_off_delay(
            slicer_info.wait_time_before_cure,
            slicer_info.wait_time_before_cure,
        )
        .light_pwm(slicer_info.light_pwm, slicer_info.bottom_light_pwm)
        .lift(slicer_info.lift_height, slicer_info.lift_speed)
        .bottom_lift(
            slicer_info.bottom_lift_height,
            slicer_info.bottom_lift_speed,
        )
        .retract(slicer_info.retract_speed)
        .build()?;

    Ok((metadata, layers))
}

fn parse_cxdlp_v1(
    data: &[u8],
    specifics: &CrealityCxdLpSpec,
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let (header, mut index) = parse_header_v1(data)?;
    validate_header_v1(&header, specifics)?;

    skip_legacy_previews(data, &mut index)?;
    let slicer_info = read_slicer_info_legacy(data, &mut index)?;
    skip_bytes(
        data,
        &mut index,
        header.layer_count as usize * 4 + 2,
        "v1 pre layer table",
    )?;
    let layers = read_legacy_layer_offsets(data, index, header.layer_count as usize)?;
    validate_v1_footer(data, &layers, index)?;

    let metadata = FileMetadata::builder()
        .machine_name("Creality CXDLP v1")
        .resolution(header.resolution_x as u32, header.resolution_y as u32)
        .display_size_mm(slicer_info.display_width, slicer_info.display_height)
        .layer_height_mm(slicer_info.layer_height)
        .layer_count(header.layer_count as u32)
        .bottom_layer_count(slicer_info.bottom_layer_count as u32)
        .anti_alias_level(1)
        .encryption_key(0)
        .exposure(slicer_info.exposure_time, slicer_info.bottom_exposure_time)
        .light_off_delay(
            slicer_info.wait_time_before_cure,
            slicer_info.wait_time_before_cure,
        )
        .light_pwm(slicer_info.light_pwm, slicer_info.bottom_light_pwm)
        .lift(slicer_info.lift_height, slicer_info.lift_speed)
        .bottom_lift(
            slicer_info.bottom_lift_height,
            slicer_info.bottom_lift_speed,
        )
        .retract(slicer_info.retract_speed)
        .build()?;

    Ok((metadata, layers))
}

fn parse_cxdlp_v4(
    data: &[u8],
    _specifics: &CrealityCxdLpSpec,
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let header = parse_header_v4(data)?;
    validate_header_v4(&header)?;
    validate_checksum(data)?;

    let print_parameters = read_print_parameters(data, &header)?;
    let slicer_info = read_slicer_info(data, &header)?;
    let layers = read_layer_offsets(data, &header)?;

    let metadata = FileMetadata::builder()
        .machine_name(header.printer_model)
        .resolution(header.resolution_x as u32, header.resolution_y as u32)
        .display_size_mm(header.bed_size_x, header.bed_size_y)
        .machine_z_mm(header.bed_size_z)
        .mirror(header.projector_type != 0, false)
        .layer_height_mm(header.layer_height)
        .layer_count(header.layer_count)
        .bottom_layer_count(header.bottom_layer_count)
        .transition_layer_count(slicer_info.transition_layer_count)
        .anti_alias_level(header.anti_alias_level.max(1))
        .encryption_key(header.encryption_key)
        .exposure(
            print_parameters.exposure_time,
            print_parameters.bottom_exposure_time,
        )
        .light_off_delay(
            print_parameters.light_off_delay,
            print_parameters.bottom_light_off_delay,
        )
        .light_pwm(header.light_pwm, header.bottom_light_pwm)
        .lift(print_parameters.lift_height, print_parameters.lift_speed)
        .lift2(slicer_info.lift_height2, slicer_info.lift_speed2)
        .bottom_lift(
            print_parameters.bottom_lift_height,
            print_parameters.bottom_lift_speed,
        )
        .retract(print_parameters.retract_speed)
        .retract2(slicer_info.retract_height2, slicer_info.retract_speed2)
        .wait_times(
            slicer_info.rest_time_after_retract,
            slicer_info.rest_time_before_lift,
            slicer_info.rest_time_after_lift,
        )
        .statistics(
            header.print_time,
            print_parameters.volume_ml,
            print_parameters.weight_g,
            print_parameters.cost,
        )
        .build()?;

    Ok((metadata, layers))
}

fn parse_header_legacy(data: &[u8]) -> Result<(HeaderLegacy, usize), ConvertError> {
    let mut index = 0;
    let magic_size = read_be_u32_at(data, &mut index, "magic size")?;
    let magic = read_fixed_string_at(data, &mut index, magic_size as usize, "magic")?;
    let version = read_be_u16_at(data, &mut index, "version")?;
    let printer_model = read_be_len_string_at(data, &mut index, "printer model")?;
    let layer_count = read_be_u16_at(data, &mut index, "layer count")?;
    let resolution_x = read_be_u16_at(data, &mut index, "resolution x")?;
    let resolution_y = read_be_u16_at(data, &mut index, "resolution y")?;
    skip_bytes(data, &mut index, 64, "legacy offset padding")?;

    Ok((
        HeaderLegacy {
            magic_size,
            magic,
            version,
            printer_model,
            layer_count,
            resolution_x,
            resolution_y,
        },
        index,
    ))
}

fn parse_header_v1(data: &[u8]) -> Result<(HeaderV1, usize), ConvertError> {
    let mut index = 0;
    let magic_size = read_be_u32_at(data, &mut index, "header size")?;
    let magic = read_fixed_string_at(data, &mut index, magic_size as usize, "header magic")?;
    let layer_count = read_be_u16_at(data, &mut index, "layer count")?;
    let resolution_x = read_be_u16_at(data, &mut index, "resolution x")?;
    let resolution_y = read_be_u16_at(data, &mut index, "resolution y")?;

    Ok((
        HeaderV1 {
            magic_size,
            magic,
            layer_count,
            resolution_x,
            resolution_y,
        },
        index,
    ))
}

fn validate_header_legacy(
    header: &HeaderLegacy,
    specifics: &CrealityCxdLpSpec,
) -> Result<(), ConvertError> {
    if header.magic_size as usize != MAGIC_SIZE
        || header.magic != MAGIC_VALUE
        || !header.magic.starts_with(HEADER_VALUE_GENERIC)
    {
        return Err(ConvertError::InvalidMagic {
            expected: u32::from_be_bytes(*b"CXSW"),
            actual: header.magic.as_bytes().get(0..4).map_or(0, |bytes| {
                u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
            }),
        });
    }
    if u32::from(header.version) != specifics.version {
        return Err(ConvertError::UnsupportedVersion(header.version as u32));
    }
    Ok(())
}

fn validate_header_v1(
    header: &HeaderV1,
    specifics: &CrealityCxdLpSpec,
) -> Result<(), ConvertError> {
    if specifics.version != 1 {
        return Err(ConvertError::UnsupportedVersion(specifics.version));
    }
    if header.magic_size as usize != MAGIC_SIZE
        || header.magic != MAGIC_VALUE
        || !header.magic.starts_with(HEADER_VALUE_GENERIC)
    {
        return Err(ConvertError::InvalidMagic {
            expected: u32::from_be_bytes(*b"CXSW"),
            actual: header.magic.as_bytes().get(0..4).map_or(0, |bytes| {
                u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
            }),
        });
    }
    Ok(())
}

fn validate_legacy_checksum(data: &[u8], version: u16) -> Result<(), ConvertError> {
    if data.len() < 4 {
        return Err(ConvertError::BufferTooSmall {
            need: 4,
            got: data.len(),
        });
    }
    if version >= 3 {
        let expected = read_be_u32(data, data.len() - 4)?;
        let actual = cxdlp_crc32(&data[..data.len() - 4]);
        if expected != actual {
            return Err(ConvertError::BinrwError(format!(
                "CXDLP checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
            )));
        }
        return Ok(());
    }

    let expected = data[data.len() - 1];
    let actual = data[..data.len() - 4]
        .iter()
        .fold(0u8, |checksum, byte| checksum ^ byte);
    if expected != actual {
        return Err(ConvertError::BinrwError(format!(
            "CXDLP legacy checksum mismatch: expected {expected:#04x}, got {actual:#04x}"
        )));
    }
    Ok(())
}

fn validate_v1_footer(
    data: &[u8],
    layers: &[LayerOffset],
    layer_start: usize,
) -> Result<(), ConvertError> {
    let footer_index = layers
        .last()
        .map(|layer| (layer.offset + layer.size) as usize)
        .unwrap_or(layer_start);
    let mut index = footer_index;
    let footer_size = read_be_u32_at(data, &mut index, "v1 footer size")?;
    let footer_magic =
        read_fixed_string_at(data, &mut index, footer_size as usize, "v1 footer magic")?;
    let _unknown = read_be_u32_at(data, &mut index, "v1 footer unknown")?;
    if footer_size as usize != MAGIC_SIZE || footer_magic != MAGIC_VALUE {
        return Err(ConvertError::InvalidMagic {
            expected: u32::from_be_bytes(*b"CXSW"),
            actual: footer_magic.as_bytes().get(0..4).map_or(0, |bytes| {
                u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
            }),
        });
    }
    Ok(())
}

fn skip_legacy_previews(data: &[u8], index: &mut usize) -> Result<(), ConvertError> {
    for (width, height) in LEGACY_THUMBNAIL_SIZES {
        let bytes = width
            .checked_mul(*height)
            .and_then(|area| area.checked_mul(2))
            .and_then(|area| area.checked_add(2))
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
        skip_bytes(data, index, bytes, "legacy preview")?;
    }
    Ok(())
}

fn read_slicer_info_legacy(
    data: &[u8],
    index: &mut usize,
) -> Result<SlicerInfoLegacy, ConvertError> {
    let display_width = read_be_len_utf16_f32_at(data, index, "display width")?;
    let display_height = read_be_len_utf16_f32_at(data, index, "display height")?;
    let layer_height = read_be_len_utf16_f32_at(data, index, "layer height")?;
    let exposure_time = f32::from(read_be_u16_at(data, index, "exposure time")?) / 10.0;
    let wait_time_before_cure = f32::from(read_be_u16_at(data, index, "wait before cure")?);
    let bottom_exposure_time = f32::from(read_be_u16_at(data, index, "bottom exposure time")?);
    let bottom_layer_count = read_be_u16_at(data, index, "bottom layer count")?;
    let bottom_lift_height = f32::from(read_be_u16_at(data, index, "bottom lift height")?);
    let bottom_lift_speed = f32::from(read_be_u16_at(data, index, "bottom lift speed")?);
    let lift_height = f32::from(read_be_u16_at(data, index, "lift height")?);
    let lift_speed = f32::from(read_be_u16_at(data, index, "lift speed")?);
    let retract_speed = f32::from(read_be_u16_at(data, index, "retract speed")?);
    let bottom_light_pwm = read_be_u16_at(data, index, "bottom light pwm")?;
    let light_pwm = read_be_u16_at(data, index, "light pwm")?;

    Ok(SlicerInfoLegacy {
        display_width,
        display_height,
        layer_height,
        exposure_time,
        wait_time_before_cure,
        bottom_exposure_time,
        bottom_layer_count,
        bottom_lift_height,
        bottom_lift_speed,
        lift_height,
        lift_speed,
        retract_speed,
        bottom_light_pwm,
        light_pwm,
    })
}

fn skip_slicer_info_v3_legacy(data: &[u8], index: &mut usize) -> Result<(), ConvertError> {
    let _software_name = read_be_len_string_at(data, index, "software name")?;
    let _material_name = read_be_len_string_at(data, index, "material name")?;
    skip_bytes(data, index, 22, "legacy v3 slicer info")?;
    Ok(())
}

fn read_legacy_layer_offsets(
    data: &[u8],
    mut index: usize,
    layer_count: usize,
) -> Result<Vec<LayerOffset>, ConvertError> {
    let mut layers = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let offset = index as u64;
        let line_count = read_be_u32(data, index + 4)? as usize;
        let size = LEGACY_LAYER_HEADER_SIZE
            .checked_add(line_count.checked_mul(LEGACY_LAYER_LINE_SIZE).ok_or(
                ConvertError::BufferTooSmall {
                    need: usize::MAX,
                    got: data.len(),
                },
            )?)
            .and_then(|size| size.checked_add(2))
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
        let end = index
            .checked_add(size)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
        if end > data.len() {
            return Err(ConvertError::BufferTooSmall {
                need: end,
                got: data.len(),
            });
        }
        layers.push(LayerOffset {
            offset,
            size: size as u64,
        });
        index = end;
    }
    Ok(layers)
}

fn parse_header_v4(data: &[u8]) -> Result<HeaderV4, ConvertError> {
    let mut index = 0;
    let magic_size = read_be_u32_at(data, &mut index, "magic size")?;
    let magic = read_fixed_string_at(data, &mut index, magic_size as usize, "magic")?;
    let version = read_be_u16_at(data, &mut index, "version")?;
    let printer_model_len = read_be_u32_at(data, &mut index, "printer model length")?;
    let printer_model = read_fixed_string_at(
        data,
        &mut index,
        printer_model_len as usize,
        "printer model",
    )?;
    let resolution_x = read_le_u16_at(data, &mut index, "resolution x")?;
    let resolution_y = read_le_u16_at(data, &mut index, "resolution y")?;
    let bed_size_x = read_le_f32_at(data, &mut index, "bed size x")?;
    let bed_size_y = read_le_f32_at(data, &mut index, "bed size y")?;
    let bed_size_z = read_le_f32_at(data, &mut index, "bed size z")?;
    let _print_height = read_le_f32_at(data, &mut index, "print height")?;
    let layer_height = read_le_f32_at(data, &mut index, "layer height")?;
    let bottom_layer_count = read_le_u32_at(data, &mut index, "bottom layer count")?;
    let _preview_small_offset = read_le_u32_at(data, &mut index, "preview small offset")?;
    let layers_definition_offset = read_le_u32_at(data, &mut index, "layers definition offset")?;
    let layer_count = read_le_u32_at(data, &mut index, "layer count")?;
    let _preview_large_offset = read_le_u32_at(data, &mut index, "preview large offset")?;
    let print_time = read_le_u32_at(data, &mut index, "print time")?;
    let projector_type = read_le_u32_at(data, &mut index, "projector type")?;
    let print_parameters_offset = read_le_u32_at(data, &mut index, "print parameters offset")?;
    let print_parameters_size = read_le_u32_at(data, &mut index, "print parameters size")?;
    let anti_alias_level = read_le_u32_at(data, &mut index, "anti alias level")?;
    let light_pwm = read_le_u16_at(data, &mut index, "light pwm")?;
    let bottom_light_pwm = read_le_u16_at(data, &mut index, "bottom light pwm")?;
    let encryption_key = read_le_u32_at(data, &mut index, "encryption key")?;
    let slicer_offset = read_le_u32_at(data, &mut index, "slicer offset")?;
    let slicer_size = read_le_u32_at(data, &mut index, "slicer size")?;

    Ok(HeaderV4 {
        magic_size,
        magic,
        version,
        printer_model,
        resolution_x,
        resolution_y,
        bed_size_x,
        bed_size_y,
        bed_size_z,
        layer_height,
        bottom_layer_count,
        layers_definition_offset,
        layer_count,
        print_time,
        projector_type,
        print_parameters_offset,
        print_parameters_size,
        anti_alias_level,
        light_pwm,
        bottom_light_pwm,
        encryption_key,
        slicer_offset,
        slicer_size,
    })
}

fn validate_header_v4(header: &HeaderV4) -> Result<(), ConvertError> {
    if header.magic_size as usize != MAGIC_SIZE || header.magic != MAGIC_VALUE {
        return Err(ConvertError::InvalidMagic {
            expected: u32::from_be_bytes(*b"CXSW"),
            actual: header.magic.as_bytes().get(0..4).map_or(0, |bytes| {
                u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
            }),
        });
    }
    if !header.magic.starts_with(HEADER_VALUE_GENERIC) {
        return Err(ConvertError::InvalidMagic {
            expected: u32::from_be_bytes(*b"CXSW"),
            actual: 0,
        });
    }
    if header.version != 4 {
        return Err(ConvertError::UnsupportedVersion(header.version as u32));
    }
    Ok(())
}

fn validate_checksum(data: &[u8]) -> Result<(), ConvertError> {
    if data.len() < 4 {
        return Err(ConvertError::BufferTooSmall {
            need: 4,
            got: data.len(),
        });
    }
    let expected = read_be_u32(data, data.len() - 4)?;
    let actual = cxdlp_crc32(&data[..data.len() - 4]);
    if expected != actual {
        return Err(ConvertError::BinrwError(format!(
            "CXDLPv4 checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
        )));
    }
    Ok(())
}

fn read_print_parameters(
    data: &[u8],
    header: &HeaderV4,
) -> Result<PrintParametersV4, ConvertError> {
    let mut index = header.print_parameters_offset as usize;
    let minimum = index.saturating_add(header.print_parameters_size as usize);
    if minimum > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: minimum,
            got: data.len(),
        });
    }

    let bottom_lift_height = read_le_f32_at(data, &mut index, "bottom lift height")?;
    let bottom_lift_speed = read_le_f32_at(data, &mut index, "bottom lift speed")?;
    let lift_height = read_le_f32_at(data, &mut index, "lift height")?;
    let lift_speed = read_le_f32_at(data, &mut index, "lift speed")?;
    let retract_speed = read_le_f32_at(data, &mut index, "retract speed")?;
    let volume_ml = read_le_f32_at(data, &mut index, "volume ml")?;
    let weight_g = read_le_f32_at(data, &mut index, "weight g")?;
    let cost = read_le_f32_at(data, &mut index, "cost")?;
    let bottom_light_off_delay = read_le_f32_at(data, &mut index, "bottom light off delay")?;
    let light_off_delay = read_le_f32_at(data, &mut index, "light off delay")?;
    let _bottom_layer_count = read_le_u32_at(data, &mut index, "bottom layer count")?;
    let exposure_time = read_le_f32_at(data, &mut index, "exposure time")?;
    let bottom_exposure_time = read_le_f32_at(data, &mut index, "bottom exposure time")?;

    Ok(PrintParametersV4 {
        bottom_lift_height,
        bottom_lift_speed,
        lift_height,
        lift_speed,
        retract_speed,
        volume_ml,
        weight_g,
        cost,
        bottom_light_off_delay,
        light_off_delay,
        exposure_time,
        bottom_exposure_time,
    })
}

fn read_slicer_info(data: &[u8], header: &HeaderV4) -> Result<SlicerInfoV4, ConvertError> {
    let mut index = header.slicer_offset as usize;
    let minimum = index.saturating_add(header.slicer_size as usize);
    if minimum > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: minimum,
            got: data.len(),
        });
    }

    let _bottom_lift_height2 = read_le_f32_at(data, &mut index, "bottom lift height 2")?;
    let _bottom_lift_speed2 = read_le_f32_at(data, &mut index, "bottom lift speed 2")?;
    let lift_height2 = read_le_f32_at(data, &mut index, "lift height 2")?;
    let lift_speed2 = read_le_f32_at(data, &mut index, "lift speed 2")?;
    let retract_height2 = read_le_f32_at(data, &mut index, "retract height 2")?;
    let retract_speed2 = read_le_f32_at(data, &mut index, "retract speed 2")?;
    let rest_time_after_lift = read_le_f32_at(data, &mut index, "rest time after lift")?;
    let _per_layer_settings = read_le_u32_at(data, &mut index, "per layer settings")?;
    let _modified_timestamp_minutes =
        read_le_u32_at(data, &mut index, "modified timestamp minutes")?;
    let _anti_alias_level = read_le_u32_at(data, &mut index, "slicer anti alias")?;
    let _software_version = read_le_u32_at(data, &mut index, "software version")?;
    let rest_time_after_retract = read_le_f32_at(data, &mut index, "rest time after retract")?;
    let rest_time_before_lift = read_le_f32_at(data, &mut index, "rest time before lift")?;
    let _exposure_time = read_le_f32_at(data, &mut index, "slicer exposure time")?;
    let _bottom_exposure_time = read_le_f32_at(data, &mut index, "slicer bottom exposure time")?;
    let _rest_time_after_lift2 = read_le_f32_at(data, &mut index, "rest time after lift 2")?;
    let transition_layer_count = read_le_u32_at(data, &mut index, "transition layer count")?;
    let _padding1 = read_le_u32_at(data, &mut index, "slicer padding 1")?;
    let _padding2 = read_le_u32_at(data, &mut index, "slicer padding 2")?;

    Ok(SlicerInfoV4 {
        lift_height2,
        lift_speed2,
        retract_height2,
        retract_speed2,
        rest_time_after_lift,
        rest_time_after_retract,
        rest_time_before_lift,
        transition_layer_count,
    })
}

fn read_layer_offsets(data: &[u8], header: &HeaderV4) -> Result<Vec<LayerOffset>, ConvertError> {
    let table_start = header.layers_definition_offset as usize;
    let table_len = (header.layer_count as usize)
        .checked_mul(LAYER_DEF_SIZE)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: data.len(),
        })?;
    let table_end = table_start
        .checked_add(table_len)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: data.len(),
        })?;
    if table_end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: table_end,
            got: data.len(),
        });
    }

    let mut layers = Vec::with_capacity(header.layer_count as usize);
    for layer_index in 0..header.layer_count as usize {
        let base = table_start + layer_index * LAYER_DEF_SIZE;
        let offset = read_le_u32(data, base + 12)? as u64;
        let size = read_le_u32(data, base + 16)? as u64;
        let end = offset
            .checked_add(size)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
        if end > data.len() as u64 {
            return Err(ConvertError::BufferTooSmall {
                need: end as usize,
                got: data.len(),
            });
        }
        layers.push(LayerOffset { offset, size });
    }
    Ok(layers)
}

fn write_cxdlp_v4_document(
    metadata: &FileMetadata,
    layers: &[Vec<u8>],
    sink: &mut dyn OutputSink,
) -> Result<usize, ConvertError> {
    let layer_sizes = layers
        .iter()
        .map(|layer| checked_u32(layer.len(), "CXDLPv4 layer data size"))
        .collect::<Result<Vec<_>, _>>()?;
    let header = build_cxdlp_v4_document_header(metadata, &layer_sizes)?;
    let layer_offsets = cxdlp_v4_layer_offsets(header.len(), &layer_sizes)?;

    let mut checksum = 0;
    let mut bytes_written = 0;
    write_cxdlp_v4_checked(sink, &header, &mut checksum, &mut bytes_written)?;

    let mut layer_def = Vec::with_capacity(LAYER_DEF_SIZE);
    for (layer_index, (offset, size)) in layer_offsets.iter().copied().enumerate() {
        layer_def.clear();
        write_cxdlp_v4_layer_def(&mut layer_def, metadata, layer_index, offset, size);
        write_cxdlp_v4_checked(sink, &layer_def, &mut checksum, &mut bytes_written)?;
    }

    for layer in layers {
        write_cxdlp_v4_checked(sink, layer, &mut checksum, &mut bytes_written)?;
    }

    let checksum_bytes = checksum.to_be_bytes();
    sink.write_all(&checksum_bytes)?;
    bytes_written += checksum_bytes.len();
    Ok(bytes_written)
}

fn write_cxdlp_v4_streaming_document(
    metadata: &FileMetadata,
    layer_count: usize,
    stream: &mut dyn crate::formats::common::EncodedLayerStream,
    sink: &mut dyn OutputSink,
) -> Result<DocumentEncodeStats, ConvertError> {
    let mut layer_sizes = Vec::with_capacity(layer_count);
    let mut layer = Vec::new();
    for layer_index in 0..layer_count {
        layer.clear();
        stream.read_layer_into(layer_index, &mut layer)?;
        layer_sizes.push(checked_u32(layer.len(), "CXDLPv4 layer data size")?);
    }

    let header = build_cxdlp_v4_document_header(metadata, &layer_sizes)?;
    let layer_offsets = cxdlp_v4_layer_offsets(header.len(), &layer_sizes)?;

    let mut checksum = 0;
    let mut bytes_written = 0;
    write_cxdlp_v4_checked(sink, &header, &mut checksum, &mut bytes_written)?;

    let mut layer_def = Vec::with_capacity(LAYER_DEF_SIZE);
    for (layer_index, (offset, size)) in layer_offsets.iter().copied().enumerate() {
        layer_def.clear();
        write_cxdlp_v4_layer_def(&mut layer_def, metadata, layer_index, offset, size);
        write_cxdlp_v4_checked(sink, &layer_def, &mut checksum, &mut bytes_written)?;
    }

    for layer_index in 0..layer_count {
        layer.clear();
        stream.read_layer_into(layer_index, &mut layer)?;
        write_cxdlp_v4_checked(sink, &layer, &mut checksum, &mut bytes_written)?;
    }

    let checksum_bytes = checksum.to_be_bytes();
    sink.write_all(&checksum_bytes)?;
    bytes_written += checksum_bytes.len();

    Ok(DocumentEncodeStats {
        bytes_written,
        layers_written: layer_count,
    })
}

pub fn build_cxdlp_v4_document_header(
    metadata: &FileMetadata,
    layer_sizes: &[u32],
) -> Result<Vec<u8>, ConvertError> {
    let layer_count = checked_u32(layer_sizes.len(), "CXDLPv4 layer count")?;
    if metadata.slicing.layer_count != 0 && metadata.slicing.layer_count != layer_count {
        return Err(ConvertError::BinrwError(format!(
            "CXDLPv4 metadata layer count {} does not match {} encoded layers",
            metadata.slicing.layer_count, layer_count
        )));
    }

    let header_size = cxdlp_v4_header_size(&metadata.printer.machine_name)?;
    let print_parameters_offset = checked_u32(header_size, "CXDLPv4 print parameters offset")?;
    let slicer_offset = checked_u32(
        header_size + PRINT_PARAMETERS_V4_SIZE,
        "CXDLPv4 slicer info offset",
    )?;
    let layers_definition_offset = checked_u32(
        header_size + PRINT_PARAMETERS_V4_SIZE + SLICER_INFO_V4_SIZE,
        "CXDLPv4 layer table offset",
    )?;
    let mut document =
        Vec::with_capacity(header_size + PRINT_PARAMETERS_V4_SIZE + SLICER_INFO_V4_SIZE);
    write_cxdlp_v4_header(
        &mut document,
        metadata,
        layer_count,
        print_parameters_offset,
        PRINT_PARAMETERS_V4_SIZE as u32,
        slicer_offset,
        SLICER_INFO_V4_SIZE as u32,
        layers_definition_offset,
    )?;
    debug_assert_eq!(document.len(), header_size);
    write_cxdlp_v4_print_parameters(&mut document, metadata);
    write_cxdlp_v4_slicer_info(&mut document, metadata);
    Ok(document)
}

pub fn build_cxdlp_v4_layer_def(
    metadata: &FileMetadata,
    layer_index: usize,
    data_address: u32,
    data_size: u32,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(LAYER_DEF_SIZE);
    write_cxdlp_v4_layer_def(&mut output, metadata, layer_index, data_address, data_size);
    output
}

fn cxdlp_v4_layer_offsets(
    header_size: usize,
    layer_sizes: &[u32],
) -> Result<Vec<(u32, u32)>, ConvertError> {
    let layer_table_size =
        checked_mul_usize(layer_sizes.len(), LAYER_DEF_SIZE, "CXDLPv4 layer table")?;
    let mut data_offset =
        checked_add_usize(header_size, layer_table_size, "CXDLPv4 first layer offset")?;

    let mut layer_offsets = Vec::with_capacity(layer_sizes.len());
    for size in layer_sizes {
        let offset = checked_u32(data_offset, "CXDLPv4 layer data offset")?;
        layer_offsets.push((offset, *size));
        data_offset = checked_add_usize(data_offset, *size as usize, "CXDLPv4 layer data end")?;
    }
    Ok(layer_offsets)
}

fn write_cxdlp_v4_checked(
    sink: &mut dyn OutputSink,
    bytes: &[u8],
    checksum: &mut u32,
    bytes_written: &mut usize,
) -> Result<(), ConvertError> {
    sink.write_all(bytes)?;
    *checksum = cxdlp_crc32_update(*checksum, bytes);
    *bytes_written += bytes.len();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_cxdlp_v4_header(
    output: &mut Vec<u8>,
    metadata: &FileMetadata,
    layer_count: u32,
    print_parameters_offset: u32,
    print_parameters_size: u32,
    slicer_offset: u32,
    slicer_size: u32,
    layers_definition_offset: u32,
) -> Result<(), ConvertError> {
    let mut writer = BinaryWriter::new(output);
    writer.write_be_u32(MAGIC_SIZE as u32);
    writer.write_bytes(MAGIC_VALUE.as_bytes());
    writer.write_u8(0);
    writer.write_be_u16(4);
    write_cxdlp_v4_string(output, &metadata.printer.machine_name)?;
    let mut writer = BinaryWriter::new(output);
    writer.write_le_u16(checked_u16(
        metadata.printer.resolution_x,
        "CXDLPv4 resolution x",
    )?);
    writer.write_le_u16(checked_u16(
        metadata.printer.resolution_y,
        "CXDLPv4 resolution y",
    )?);
    writer.write_le_f32(metadata.printer.display_width_mm);
    writer.write_le_f32(metadata.printer.display_height_mm);
    writer.write_le_f32(metadata.printer.machine_z_mm);
    writer.write_le_f32(metadata.slicing.layer_height_mm * layer_count as f32);
    writer.write_le_f32(metadata.slicing.layer_height_mm);
    writer.write_le_u32(metadata.slicing.bottom_layer_count);
    writer.write_le_u32(0);
    writer.write_le_u32(layers_definition_offset);
    writer.write_le_u32(layer_count);
    writer.write_le_u32(0);
    writer.write_le_u32(metadata.stats.print_time_sec);
    writer.write_le_u32(u32::from(metadata.printer.mirror_x));
    writer.write_le_u32(print_parameters_offset);
    writer.write_le_u32(print_parameters_size);
    writer.write_le_u32(metadata.slicing.anti_alias_level.max(1));
    writer.write_le_u16(metadata.exposure.light_pwm);
    writer.write_le_u16(metadata.exposure.bottom_light_pwm);
    writer.write_le_u32(0);
    writer.write_le_u32(slicer_offset);
    writer.write_le_u32(slicer_size);
    Ok(())
}

fn write_cxdlp_v4_print_parameters(output: &mut Vec<u8>, metadata: &FileMetadata) {
    let mut writer = BinaryWriter::new(output);
    writer.write_le_f32(metadata.motion.bottom_lift_height);
    writer.write_le_f32(metadata.motion.bottom_lift_speed);
    writer.write_le_f32(metadata.motion.lift_height);
    writer.write_le_f32(metadata.motion.lift_speed);
    writer.write_le_f32(metadata.motion.retract_speed);
    writer.write_le_f32(metadata.stats.volume_ml);
    writer.write_le_f32(metadata.stats.weight_g);
    writer.write_le_f32(metadata.stats.cost);
    writer.write_le_f32(metadata.exposure.bottom_light_off_delay);
    writer.write_le_f32(metadata.exposure.light_off_delay);
    writer.write_le_u32(metadata.slicing.bottom_layer_count);
    writer.write_le_f32(metadata.exposure.exposure_time);
    writer.write_le_f32(metadata.exposure.bottom_exposure_time);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
}

fn write_cxdlp_v4_slicer_info(output: &mut Vec<u8>, metadata: &FileMetadata) {
    let mut writer = BinaryWriter::new(output);
    writer.write_le_f32(0.0);
    writer.write_le_f32(0.0);
    writer.write_le_f32(metadata.motion.lift_height2);
    writer.write_le_f32(metadata.motion.lift_speed2);
    writer.write_le_f32(metadata.motion.retract_height2);
    writer.write_le_f32(metadata.motion.retract_speed2);
    writer.write_le_f32(metadata.waits.after_lift);
    writer.write_le_u32(u32::from(!metadata.per_layer_settings.is_empty()));
    writer.write_le_u32(0);
    writer.write_le_u32(metadata.slicing.anti_alias_level.max(1));
    writer.write_le_u32(0);
    writer.write_le_f32(metadata.waits.before_cure);
    writer.write_le_f32(metadata.waits.after_cure);
    writer.write_le_f32(metadata.exposure.exposure_time);
    writer.write_le_f32(metadata.exposure.bottom_exposure_time);
    writer.write_le_f32(0.0);
    writer.write_le_u32(metadata.slicing.transition_layer_count);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
}

fn write_cxdlp_v4_layer_def(
    output: &mut Vec<u8>,
    metadata: &FileMetadata,
    layer_index: usize,
    data_address: u32,
    data_size: u32,
) {
    let override_settings = metadata.per_layer_settings.get(layer_index);
    let is_bottom_layer = layer_index < metadata.slicing.bottom_layer_count as usize;
    let position_z = override_settings
        .map(|settings| settings.position_z)
        .unwrap_or((layer_index as f32 + 1.0) * metadata.slicing.layer_height_mm);
    let exposure_time = override_settings
        .and_then(|settings| settings.exposure_time)
        .unwrap_or(if is_bottom_layer {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        });
    let light_off_seconds = if is_bottom_layer {
        metadata.exposure.bottom_light_off_delay
    } else {
        metadata.exposure.light_off_delay
    };

    let mut writer = BinaryWriter::new(output);
    writer.write_le_f32(position_z);
    writer.write_le_f32(exposure_time);
    writer.write_le_f32(light_off_seconds);
    writer.write_le_u32(data_address);
    writer.write_le_u32(data_size);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
    writer.write_le_u32(0);
}

fn cxdlp_v4_header_size(machine_name: &str) -> Result<usize, ConvertError> {
    Ok(99 + cxdlp_v4_string_size(machine_name)?)
}

fn cxdlp_v4_string_size(value: &str) -> Result<usize, ConvertError> {
    if value.as_bytes().contains(&0) {
        return Err(ConvertError::BinrwError(
            "CXDLPv4 strings must not contain NUL bytes".to_string(),
        ));
    }
    value
        .len()
        .checked_add(1)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: value.len(),
        })
}

fn write_cxdlp_v4_string(output: &mut Vec<u8>, value: &str) -> Result<(), ConvertError> {
    let size = cxdlp_v4_string_size(value)?;
    let mut writer = BinaryWriter::new(output);
    writer.write_be_u32(checked_u32(size, "CXDLPv4 string length")?);
    writer.write_bytes(value.as_bytes());
    writer.write_u8(0);
    Ok(())
}

fn looks_like_cxdlp_legacy(
    data: &[u8],
    specifics: &CrealityCxdLpSpec,
) -> Result<bool, ConvertError> {
    let (header, _) = parse_header_legacy(data)?;
    Ok(header.magic_size as usize == MAGIC_SIZE
        && header.magic == MAGIC_VALUE
        && u32::from(header.version) == specifics.version)
}

fn looks_like_cxdlp_v1(data: &[u8], specifics: &CrealityCxdLpSpec) -> Result<bool, ConvertError> {
    let (header, _) = parse_header_v1(data)?;
    Ok(specifics.version == 1
        && header.magic_size as usize == MAGIC_SIZE
        && header.magic == MAGIC_VALUE)
}

fn looks_like_cxdlp_v4(data: &[u8]) -> Result<bool, ConvertError> {
    let header = parse_header_v4(data)?;
    Ok(header.magic_size as usize == MAGIC_SIZE
        && header.magic == MAGIC_VALUE
        && header.version == 4)
}

fn decode_cxdlp_line_layer_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let expected =
        (width as usize)
            .checked_mul(height as usize)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: 0,
            })?;
    output.clear();
    output.resize(expected, 0);

    if data.len() < LEGACY_LAYER_HEADER_SIZE {
        return Err(ConvertError::BufferTooSmall {
            need: LEGACY_LAYER_HEADER_SIZE,
            got: data.len(),
        });
    }

    let line_count = read_be_u32(data, 4)? as usize;
    let line_bytes =
        line_count
            .checked_mul(LEGACY_LAYER_LINE_SIZE)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
    let line_end =
        LEGACY_LAYER_HEADER_SIZE
            .checked_add(line_bytes)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
    if line_end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: line_end,
            got: data.len(),
        });
    }

    for line_index in 0..line_count {
        let base = LEGACY_LAYER_HEADER_SIZE + line_index * LEGACY_LAYER_LINE_SIZE;
        let start_y = ((((data[base] as u16) << 8) | data[base + 1] as u16) >> 3) & 0x1fff;
        let end_y = ((((data[base + 1] as u32) << 16)
            | ((data[base + 2] as u32) << 8)
            | data[base + 3] as u32)
            >> 6)
            & 0x1fff;
        let start_x = (((data[base + 3] as u16) << 8) | data[base + 4] as u16) & 0x3fff;
        let gray = data[base + 5];

        if start_x as u32 >= width || start_y as u32 >= height || end_y >= height {
            return Err(ConvertError::RleDecodeError {
                offset: base,
                detail: "CXDLP line segment outside layer bounds".to_string(),
            });
        }
        if end_y < u32::from(start_y) {
            return Err(ConvertError::RleDecodeError {
                offset: base,
                detail: "CXDLP line segment has negative height".to_string(),
            });
        }

        for y in u32::from(start_y)..=end_y {
            let pixel_index = y as usize * width as usize + start_x as usize;
            output[pixel_index] = gray;
        }
    }

    Ok(())
}

fn decode_cxdlp_rle_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let expected =
        (width as usize)
            .checked_mul(height as usize)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: 0,
            })?;
    output.clear();
    output.reserve(expected);

    let mut index = 0;
    while index < data.len() && output.len() < expected {
        let (pixel, stride) = read_cxdlp_rle_span(data, &mut index)?;
        let new_len = output
            .len()
            .checked_add(stride)
            .ok_or(ConvertError::RleDecodeError {
                offset: index,
                detail: "CXDLP run length overflow".to_string(),
            })?;
        if new_len > expected {
            return Err(ConvertError::RleDecodeError {
                offset: index,
                detail: format!("CXDLP RLE expands beyond {expected} pixels"),
            });
        }
        output.resize(new_len, pixel);
    }

    if output.len() != expected {
        return Err(ConvertError::RleDecodeError {
            offset: data.len(),
            detail: format!(
                "CXDLP RLE produced {} pixels, expected {expected}",
                output.len()
            ),
        });
    }
    Ok(())
}

fn decode_cxdlp_rle_preview_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    max_preview_size: u32,
) -> Result<PreviewLayerImage, ConvertError> {
    let Some(plan) = PreviewDecodePlan::for_downsample(width, height, max_preview_size) else {
        let mut rgba = Vec::new();
        decode_cxdlp_rle_rgba_into(data, width, height, &mut rgba)?;
        return Ok(PreviewLayerImage {
            pixels: rgba,
            width,
            height,
        });
    };
    let expected =
        (width as usize)
            .checked_mul(height as usize)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: 0,
            })?;
    let mut rgba = plan.new_rgba_buffer();
    let targets = plan.target_indices();
    let mut target_index = 0;
    let mut source_index = 0usize;
    let mut index = 0usize;

    while index < data.len() && source_index < expected && target_index < targets.len() {
        let span_start = source_index;
        let (pixel, stride) = read_cxdlp_rle_span(data, &mut index)?;
        let span_end = span_start
            .checked_add(stride)
            .ok_or(ConvertError::RleDecodeError {
                offset: index,
                detail: "CXDLP run length overflow".to_string(),
            })?;
        if span_end > expected {
            return Err(ConvertError::RleDecodeError {
                offset: index,
                detail: format!("CXDLP RLE expands beyond {expected} pixels"),
            });
        }

        while target_index < targets.len() && targets[target_index] < span_end {
            if targets[target_index] >= span_start && pixel != 0 {
                let out = target_index * 4;
                rgba[out] = pixel;
                rgba[out + 1] = pixel;
                rgba[out + 2] = pixel;
            }
            target_index += 1;
        }
        source_index = span_end;
    }

    Ok(PreviewLayerImage {
        pixels: rgba,
        width: plan.dst_width,
        height: plan.dst_height,
    })
}

fn decode_cxdlp_rle_rgba_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let expected =
        (width as usize)
            .checked_mul(height as usize)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: 0,
            })?;
    let rgba_capacity = expected
        .checked_mul(4)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: 0,
        })?;
    output.clear();
    output.reserve(rgba_capacity);

    let mut index = 0usize;
    let mut source_len = 0usize;
    while index < data.len() && source_len < expected {
        let (pixel, stride) = read_cxdlp_rle_span(data, &mut index)?;
        let new_len = source_len
            .checked_add(stride)
            .ok_or(ConvertError::RleDecodeError {
                offset: index,
                detail: "CXDLP run length overflow".to_string(),
            })?;
        if new_len > expected {
            return Err(ConvertError::RleDecodeError {
                offset: index,
                detail: format!("CXDLP RLE expands beyond {expected} pixels"),
            });
        }
        let output_start = output.len();
        let output_end = output_start
            .checked_add(stride.checked_mul(4).ok_or(ConvertError::RleDecodeError {
                offset: index,
                detail: "CXDLP RGBA run length overflow".to_string(),
            })?)
            .ok_or(ConvertError::RleDecodeError {
                offset: index,
                detail: "CXDLP RGBA output length overflow".to_string(),
            })?;
        output.resize(output_end, 0);
        for rgba in output[output_start..output_end].chunks_exact_mut(4) {
            rgba[0] = pixel;
            rgba[1] = pixel;
            rgba[2] = pixel;
            rgba[3] = 255;
        }
        source_len = new_len;
    }

    if source_len != expected {
        return Err(ConvertError::RleDecodeError {
            offset: data.len(),
            detail: format!("CXDLP RLE produced {source_len} pixels, expected {expected}"),
        });
    }
    Ok(())
}

fn encode_cxdlp_v4_layer_into(
    pixels: &[u8],
    ctx: &LayerEncodeContext<'_>,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let expected = (ctx.width as usize)
        .checked_mul(ctx.height as usize)
        .ok_or(ConvertError::RleEncodeError(
            "CXDLP layer dimensions overflow".to_string(),
        ))?;
    if pixels.len() != expected {
        return Err(ConvertError::RleEncodeError(format!(
            "CXDLP layer has {} pixels, expected {expected}",
            pixels.len()
        )));
    }

    output.clear();
    output.reserve(44 + pixels.len() / 2);
    write_cxdlp_v4_layer_parameters(ctx, output);
    encode_cxdlp_rle_payload_into(pixels, output)?;
    Ok(())
}

fn write_cxdlp_v4_layer_parameters(ctx: &LayerEncodeContext<'_>, output: &mut Vec<u8>) {
    let params = cxdlp_v4_layer_parameters(ctx);
    let mut writer = BinaryWriter::new(output);
    for value in params {
        writer.write_le_f32(value);
    }
}

fn cxdlp_v4_layer_parameters(ctx: &LayerEncodeContext<'_>) -> [f32; 11] {
    let Some(metadata) = ctx.layer_metadata else {
        return [5.0, 65.0, 0.0, 0.0, 150.0, 0.0, 0.0, 0.0, 0.0, 0.0, 255.0];
    };
    let is_bottom_layer = ctx.layer_index < metadata.slicing.bottom_layer_count;
    let lift_height = if is_bottom_layer {
        metadata.motion.bottom_lift_height
    } else {
        metadata.motion.lift_height
    };
    let lift_speed = if is_bottom_layer {
        metadata.motion.bottom_lift_speed
    } else {
        metadata.motion.lift_speed
    };
    let light_pwm = if is_bottom_layer {
        metadata.exposure.bottom_light_pwm
    } else {
        metadata.exposure.light_pwm
    };

    [
        lift_height + metadata.motion.lift_height2,
        lift_speed,
        metadata.motion.lift_height2,
        metadata.motion.lift_speed2,
        metadata.motion.retract_speed,
        metadata.motion.retract_height2,
        metadata.motion.retract_speed2,
        metadata.waits.after_cure,
        metadata.waits.after_lift,
        metadata.waits.before_cure,
        f32::from(light_pwm),
    ]
}

fn encode_cxdlp_rle_payload_into(pixels: &[u8], output: &mut Vec<u8>) -> Result<(), ConvertError> {
    let mut color = 0x7f;
    let mut stride = 0usize;

    for pixel in pixels {
        let gray7 = pixel >> 1;
        if gray7 == color {
            stride += 1;
        } else {
            write_cxdlp_rle_run(output, color, stride)?;
            color = gray7;
            stride = 1;
        }
    }
    write_cxdlp_rle_run(output, color, stride)
}

fn write_cxdlp_rle_run(
    output: &mut Vec<u8>,
    color: u8,
    mut stride: usize,
) -> Result<(), ConvertError> {
    const MAX_CXDLP_RLE_RUN: usize = 0x0fff_ffff;

    while stride > 0 {
        let chunk = stride.min(MAX_CXDLP_RLE_RUN);
        let code = color & 0x7f;
        if chunk == 1 {
            output.push(code);
        } else {
            output.push(code | 0x80);
            write_cxdlp_rle_run_length(output, chunk)?;
        }
        stride -= chunk;
    }

    Ok(())
}

fn write_cxdlp_rle_run_length(output: &mut Vec<u8>, stride: usize) -> Result<(), ConvertError> {
    if stride <= 0x7f {
        output.push(stride as u8);
    } else if stride <= 0x3fff {
        output.push(((stride >> 8) as u8) | 0x80);
        output.push(stride as u8);
    } else if stride <= 0x1f_ffff {
        output.push(((stride >> 16) as u8) | 0xc0);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    } else if stride <= 0x0fff_ffff {
        output.push(((stride >> 24) as u8) | 0xe0);
        output.push((stride >> 16) as u8);
        output.push((stride >> 8) as u8);
        output.push(stride as u8);
    } else {
        return Err(ConvertError::RleEncodeError(format!(
            "CXDLP run length {stride} exceeds 28-bit encoding"
        )));
    }
    Ok(())
}

fn read_cxdlp_rle_span(data: &[u8], index: &mut usize) -> Result<(u8, usize), ConvertError> {
    let mut code = *data.get(*index).ok_or(ConvertError::RleDecodeError {
        offset: *index,
        detail: "missing CXDLP RLE code".to_string(),
    })?;
    *index += 1;
    let mut stride = 1usize;

    if code & 0x80 != 0 {
        code &= 0x7f;
        let slen = *data.get(*index).ok_or(ConvertError::RleDecodeError {
            offset: *index,
            detail: "missing CXDLP run length".to_string(),
        })?;
        *index += 1;
        if slen & 0x80 == 0 {
            stride = slen as usize;
        } else if slen & 0xc0 == 0x80 {
            let b1 = *data.get(*index).ok_or(ConvertError::RleDecodeError {
                offset: *index,
                detail: "truncated 14-bit CXDLP run length".to_string(),
            })?;
            *index += 1;
            stride = (((slen & 0x3f) as usize) << 8) | b1 as usize;
        } else if slen & 0xe0 == 0xc0 {
            let b1 = *data.get(*index).ok_or(ConvertError::RleDecodeError {
                offset: *index,
                detail: "truncated 21-bit CXDLP run length".to_string(),
            })?;
            let b2 = *data.get(*index + 1).ok_or(ConvertError::RleDecodeError {
                offset: *index + 1,
                detail: "truncated 21-bit CXDLP run length".to_string(),
            })?;
            *index += 2;
            stride = (((slen & 0x1f) as usize) << 16) | ((b1 as usize) << 8) | b2 as usize;
        } else if slen & 0xf0 == 0xe0 {
            let b1 = *data.get(*index).ok_or(ConvertError::RleDecodeError {
                offset: *index,
                detail: "truncated 28-bit CXDLP run length".to_string(),
            })?;
            let b2 = *data.get(*index + 1).ok_or(ConvertError::RleDecodeError {
                offset: *index + 1,
                detail: "truncated 28-bit CXDLP run length".to_string(),
            })?;
            let b3 = *data.get(*index + 2).ok_or(ConvertError::RleDecodeError {
                offset: *index + 2,
                detail: "truncated 28-bit CXDLP run length".to_string(),
            })?;
            *index += 3;
            stride = (((slen & 0x0f) as usize) << 24)
                | ((b1 as usize) << 16)
                | ((b2 as usize) << 8)
                | b3 as usize;
        } else {
            return Err(ConvertError::RleDecodeError {
                offset: *index - 1,
                detail: "corrupted CXDLP run length".to_string(),
            });
        }
    }

    let pixel = if code == 0 { 0 } else { (code << 1) | 1 };
    Ok((pixel, stride))
}

fn crypt_layer_rle(seed: u32, layer_index: u32, data: &mut [u8]) {
    if seed == 0 {
        return;
    }
    let init = seed.wrapping_mul(0x2d83_cdac).wrapping_add(0xd8a8_3423);
    let mut key = layer_index
        .wrapping_mul(0x1e15_30cd)
        .wrapping_add(0xec3d_47cd)
        .wrapping_mul(init);
    let mut index = 0;
    for byte in data {
        *byte ^= (key >> (8 * index)) as u8;
        index += 1;
        if index & 3 == 0 {
            key = key.wrapping_add(init);
            index = 0;
        }
    }
}

pub fn cxdlp_crc32(data: &[u8]) -> u32 {
    cxdlp_crc32_update(0, data)
}

pub fn cxdlp_crc32_update(mut checksum: u32, data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 == 0 {
                value >> 1
            } else {
                (value >> 1) ^ 0xedb8_8320
            };
        }
        *slot = value;
    }

    for byte in data {
        let table_index = ((checksum as u8) ^ byte) as usize;
        checksum = table[table_index] ^ (checksum >> 8);
    }
    checksum
}

fn checked_add_usize(a: usize, b: usize, label: &'static str) -> Result<usize, ConvertError> {
    a.checked_add(b)
        .ok_or_else(|| ConvertError::BinrwError(format!("{label} overflows platform usize bounds")))
}

fn checked_mul_usize(a: usize, b: usize, label: &'static str) -> Result<usize, ConvertError> {
    a.checked_mul(b)
        .ok_or_else(|| ConvertError::BinrwError(format!("{label} overflows platform usize bounds")))
}

fn checked_u16(value: u32, label: &'static str) -> Result<u16, ConvertError> {
    u16::try_from(value).map_err(|_| {
        ConvertError::BinrwError(format!("{label} value {value} exceeds 16-bit field size"))
    })
}

fn checked_u32(value: usize, label: &'static str) -> Result<u32, ConvertError> {
    u32::try_from(value).map_err(|_| {
        ConvertError::BinrwError(format!("{label} value {value} exceeds 32-bit field size"))
    })
}

fn read_fixed_string_at(
    data: &[u8],
    index: &mut usize,
    len: usize,
    label: &'static str,
) -> Result<String, ConvertError> {
    let end = index.checked_add(len).ok_or(ConvertError::BufferTooSmall {
        need: usize::MAX,
        got: data.len(),
    })?;
    if end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: end,
            got: data.len(),
        });
    }
    let bytes = &data[*index..end];
    *index = end;
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    std::str::from_utf8(bytes)
        .map(|value| value.to_string())
        .map_err(|error| ConvertError::BinrwError(format!("invalid {label}: {error}")))
}

fn read_be_len_string_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<String, ConvertError> {
    let len = read_be_u32_at(data, index, label)? as usize;
    read_fixed_string_at(data, index, len, label)
}

fn read_be_len_utf16_f32_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<f32, ConvertError> {
    let len = read_be_u32_at(data, index, label)? as usize;
    let end = index.checked_add(len).ok_or(ConvertError::BufferTooSmall {
        need: usize::MAX,
        got: data.len(),
    })?;
    if end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: end,
            got: data.len(),
        });
    }
    if len % 2 != 0 {
        return Err(ConvertError::BinrwError(format!(
            "invalid {label}: odd UTF-16BE byte length {len}"
        )));
    }

    let units = data[*index..end]
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    *index = end;
    let value = String::from_utf16(&units)
        .map_err(|error| ConvertError::BinrwError(format!("invalid {label}: {error}")))?;
    value
        .parse::<f32>()
        .map_err(|error| ConvertError::BinrwError(format!("invalid {label}: {error}")))
}

fn skip_bytes(
    data: &[u8],
    index: &mut usize,
    count: usize,
    _label: &'static str,
) -> Result<(), ConvertError> {
    let end = index
        .checked_add(count)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: data.len(),
        })?;
    if end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: end,
            got: data.len(),
        });
    }
    *index = end;
    Ok(())
}

fn read_be_u16_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<u16, ConvertError> {
    let value = read_be_u16(data, *index).map_err(|error| label_error(label, error))?;
    *index += 2;
    Ok(value)
}

fn read_be_u32_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<u32, ConvertError> {
    let value = read_be_u32(data, *index).map_err(|error| label_error(label, error))?;
    *index += 4;
    Ok(value)
}

fn read_le_u16_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<u16, ConvertError> {
    let value = read_le_u16(data, *index).map_err(|error| label_error(label, error))?;
    *index += 2;
    Ok(value)
}

fn read_le_u32_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<u32, ConvertError> {
    let value = read_le_u32(data, *index).map_err(|error| label_error(label, error))?;
    *index += 4;
    Ok(value)
}

fn read_le_f32_at(
    data: &[u8],
    index: &mut usize,
    label: &'static str,
) -> Result<f32, ConvertError> {
    let value = read_le_f32(data, *index).map_err(|error| label_error(label, error))?;
    *index += 4;
    Ok(value)
}

fn label_error(label: &'static str, error: ConvertError) -> ConvertError {
    ConvertError::BinrwError(format!("{label}: {error}"))
}
