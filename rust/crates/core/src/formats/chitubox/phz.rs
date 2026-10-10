use std::borrow::Cow;

use crate::{
    common::{
        ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset, PerLayerSettings, Thumbnail,
    },
    formats::{
        common::{
            binary::{
                put_le_f32, put_le_u16, put_le_u32, read_le_f32, read_le_u16, read_le_u32,
                read_range,
            },
            ExportTargetOptions, FormatSpec, HeaderLayout, LayerCodec,
        },
        ctb, LayerDecodeContext, LayerEncodeContext,
    },
};

const MAGIC: u32 = 0x9FDA_83AE;
const HEADER_SIZE: usize = 216;
const PREVIEW_SIZE: usize = 32;
const LAYER_DEF_SIZE: usize = 36;
const PAGE_SIZE: u64 = 4_294_967_296;

pub struct PhzHeaderLayout;
pub struct PhzLayerCodec;

#[derive(Debug, Clone)]
struct Header {
    magic: u32,
    version: u32,
    layer_height: f32,
    exposure_seconds: f32,
    bottom_exposure_seconds: f32,
    bottom_layer_count: u32,
    resolution_x: u32,
    resolution_y: u32,
    preview_large_offset: u32,
    layers_offset: u32,
    layer_count: u32,
    preview_small_offset: u32,
    print_time: u32,
    projector_type: u32,
    light_pwm: u16,
    bottom_light_pwm: u16,
    display_width: f32,
    display_height: f32,
    machine_z: f32,
    encryption_key: u32,
    bottom_light_off_delay: f32,
    light_off_delay: f32,
    bottom_lift_height: f32,
    bottom_lift_speed: f32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    volume_ml: f32,
    weight_g: f32,
    cost: f32,
    machine_name_offset: u32,
    machine_name_size: u32,
    anti_alias_level: u32,
}

#[derive(Debug, Clone)]
struct LayerDef {
    position_z: f32,
    exposure_time: f32,
    data_offset: u32,
    data_size: u32,
    page_number: u32,
}

#[derive(Debug, Clone)]
struct PreviewDef {
    width: u32,
    height: u32,
    image_offset: u32,
    image_length: u32,
}

impl HeaderLayout for PhzHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        data.len() >= 4 && read_le_u32(data, 0).is_ok_and(|magic| magic == MAGIC)
    }

    fn parse_header(
        &self,
        data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        parse_document_header(data)
    }

    fn build_header(
        &self,
        metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        build_phz_header(metadata)
    }

    fn finalize_header(
        &self,
        header: &[u8],
        layer_sizes: &[u32],
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        finalize_phz_header(header, layer_sizes)
    }
}

impl LayerCodec for PhzLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let mut payload = if ctx.options.encryption_key == 0 {
            Cow::Borrowed(data)
        } else {
            Cow::Owned(data.to_vec())
        };
        if ctx.options.encryption_key != 0 {
            phz_crypt_buffer(
                ctx.options.encryption_key,
                ctx.layer_index,
                payload.to_mut(),
            );
        }
        decode_phz_rle_into(&payload, ctx.width, ctx.height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        encode_phz_rle_into(pixels, output);
        let key = ctx
            .layer_metadata
            .map(|metadata| metadata.slicing.encryption_key)
            .unwrap_or(0);
        phz_crypt_buffer(key, ctx.layer_index, output);
        Ok(())
    }
}

