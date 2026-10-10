use crate::{
    common::{ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset, Thumbnail},
    formats::{
        common::{
            binary::{
                put_be_f64, put_be_u16, put_be_u32, read_be_f64, read_be_u16, read_be_u32,
                read_range,
            },
            ExportTargetOptions, FormatSpec, HeaderLayout, LayerCodec,
        },
        goo, LayerDecodeContext, LayerEncodeContext,
    },
};

const TAG1: u32 = 2;
const TAG2: u16 = 49;
const HEADER_SIZE: usize = 98;
const LAYER_COUNT_SIZE: usize = 4;
const LAYER_DEF_SIZE: usize = 28;
const PREVIEW_WIDTH: u32 = 224;
const PREVIEW_HEIGHT: u32 = 168;
const PREVIEW_BYTES: usize = (PREVIEW_WIDTH as usize) * (PREVIEW_HEIGHT as usize) * 2;
const XY_PIXEL_SIZE: f64 = 0.04725;
const HEADER_UNKNOWN2: u32 = 42;
const HEADER_UNKNOWN4: u32 = 10;
const LAYER_UNKNOWN1: u32 = 44_944;
const LAYER_UNKNOWN5: u32 = 2_684_702_720;
const PHOTON_S_RESOLUTION_X: u32 = 1440;
const PHOTON_S_RESOLUTION_Y: u32 = 2560;
const DISPLAY_WIDTH: f32 = 68.04;
const DISPLAY_HEIGHT: f32 = 120.96;
const MACHINE_Z: f32 = 165.0;
const MACHINE_NAME: &str = "Anycubic Photon S";

/// Fixed Photon S machine geometry written by the exporter.
///
/// PhotonS 导出器写入的固定机器几何。分辨率不一致时图层会按这个尺寸重采样。
pub(crate) fn export_machine_geometry() -> (u32, u32, f32, f32, &'static str) {
    (
        PHOTON_S_RESOLUTION_X,
        PHOTON_S_RESOLUTION_Y,
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        MACHINE_NAME,
    )
}

pub struct PhotonSHeaderLayout;
pub struct PhotonSLayerCodec;

#[derive(Debug, Clone)]
struct Header {
    tag1: u32,
    tag2: u16,
    layer_height: f64,
    exposure_seconds: f64,
    light_off_delay: f64,
    bottom_exposure_seconds: f64,
    bottom_layer_count: u32,
    lift_height: f64,
    lift_speed: f64,
    retract_speed: f64,
    volume_ml: f64,
    preview_width: u32,
    preview_height: u32,
}

#[derive(Debug, Clone)]
struct LayerDef {
    width: u32,
    height: u32,
    rle_len: usize,
}

impl HeaderLayout for PhotonSHeaderLayout {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        data.len() >= 6
            && read_be_u32(data, 0).is_ok_and(|value| value == TAG1)
            && read_be_u16(data, 4).is_ok_and(|value| value == TAG2)
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
        build_photons_header(metadata)
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

impl LayerCodec for PhotonSLayerCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        let def = parse_layer_def(read_range(data, 0, LAYER_DEF_SIZE)?)?;
        let rle = read_range(data, LAYER_DEF_SIZE, def.rle_len)?;
        let width = if def.width > 0 { def.width } else { ctx.width };
        let height = if def.height > 0 {
            def.height
        } else {
            ctx.height
        };
        decode_photons_rle_into(rle, width, height, output)
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        encode_photons_layer_into(pixels, ctx.width, ctx.height, output)
    }
}

fn build_photons_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let mut data = vec![0u8; HEADER_SIZE + PREVIEW_BYTES + LAYER_COUNT_SIZE];
    put_be_u32(&mut data, 0, TAG1)?;
    put_be_u16(&mut data, 4, TAG2)?;
    put_be_f64(&mut data, 6, XY_PIXEL_SIZE)?;
    put_be_f64(&mut data, 14, metadata.slicing.layer_height_mm as f64)?;
    put_be_f64(&mut data, 22, metadata.exposure.exposure_time as f64)?;
    put_be_f64(&mut data, 30, metadata.exposure.light_off_delay as f64)?;
    put_be_f64(&mut data, 38, metadata.exposure.bottom_exposure_time as f64)?;
    put_be_u32(&mut data, 46, metadata.slicing.bottom_layer_count)?;
    put_be_f64(&mut data, 50, metadata.motion.lift_height as f64)?;
    put_be_f64(&mut data, 58, (metadata.motion.lift_speed / 60.0) as f64)?;
    put_be_f64(&mut data, 66, (metadata.motion.retract_speed / 60.0) as f64)?;
    put_be_f64(&mut data, 74, metadata.stats.volume_ml as f64)?;
    put_be_u32(&mut data, 82, PREVIEW_WIDTH)?;
    put_be_u32(&mut data, 86, HEADER_UNKNOWN2)?;
    put_be_u32(&mut data, 90, PREVIEW_HEIGHT)?;
    put_be_u32(&mut data, 94, HEADER_UNKNOWN4)?;
    put_be_u32(
        &mut data,
        HEADER_SIZE + PREVIEW_BYTES,
        metadata.slicing.layer_count,
    )?;
    Ok(data)
}

