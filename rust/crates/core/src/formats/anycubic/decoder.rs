use std::io::Cursor;

use binrw::BinRead;

use crate::common::{ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset};
use crate::formats::common::binary::read_le_u32;

use super::header::{
    FileMark, FileMarkV516, FileMarkV517, FileMarkV518, HeaderTable, HeaderTableV516,
    HeaderTableV517, HeaderTableV518, LayerDef,
};

/// Parse an Anycubic v1-family binary header.
///
/// Returns shared metadata plus byte offsets to encoded layer payloads. The
/// v1-family parser handles the original `ANYCUBIC` file mark and 32-byte
/// `LayerDef` table entries.
///
/// 解析 Anycubic v1-family binary header，返回共享 metadata 与 encoded layer payloads 的
/// byte offsets。v1-family parser 处理原始 `ANYCUBIC` file mark 与 32-byte `LayerDef`
/// table entries。
pub fn parse_anycubic_v1_header_data(
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    parse_anycubic_v1_header_le(data)
}

fn parse_anycubic_v1_header_le(
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    if data.len() < 12 {
        return Err(ConvertError::BufferTooSmall {
            need: 12,
            got: data.len(),
        });
    }

    if &data[0..8] != b"ANYCUBIC" {
        return Err(ConvertError::InvalidMagic {
            expected: 0x414E5943,
            actual: read_le_u32(data, 0)?,
        });
    }

    let mut cursor = Cursor::new(data);
    let file_mark = FileMark::read(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(format!("FileMark parse error: {e}")))?;

    let header_addr = file_mark.header_address as usize;
    if header_addr + 12 > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: header_addr + 12,
            got: data.len(),
        });
    }
    if &data[header_addr..header_addr + 6] != b"HEADER" {
        return Err(ConvertError::BinrwError(
            "HEADER table mark not found".into(),
        ));
    }

    let mut cursor = Cursor::new(&data[header_addr..]);
    let header = HeaderTable::read(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(format!("HeaderTable parse error: {e}")))?;

    let layer_def_addr = file_mark.layer_definition_address as usize;
    if layer_def_addr + 12 > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: layer_def_addr + 12,
            got: data.len(),
        });
    }
    if &data[layer_def_addr..layer_def_addr + 8] != b"LAYERDEF" {
        return Err(ConvertError::BinrwError(
            "LAYERDEF table mark not found".into(),
        ));
    }

    let mut layer_cursor = Cursor::new(&data[layer_def_addr..]);
    let mut mark = [0u8; 12];
    std::io::Read::read_exact(&mut layer_cursor, &mut mark)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef mark read error: {e}")))?;
    let _table_length = u32::read_le(&mut layer_cursor)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef table_length: {e}")))?;
    let layer_count = u32::read_le(&mut layer_cursor)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef layer_count: {e}")))?;

    let mut layer_defs = Vec::with_capacity(layer_count as usize);
    for _ in 0..layer_count {
        let layer_def = LayerDef::read(&mut layer_cursor)
            .map_err(|e| ConvertError::BinrwError(format!("LayerDef parse error: {e}")))?;
        layer_defs.push(layer_def);
    }

    build_parse_result_from_v1_header(&header, &layer_defs, data)
}

fn build_parse_result_from_v1_header(
    header: &HeaderTable,
    layer_defs: &[LayerDef],
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let bottom_layer_count = header.bottom_layer_count.round().max(0.0) as u32;
    let (bottom_lift_height, bottom_lift_speed) = derive_bottom_motion_v1(
        layer_defs,
        bottom_layer_count,
        header.lift_height,
        header.lift_speed,
    );
    let (lift_height, lift_speed) = derive_normal_motion_v1(
        layer_defs,
        bottom_layer_count,
        header.lift_height,
        header.lift_speed,
    );

    let metadata = FileMetadataBuilder::new()
        .resolution(header.resolution_x, header.resolution_y)
        .display_size_mm(
            header.pixel_size_um * header.resolution_x as f32 / 1000.0,
            header.pixel_size_um * header.resolution_y as f32 / 1000.0,
        )
        .layer_height_mm(header.layer_height_mm)
        .layer_count(layer_defs.len() as u32)
        .bottom_layer_count(bottom_layer_count)
        .transition_layer_count(header.transition_layer_count)
        .anti_alias_level(header.anti_aliasing.max(1))
        .exposure(header.exposure_time, header.bottom_exposure_time)
        .light_off_delay(header.wait_time_before_cure, header.wait_time_before_cure)
        .lift(lift_height, lift_speed)
        .bottom_lift(bottom_lift_height, bottom_lift_speed)
        .retract(header.retract_speed)
        .wait_times(header.wait_time_before_cure, 0.0, 0.0)
        .statistics(
            header.print_time_sec,
            header.volume_ml,
            header.weight_g,
            header.cost,
        )
        .build()?;

    let layer_offsets = collect_layer_offsets_v1(layer_defs, data)?;
    Ok((metadata, layer_offsets))
}

