use std::io::Cursor;

use binrw::BinWrite;

use super::header::{
    CtbHeader, CtbLayerDef, CtbLayerDefEx, CtbPreviewHeader, CtbPrintParams, CtbPrintParamsV4,
    CtbResinParameters, CtbSlicerInfo, CTB_PRINT_PARAMS_V4_SIZE, CTB_RESIN_PARAMS_FIXED_SIZE,
    CTB_SLICER_INFO_FIXED_SIZE, CTB_V4_DISCLAIMER_SIZE, CTB_V4_GKTWO_DISCLAIMER_SIZE,
};
use super::MAGIC_CTB;
use crate::{
    common::{thumbnail_rgba_for_size, ConvertError, FileMetadata},
    formats::common::binary::{put_le_u32, read_le_u32},
};

/// CTB v4 disclaimer text emitted for generic Chitubox-compatible files.
///
/// CTB v4 通用 Chitubox-compatible 文件写入的 disclaimer 文本。
const CTB_V4_DISCLAIMER: &str = "Layout and record format for the ctb and cbddlp file types are the copyrighted programs or codes of CBD Technology (China) Inc..The Customer or User shall not in any manner reproduce, distribute, modify, decompile, disassemble, decrypt, extract, reverse engineer, lease, assign, or sublicense the said programs or codes.";
/// GKtwo-specific disclaimer text used by UniFormation CTB v4/v5 profiles.
///
/// UniFormation GKtwo CTB v4/v5 profile 使用的 disclaimer 文本。
const CTB_V4_GKTWO_DISCLAIMER: &str = "Layout and record format for the jxs,ctb and cbddlp file types are the copyrighted programs or codes of CBD Technology (China) Inc.The Customer or User shall not in any manner reproduce, distribute, modify, decompile, disassemble, decrypt, extract, reverse engineer, lease, assign, or sublicense the said programs or codes.";

/// Build a CTB v2 file header (header + previews + print params + layer table) from universal metadata.
///
/// Returns the header bytes. Layer table entries have placeholder offsets.
/// Use [`finalize_ctb_header`] after layer payloads have been encoded.
///
/// 从通用元数据构建 CTB v2 文件头部（头部+预览+打印参数+图层表），图层偏移为占位符。
/// layer payload 编码完成后需要调用 [`finalize_ctb_header`] 回填偏移与大小。
pub fn build_ctb_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    build_ctb_header_with_layer_def_size(metadata, 36)
}

/// Build a CTB file header with a profile-specific layer table entry size.
///
/// A size of 36 writes only the main [`CtbLayerDef`] table. Larger sizes enable
/// CTB v3+ style extended records that are later moved before each layer payload
/// by [`finalize_ctb_header`].
///
/// 使用 profile-specific layer table entry size 构建 CTB header。36 表示只写主
/// [`CtbLayerDef`] table；更大的 size 会启用 CTB v3+ extended records，并在 finalize
/// 时放到每个 layer payload 前面。
pub fn build_ctb_header_with_layer_def_size(
    metadata: &FileMetadata,
    layer_def_size: u32,
) -> Result<Vec<u8>, ConvertError> {
    build_ctb_header_with_profile(metadata, layer_def_size, 2, false)
}

