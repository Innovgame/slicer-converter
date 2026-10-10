use std::io::Cursor;

use binrw::BinWrite;

use crate::{
    common::{ConvertError, FileMetadata},
    formats::common::binary::{put_le_u32, BinaryWriter},
};

use super::header::{
    ExtraTable, FileMark, FileMarkV516, FileMarkV517, FileMarkV518, HeaderTable, HeaderTableV516,
    HeaderTableV517, HeaderTableV518, LayerDef, LayerImageColorTable, MachineTableV516,
    MachineTableV518, ModelTable, Preview2Table, PreviewTable, SoftwareTable, SubLayerDef,
    EXTRA_TABLE_LENGTH_V516, EXTRA_TABLE_SIZE_V516, FILE_MARK_SIZE, FILE_MARK_V516_SIZE,
    FILE_MARK_V517_SIZE, FILE_MARK_V518_SIZE, HEADER_TABLE_LENGTH, HEADER_TABLE_SIZE,
    HEADER_TABLE_V516_LENGTH, HEADER_TABLE_V516_SIZE, HEADER_TABLE_V517_LENGTH,
    HEADER_TABLE_V517_SIZE, HEADER_TABLE_V518_LENGTH, HEADER_TABLE_V518_SIZE,
    LAYERDEF_TABLE_BASE_SIZE, LAYER_IMAGE_COLOR_TABLE_SIZE, MACHINE_TABLE_SIZE_V516,
    MACHINE_TABLE_SIZE_V518, MODEL_TABLE_LENGTH, MODEL_TABLE_SIZE, PREVIEW_HEIGHT, PREVIEW_SIZE,
    PREVIEW_TABLE_SIZE, PREVIEW_WIDTH, SOFTWARE_TABLE_LENGTH, SOFTWARE_TABLE_SIZE,
    SUB_LAYER_DEF_SIZE, SUB_LAYER_DEF_TABLE_BASE_SIZE,
};
use super::AnycubicSpec;

/// Build an Anycubic v1 header with placeholder layer offsets.
///
/// The returned bytes contain the file mark, header table, fixed preview block,
/// and layer-definition table. Layer payload offsets are patched later by
/// [`finalize_anycubic_header_bytes`].
///
/// 构建带 placeholder layer offsets 的 Anycubic v1 header。返回值包含 file mark、
/// header table、固定 preview block 与 layer-definition table；layer payload offsets
/// 稍后由 [`finalize_anycubic_header_bytes`] 回填。
pub(crate) fn build_v1_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    build_v1_header(metadata)
}

/// Build an Anycubic v5.15 header with placeholder layer offsets.
///
/// V5.15 reuses the v1 file mark size and adds the layer image color table.
///
/// 构建带 placeholder layer offsets 的 Anycubic v5.15 header。v5.15 复用 v1 file mark
/// size，并增加 layer image color table。
pub(crate) fn build_v515_header_bytes(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    build_v515_header(metadata)
}

/// Build an Anycubic v5.16 header with placeholder layer offsets.
///
/// V5.16 adds the motion `EXTRA` table and machine table; machine facts come
/// from `specifics` when source metadata is incomplete.
///
/// 构建带 placeholder layer offsets 的 Anycubic v5.16 header。v5.16 增加 motion
/// `EXTRA` table 与 machine table；当源 metadata 不完整时，machine facts 来自 `specifics`。
pub(crate) fn build_v516_header_bytes(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    build_v516_header(metadata, specifics)
}

/// Build an Anycubic v5.17 header with placeholder layer offsets.
///
/// V5.17 extends v5.16 with software and model tables.
///
/// 构建带 placeholder layer offsets 的 Anycubic v5.17 header。v5.17 在 v5.16 基础上增加
/// software 与 model tables。
pub(crate) fn build_v517_header_bytes(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    build_v517_header(metadata, specifics)
}