fn collect_layer_offsets_v1(
    layer_defs: &[LayerDef],
    _data: &[u8],
) -> Result<Vec<LayerOffset>, ConvertError> {
    Ok(layer_defs
        .iter()
        .map(|ld| LayerOffset {
            offset: ld.data_address as u64,
            size: ld.data_length as u64,
        })
        .collect())
}

fn derive_bottom_motion_v1(
    layer_defs: &[LayerDef],
    bottom_layer_count: u32,
    default_height: f32,
    default_speed: f32,
) -> (f32, f32) {
    layer_defs
        .iter()
        .take(bottom_layer_count as usize)
        .find(|_| true)
        .map(|layer| (layer.lift_height, layer.lift_speed))
        .unwrap_or((default_height, default_speed))
}

fn derive_normal_motion_v1(
    layer_defs: &[LayerDef],
    bottom_layer_count: u32,
    default_height: f32,
    default_speed: f32,
) -> (f32, f32) {
    layer_defs
        .iter()
        .skip(bottom_layer_count as usize)
        .find(|_| true)
        .map(|layer| (layer.lift_height, layer.lift_speed))
        .unwrap_or((default_height, default_speed))
}

// ───────────────────────── V516 / V517 / V518 ─────────────────────────
//
// These paths all share the same LAYERDEF table shape (12-byte mark + u32
// table_length + u32 layer_count + LayerDef[]). Their FileMark and
// HeaderTable sizes / field sets differ, so we parse those with the
// version-specific binrw structs and then reuse `read_layer_defs` for
// the layer definition table.

fn check_magic_and_header_mark(data: &[u8], header_addr: usize) -> Result<(), ConvertError> {
    if data.len() < 12 {
        return Err(ConvertError::BufferTooSmall {
            need: 12,
            got: data.len(),
        });
    }
    if &data[0..8] != b"ANYCUBIC" {
        return Err(ConvertError::InvalidMagic {
            expected: 0x414E5943,
            actual: read_le_u32(data, 0)?,
        });
    }
    if header_addr + 12 > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: header_addr + 12,
            got: data.len(),
        });
    }
    if &data[header_addr..header_addr + 6] != b"HEADER" {
        return Err(ConvertError::BinrwError(
            "HEADER table mark not found".into(),
        ));
    }
    Ok(())
}

fn read_layer_defs(data: &[u8], layer_def_addr: usize) -> Result<Vec<LayerDef>, ConvertError> {
    if layer_def_addr + 20 > data.len() {
        return Err(ConvertError::BufferTooSmall {
            need: layer_def_addr + 20,
            got: data.len(),
        });
    }
    if &data[layer_def_addr..layer_def_addr + 8] != b"LAYERDEF" {
        return Err(ConvertError::BinrwError(
            "LAYERDEF table mark not found".into(),
        ));
    }
    let mut layer_cursor = Cursor::new(&data[layer_def_addr..]);
    let mut mark = [0u8; 12];
    std::io::Read::read_exact(&mut layer_cursor, &mut mark)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef mark read error: {e}")))?;
    let _table_length = u32::read_le(&mut layer_cursor)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef table_length: {e}")))?;
    let layer_count = u32::read_le(&mut layer_cursor)
        .map_err(|e| ConvertError::BinrwError(format!("LayerDef layer_count: {e}")))?;

    let mut layer_defs = Vec::with_capacity(layer_count as usize);
    for _ in 0..layer_count {
        let layer_def = LayerDef::read(&mut layer_cursor)
            .map_err(|e| ConvertError::BinrwError(format!("LayerDef parse error: {e}")))?;
        layer_defs.push(layer_def);
    }
    Ok(layer_defs)
}

