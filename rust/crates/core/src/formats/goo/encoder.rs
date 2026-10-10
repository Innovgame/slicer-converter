use std::io::Cursor;

use binrw::BinWrite;

use super::header::{
    GooHeader, GooLayerDef, BIG_PREVIEW_H, BIG_PREVIEW_SIZE, BIG_PREVIEW_W, SMALL_PREVIEW_H,
    SMALL_PREVIEW_SIZE, SMALL_PREVIEW_W,
};
use crate::common::{thumbnail_rgba_for_size, ConvertError, FileMetadata, LayerEncodeMetadata};
use crate::formats::common::binary::put_be_u32;

/// Build a complete GOO file header from universal [`FileMetadata`].
///
/// The returned header already contains fixed-size preview buffers and has
/// `layer_def_address` patched to the byte immediately after the header. Layer
/// records and payloads are appended separately.
///
/// 从通用 [`FileMetadata`] 构建完整 GOO 文件头部字节流。返回值已经包含固定尺寸 preview
/// buffers，并把 `layer_def_address` patch 到 header 后第一个字节；layer records 与
/// payloads 由后续流程追加。
pub fn build_goo_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let mut machine_name = [0u8; 32];
    let name_bytes = metadata.printer.machine_name.as_bytes();
    let copy_len = name_bytes.len().min(32);
    machine_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

    let header = GooHeader {
        version: *super::VERSION_STRING,
        magic: *super::MAGIC_BYTES,
        software_name: padded_str(b"slicer-converter", 32),
        software_version: padded_str(b"0.1.0", 24),
        file_create_time: padded_str(b"2024-01-01 00:00:00", 24),
        machine_name,
        machine_type: padded_str(b"DLP", 32),
        profile_name: padded_str(b"default", 32),
        anti_aliasing_level: metadata
            .slicing
            .anti_alias_level
            .max(1)
            .min(u16::MAX as u32) as u16,
        grey_level: 1,
        blur_level: 0,
        small_preview_565: find_and_encode_preview(
            &metadata.thumbnails,
            SMALL_PREVIEW_W,
            SMALL_PREVIEW_H,
            SMALL_PREVIEW_SIZE,
        ),
        big_preview_565: find_and_encode_preview(
            &metadata.thumbnails,
            BIG_PREVIEW_W,
            BIG_PREVIEW_H,
            BIG_PREVIEW_SIZE,
        ),
        layer_count: metadata.slicing.layer_count,
        resolution_x: metadata.printer.resolution_x as u16,
        resolution_y: metadata.printer.resolution_y as u16,
        mirror_x: metadata.printer.mirror_x,
        mirror_y: metadata.printer.mirror_y,
        display_width_mm: metadata.printer.display_width_mm,
        display_height_mm: metadata.printer.display_height_mm,
        machine_z_mm: metadata.printer.machine_z_mm,
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_time: metadata.exposure.exposure_time,
        delay_mode: goo_delay_mode(metadata),
        light_off_delay: metadata.exposure.light_off_delay,
        bottom_wait_time_after_cure: metadata.waits.after_cure,
        bottom_wait_time_after_lift: metadata.waits.after_lift,
        bottom_wait_time_before_cure: metadata.waits.before_cure,
        wait_time_after_cure: metadata.waits.after_cure,
        wait_time_after_lift: metadata.waits.after_lift,
        wait_time_before_cure: metadata.waits.before_cure,
        bottom_exposure_time: metadata.exposure.bottom_exposure_time,
        bottom_layer_count: metadata.slicing.bottom_layer_count,
        bottom_lift_height: metadata.motion.bottom_lift_height,
        bottom_lift_speed: metadata.motion.bottom_lift_speed,
        lift_height: metadata.motion.lift_height,
        lift_speed: metadata.motion.lift_speed,
        bottom_retract_height: first_stage_retract_height(
            metadata.motion.bottom_lift_height,
            metadata.motion.lift_height2,
            metadata.motion.retract_height2,
        ),
        bottom_retract_speed: metadata.motion.retract_speed,
        retract_height: first_stage_retract_height(
            metadata.motion.lift_height,
            metadata.motion.lift_height2,
            metadata.motion.retract_height2,
        ),
        retract_speed: metadata.motion.retract_speed,
        bottom_lift_height2: metadata.motion.lift_height2,
        bottom_lift_speed2: metadata.motion.lift_speed2,
        lift_height2: metadata.motion.lift_height2,
        lift_speed2: metadata.motion.lift_speed2,
        bottom_retract_height2: metadata.motion.retract_height2,
        bottom_retract_speed2: metadata.motion.retract_speed2,
        retract_height2: metadata.motion.retract_height2,
        retract_speed2: metadata.motion.retract_speed2,
        bottom_light_pwm: metadata.exposure.bottom_light_pwm,
        light_pwm: metadata.exposure.light_pwm,
        per_layer_settings: !metadata.per_layer_settings.is_empty(),
        print_time_sec: metadata.stats.print_time_sec,
        volume_mm3: metadata.stats.volume_ml * 1000.0,
        material_grams: metadata.stats.weight_g,
        material_cost: metadata.stats.cost,
        currency_symbol: *b"$\0\0\0\0\0\0\0",
        layer_def_address: 0, // will be patched
        gray_scale_level: 1,
        transition_layer_count: metadata.slicing.transition_layer_count as u16,
    };

    let mut buf = header.to_bytes();

    // Patch layer_def_address: it's 7 bytes from the end of the header
    // (layer_def_address(4) + gray_scale_level(1) + transition_layer_count(2))
    let header_size = buf.len() as u32;
    let lda_offset = buf.len() - 7;
    put_be_u32(&mut buf, lda_offset, header_size)?;

    Ok(buf)
}