/// Build an Anycubic v5.18 header with placeholder layer offsets.
///
/// V5.18 adds a sub-layer definition table and a second preview table before the
/// layer image payload region.
///
/// 构建带 placeholder layer offsets 的 Anycubic v5.18 header。v5.18 在 layer image
/// payload region 前增加 sub-layer definition table 与 second preview table。
pub(crate) fn build_v518_header_bytes(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    build_v518_header(metadata, specifics)
}

/// Patch an Anycubic binary header with final layer payload offsets and sizes.
///
/// Layer payloads are assumed to be appended immediately after the header. The
/// function locates the `LAYERDEF` table by mark and updates each 32-byte
/// [`LayerDef`] entry in order.
///
/// 使用最终 layer payload offsets 与 sizes patch Anycubic binary header。layer payloads
/// 默认紧跟在 header 后；该函数通过 `LAYERDEF` mark 定位表，并按顺序更新每个 32-byte
/// [`LayerDef`] entry。
pub(crate) fn finalize_anycubic_header_bytes(
    mut header: Vec<u8>,
    layer_sizes: &[u32],
) -> Result<Vec<u8>, ConvertError> {
    let layerdef_pos = find_table_mark(&header, b"LAYERDEF")
        .ok_or_else(|| ConvertError::BinrwError("LAYERDEF mark not found".into()))?;
    let entries_start = layerdef_pos + 12 + 4 + 4;
    let subimgs_entries_start =
        find_table_mark(&header, b"SUBIMGS").map(|subimgs_pos| subimgs_pos + 12 + 4 + 4 + 4);

    let mut data_offset = header.len() as u32;
    for (i, &size) in layer_sizes.iter().enumerate() {
        let entry_offset = entries_start + i * 32;
        if entry_offset + 8 > header.len() {
            return Err(ConvertError::BinrwError(
                "LayerDef entry out of bounds".into(),
            ));
        }
        put_le_u32(&mut header, entry_offset, data_offset)?;
        put_le_u32(&mut header, entry_offset + 4, size)?;
        if let Some(subimgs_start) = subimgs_entries_start {
            let subimgs_entry_offset = subimgs_start + i * SUB_LAYER_DEF_SIZE as usize;
            if subimgs_entry_offset + 8 > header.len() {
                return Err(ConvertError::BinrwError(
                    "SubLayerDef entry out of bounds".into(),
                ));
            }
            put_le_u32(&mut header, subimgs_entry_offset, data_offset)?;
            put_le_u32(&mut header, subimgs_entry_offset + 4, size)?;
        }
        data_offset += size;
    }

    Ok(header)
}

fn build_v1_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;

    let header_address = FILE_MARK_SIZE;
    let preview_address = header_address + HEADER_TABLE_SIZE;
    let preview_table_size = PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;
    let layer_definition_address = preview_address + preview_table_size;
    let layer_image_address =
        layer_definition_address + LAYERDEF_TABLE_BASE_SIZE + layer_count * LayerDef::BYTE_SIZE;

    let mut buf = Vec::new();

    let file_mark = FileMark {
        mark: *b"ANYCUBIC\x00\x00\x00\x00",
        version: 1,
        number_of_tables: 4,
        header_address,
        software_address: 0,
        preview_address,
        layer_image_color_table_address: 0,
        layer_definition_address,
        extra_address: 0,
        layer_image_address,
    };
    write_struct(&mut buf, &file_mark)?;

    let header = build_v1_header_table(metadata);
    write_struct(&mut buf, &header)?;

    let preview = build_preview_table();
    write_struct(&mut buf, &preview)?;
    buf.resize(buf.len() + PREVIEW_SIZE, 0);

    write_layer_definition_table(&mut buf, metadata)?;

    Ok(buf)
}