fn encode_photons_layer_into(
    pixels: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    if width != PHOTON_S_RESOLUTION_X || height != PHOTON_S_RESOLUTION_Y {
        let normalized = resize_grayscale_nearest(
            pixels,
            width,
            height,
            PHOTON_S_RESOLUTION_X,
            PHOTON_S_RESOLUTION_Y,
        )?;
        return encode_photons_layer_payload_into(
            &normalized,
            PHOTON_S_RESOLUTION_X,
            PHOTON_S_RESOLUTION_Y,
            output,
        );
    }

    encode_photons_layer_payload_into(pixels, width, height, output)
}

fn encode_photons_layer_payload_into(
    pixels: &[u8],
    width: u32,
    height: u32,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    output.resize(LAYER_DEF_SIZE, 0);

    let mut index = 0;
    while index < pixels.len() {
        let value = pixels[index] >= 128;
        let mut run_len = 1usize;
        while index + run_len < pixels.len()
            && (pixels[index + run_len] >= 128) == value
            && run_len < 128
        {
            run_len += 1;
        }
        output.push(encode_photons_run(value, run_len));
        index += run_len;
    }

    let rle_len = output.len() - LAYER_DEF_SIZE;
    put_be_u32(output, 0, LAYER_UNKNOWN1)?;
    put_be_u32(output, 12, width)?;
    put_be_u32(output, 16, height)?;
    put_be_u32(output, 20, ((rle_len as u32) + 4) << 3)?;
    put_be_u32(output, 24, LAYER_UNKNOWN5)?;
    Ok(())
}

fn resize_grayscale_nearest(
    pixels: &[u8],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> Result<Vec<u8>, ConvertError> {
    let expected = (src_width as usize).saturating_mul(src_height as usize);
    if pixels.len() < expected {
        return Err(ConvertError::BufferTooSmall {
            need: expected,
            got: pixels.len(),
        });
    }

    let mut resized = vec![0u8; (dst_width as usize) * (dst_height as usize)];
    for y in 0..dst_height {
        let src_y = (y as u64 * src_height as u64 / dst_height as u64) as u32;
        let src_row = (src_y.min(src_height - 1) * src_width) as usize;
        let dst_row = (y * dst_width) as usize;
        for x in 0..dst_width {
            let src_x = (x as u64 * src_width as u64 / dst_width as u64) as u32;
            resized[dst_row + x as usize] = pixels[src_row + src_x.min(src_width - 1) as usize];
        }
    }
    Ok(resized)
}

fn encode_photons_run(value: bool, run_len: usize) -> u8 {
    let bits = (run_len.saturating_sub(1).min(127)) as u8;
    ((bits & 0x01) << 7)
        | ((bits & 0x02) << 5)
        | ((bits & 0x04) << 3)
        | ((bits & 0x08) << 1)
        | ((bits & 0x10) >> 1)
        | ((bits & 0x20) >> 3)
        | ((bits & 0x40) >> 5)
        | u8::from(value)
}

fn parse_document_header(data: &[u8]) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let header = parse_header(data)?;
    if header.tag1 != TAG1 {
        return Err(ConvertError::InvalidMagic {
            expected: TAG1,
            actual: header.tag1,
        });
    }
    if header.tag2 != TAG2 {
        return Err(ConvertError::BinrwError(format!(
            "PhotonS tag2 mismatch: expected {TAG2}, got {}",
            header.tag2
        )));
    }

    let preview_size = checked_mul_u32(header.preview_width, header.preview_height)?
        .checked_mul(2)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: data.len(),
        })?;
    let preview_offset = HEADER_SIZE;
    let layer_count_offset =
        preview_offset
            .checked_add(preview_size)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
    let layer_count_end =
        layer_count_offset
            .checked_add(LAYER_COUNT_SIZE)
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
    if layer_count_end > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: layer_count_end,
            got: data.len(),
        });
    }

    let layer_count = read_be_u32(data, layer_count_offset).unwrap();
    let mut cursor = layer_count_end;
    let mut layers = Vec::with_capacity(layer_count as usize);
    let mut first_layer_size = None;
    for _ in 0..layer_count {
        let layer_start = cursor;
        let def = parse_layer_def(read_range(data, layer_start, LAYER_DEF_SIZE)?)?;
        first_layer_size.get_or_insert((def.width, def.height));
        cursor = cursor
            .checked_add(LAYER_DEF_SIZE)
            .and_then(|offset| offset.checked_add(def.rle_len))
            .ok_or(ConvertError::BufferTooSmall {
                need: usize::MAX,
                got: data.len(),
            })?;
        if cursor > data.len() {
            return Err(ConvertError::BufferTooSmall {
                need: cursor,
                got: data.len(),
            });
        }
        layers.push(LayerOffset {
            offset: layer_start as u64,
            size: (LAYER_DEF_SIZE + def.rle_len) as u64,
        });
    }

    let (resolution_x, resolution_y) = first_layer_size.unwrap_or((1440, 2560));
    let mut thumbnails = Vec::new();
    let preview_end = preview_offset + preview_size;
    if preview_size > 0 && preview_end <= data.len() {
        if let Ok(pixels_rgba) = goo::preview::decode_rgb565_be(
            &data[preview_offset..preview_end],
            header.preview_width,
            header.preview_height,
        ) {
            thumbnails.push(Thumbnail {
                width: header.preview_width,
                height: header.preview_height,
                pixels_rgba,
            });
        }
    }

    let metadata = FileMetadataBuilder::new()
        .resolution(resolution_x, resolution_y)
        .display_size_mm(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .machine_z_mm(MACHINE_Z)
        .machine_name(MACHINE_NAME)
        .mirror(true, false)
        .layer_height_mm(header.layer_height as f32)
        .layer_count(layer_count)
        .bottom_layer_count(header.bottom_layer_count)
        .exposure(
            header.exposure_seconds as f32,
            header.bottom_exposure_seconds as f32,
        )
        .light_off_delay(header.light_off_delay as f32, header.light_off_delay as f32)
        .lift(header.lift_height as f32, (header.lift_speed * 60.0) as f32)
        .retract((header.retract_speed * 60.0) as f32)
        .statistics(0, header.volume_ml as f32, 0.0, 0.0)
        .thumbnails(thumbnails)
        .build()?;

    Ok((metadata, layers))
}