/// Build a GOO layer definition plus RLE data block for one layer.
///
/// This legacy convenience path uses fixed default motion values. New export
/// paths should prefer [`build_goo_layer_with_metadata`] so bottom/normal layer
/// settings are preserved.
///
/// 构建单个图层的 GOO layer definition + RLE data block。该 legacy convenience path
/// 使用固定默认运动参数；新的 export path 应优先使用 [`build_goo_layer_with_metadata`]，
/// 以保留 bottom/normal layer settings。
pub fn build_goo_layer(
    layer_index: u32,
    layer_height_mm: f32,
    exposure_time: f32,
    rle_data: &[u8],
) -> Result<Vec<u8>, ConvertError> {
    let layer_def = GooLayerDef {
        pause_flag: 0,
        pause_position_z: 0.0,
        position_z: (layer_index + 1) as f32 * layer_height_mm,
        exposure_time,
        light_off_delay: 0.0,
        wait_time_after_cure: 0.0,
        wait_time_after_lift: 0.0,
        wait_time_before_cure: 0.0,
        lift_height: 5.0,
        lift_speed: 65.0,
        lift_height2: 0.0,
        lift_speed2: 0.0,
        retract_height: 5.0,
        retract_speed: 150.0,
        retract_height2: 0.0,
        retract_speed2: 0.0,
        light_pwm: 255,
        delimiter: super::DELIMITER,
        data_length: rle_data.len() as u32,
    };

    serialize_goo_layer(layer_def, rle_data)
}

/// Build a GOO-family layer block from file-level layer settings.
///
/// The layer definition is derived from universal metadata and switches exposure,
/// lift, and PWM values based on whether `layer_index` is inside the bottom-layer
/// range.
///
/// 根据 file-level layer settings 构建 GOO 系列图层块，并按 `layer_index` 是否属于
/// bottom-layer range 自动切换 exposure、lift 与 PWM 参数。
pub fn build_goo_layer_with_metadata(
    metadata: LayerEncodeMetadata<'_>,
    layer_index: u32,
    rle_data: &[u8],
) -> Result<Vec<u8>, ConvertError> {
    let is_bottom_layer = layer_index < metadata.slicing.bottom_layer_count;
    let layer_def = GooLayerDef {
        pause_flag: 0,
        pause_position_z: metadata.printer.machine_z_mm,
        position_z: (layer_index + 1) as f32 * metadata.slicing.layer_height_mm,
        exposure_time: if is_bottom_layer {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        },
        light_off_delay: if is_bottom_layer {
            metadata.exposure.bottom_light_off_delay
        } else {
            metadata.exposure.light_off_delay
        },
        wait_time_after_cure: metadata.waits.after_cure,
        wait_time_after_lift: metadata.waits.after_lift,
        wait_time_before_cure: metadata.waits.before_cure,
        lift_height: if is_bottom_layer {
            metadata.motion.bottom_lift_height
        } else {
            metadata.motion.lift_height
        },
        lift_speed: if is_bottom_layer {
            metadata.motion.bottom_lift_speed
        } else {
            metadata.motion.lift_speed
        },
        lift_height2: metadata.motion.lift_height2,
        lift_speed2: metadata.motion.lift_speed2,
        retract_height: first_stage_retract_height(
            if is_bottom_layer {
                metadata.motion.bottom_lift_height
            } else {
                metadata.motion.lift_height
            },
            metadata.motion.lift_height2,
            metadata.motion.retract_height2,
        ),
        retract_speed: metadata.motion.retract_speed,
        retract_height2: metadata.motion.retract_height2,
        retract_speed2: metadata.motion.retract_speed2,
        light_pwm: if is_bottom_layer {
            metadata.exposure.bottom_light_pwm
        } else {
            metadata.exposure.light_pwm
        },
        delimiter: super::DELIMITER,
        data_length: rle_data.len() as u32,
    };

    serialize_goo_layer(layer_def, rle_data)
}