fn build_v515_header(metadata: &FileMetadata) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let header_address = FILE_MARK_SIZE;
    let preview_address = header_address + HEADER_TABLE_SIZE;
    let preview_table_size = PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;
    let layer_image_color_table_address = preview_address + preview_table_size;
    let layer_definition_address = layer_image_color_table_address + LAYER_IMAGE_COLOR_TABLE_SIZE;
    let layer_image_address =
        layer_definition_address + LAYERDEF_TABLE_BASE_SIZE + layer_count * LayerDef::BYTE_SIZE;

    let mut buf = Vec::new();

    let file_mark = FileMark {
        mark: *b"ANYCUBIC\x00\x00\x00\x00",
        version: 515,
        number_of_tables: 5,
        header_address,
        software_address: 0,
        preview_address,
        layer_image_color_table_address,
        layer_definition_address,
        extra_address: layer_image_address,
        layer_image_address,
    };
    write_struct(&mut buf, &file_mark)?;

    let header = build_v1_header_table(metadata);
    write_struct(&mut buf, &header)?;

    let preview = build_preview_table();
    write_struct(&mut buf, &preview)?;
    buf.resize(buf.len() + PREVIEW_SIZE, 0);

    let color_table = build_layer_image_color_table(metadata);
    write_struct(&mut buf, &color_table)?;

    write_layer_definition_table(&mut buf, metadata)?;

    Ok(buf)
}

fn build_v516_header(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let header_address = FILE_MARK_V516_SIZE;
    let preview_address = header_address + HEADER_TABLE_V516_SIZE;
    let preview_table_size = PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;
    let layer_image_color_table_address = preview_address + preview_table_size;
    let layer_definition_address = layer_image_color_table_address + LAYER_IMAGE_COLOR_TABLE_SIZE;
    let extra_address =
        layer_definition_address + LAYERDEF_TABLE_BASE_SIZE + layer_count * LayerDef::BYTE_SIZE;
    let machine_address = extra_address + EXTRA_TABLE_SIZE_V516;
    let layer_image_address = machine_address + MACHINE_TABLE_SIZE_V516;

    let mut buf = Vec::new();

    let file_mark = FileMarkV516 {
        mark: *b"ANYCUBIC\x00\x00\x00\x00",
        version: 516,
        number_of_tables: 8,
        header_address,
        software_address: 0,
        preview_address,
        layer_image_color_table_address,
        layer_definition_address,
        extra_address,
        machine_address,
        layer_image_address,
    };
    write_struct(&mut buf, &file_mark)?;

    let header = build_v516_header_table(metadata);
    write_struct(&mut buf, &header)?;

    let preview = build_preview_table();
    write_struct(&mut buf, &preview)?;
    buf.resize(buf.len() + PREVIEW_SIZE, 0);

    let color_table = build_layer_image_color_table(metadata);
    write_struct(&mut buf, &color_table)?;

    write_layer_definition_table(&mut buf, metadata)?;

    let extra = build_extra_table(metadata);
    write_struct(&mut buf, &extra)?;

    let machine = build_machine_table_156(metadata, specifics, 1);
    write_struct(&mut buf, &machine)?;

    Ok(buf)
}

fn build_v517_header(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let header_address = FILE_MARK_V517_SIZE;
    let preview_address = header_address + HEADER_TABLE_V517_SIZE;
    let preview_table_size = PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;
    let layer_image_color_table_address = preview_address + preview_table_size;
    let layer_definition_address = layer_image_color_table_address + LAYER_IMAGE_COLOR_TABLE_SIZE;
    let extra_address =
        layer_definition_address + LAYERDEF_TABLE_BASE_SIZE + layer_count * LayerDef::BYTE_SIZE;
    let machine_address = extra_address + EXTRA_TABLE_SIZE_V516;
    let software_address = machine_address + MACHINE_TABLE_SIZE_V516;
    let model_address = software_address + SOFTWARE_TABLE_SIZE;
    let layer_image_address = model_address + MODEL_TABLE_SIZE;

    let mut buf = Vec::new();

    let file_mark = FileMarkV517 {
        mark: *b"ANYCUBIC\x00\x00\x00\x00",
        version: 517,
        number_of_tables: 9,
        header_address,
        software_address,
        preview_address,
        layer_image_color_table_address,
        layer_definition_address,
        extra_address,
        machine_address,
        model_address,
        reserved_address: 0,
        layer_image_address,
    };
    write_struct(&mut buf, &file_mark)?;

    let header = build_v517_header_table(metadata);
    write_struct(&mut buf, &header)?;

    write_preview_block(&mut buf, "PREVIEW")?;

    let color_table = build_layer_image_color_table(metadata);
    write_struct(&mut buf, &color_table)?;

    write_layer_definition_table(&mut buf, metadata)?;

    let extra = build_extra_table(metadata);
    write_struct(&mut buf, &extra)?;

    let machine = build_machine_table_156(metadata, specifics, 7);
    write_struct(&mut buf, &machine)?;

    let software = build_software_table();
    write_struct(&mut buf, &software)?;

    let model = build_model_table(metadata, specifics);
    write_struct(&mut buf, &model)?;

    Ok(buf)
}