/// Common per-version core fields extracted from version-specific header tables.
///
/// Each `HeaderTableVxxx` carries the same print-facing values plus a few
/// version-only fields. Projecting them into this struct keeps metadata building
/// and layer offset collection shared.
///
/// 从 version-specific header tables 提取的公共核心字段。每个 `HeaderTableVxxx` 都携带相同的
/// print-facing values 以及少量版本专属字段；投影到该结构后可共享 metadata 构建和 layer
/// offset 收集逻辑。
#[allow(dead_code)]
struct V5xxHeaderCore {
    pixel_size_um: f32,
    layer_height_mm: f32,
    exposure_time: f32,
    wait_time_before_cure: f32,
    bottom_exposure_time: f32,
    bottom_layer_count: f32,
    lift_height: f32,
    lift_speed: f32,
    retract_speed: f32,
    volume_ml: f32,
    anti_aliasing: u32,
    resolution_x: u32,
    resolution_y: u32,
    weight_g: f32,
    cost: f32,
    per_layer_settings: u32,
    print_time_sec: u32,
    transition_layer_count: u32,
}

impl From<&HeaderTableV516> for V5xxHeaderCore {
    fn from(h: &HeaderTableV516) -> Self {
        Self {
            pixel_size_um: h.pixel_size_um,
            layer_height_mm: h.layer_height_mm,
            exposure_time: h.exposure_time,
            wait_time_before_cure: h.wait_time_before_cure,
            bottom_exposure_time: h.bottom_exposure_time,
            bottom_layer_count: h.bottom_layer_count,
            lift_height: h.lift_height,
            lift_speed: h.lift_speed,
            retract_speed: h.retract_speed,
            volume_ml: h.volume_ml,
            anti_aliasing: h.anti_aliasing,
            resolution_x: h.resolution_x,
            resolution_y: h.resolution_y,
            weight_g: h.weight_g,
            cost: h.cost,
            per_layer_settings: h.per_layer_settings,
            print_time_sec: h.print_time_sec,
            transition_layer_count: h.transition_layer_count,
        }
    }
}

impl From<&HeaderTableV517> for V5xxHeaderCore {
    fn from(h: &HeaderTableV517) -> Self {
        Self {
            pixel_size_um: h.pixel_size_um,
            layer_height_mm: h.layer_height_mm,
            exposure_time: h.exposure_time,
            wait_time_before_cure: h.wait_time_before_cure,
            bottom_exposure_time: h.bottom_exposure_time,
            bottom_layer_count: h.bottom_layer_count,
            lift_height: h.lift_height,
            lift_speed: h.lift_speed,
            retract_speed: h.retract_speed,
            volume_ml: h.volume_ml,
            anti_aliasing: h.anti_aliasing,
            resolution_x: h.resolution_x,
            resolution_y: h.resolution_y,
            weight_g: h.weight_g,
            cost: h.cost,
            per_layer_settings: h.per_layer_settings,
            print_time_sec: h.print_time_sec,
            transition_layer_count: h.transition_layer_count,
        }
    }
}

impl From<&HeaderTableV518> for V5xxHeaderCore {
    fn from(h: &HeaderTableV518) -> Self {
        Self {
            pixel_size_um: h.pixel_size_um,
            layer_height_mm: h.layer_height_mm,
            exposure_time: h.exposure_time,
            wait_time_before_cure: h.wait_time_before_cure,
            bottom_exposure_time: h.bottom_exposure_time,
            bottom_layer_count: h.bottom_layer_count,
            lift_height: h.lift_height,
            lift_speed: h.lift_speed,
            retract_speed: h.retract_speed,
            volume_ml: h.volume_ml,
            anti_aliasing: h.anti_aliasing,
            resolution_x: h.resolution_x,
            resolution_y: h.resolution_y,
            weight_g: h.weight_g,
            cost: h.cost,
            per_layer_settings: h.per_layer_settings,
            print_time_sec: h.print_time_sec,
            transition_layer_count: h.transition_layer_count,
        }
    }
}

fn build_metadata_v5xx(
    core: &V5xxHeaderCore,
    layer_defs: &[LayerDef],
) -> Result<FileMetadata, ConvertError> {
    let bottom_layer_count = core.bottom_layer_count.round().max(0.0) as u32;
    let (bottom_lift_height, bottom_lift_speed) = derive_bottom_motion_v1(
        layer_defs,
        bottom_layer_count,
        core.lift_height,
        core.lift_speed,
    );
    let (lift_height, lift_speed) = derive_normal_motion_v1(
        layer_defs,
        bottom_layer_count,
        core.lift_height,
        core.lift_speed,
    );

    FileMetadataBuilder::new()
        .resolution(core.resolution_x, core.resolution_y)
        .display_size_mm(
            core.pixel_size_um * core.resolution_x as f32 / 1000.0,
            core.pixel_size_um * core.resolution_y as f32 / 1000.0,
        )
        .layer_height_mm(core.layer_height_mm)
        .layer_count(layer_defs.len() as u32)
        .bottom_layer_count(bottom_layer_count)
        .transition_layer_count(core.transition_layer_count)
        .anti_alias_level(core.anti_aliasing.max(1))
        .exposure(core.exposure_time, core.bottom_exposure_time)
        .light_off_delay(core.wait_time_before_cure, core.wait_time_before_cure)
        .lift(lift_height, lift_speed)
        .bottom_lift(bottom_lift_height, bottom_lift_speed)
        .retract(core.retract_speed)
        .wait_times(core.wait_time_before_cure, 0.0, 0.0)
        .statistics(
            core.print_time_sec,
            core.volume_ml,
            core.weight_g,
            core.cost,
        )
        .build()
}