/// Build a CTB header using a concrete Chitubox profile version.
///
/// `version` controls the optional v2 slicer-info block and v4/v5 extensions;
/// `is_gktwo` selects the GKtwo disclaimer size/text required by that profile.
///
/// 使用具体 Chitubox profile version 构建 CTB header。`version` 决定 v2 slicer-info
/// 与 v4/v5 extension 是否写入；`is_gktwo` 选择 GKtwo profile 要求的 disclaimer。
pub fn build_ctb_header_with_profile(
    metadata: &FileMetadata,
    layer_def_size: u32,
    version: u32,
    is_gktwo: bool,
) -> Result<Vec<u8>, ConvertError> {
    let header_size = 128u32;
    let preview_header_size = 32u32;
    let layer_def_table_size = layer_def_size.max(36);
    let uses_layer_def_ex = layer_def_table_size > 36;
    let layer_def_ex_size = if uses_layer_def_ex { 84 } else { 0 };

    // Encode preview images if thumbnails are available
    let (large_preview_data, small_preview_data) = encode_preview_pair(&metadata.thumbnails);

    // Calculate layout offsets:
    // [Header 128B] [Large Preview?] [Small Preview?] [PrintParams 60B] [Layer Table]
    let mut offset = header_size;

    let preview_large_offset = if !large_preview_data.is_empty() {
        let off = offset;
        offset += preview_header_size + large_preview_data.len() as u32;
        off
    } else {
        0
    };

    let preview_small_offset = if !small_preview_data.is_empty() {
        let off = offset;
        offset += preview_header_size + small_preview_data.len() as u32;
        off
    } else {
        0
    };

    let (print_params_offset, print_params_size) = if version >= 2 {
        let off = offset;
        let size = 60u32;
        offset += size;
        (off, size)
    } else {
        (0, 0)
    };

    let machine_name = effective_machine_name(metadata);
    let machine_name_bytes = machine_name.as_bytes();
    let disclaimer = if is_gktwo {
        CTB_V4_GKTWO_DISCLAIMER.as_bytes()
    } else {
        CTB_V4_DISCLAIMER.as_bytes()
    };
    let (
        slicer_info_offset,
        slicer_info_size,
        slicer_machine_name_address,
        disclaimer_address,
        print_params_v4_address,
        resin_parameters_address,
    ) = if version >= 2 {
        let slicer_info_offset = offset;
        let slicer_info_size = CTB_SLICER_INFO_FIXED_SIZE;
        let slicer_machine_name_address = slicer_info_offset + slicer_info_size;
        offset += slicer_info_size + machine_name_bytes.len() as u32;
        let (disclaimer_address, print_params_v4_address, resin_parameters_address) =
            if version >= 4 {
                let disclaimer_address = offset;
                offset += disclaimer.len() as u32;
                let print_params_v4_address = offset;
                offset += CTB_PRINT_PARAMS_V4_SIZE;
                let resin_parameters_address = if version >= 5 {
                    let resin_parameters_address = offset;
                    offset += resin_parameters_size(machine_name_bytes);
                    resin_parameters_address
                } else {
                    0
                };
                (
                    disclaimer_address,
                    print_params_v4_address,
                    resin_parameters_address,
                )
            } else {
                (0, 0, 0)
            };
        (
            slicer_info_offset,
            slicer_info_size,
            slicer_machine_name_address,
            disclaimer_address,
            print_params_v4_address,
            resin_parameters_address,
        )
    } else {
        (0, 0, 0, 0, 0, 0)
    };

    let layer_table_offset = offset;
    let layer_table_length = 36 * metadata.slicing.layer_count;
    let layer_data_offset = layer_table_offset + layer_table_length;

    let header_capacity = if uses_layer_def_ex {
        layer_table_offset + layer_table_length + layer_def_ex_size * metadata.slicing.layer_count
    } else {
        layer_table_offset + layer_table_length
    };

    let per_layer_settings = if version >= 2 {
        (version * 0x10).min(u8::MAX as u32) as u8
    } else {
        0
    };

    let software_version = match version {
        0..=3 => 0x0106_0300,
        4 => 0x0109_0000,
        _ => 0x0200_0000,
    };

    let print_params = if version >= 2 {
        Some(CtbPrintParams {
            bottom_lift_height: metadata.motion.bottom_lift_height,
            bottom_lift_speed: metadata.motion.bottom_lift_speed,
            // UVTools v4+ stores first stage plus second stage in PrintParameters.
            lift_height: if version >= 4 {
                metadata.motion.lift_height + metadata.motion.lift_height2
            } else {
                metadata.motion.lift_height
            },
            lift_speed: metadata.motion.lift_speed,
            retract_speed: metadata.motion.retract_speed,
            volume_ml: metadata.stats.volume_ml,
            weight_g: metadata.stats.weight_g,
            cost_dollars: metadata.stats.cost,
            bottom_light_off_delay: metadata.exposure.bottom_light_off_delay,
            light_off_delay: metadata.exposure.light_off_delay,
            bottom_layer_count: metadata.slicing.bottom_layer_count,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            _pad4: if version <= 2 { 0x1234 } else { 0 },
        })
    } else {
        None
    };

    let slicer_info = if version >= 2 {
        Some(CtbSlicerInfo {
            bottom_lift_height2: 0.0,
            bottom_lift_speed2: 0.0,
            lift_height2: metadata.motion.lift_height2,
            lift_speed2: metadata.motion.lift_speed2,
            retract_height2: metadata.motion.retract_height2,
            retract_speed2: metadata.motion.retract_speed2,
            rest_time_after_lift: metadata.waits.after_lift,
            machine_name_address: slicer_machine_name_address,
            machine_name_size: machine_name_bytes.len() as u32,
            anti_alias_flag: 0x0F,
            padding: 0,
            per_layer_settings,
            modified_timestamp_minutes: 0,
            anti_alias_level: metadata.slicing.anti_alias_level.max(1),
            software_version,
            rest_time_after_retract: metadata.waits.before_cure,
            rest_time_after_lift2: metadata.waits.after_lift,
            transition_layer_count: metadata.slicing.transition_layer_count,
            print_parameters_v4_address: print_params_v4_address,
            padding2: 0,
            padding3: 0,
        })
    } else {
        None
    };

    let print_params_v4 = if version >= 4 {
        Some(CtbPrintParamsV4 {
            bottom_retract_speed: metadata.motion.retract_speed,
            bottom_retract_speed2: metadata.motion.retract_speed2,
            padding1: 0,
            four1: 4.0,
            padding2: 0,
            four2: 4.0,
            rest_time_after_retract: metadata.waits.before_cure,
            rest_time_after_lift: metadata.waits.after_lift,
            rest_time_before_lift: metadata.waits.after_cure,
            bottom_retract_height2: metadata.motion.retract_height2,
            unknown1: 0.0,
            unknown2: 0,
            unknown3: 5,
            last_layer_index: metadata.slicing.layer_count.saturating_sub(1),
            padding3: 0,
            padding4: 0,
            padding5: 0,
            padding6: 0,
            disclaimer_address,
            disclaimer_length: if is_gktwo {
                CTB_V4_GKTWO_DISCLAIMER_SIZE
            } else {
                CTB_V4_DISCLAIMER_SIZE
            },
            resin_parameters_address,
            reserved: [0; super::header::CTB_V4_RESERVED_SIZE],
        })
    } else {
        None
    };

    let resin = if version >= 5 {
        let resin_density = if metadata.stats.volume_ml > 0.0 && metadata.stats.weight_g > 0.0 {
            metadata.stats.weight_g / metadata.stats.volume_ml
        } else {
            1.1
        };
        Some(CtbResinParameters {
            padding1: 0,
            resin_color_b: 255,
            resin_color_g: 255,
            resin_color_r: 255,
            resin_color_a: 255,
            machine_name_address: resin_parameters_address + CTB_RESIN_PARAMS_FIXED_SIZE,
            resin_type_length: 0,
            resin_type_address: resin_parameters_address + CTB_RESIN_PARAMS_FIXED_SIZE,
            resin_name_length: 0,
            resin_name_address: resin_parameters_address + CTB_RESIN_PARAMS_FIXED_SIZE,
            machine_name_length: machine_name_bytes.len() as u32,
            resin_density,
            padding2: 0,
        })
    } else {
        None
    };

    let header = CtbHeader {
        magic: MAGIC_CTB,
        version,
        bed_size_x: metadata.printer.display_width_mm,
        bed_size_y: metadata.printer.display_height_mm,
        bed_size_z: metadata.printer.machine_z_mm,
        _reserved1: 0,
        _reserved2: 0,
        total_height_mm: metadata.slicing.layer_height_mm * metadata.slicing.layer_count as f32,
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_seconds: metadata.exposure.exposure_time,
        bottom_exposure_seconds: metadata.exposure.bottom_exposure_time,
        light_off_delay: metadata.exposure.light_off_delay,
        bottom_layer_count: metadata.slicing.bottom_layer_count,
        resolution_x: metadata.printer.resolution_x,
        resolution_y: metadata.printer.resolution_y,
        preview_large_offset,
        layer_table_offset,
        layer_count: metadata.slicing.layer_count,
        preview_small_offset,
        print_time_sec: metadata.stats.print_time_sec,
        projector_type: u32::from(metadata.printer.mirror_x),
        print_params_offset,
        print_params_size,
        anti_alias_level: 1,
        light_pwm: metadata.exposure.light_pwm,
        bottom_light_pwm: metadata.exposure.bottom_light_pwm,
        encryption_key: metadata.slicing.encryption_key,
        slicer_info_offset,
        slicer_info_size,
        _reserved3: 0,
        _reserved4: 0,
        _reserved5: 0,
        _reserved6: 0,
    };

    let mut buf = Vec::with_capacity(header_capacity as usize);
    let mut cursor = Cursor::new(&mut buf);

    header
        .write(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(e.to_string()))?;

    // Write large preview if present
    if !large_preview_data.is_empty() {
        let ph = CtbPreviewHeader {
            resolution_x: large_preview_size().0,
            resolution_y: large_preview_size().1,
            image_offset: preview_large_offset + preview_header_size,
            image_length: large_preview_data.len() as u32,
            _reserved1: 0,
            _reserved2: 0,
            _reserved3: 0,
            _reserved4: 0,
        };
        ph.write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
        let buf_inner = cursor.into_inner();
        buf_inner.extend_from_slice(&large_preview_data);
        cursor = Cursor::new(buf_inner);
        cursor.set_position(cursor.get_ref().len() as u64);
    }

    // Write small preview if present
    if !small_preview_data.is_empty() {
        let ph = CtbPreviewHeader {
            resolution_x: small_preview_size().0,
            resolution_y: small_preview_size().1,
            image_offset: preview_small_offset + preview_header_size,
            image_length: small_preview_data.len() as u32,
            _reserved1: 0,
            _reserved2: 0,
            _reserved3: 0,
            _reserved4: 0,
        };
        ph.write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
        let buf_inner = cursor.into_inner();
        buf_inner.extend_from_slice(&small_preview_data);
        cursor = Cursor::new(buf_inner);
        cursor.set_position(cursor.get_ref().len() as u64);
    }

    if let Some(print_params) = print_params {
        print_params
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
    }

    if let Some(slicer_info) = slicer_info {
        slicer_info
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
        cursor.get_mut().extend_from_slice(machine_name_bytes);
        cursor.set_position(cursor.get_ref().len() as u64);

        if version >= 4 {
            cursor.get_mut().extend_from_slice(disclaimer);
            cursor.set_position(cursor.get_ref().len() as u64);
        }
    }

    if let Some(print_params_v4) = print_params_v4 {
        print_params_v4
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
    }

    if let Some(resin) = resin {
        resin
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
        cursor.get_mut().extend_from_slice(machine_name_bytes);
        cursor.set_position(cursor.get_ref().len() as u64);
    }

    // Write placeholder layer defs (to be filled in after encoding all layers)
    for i in 0..metadata.slicing.layer_count {
        let placeholder = CtbLayerDef {
            position_z: metadata.slicing.layer_height_mm * (i + 1) as f32,
            exposure_time: if i < metadata.slicing.bottom_layer_count {
                metadata.exposure.bottom_exposure_time
            } else {
                metadata.exposure.exposure_time
            },
            light_off_seconds: metadata.exposure.light_off_delay,
            data_address: 0, // to be filled
            data_size: 0,    // to be filled
            page_number: 0,
            table_size: layer_def_table_size,
            _reserved1: 0,
            _reserved2: 0,
        };
        placeholder
            .write(&mut cursor)
            .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
    }

    if uses_layer_def_ex {
        for i in 0..metadata.slicing.layer_count {
            let layer_def = CtbLayerDef {
                position_z: metadata.slicing.layer_height_mm * (i + 1) as f32,
                exposure_time: if i < metadata.slicing.bottom_layer_count {
                    metadata.exposure.bottom_exposure_time
                } else {
                    metadata.exposure.exposure_time
                },
                light_off_seconds: metadata.exposure.light_off_delay,
                data_address: 0,
                data_size: 0,
                page_number: 0,
                table_size: layer_def_table_size,
                _reserved1: 0,
                _reserved2: 0,
            };
            let layer_def_ex = CtbLayerDefEx {
                total_size: 0,
                lift_height: metadata.motion.lift_height
                    + if version >= 4 {
                        metadata.motion.lift_height2
                    } else {
                        0.0
                    },
                lift_speed: metadata.motion.lift_speed,
                lift_height2: if version >= 4 {
                    metadata.motion.lift_height2
                } else {
                    0.0
                },
                lift_speed2: if version >= 4 {
                    metadata.motion.lift_speed2
                } else {
                    0.0
                },
                retract_speed: metadata.motion.retract_speed,
                retract_height2: if version >= 4 {
                    metadata.motion.retract_height2
                } else {
                    0.0
                },
                retract_speed2: if version >= 4 {
                    metadata.motion.retract_speed2
                } else {
                    0.0
                },
                rest_time_before_lift: if version >= 4 {
                    metadata.waits.after_cure
                } else {
                    0.0
                },
                rest_time_after_lift: if version >= 4 {
                    metadata.waits.after_lift
                } else {
                    0.0
                },
                rest_time_after_retract: if version >= 4 {
                    metadata.waits.before_cure
                } else {
                    0.0
                },
                light_pwm: metadata.exposure.bottom_light_pwm as f32,
            };
            let expected_position = layer_data_offset + i * layer_def_ex_size;
            cursor.set_position(expected_position as u64);
            layer_def
                .write(&mut cursor)
                .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
            layer_def_ex
                .write(&mut cursor)
                .map_err(|e| ConvertError::BinrwError(e.to_string()))?;
        }
    }

    Ok(buf)
}