fn build_v518_header(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
) -> Result<Vec<u8>, ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let header_address = FILE_MARK_V518_SIZE;
    let preview_address = header_address + HEADER_TABLE_V518_SIZE;
    let preview_table_size = PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;
    let layer_image_color_table_address = preview_address + preview_table_size;
    let layer_definition_address = layer_image_color_table_address + LAYER_IMAGE_COLOR_TABLE_SIZE;
    let extra_address =
        layer_definition_address + LAYERDEF_TABLE_BASE_SIZE + layer_count * LayerDef::BYTE_SIZE;
    let machine_address = extra_address + EXTRA_TABLE_SIZE_V516;
    let software_address = machine_address + MACHINE_TABLE_SIZE_V518;
    let model_address = software_address + SOFTWARE_TABLE_SIZE;
    let sub_layer_definition_address = model_address + MODEL_TABLE_SIZE;
    let sub_layer_definition_size =
        SUB_LAYER_DEF_TABLE_BASE_SIZE + layer_count * SUB_LAYER_DEF_SIZE;
    let preview2_address = sub_layer_definition_address + sub_layer_definition_size;
    let layer_image_address = preview2_address + PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32;

    let mut buf = Vec::new();

    let file_mark = FileMarkV518 {
        mark: *b"ANYCUBIC\x00\x00\x00\x00",
        version: 518,
        number_of_tables: 11,
        header_address,
        software_address,
        preview_address,
        layer_image_color_table_address,
        layer_definition_address,
        extra_address,
        machine_address,
        layer_image_address,
        model_address,
        sub_layer_definition_address,
        preview2_address,
        reserved_address: 0,
    };
    write_struct(&mut buf, &file_mark)?;

    let header = build_v518_header_table(metadata);
    write_struct(&mut buf, &header)?;

    write_preview_block(&mut buf, "PREVIEW")?;

    let color_table = build_layer_image_color_table(metadata);
    write_struct(&mut buf, &color_table)?;

    write_layer_definition_table(&mut buf, metadata)?;

    let extra = build_extra_table(metadata);
    write_struct(&mut buf, &extra)?;

    let machine = build_machine_table_v518(metadata, specifics);
    write_struct(&mut buf, &machine)?;

    let software = build_software_table();
    write_struct(&mut buf, &software)?;

    let model = build_model_table(metadata, specifics);
    write_struct(&mut buf, &model)?;

    write_sub_layer_definition_table(&mut buf, metadata.slicing.layer_count)?;
    write_preview_block(&mut buf, "PREVIEW2")?;

    Ok(buf)
}