fn build_phz_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let machine_name = effective_machine_name(metadata).as_bytes();
    let machine_name_offset = HEADER_SIZE;
    let layers_offset = machine_name_offset + machine_name.len();
    let mut data = vec![0u8; layers_offset + LAYER_DEF_SIZE * layer_count as usize];

    put_le_u32(&mut data, 0, MAGIC)?;
    put_le_u32(&mut data, 4, 2)?;
    put_le_f32(&mut data, 8, metadata.slicing.layer_height_mm)?;
    put_le_f32(&mut data, 12, metadata.exposure.exposure_time)?;
    put_le_f32(&mut data, 16, metadata.exposure.bottom_exposure_time)?;
    put_le_u32(&mut data, 20, metadata.slicing.bottom_layer_count)?;
    put_le_u32(&mut data, 24, metadata.printer.resolution_x)?;
    put_le_u32(&mut data, 28, metadata.printer.resolution_y)?;
    put_le_u32(&mut data, 32, 0)?;
    put_le_u32(&mut data, 36, layers_offset as u32)?;
    put_le_u32(&mut data, 40, layer_count)?;
    put_le_u32(&mut data, 44, 0)?;
    put_le_u32(&mut data, 48, metadata.stats.print_time_sec)?;
    put_le_u32(&mut data, 52, if metadata.printer.mirror_x { 1 } else { 0 })?;
    put_le_u16(&mut data, 60, metadata.exposure.light_pwm)?;
    put_le_u16(&mut data, 62, metadata.exposure.bottom_light_pwm)?;
    put_le_f32(
        &mut data,
        72,
        metadata.slicing.layer_height_mm * layer_count as f32,
    )?;
    put_le_f32(&mut data, 76, metadata.printer.display_width_mm)?;
    put_le_f32(&mut data, 80, metadata.printer.display_height_mm)?;
    put_le_f32(&mut data, 84, metadata.printer.machine_z_mm)?;
    put_le_u32(&mut data, 88, metadata.slicing.encryption_key)?;
    put_le_f32(&mut data, 92, metadata.exposure.bottom_light_off_delay)?;
    put_le_f32(&mut data, 96, metadata.exposure.light_off_delay)?;
    put_le_u32(&mut data, 100, metadata.slicing.bottom_layer_count)?;
    put_le_f32(&mut data, 108, metadata.motion.bottom_lift_height)?;
    put_le_f32(&mut data, 112, metadata.motion.bottom_lift_speed)?;
    put_le_f32(&mut data, 116, metadata.motion.lift_height)?;
    put_le_f32(&mut data, 120, metadata.motion.lift_speed)?;
    put_le_f32(&mut data, 124, metadata.motion.retract_speed)?;
    put_le_f32(&mut data, 128, metadata.stats.volume_ml)?;
    put_le_f32(&mut data, 132, metadata.stats.weight_g)?;
    put_le_f32(&mut data, 136, metadata.stats.cost)?;
    put_le_u32(&mut data, 144, machine_name_offset as u32)?;
    put_le_u32(&mut data, 148, machine_name.len() as u32)?;
    put_le_u32(&mut data, 176, 28)?;
    put_le_u32(&mut data, 184, metadata.slicing.anti_alias_level.max(1))?;
    put_le_u32(&mut data, 188, 0x0106_0300)?;
    data[machine_name_offset..machine_name_offset + machine_name.len()]
        .copy_from_slice(machine_name);

    write_placeholder_layer_defs(&mut data, metadata, layers_offset)?;
    Ok(data)
}

fn finalize_phz_header(header: &[u8], layer_sizes: &[u32]) -> Result<Vec<u8>, ConvertError> {
    let mut data = header.to_vec();
    let layers_offset = read_le_u32(&data, 36)? as usize;
    let mut data_offset = data.len();
    for (index, size) in layer_sizes.iter().enumerate() {
        let offset = layers_offset + index * LAYER_DEF_SIZE;
        put_le_u32(&mut data, offset + 12, data_offset as u32)?;
        put_le_u32(&mut data, offset + 16, *size)?;
        put_le_u32(&mut data, offset + 20, 0)?;
        data_offset += *size as usize;
    }
    Ok(data)
}

fn write_placeholder_layer_defs(
    data: &mut [u8],
    metadata: &FileMetadata,
    layers_offset: usize,
) -> Result<(), ConvertError> {
    for index in 0..metadata.slicing.layer_count as usize {
        let offset = layers_offset + index * LAYER_DEF_SIZE;
        let position_z = metadata.slicing.layer_height_mm * (index + 1) as f32;
        let exposure = if index < metadata.slicing.bottom_layer_count as usize {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        };
        put_le_f32(data, offset, position_z)?;
        put_le_f32(data, offset + 4, exposure)?;
        put_le_f32(data, offset + 8, metadata.exposure.light_off_delay)?;
    }
    Ok(())
}