fn effective_machine_name(metadata: &FileMetadata) -> &str {
    let trimmed = metadata.printer.machine_name.trim();
    if trimmed.is_empty() {
        "Default"
    } else {
        trimmed
    }
}

fn resin_parameters_size(machine_name: &[u8]) -> u32 {
    CTB_RESIN_PARAMS_FIXED_SIZE + machine_name.len() as u32
}

/// Patch a CTB header with actual encoded layer offsets and sizes.
///
/// `header_bytes` must be produced by one of the CTB header builders in this
/// module. Layer payloads are assumed to be concatenated immediately after the
/// finalized header. For v3+ extended records, this also copies the prepared
/// `LayerDefEx` template before each payload and updates both the main table and
/// the extended record.
///
/// 使用实际 encoded layer offsets 与 sizes 回填 CTB header。`header_bytes` 必须来自本
/// 模块的 header builder；layer payloads 默认紧跟在 finalized header 后。对于 v3+
/// extended records，这里还会把准备好的 `LayerDefEx` template 复制到每个 payload 前，
/// 并同步更新主表与扩展记录。
pub fn finalize_ctb_header(
    header_bytes: &[u8],
    layer_sizes: &[u32],
) -> Result<Vec<u8>, ConvertError> {
    let mut buf = header_bytes.to_vec();
    let layer_count = layer_sizes.len() as u32;

    // Read layer_table_offset from header at offset 0x40 (4 bytes LE)
    if buf.len() < 0x44 {
        return Err(ConvertError::BufferTooSmall {
            need: 0x44,
            got: buf.len(),
        });
    }
    let layer_table_offset = read_le_u32(&buf, 0x40)?;
    let table_size = if (layer_table_offset as usize) + 28 <= buf.len() {
        read_le_u32(&buf, layer_table_offset as usize + 24)?.max(36)
    } else {
        36
    };
    let has_layer_def_ex = table_size > 36;
    let main_layer_def_size = 36u32;
    let layer_def_ex_size = if has_layer_def_ex { table_size } else { 0 };

    // UVTools keeps the main table at 36 bytes per layer; v3+ writes one
    // LayerDefEx block immediately before each layer payload.
    let data_start = layer_table_offset + main_layer_def_size * layer_count;
    let layer_def_ex_templates = if has_layer_def_ex {
        let mut templates = Vec::with_capacity(layer_count as usize);
        for i in 0..layer_count {
            let placeholder_ex_offset = (data_start + i * layer_def_ex_size) as usize;
            if placeholder_ex_offset + layer_def_ex_size as usize > buf.len() {
                return Err(ConvertError::BufferTooSmall {
                    need: placeholder_ex_offset + layer_def_ex_size as usize,
                    got: buf.len(),
                });
            }
            templates.push(
                buf[placeholder_ex_offset..placeholder_ex_offset + layer_def_ex_size as usize]
                    .to_vec(),
            );
        }
        templates
    } else {
        Vec::new()
    };
    let mut current_offset = data_start;

    for i in 0..layer_count {
        let def_offset = (layer_table_offset + i * main_layer_def_size) as usize;
        // data_address is at byte 12 within each CtbLayerDef
        let addr_offset = def_offset + 12;
        // data_size is at byte 16 within each CtbLayerDef
        let size_offset = def_offset + 16;
        let table_size_offset = def_offset + 24;

        if table_size_offset + 4 > buf.len() {
            return Err(ConvertError::BufferTooSmall {
                need: table_size_offset + 4,
                got: buf.len(),
            });
        }

        if has_layer_def_ex {
            let ex_offset = current_offset as usize;
            let payload_offset = current_offset + layer_def_ex_size;
            if ex_offset + layer_def_ex_size as usize > buf.len() {
                buf.resize(ex_offset + layer_def_ex_size as usize, 0);
            }
            buf[ex_offset..ex_offset + layer_def_ex_size as usize]
                .copy_from_slice(&layer_def_ex_templates[i as usize]);
            let ex_total_size = layer_def_ex_size + layer_sizes[i as usize];
            put_le_u32(&mut buf, ex_offset + 12, payload_offset)?;
            put_le_u32(&mut buf, ex_offset + 16, layer_sizes[i as usize])?;
            put_le_u32(&mut buf, ex_offset + 24, table_size)?;
            put_le_u32(&mut buf, ex_offset + 36, ex_total_size)?;
            put_le_u32(&mut buf, addr_offset, payload_offset)?;
            current_offset = payload_offset;
        } else {
            put_le_u32(&mut buf, addr_offset, current_offset)?;
        }
        put_le_u32(&mut buf, size_offset, layer_sizes[i as usize])?;
        put_le_u32(&mut buf, table_size_offset, table_size)?;
        current_offset += layer_sizes[i as usize];
    }

    Ok(buf)
}

// CTB standard preview sizes
fn large_preview_size() -> (u32, u32) {
    (400, 300)
}
fn small_preview_size() -> (u32, u32) {
    (200, 125)
}

/// Encode thumbnails from metadata into RGB15 RLE for CTB large and small previews.
/// Returns (large_rle_data, small_rle_data) — empty Vec if no thumbnails.
fn encode_preview_pair(thumbnails: &[crate::common::Thumbnail]) -> (Vec<u8>, Vec<u8>) {
    if thumbnails.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let large = encode_preview_for_size(thumbnails, large_preview_size().0, large_preview_size().1);
    let small = encode_preview_for_size(thumbnails, small_preview_size().0, small_preview_size().1);

    (large, small)
}

fn encode_preview_for_size(
    thumbnails: &[crate::common::Thumbnail],
    target_w: u32,
    target_h: u32,
) -> Vec<u8> {
    let rgba = thumbnail_rgba_for_size(thumbnails, target_w, target_h);
    if rgba.is_empty() {
        return Vec::new();
    }

    super::preview::encode_rgb15_rle(&rgba, target_w, target_h).unwrap_or_default()
}