fn build_v1_header_table(metadata: &FileMetadata) -> HeaderTable {
    HeaderTable {
        mark: table_mark("HEADER"),
        table_length: HEADER_TABLE_LENGTH,
        pixel_size_um: pixel_size_um(metadata),
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_time: metadata.exposure.exposure_time,
        wait_time_before_cure: metadata.exposure.light_off_delay,
        bottom_exposure_time: metadata.exposure.bottom_exposure_time,
        bottom_layer_count: metadata.slicing.bottom_layer_count as f32,
        lift_height: metadata.motion.lift_height,
        lift_speed: metadata.motion.lift_speed,
        retract_speed: metadata.motion.retract_speed,
        volume_ml: metadata.stats.volume_ml,
        anti_aliasing: metadata.slicing.anti_alias_level.max(1),
        resolution_x: metadata.printer.resolution_x,
        resolution_y: metadata.printer.resolution_y,
        weight_g: metadata.stats.weight_g,
        cost: metadata.stats.cost,
        price_currency_symbol: [b'$', 0, 0, 0],
        per_layer_settings: u32::from(!metadata.per_layer_settings.is_empty()),
        print_time_sec: metadata.stats.print_time_sec,
        transition_layer_count: metadata.slicing.transition_layer_count,
        transition_layer_type: 0,
    }
}

fn build_v516_header_table(metadata: &FileMetadata) -> HeaderTableV516 {
    HeaderTableV516 {
        mark: table_mark("HEADER"),
        table_length: HEADER_TABLE_V516_LENGTH,
        pixel_size_um: pixel_size_um(metadata),
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_time: metadata.exposure.exposure_time,
        wait_time_before_cure: metadata.exposure.light_off_delay,
        bottom_exposure_time: metadata.exposure.bottom_exposure_time,
        bottom_layer_count: metadata.slicing.bottom_layer_count as f32,
        lift_height: metadata.motion.lift_height,
        lift_speed: metadata.motion.lift_speed,
        retract_speed: metadata.motion.retract_speed,
        volume_ml: metadata.stats.volume_ml,
        anti_aliasing: metadata.slicing.anti_alias_level.max(1),
        resolution_x: metadata.printer.resolution_x,
        resolution_y: metadata.printer.resolution_y,
        weight_g: metadata.stats.weight_g,
        cost: metadata.stats.cost,
        price_currency_symbol: [b'$', 0, 0, 0],
        per_layer_settings: u32::from(!metadata.per_layer_settings.is_empty()),
        print_time_sec: metadata.stats.print_time_sec,
        transition_layer_count: metadata.slicing.transition_layer_count,
        transition_layer_type: 0,
        advanced_mode: 0,
    }
}

fn build_v517_header_table(metadata: &FileMetadata) -> HeaderTableV517 {
    HeaderTableV517 {
        mark: table_mark("HEADER"),
        table_length: HEADER_TABLE_V517_LENGTH,
        pixel_size_um: pixel_size_um(metadata),
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_time: metadata.exposure.exposure_time,
        wait_time_before_cure: metadata.exposure.light_off_delay,
        bottom_exposure_time: metadata.exposure.bottom_exposure_time,
        bottom_layer_count: metadata.slicing.bottom_layer_count as f32,
        lift_height: metadata.motion.lift_height,
        lift_speed: metadata.motion.lift_speed,
        retract_speed: metadata.motion.retract_speed,
        volume_ml: metadata.stats.volume_ml,
        anti_aliasing: metadata.slicing.anti_alias_level.max(1),
        resolution_x: metadata.printer.resolution_x,
        resolution_y: metadata.printer.resolution_y,
        weight_g: metadata.stats.weight_g,
        cost: metadata.stats.cost,
        price_currency_symbol: [b'$', 0, 0, 0],
        per_layer_settings: u32::from(!metadata.per_layer_settings.is_empty()),
        print_time_sec: metadata.stats.print_time_sec,
        transition_layer_count: metadata.slicing.transition_layer_count,
        transition_layer_type: 0,
        advanced_mode: 0,
        software_version: 1,
        model_version: 1,
    }
}