/// In-place variant of [`build_goo_layer_with_metadata`] writing the wrapped
/// layer block (header + RLE + delimiter) into a caller-provided buffer.
///
/// Used by hot export paths with thread-local buffers to avoid allocating a new
/// layer wrapper for each layer.
///
/// [`build_goo_layer_with_metadata`] 的就地写入版本，向调用方提供的 buffer 写入
/// layer block（header + RLE + delimiter）。热路径通过 thread-local buffers 复用分配。
pub fn build_goo_layer_with_metadata_into(
    metadata: LayerEncodeMetadata<'_>,
    layer_index: u32,
    rle_data: &[u8],
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    let rle_start = begin_goo_layer_with_metadata_into(metadata, layer_index, output)?;
    output.extend_from_slice(rle_data);
    finish_goo_layer_with_metadata_in_place(output, rle_start)
}

/// Start a metadata-wrapped GOO layer in `output` and return the offset where
/// RLE bytes should be appended. Call [`finish_goo_layer_with_metadata_in_place`]
/// after writing the RLE payload.
///
/// This split begin/finish API lets callers stream or append encoded RLE bytes
/// directly into the final layer buffer before the `data_length` field is known.
///
/// 在 `output` 中开始写入 metadata-wrapped GOO layer，并返回 RLE bytes 应追加的位置。
/// 调用方写完 RLE payload 后必须调用 [`finish_goo_layer_with_metadata_in_place`]。这个
/// begin/finish API 允许在 `data_length` 尚未知时直接向最终 layer buffer 追加 encoded RLE。
pub fn begin_goo_layer_with_metadata_into(
    metadata: LayerEncodeMetadata<'_>,
    layer_index: u32,
    output: &mut Vec<u8>,
) -> Result<usize, ConvertError> {
    output.clear();
    let is_bottom_layer = layer_index < metadata.slicing.bottom_layer_count;
    let layer_def = GooLayerDef {
        pause_flag: 0,
        pause_position_z: metadata.printer.machine_z_mm,
        position_z: (layer_index + 1) as f32 * metadata.slicing.layer_height_mm,
        exposure_time: if is_bottom_layer {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        },
        light_off_delay: if is_bottom_layer {
            metadata.exposure.bottom_light_off_delay
        } else {
            metadata.exposure.light_off_delay
        },
        wait_time_after_cure: metadata.waits.after_cure,
        wait_time_after_lift: metadata.waits.after_lift,
        wait_time_before_cure: metadata.waits.before_cure,
        lift_height: if is_bottom_layer {
            metadata.motion.bottom_lift_height
        } else {
            metadata.motion.lift_height
        },
        lift_speed: if is_bottom_layer {
            metadata.motion.bottom_lift_speed
        } else {
            metadata.motion.lift_speed
        },
        lift_height2: metadata.motion.lift_height2,
        lift_speed2: metadata.motion.lift_speed2,
        retract_height: first_stage_retract_height(
            if is_bottom_layer {
                metadata.motion.bottom_lift_height
            } else {
                metadata.motion.lift_height
            },
            metadata.motion.lift_height2,
            metadata.motion.retract_height2,
        ),
        retract_speed: metadata.motion.retract_speed,
        retract_height2: metadata.motion.retract_height2,
        retract_speed2: metadata.motion.retract_speed2,
        light_pwm: if is_bottom_layer {
            metadata.exposure.bottom_light_pwm
        } else {
            metadata.exposure.light_pwm
        },
        delimiter: super::DELIMITER,
        data_length: 0,
    };

    {
        let mut cursor = Cursor::new(&mut *output);
        layer_def
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
    }
    Ok(output.len())
}