/// Parse an Anycubic v5.16 binary header.
///
/// v5.16 introduces the machine and extra tables, but the layer definition
/// table keeps the same 32-byte `LayerDef` entry shape.
///
/// 解析 Anycubic v5.16 binary header。v5.16 引入 machine 与 extra tables，但
/// layer definition table 仍使用相同的 32-byte `LayerDef` entry shape。
pub fn parse_anycubic_v516_header_data(
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let mut cursor = Cursor::new(data);
    let file_mark = FileMarkV516::read(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(format!("FileMarkV516 parse error: {e}")))?;
    check_magic_and_header_mark(data, file_mark.header_address as usize)?;

    let header_addr = file_mark.header_address as usize;
    let mut hcursor = Cursor::new(&data[header_addr..]);
    let header = HeaderTableV516::read(&mut hcursor)
        .map_err(|e| ConvertError::BinrwError(format!("HeaderTableV516 parse error: {e}")))?;

    let layer_defs = read_layer_defs(data, file_mark.layer_definition_address as usize)?;
    let core = V5xxHeaderCore::from(&header);
    let metadata = build_metadata_v5xx(&core, &layer_defs)?;

    let layer_offsets = collect_layer_offsets_v1(&layer_defs, data)?;
    Ok((metadata, layer_offsets))
}

/// Parse an Anycubic v5.17 binary header.
///
/// v5.17 adds software/model table addresses while preserving the shared layer
/// definition table shape.
///
/// 解析 Anycubic v5.17 binary header。v5.17 增加 software/model table addresses，
/// 同时保留共享 layer definition table shape。
pub fn parse_anycubic_v517_header_data(
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let mut cursor = Cursor::new(data);
    let file_mark = FileMarkV517::read(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(format!("FileMarkV517 parse error: {e}")))?;
    check_magic_and_header_mark(data, file_mark.header_address as usize)?;

    let header_addr = file_mark.header_address as usize;
    let mut hcursor = Cursor::new(&data[header_addr..]);
    let header = HeaderTableV517::read(&mut hcursor)
        .map_err(|e| ConvertError::BinrwError(format!("HeaderTableV517 parse error: {e}")))?;

    let layer_defs = read_layer_defs(data, file_mark.layer_definition_address as usize)?;
    let core = V5xxHeaderCore::from(&header);
    let metadata = build_metadata_v5xx(&core, &layer_defs)?;

    let layer_offsets = collect_layer_offsets_v1(&layer_defs, data)?;
    Ok((metadata, layer_offsets))
}

/// Parse an Anycubic v5.18 binary header.
///
/// v5.18 adds sub-layer and second-preview addresses. The current importer uses
/// the primary layer definition table for layer payload offsets.
///
/// 解析 Anycubic v5.18 binary header。v5.18 增加 sub-layer 与 second-preview addresses；
/// 当前 importer 使用 primary layer definition table 获取 layer payload offsets。
pub fn parse_anycubic_v518_header_data(
    data: &[u8],
) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
    let mut cursor = Cursor::new(data);
    let file_mark = FileMarkV518::read(&mut cursor)
        .map_err(|e| ConvertError::BinrwError(format!("FileMarkV518 parse error: {e}")))?;
    check_magic_and_header_mark(data, file_mark.header_address as usize)?;

    let header_addr = file_mark.header_address as usize;
    let mut hcursor = Cursor::new(&data[header_addr..]);
    let header = HeaderTableV518::read(&mut hcursor)
        .map_err(|e| ConvertError::BinrwError(format!("HeaderTableV518 parse error: {e}")))?;

    let layer_defs = read_layer_defs(data, file_mark.layer_definition_address as usize)?;
    let core = V5xxHeaderCore::from(&header);
    let metadata = build_metadata_v5xx(&core, &layer_defs)?;

    let layer_offsets = collect_layer_offsets_v1(&layer_defs, data)?;
    Ok((metadata, layer_offsets))
}