fn build_v518_header_table(metadata: &FileMetadata) -> HeaderTableV518 {
    HeaderTableV518 {
        mark: table_mark("HEADER"),
        table_length: HEADER_TABLE_V518_LENGTH,
        pixel_size_um: pixel_size_um(metadata),
        layer_height_mm: metadata.slicing.layer_height_mm,
        exposure_time: metadata.exposure.exposure_time,
        wait_time_before_cure: metadata.exposure.light_off_delay,
        bottom_exposure_time: metadata.exposure.bottom_exposure_time,
        bottom_layer_count: metadata.slicing.bottom_layer_count as f32,
        lift_height: metadata.motion.lift_height,
        lift_speed: metadata.motion.lift_speed,
        retract_speed: metadata.motion.retract_speed,
        volume_ml: metadata.stats.volume_ml,
        anti_aliasing: metadata.slicing.anti_alias_level.max(1),
        resolution_x: metadata.printer.resolution_x,
        resolution_y: metadata.printer.resolution_y,
        weight_g: metadata.stats.weight_g,
        cost: metadata.stats.cost,
        price_currency_symbol: [b'$', 0, 0, 0],
        per_layer_settings: u32::from(!metadata.per_layer_settings.is_empty()),
        print_time_sec: metadata.stats.print_time_sec,
        transition_layer_count: metadata.slicing.transition_layer_count,
        transition_layer_type: 0,
        advanced_mode: 0,
        software_version: 1,
        model_version: 1,
        preview2_enabled: u32::from(metadata.thumbnails.len() > 1),
    }
}

fn build_preview_table() -> PreviewTable {
    PreviewTable {
        mark: table_mark("PREVIEW"),
        table_length: PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32,
        resolution_x: PREVIEW_WIDTH,
        x_mark: [b'x', 0, 0, 0],
        resolution_y: PREVIEW_HEIGHT,
    }
}

fn build_layer_image_color_table(metadata: &FileMetadata) -> LayerImageColorTable {
    let anti_alias = metadata.slicing.anti_alias_level.clamp(1, 16);
    let pixel_increment = 255.0 / anti_alias as f32;
    let mut grey = [0u8; 16];
    for (index, item) in grey.iter_mut().enumerate() {
        let value = (((index + 1) as f32) * pixel_increment).min(255.0);
        *item = value as u8;
    }

    LayerImageColorTable {
        use_full_greyscale: 0,
        grey_max_count: 16,
        grey,
        unknown: 0,
    }
}

fn build_extra_table(metadata: &FileMetadata) -> ExtraTable {
    ExtraTable {
        mark: table_mark("EXTRA"),
        table_length: EXTRA_TABLE_LENGTH_V516,
        bottom_lift_count: 2,
        bottom_lift_height1: metadata.motion.bottom_lift_height,
        bottom_lift_speed1: metadata.motion.bottom_lift_speed,
        bottom_retract_speed2: choose_speed(
            metadata.motion.retract_speed2,
            metadata.motion.retract_speed,
        ),
        bottom_lift_height2: 0.0,
        bottom_lift_speed2: metadata.motion.bottom_lift_speed,
        bottom_retract_speed1: metadata.motion.retract_speed,
        normal_lift_count: 2,
        lift_height1: metadata.motion.lift_height,
        lift_speed1: metadata.motion.lift_speed,
        retract_speed2: choose_speed(
            metadata.motion.retract_speed2,
            metadata.motion.retract_speed,
        ),
        lift_height2: metadata.motion.lift_height2,
        lift_speed2: choose_speed(metadata.motion.lift_speed2, metadata.motion.lift_speed),
        retract_speed1: metadata.motion.retract_speed,
    }
}

fn build_machine_table_156(
    metadata: &FileMetadata,
    specifics: &AnycubicSpec,
    property_fields: u32,
) -> MachineTableV516 {
    MachineTableV516 {
        mark: table_mark("MACHINE"),
        table_length: MACHINE_TABLE_SIZE_V516,
        machine_name: fixed_string::<96>(effective_machine_name(metadata, specifics)),
        layer_image_format: fixed_string::<16>(layer_image_format(specifics.extension)),
        max_antialiasing_level: 16,
        property_fields,
        display_width: effective_display_width(metadata, specifics),
        display_height: effective_display_height(metadata, specifics),
        machine_z: effective_machine_z(metadata, specifics),
        max_file_version: max_supported_version(specifics),
        machine_background: 6_506_241,
    }
}