/// Finish a metadata-wrapped GOO layer after appending RLE bytes in-place.
///
/// The function patches `data_length` at the end of the layer definition and
/// appends the GOO layer delimiter. `rle_start` must be the offset returned by
/// [`begin_goo_layer_with_metadata_into`].
///
/// 在调用方原地追加 RLE bytes 后完成 metadata-wrapped GOO layer。该函数会回填 layer
/// definition 末尾的 `data_length` 并追加 GOO layer delimiter；`rle_start` 必须来自
/// [`begin_goo_layer_with_metadata_into`] 的返回值。
pub fn finish_goo_layer_with_metadata_in_place(
    output: &mut Vec<u8>,
    rle_start: usize,
) -> Result<(), ConvertError> {
    if rle_start < 4 {
        return Err(ConvertError::BufferTooSmall {
            need: 4,
            got: rle_start,
        });
    }
    if rle_start > output.len() {
        return Err(ConvertError::BufferTooSmall {
            need: rle_start,
            got: output.len(),
        });
    }

    let rle_len = output.len() - rle_start;
    let data_length: u32 = rle_len.try_into().map_err(|_| {
        ConvertError::RleEncodeError("GOO layer RLE payload is too large".to_string())
    })?;
    put_be_u32(output, rle_start - 4, data_length)?;
    output.extend_from_slice(&super::DELIMITER);
    Ok(())
}

/// `0` means the printer uses light-off delay. `1` means it uses wait times.
///
/// Light-off delay wins when both are present, because UVTools clears light-off
/// delay when it selects wait-time mode. A zero light-off delay with any wait
/// therefore has to select wait-time mode or the printer ignores those waits.
///
/// `0` 表示打印机使用关灯延时，`1` 表示使用等待时间。两者同时存在时优先关灯延时，
/// 因为 UVTools 选择等待时间模式时会清掉关灯延时。关灯延时为 0 且任一等待时间大于 0
/// 时必须选等待时间模式，否则打印机会忽略这些等待。
fn first_stage_retract_height(lift_height: f32, lift_height2: f32, retract_height2: f32) -> f32 {
    // UVTools stores first-stage retract as total lift minus the second retract stage.
    // A zero first stage leaves the speed set but does not return the plate.
    (lift_height + lift_height2 - retract_height2).max(0.0)
}

fn goo_delay_mode(metadata: &FileMetadata) -> u8 {
    if metadata.exposure.light_off_delay > 0.0 || metadata.exposure.bottom_light_off_delay > 0.0 {
        return 0;
    }
    if metadata.waits.before_cure > 0.0
        || metadata.waits.after_cure > 0.0
        || metadata.waits.after_lift > 0.0
    {
        return 1;
    }
    0
}

fn serialize_goo_layer(layer_def: GooLayerDef, rle_data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let mut buf = Vec::with_capacity(70 + rle_data.len() + 2);
    serialize_goo_layer_into(layer_def, rle_data, &mut buf)?;
    Ok(buf)
}

fn serialize_goo_layer_into(
    layer_def: GooLayerDef,
    rle_data: &[u8],
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    output.clear();
    let needed = 70 + rle_data.len() + 2;
    if output.capacity() < needed {
        output.reserve(needed - output.capacity());
    }
    {
        let mut cursor = Cursor::new(&mut *output);
        // Skip past any already-written header bytes; clear() reset len to 0.
        layer_def
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
    }
    output.extend_from_slice(rle_data);
    // Append layer delimiter after RLE data (matches UVtools format)
    output.extend_from_slice(&super::DELIMITER);

    Ok(())
}

/// Pad a byte string to a fixed-size array.
fn padded_str<const N: usize>(s: &[u8], _len: usize) -> [u8; N] {
    let mut out = [0u8; N];
    let copy_len = s.len().min(N);
    out[..copy_len].copy_from_slice(&s[..copy_len]);
    out
}

/// Find a thumbnail matching the target size, or the closest one, and encode as RGB565 BE.
/// Returns zero-filled data if no thumbnails are available.
fn find_and_encode_preview(
    thumbnails: &[crate::common::Thumbnail],
    target_w: u32,
    target_h: u32,
    expected_size: usize,
) -> Vec<u8> {
    let rgba = thumbnail_rgba_for_size(thumbnails, target_w, target_h);
    if !rgba.is_empty() {
        let encoded = super::preview::rgba_to_rgb565_be(&rgba);
        if encoded.len() == expected_size {
            return encoded;
        }
    }

    vec![0u8; expected_size]
}