fn parse_document_header(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let header = parse_header(data)?;
    if header.magic != MAGIC {
        return Err(ConvertError::InvalidMagic {
            expected: MAGIC,
            actual: header.magic,
        });
    }
    if header.version != 2 {
        return Err(ConvertError::UnsupportedVersion(header.version));
    }

    let machine_name = read_string(
        data,
        header.machine_name_offset as usize,
        header.machine_name_size as usize,
    )
    .unwrap_or_default();
    let thumbnails = parse_thumbnails(data, &header);

    let mut layers = Vec::with_capacity(header.layer_count as usize);
    let mut per_layer_settings = Vec::with_capacity(header.layer_count as usize);
    for index in 0..header.layer_count as usize {
        let def_offset = header.layers_offset as usize + index * LAYER_DEF_SIZE;
        let def = parse_layer_def(read_range(data, def_offset, LAYER_DEF_SIZE)?)?;
        let data_offset = absolute_offset(def.page_number, def.data_offset);
        layers.push(LayerOffset {
            offset: data_offset,
            size: def.data_size as u64,
        });
        per_layer_settings.push(def.to_per_layer_settings());
    }

    let metadata = FileMetadataBuilder::new()
        .resolution(header.resolution_x, header.resolution_y)
        .display_size_mm(header.display_width, header.display_height)
        .machine_z_mm(header.machine_z)
        .machine_name(machine_name)
        .mirror(header.projector_type == 1, false)
        .layer_height_mm(header.layer_height)
        .layer_count(header.layer_count)
        .bottom_layer_count(header.bottom_layer_count)
        .anti_alias_level(header.anti_alias_level.max(1))
        .encryption_key(header.encryption_key)
        .exposure(header.exposure_seconds, header.bottom_exposure_seconds)
        .light_off_delay(header.light_off_delay, header.bottom_light_off_delay)
        .light_pwm(header.light_pwm, header.bottom_light_pwm)
        .lift(header.lift_height, header.lift_speed)
        .bottom_lift(header.bottom_lift_height, header.bottom_lift_speed)
        .retract(header.retract_speed)
        .statistics(
            header.print_time,
            header.volume_ml,
            header.weight_g,
            header.cost,
        )
        .per_layer_settings(per_layer_settings)
        .thumbnails(thumbnails)
        .build()?;

    Ok((metadata, layers))
}

fn parse_header(data: &[u8]) -> Result<Header, ConvertError> {
    let header = read_range(data, 0, HEADER_SIZE)?;
    Ok(Header {
        magic: read_le_u32(header, 0).unwrap(),
        version: read_le_u32(header, 4).unwrap(),
        layer_height: read_le_f32(header, 8).unwrap(),
        exposure_seconds: read_le_f32(header, 12).unwrap(),
        bottom_exposure_seconds: read_le_f32(header, 16).unwrap(),
        bottom_layer_count: read_le_u32(header, 20).unwrap(),
        resolution_x: read_le_u32(header, 24).unwrap(),
        resolution_y: read_le_u32(header, 28).unwrap(),
        preview_large_offset: read_le_u32(header, 32).unwrap(),
        layers_offset: read_le_u32(header, 36).unwrap(),
        layer_count: read_le_u32(header, 40).unwrap(),
        preview_small_offset: read_le_u32(header, 44).unwrap(),
        print_time: read_le_u32(header, 48).unwrap(),
        projector_type: read_le_u32(header, 52).unwrap(),
        light_pwm: read_le_u16(header, 60).unwrap(),
        bottom_light_pwm: read_le_u16(header, 62).unwrap(),
        display_width: read_le_f32(header, 76).unwrap(),
        display_height: read_le_f32(header, 80).unwrap(),
        machine_z: read_le_f32(header, 84).unwrap(),
        encryption_key: read_le_u32(header, 88).unwrap(),
        bottom_light_off_delay: read_le_f32(header, 92).unwrap(),
        light_off_delay: read_le_f32(header, 96).unwrap(),
        bottom_lift_height: read_le_f32(header, 108).unwrap(),
        bottom_lift_speed: read_le_f32(header, 112).unwrap(),
        lift_height: read_le_f32(header, 116).unwrap(),
        lift_speed: read_le_f32(header, 120).unwrap(),
        retract_speed: read_le_f32(header, 124).unwrap(),
        volume_ml: read_le_f32(header, 128).unwrap(),
        weight_g: read_le_f32(header, 132).unwrap(),
        cost: read_le_f32(header, 136).unwrap(),
        machine_name_offset: read_le_u32(header, 144).unwrap(),
        machine_name_size: read_le_u32(header, 148).unwrap(),
        anti_alias_level: read_le_u32(header, 184).unwrap(),
    })
}

fn parse_layer_def(data: &[u8]) -> Result<LayerDef, ConvertError> {
    let def = read_range(data, 0, LAYER_DEF_SIZE)?;
    Ok(LayerDef {
        position_z: read_le_f32(def, 0).unwrap(),
        exposure_time: read_le_f32(def, 4).unwrap(),
        data_offset: read_le_u32(def, 12).unwrap(),
        data_size: read_le_u32(def, 16).unwrap(),
        page_number: read_le_u32(def, 20).unwrap(),
    })
}