fn build_machine_table_v518(metadata: &FileMetadata, specifics: &AnycubicSpec) -> MachineTableV518 {
    MachineTableV518 {
        mark: table_mark("MACHINE"),
        table_length: MACHINE_TABLE_SIZE_V518,
        machine_name: fixed_string::<96>(effective_machine_name(metadata, specifics)),
        layer_image_format: fixed_string::<16>(layer_image_format(specifics.extension)),
        max_antialiasing_level: 16,
        property_fields: 15,
        display_width: effective_display_width(metadata, specifics),
        display_height: effective_display_height(metadata, specifics),
        machine_z: effective_machine_z(metadata, specifics),
        max_file_version: max_supported_version(specifics),
        machine_background: 6_506_241,
        resolution_x: metadata.printer.resolution_x.min(u16::MAX as u32) as u16,
        resolution_y: metadata.printer.resolution_y.min(u16::MAX as u32) as u16,
        reserved: [0; 64],
    }
}

fn build_software_table() -> SoftwareTable {
    SoftwareTable {
        mark: table_mark("SOFTWARE"),
        table_length: SOFTWARE_TABLE_LENGTH,
        software_name: fixed_string::<32>("slicer-converter"),
    }
}

fn build_model_table(metadata: &FileMetadata, specifics: &AnycubicSpec) -> ModelTable {
    ModelTable {
        mark: table_mark("MODEL"),
        table_length: MODEL_TABLE_LENGTH,
        model_name: fixed_string::<32>(effective_machine_name(metadata, specifics)),
    }
}

fn write_sub_layer_definition_table(
    buf: &mut Vec<u8>,
    layer_count: u32,
) -> Result<(), ConvertError> {
    let table_length = SUB_LAYER_DEF_TABLE_BASE_SIZE + layer_count * SUB_LAYER_DEF_SIZE;
    {
        let mut writer = BinaryWriter::new(buf);
        writer.write_bytes(&table_mark("SUBIMGS"));
        writer.write_le_u32(table_length);
        writer.write_le_u32(layer_count);
        writer.write_le_u32(1);
    }

    for _ in 0..layer_count {
        let entry = SubLayerDef {
            data_address: 0,
            data_length: 0,
            non_zero_pixel_count: 0,
            padding1: 0.0,
            padding2: 0.0,
            padding3: 0.0,
            padding4: 0.0,
            padding5: 0.0,
            padding6: 0.0,
            padding7: 0.0,
            padding8: 0.0,
        };
        write_struct(buf, &entry)?;
    }

    Ok(())
}

fn write_preview_block(buf: &mut Vec<u8>, mark: &str) -> Result<(), ConvertError> {
    if mark == "PREVIEW2" {
        let preview2 = Preview2Table {
            mark: table_mark(mark),
            table_length: PREVIEW_TABLE_SIZE + PREVIEW_SIZE as u32,
            resolution_x: PREVIEW_WIDTH,
            x_mark: [b'x', 0, 0, 0],
            resolution_y: PREVIEW_HEIGHT,
        };
        write_struct(buf, &preview2)?;
    } else {
        let preview = build_preview_table();
        write_struct(buf, &preview)?;
    }

    buf.resize(buf.len() + PREVIEW_SIZE, 0);
    Ok(())
}