fn parse_header(data: &[u8]) -> Result<Header, ConvertError> {
    let header = read_range(data, 0, HEADER_SIZE)?;
    Ok(Header {
        tag1: read_be_u32(header, 0).unwrap(),
        tag2: read_be_u16(header, 4).unwrap(),
        layer_height: read_be_f64(header, 14).unwrap(),
        exposure_seconds: read_be_f64(header, 22).unwrap(),
        light_off_delay: read_be_f64(header, 30).unwrap(),
        bottom_exposure_seconds: read_be_f64(header, 38).unwrap(),
        bottom_layer_count: read_be_u32(header, 46).unwrap(),
        lift_height: read_be_f64(header, 50).unwrap(),
        lift_speed: read_be_f64(header, 58).unwrap(),
        retract_speed: read_be_f64(header, 66).unwrap(),
        volume_ml: read_be_f64(header, 74).unwrap(),
        preview_width: read_be_u32(header, 82).unwrap(),
        preview_height: read_be_u32(header, 90).unwrap(),
    })
}

fn parse_layer_def(data: &[u8]) -> Result<LayerDef, ConvertError> {
    let def = read_range(data, 0, LAYER_DEF_SIZE)?;
    let data_size = read_be_u32(def, 20).unwrap();
    let rle_len = data_size
        .checked_shr(3)
        .and_then(|value| value.checked_sub(4))
        .ok_or_else(|| ConvertError::RleDecodeError {
            offset: 20,
            detail: format!("invalid PhotonS layer data size {data_size}"),
        })? as usize;
    Ok(LayerDef {
        width: read_be_u32(def, 12).unwrap(),
        height: read_be_u32(def, 16).unwrap(),
        rle_len,
    })
}

fn decode_photons_rle_into(
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

    for &run in data {
        if output.len() >= pixel_count {
            break;
        }
        let value = if (run & 0x01) != 0 { 255 } else { 0 };
        let len = ((((run & 0x80) >> 7) as usize)
            | (((run & 0x40) >> 5) as usize)
            | (((run & 0x20) >> 3) as usize)
            | (((run & 0x10) >> 1) as usize)
            | (((run & 0x08) << 1) as usize)
            | (((run & 0x04) << 3) as usize)
            | (((run & 0x02) << 5) as usize))
            + 1;
        let count = len.min(pixel_count - output.len());
        output.resize(output.len() + count, value);
    }

    output.resize(pixel_count, 0);
    Ok(())
}

fn checked_mul_u32(left: u32, right: u32) -> Result<usize, ConvertError> {
    (left as usize)
        .checked_mul(right as usize)
        .ok_or(ConvertError::BufferTooSmall {
            need: usize::MAX,
            got: 0,
        })
}