impl LayerDef {
    fn to_per_layer_settings(&self) -> PerLayerSettings {
        PerLayerSettings {
            position_z: self.position_z,
            exposure_time: Some(self.exposure_time),
            lift_height: None,
            lift_speed: None,
            retract_speed: None,
            light_pwm: None,
        }
    }
}

fn parse_thumbnails(data: &[u8], header: &Header) -> Vec<Thumbnail> {
    let mut thumbnails = Vec::new();
    for offset in [header.preview_small_offset, header.preview_large_offset] {
        if offset == 0 {
            continue;
        }
        if let Some(thumbnail) = parse_thumbnail(data, offset as usize) {
            thumbnails.push(thumbnail);
        }
    }
    thumbnails
}

fn parse_thumbnail(data: &[u8], offset: usize) -> Option<Thumbnail> {
    let preview = parse_preview(read_range(data, offset, PREVIEW_SIZE).ok()?).ok()?;
    let image = read_range(
        data,
        preview.image_offset as usize,
        preview.image_length as usize,
    )
    .ok()?;
    ctb::preview::decode_rgb15_rle(image, preview.width, preview.height)
        .ok()
        .map(|pixels_rgba| Thumbnail {
            width: preview.width,
            height: preview.height,
            pixels_rgba,
        })
}

fn parse_preview(data: &[u8]) -> Result<PreviewDef, ConvertError> {
    let preview = read_range(data, 0, PREVIEW_SIZE)?;
    Ok(PreviewDef {
        width: read_le_u32(preview, 0).unwrap(),
        height: read_le_u32(preview, 4).unwrap(),
        image_offset: read_le_u32(preview, 8).unwrap(),
        image_length: read_le_u32(preview, 12).unwrap(),
    })
}

fn decode_phz_rle_into(
    data: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let pixel_count = (width as usize) * (height as usize);
    output.clear();
    if output.capacity() < pixel_count {
        output.reserve(pixel_count - output.capacity());
    }

    let mut last_color = 0u8;
    for &code in data {
        if output.len() >= pixel_count {
            break;
        }
        if (code & 0x80) != 0 {
            last_color = ((code & 0x7F) << 1) | (code & 1);
            if last_color >= 0xFC {
                last_color = 0xFF;
            }
            output.push(last_color);
        } else {
            let count = (code as usize).min(pixel_count - output.len());
            output.resize(output.len() + count, last_color);
        }
    }

    output.resize(pixel_count, 0);
    Ok(())
}

fn encode_phz_rle_into(pixels: &[u8], output: &mut Vec<u8>) {
    output.clear();
    let mut index = 0;
    while index < pixels.len() {
        let value = pixels[index];
        output.push(encode_phz_color(value));
        index += 1;

        let mut repeated = 0usize;
        while index < pixels.len() && pixels[index] == value && repeated < 0x7f {
            repeated += 1;
            index += 1;
        }
        if repeated > 0 {
            output.push(repeated as u8);
        }
    }
}

fn encode_phz_color(value: u8) -> u8 {
    if value >= 0xfc {
        0xff
    } else {
        0x80 | (value >> 1)
    }
}

fn effective_machine_name(metadata: &FileMetadata) -> &str {
    let name = metadata.printer.machine_name.trim();
    if name.is_empty() {
        "Default"
    } else {
        name
    }
}

fn phz_crypt_buffer(seed: u32, layer_index: u32, data: &mut [u8]) {
    if seed == 0 {
        return;
    }
    let seed = seed % 0x4324;
    let init = seed.wrapping_mul(0x34A3_2231);
    let mut key = (layer_index ^ 0x3FAD_2212)
        .wrapping_mul(seed)
        .wrapping_mul(0x4910_913D);
    let mut key_index = 0u32;

    for byte in data {
        let mask = (key >> (8 * key_index)) as u8;
        *byte ^= mask;
        key_index += 1;
        if (key_index & 3) == 0 {
            key = key.wrapping_add(init);
            key_index = 0;
        }
    }
}

fn absolute_offset(page: u32, offset: u32) -> u64 {
    page as u64 * PAGE_SIZE + offset as u64
}

fn read_string(data: &[u8], offset: usize, len: usize) -> Option<String> {
    if len == 0 || offset.checked_add(len)? > data.len() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&data[offset..offset + len])
            .trim_end_matches('\0')
            .to_string(),
    )
}