fn write_layer_definition_table(
    buf: &mut Vec<u8>,
    metadata: &FileMetadata,
) -> Result<(), ConvertError> {
    let layer_count = metadata.slicing.layer_count;
    let layerdef_len = 4 + layer_count * LayerDef::BYTE_SIZE;
    {
        let mut writer = BinaryWriter::new(buf);
        writer.write_bytes(&table_mark("LAYERDEF"));
        writer.write_le_u32(layerdef_len);
        writer.write_le_u32(layer_count);
    }

    for i in 0..layer_count {
        let is_bottom = i < metadata.slicing.bottom_layer_count;
        let layer_def = LayerDef {
            data_address: 0,
            data_length: 0,
            lift_height: if is_bottom {
                metadata.motion.bottom_lift_height
            } else {
                metadata.motion.lift_height
            },
            lift_speed: if is_bottom {
                metadata.motion.bottom_lift_speed
            } else {
                metadata.motion.lift_speed
            },
            exposure_time: if is_bottom {
                metadata.exposure.bottom_exposure_time
            } else {
                metadata.exposure.exposure_time
            },
            layer_height: metadata.slicing.layer_height_mm,
            non_zero_pixel_count: 0,
            padding1: 0,
        };
        write_struct(buf, &layer_def)?;
    }

    Ok(())
}

fn effective_display_width(metadata: &FileMetadata, specifics: &AnycubicSpec) -> f32 {
    if metadata.printer.display_width_mm > 0.0 {
        metadata.printer.display_width_mm
    } else {
        specifics.machine.display_width_mm
    }
}

fn effective_display_height(metadata: &FileMetadata, specifics: &AnycubicSpec) -> f32 {
    if metadata.printer.display_height_mm > 0.0 {
        metadata.printer.display_height_mm
    } else {
        specifics.machine.display_height_mm
    }
}

fn effective_machine_z(metadata: &FileMetadata, specifics: &AnycubicSpec) -> f32 {
    if metadata.printer.machine_z_mm > 0.0 {
        metadata.printer.machine_z_mm
    } else {
        specifics.machine.machine_z_mm
    }
}

fn effective_machine_name<'a>(metadata: &'a FileMetadata, specifics: &'a AnycubicSpec) -> &'a str {
    if metadata.printer.machine_name.trim().is_empty() {
        specifics.machine.display_name
    } else {
        metadata.printer.machine_name.trim()
    }
}

fn max_supported_version(specifics: &AnycubicSpec) -> u32 {
    specifics
        .supported_versions
        .last()
        .copied()
        .unwrap_or(specifics.default_version)
        .as_u32()
}

fn pixel_size_um(metadata: &FileMetadata) -> f32 {
    if metadata.printer.resolution_x > 0 && metadata.printer.display_width_mm > 0.0 {
        metadata.printer.display_width_mm * 1000.0 / metadata.printer.resolution_x as f32
    } else {
        47.25
    }
}

fn table_mark(name: &str) -> [u8; 12] {
    fixed_string::<12>(name)
}

fn fixed_string<const N: usize>(value: &str) -> [u8; N] {
    let mut bytes = [0u8; N];
    let raw = value.as_bytes();
    let copy_len = raw.len().min(N.saturating_sub(1));
    bytes[..copy_len].copy_from_slice(&raw[..copy_len]);
    bytes
}

fn layer_image_format(extension: &str) -> &'static str {
    if extension.eq_ignore_ascii_case("pws") {
        "pwsimg"
    } else {
        "pw0img"
    }
}

fn choose_speed(value: f32, fallback: f32) -> f32 {
    if value > 0.0 {
        value
    } else {
        fallback
    }
}

fn write_struct<T>(buf: &mut Vec<u8>, value: &T) -> Result<(), ConvertError>
where
    T: BinWrite + binrw::meta::WriteEndian,
    for<'a> T::Args<'a>: Default,
{
    let append_offset = buf.len();
    let mut cursor = Cursor::new(buf);
    // `Cursor::new(Vec)` starts at position 0; seek to the current end so BinWrite appends.
    cursor.set_position(append_offset as u64);
    if let Err(error) = value.write(&mut cursor) {
        cursor.get_mut().truncate(append_offset);
        return Err(ConvertError::BinrwError(error.to_string()));
    }
    Ok(())
}

fn find_table_mark(data: &[u8], mark: &[u8]) -> Option<usize> {
    data.windows(mark.len()).position(|window| window == mark)
}
