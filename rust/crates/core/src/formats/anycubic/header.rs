//! Anycubic binary table records.
//!
//! These structs mirror the little-endian table layouts used by Anycubic PW0/PWS
//! binary files. Version-specific layouts reuse many fields but add file-mark
//! addresses and auxiliary tables over time.
//!
//! Anycubic binary table records。这些结构体映射 Anycubic PW0/PWS binary files 使用的
//! little-endian table layouts。不同版本复用大量字段，但会逐步增加 file-mark addresses
//! 与 auxiliary tables。

use binrw::{BinRead, BinWrite};

/// PW0 file mark for Anycubic v1-family files.
///
/// Matches UVtools/Anycubic little-endian layout.
///
/// Anycubic v1-family PW0 file mark，匹配 UVTools/Anycubic little-endian layout。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct FileMark {
    pub mark: [u8; 12],
    pub version: u32,
    pub number_of_tables: u32,
    pub header_address: u32,
    pub software_address: u32,
    pub preview_address: u32,
    pub layer_image_color_table_address: u32,
    pub layer_definition_address: u32,
    pub extra_address: u32,
    pub layer_image_address: u32,
}

/// PW0 HEADER table for v1-family files.
///
/// Contains the file-level print settings and display resolution used by legacy
/// Anycubic binary files.
///
/// v1-family 文件的 PW0 HEADER table，包含旧版 Anycubic binary files 使用的 file-level
/// print settings 与 display resolution。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct HeaderTable {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub pixel_size_um: f32,
    pub layer_height_mm: f32,
    pub exposure_time: f32,
    pub wait_time_before_cure: f32,
    pub bottom_exposure_time: f32,
    pub bottom_layer_count: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub retract_speed: f32,
    pub volume_ml: f32,
    pub anti_aliasing: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub weight_g: f32,
    pub cost: f32,
    pub price_currency_symbol: [u8; 4],
    pub per_layer_settings: u32,
    pub print_time_sec: u32,
    pub transition_layer_count: u32,
    pub transition_layer_type: u32,
}

/// PW0 preview table header.
///
/// The preview payload is a fixed 224x168 RGB565 image following this table.
///
/// PW0 preview table header；preview payload 是紧随其后的固定 224x168 RGB565 image。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct PreviewTable {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub resolution_x: u32,
    pub x_mark: [u8; 4],
    pub resolution_y: u32,
}

/// PW0 layer definition, 32 bytes per layer.
///
/// `data_address` / `data_length` are patched after layer payloads are encoded.
/// `non_zero_pixel_count` records exposed pixel count for readers that display
/// layer statistics.
///
/// PW0 layer definition（每层 32 字节）。`data_address` / `data_length` 会在 layer
/// payloads 编码后回填；`non_zero_pixel_count` 记录曝光像素数量，供 reader 展示层统计。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct LayerDef {
    pub data_address: u32,
    pub data_length: u32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub exposure_time: f32,
    pub layer_height: f32,
    pub non_zero_pixel_count: u32,
    pub padding1: u32,
}

impl LayerDef {
    /// Serialized byte size of one [`LayerDef`] record.
    ///
    /// 单条 [`LayerDef`] record 的序列化字节长度。
    pub const BYTE_SIZE: u32 = 32;
}

/// Byte size of the v1 [`FileMark`] record.
///
/// v1 [`FileMark`] record 的字节长度。
pub const FILE_MARK_SIZE: u32 = 48;
/// Payload length of the v1 [`HeaderTable`] after mark and length fields.
///
/// v1 [`HeaderTable`] 在 mark 与 length 字段之后的 payload 长度。
pub const HEADER_TABLE_LENGTH: u32 = 80;
/// Total byte size of the v1 [`HeaderTable`] record.
///
/// v1 [`HeaderTable`] record 总字节长度。
pub const HEADER_TABLE_SIZE: u32 = 12 + 4 + HEADER_TABLE_LENGTH;
/// Total byte size of [`PreviewTable`].
///
/// [`PreviewTable`] 的总字节长度。
pub const PREVIEW_TABLE_SIZE: u32 = 12 + 4 + 4 + 4 + 4;
/// Fixed prefix size of the layer-definition table before per-layer records.
///
/// layer-definition table 在 per-layer records 前的固定前缀大小。
pub const LAYERDEF_TABLE_BASE_SIZE: u32 = 12 + 4 + 4;

/// PW0 preview dimensions.
///
/// PW0 preview 固定尺寸。
pub const PREVIEW_WIDTH: u32 = 224;
/// PW0 preview height in pixels.
///
/// PW0 preview 高度（像素）。
pub const PREVIEW_HEIGHT: u32 = 168;
/// PW0 preview RGB565 payload size in bytes.
///
/// PW0 preview RGB565 payload 字节数。
pub const PREVIEW_SIZE: usize = (PREVIEW_WIDTH * PREVIEW_HEIGHT * 2) as usize;

/// File-mark size used by Anycubic v5.15.
///
/// Anycubic v5.15 使用的 file-mark size。
pub const FILE_MARK_V515_SIZE: u32 = FILE_MARK_SIZE;
/// File-mark size used by Anycubic v5.16.
///
/// Anycubic v5.16 使用的 file-mark size。
pub const FILE_MARK_V516_SIZE: u32 = 52;
/// File-mark size used by Anycubic v5.17.
///
/// Anycubic v5.17 使用的 file-mark size。
pub const FILE_MARK_V517_SIZE: u32 = 60;
/// File-mark size used by Anycubic v5.18.
///
/// Anycubic v5.18 使用的 file-mark size。
pub const FILE_MARK_V518_SIZE: u32 = 68;
/// Header table payload length for Anycubic v5.16.
///
/// Anycubic v5.16 header table payload 长度。
pub const HEADER_TABLE_V516_LENGTH: u32 = 84;
/// Header table total size for Anycubic v5.16.
///
/// Anycubic v5.16 header table 总大小。
pub const HEADER_TABLE_V516_SIZE: u32 = 12 + 4 + HEADER_TABLE_V516_LENGTH;
/// Header table payload length for Anycubic v5.17.
///
/// Anycubic v5.17 header table payload 长度。
pub const HEADER_TABLE_V517_LENGTH: u32 = 92;
/// Header table total size for Anycubic v5.17.
///
/// Anycubic v5.17 header table 总大小。
pub const HEADER_TABLE_V517_SIZE: u32 = 12 + 4 + HEADER_TABLE_V517_LENGTH;
/// Header table payload length for Anycubic v5.18.
///
/// Anycubic v5.18 header table payload 长度。
pub const HEADER_TABLE_V518_LENGTH: u32 = 96;
/// Header table total size for Anycubic v5.18.
///
/// Anycubic v5.18 header table 总大小。
pub const HEADER_TABLE_V518_SIZE: u32 = 12 + 4 + HEADER_TABLE_V518_LENGTH;
/// Layer image color table size for v5 layouts.
///
/// v5 layouts 的 layer image color table 大小。
pub const LAYER_IMAGE_COLOR_TABLE_SIZE: u32 = 4 + 4 + 16 + 4;
/// Extra table payload length for Anycubic v5.16+ motion extensions.
///
/// Anycubic v5.16+ motion extensions 的 extra table payload 长度。
pub const EXTRA_TABLE_LENGTH_V516: u32 = 24;
/// Extra table serialized size for Anycubic v5.16+.
///
/// Anycubic v5.16+ extra table 序列化大小。
pub const EXTRA_TABLE_SIZE_V516: u32 = 12 + 4 + 56;
/// Machine table payload length for Anycubic v5.16/v5.17.
///
/// Anycubic v5.16/v5.17 machine table payload 长度。
pub const MACHINE_TABLE_LENGTH_V516: u32 = 156;
/// Machine table size for Anycubic v5.16/v5.17.
///
/// Anycubic v5.16/v5.17 machine table 大小。
pub const MACHINE_TABLE_SIZE_V516: u32 = MACHINE_TABLE_LENGTH_V516;
/// Machine table payload length for Anycubic v5.18.
///
/// Anycubic v5.18 machine table payload 长度。
pub const MACHINE_TABLE_LENGTH_V518: u32 = 224;
/// Machine table size for Anycubic v5.18.
///
/// Anycubic v5.18 machine table 大小。
pub const MACHINE_TABLE_SIZE_V518: u32 = MACHINE_TABLE_LENGTH_V518;
/// Software table payload length.
///
/// software table payload 长度。
pub const SOFTWARE_TABLE_LENGTH: u32 = 32;
/// Software table serialized size.
///
/// software table 序列化大小。
pub const SOFTWARE_TABLE_SIZE: u32 = 12 + 4 + SOFTWARE_TABLE_LENGTH;
/// Model table payload length.
///
/// model table payload 长度。
pub const MODEL_TABLE_LENGTH: u32 = 32;
/// Model table serialized size.
///
/// model table 序列化大小。
pub const MODEL_TABLE_SIZE: u32 = 12 + 4 + MODEL_TABLE_LENGTH;
/// Byte size of one v5.18 sub-layer definition record.
///
/// 单条 v5.18 sub-layer definition record 的字节长度。
pub const SUB_LAYER_DEF_SIZE: u32 = 44;
/// Fixed prefix size of the v5.18 sub-layer definition table.
///
/// v5.18 sub-layer definition table 的固定前缀大小。
pub const SUB_LAYER_DEF_TABLE_BASE_SIZE: u32 = 12 + 4 + 4 + 4;

/// Anycubic v5.16 file mark.
///
/// Adds a machine table address between `extra_address` and
/// `layer_image_address`.
///
/// Anycubic v5.16 file mark，在 `extra_address` 与 `layer_image_address` 之间增加
/// machine table address。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct FileMarkV516 {
    pub mark: [u8; 12],
    pub version: u32,
    pub number_of_tables: u32,
    pub header_address: u32,
    pub software_address: u32,
    pub preview_address: u32,
    pub layer_image_color_table_address: u32,
    pub layer_definition_address: u32,
    pub extra_address: u32,
    pub machine_address: u32,
    pub layer_image_address: u32,
}

/// Anycubic v5.17 file mark.
///
/// Adds model and reserved table addresses on top of the v5.16 layout.
///
/// Anycubic v5.17 file mark，在 v5.16 layout 基础上增加 model 与 reserved table
/// addresses。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct FileMarkV517 {
    pub mark: [u8; 12],
    pub version: u32,
    pub number_of_tables: u32,
    pub header_address: u32,
    pub software_address: u32,
    pub preview_address: u32,
    pub layer_image_color_table_address: u32,
    pub layer_definition_address: u32,
    pub extra_address: u32,
    pub machine_address: u32,
    pub model_address: u32,
    pub reserved_address: u32,
    pub layer_image_address: u32,
}

/// Anycubic v5.18 file mark.
///
/// Adds sub-layer definition and second-preview table addresses.
///
/// Anycubic v5.18 file mark，增加 sub-layer definition 与 second-preview table 地址。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct FileMarkV518 {
    pub mark: [u8; 12],
    pub version: u32,
    pub number_of_tables: u32,
    pub header_address: u32,
    pub software_address: u32,
    pub preview_address: u32,
    pub layer_image_color_table_address: u32,
    pub layer_definition_address: u32,
    pub extra_address: u32,
    pub machine_address: u32,
    pub layer_image_address: u32,
    pub model_address: u32,
    pub sub_layer_definition_address: u32,
    pub preview2_address: u32,
    pub reserved_address: u32,
}

/// Anycubic v5.16 header table.
///
/// Extends the v1/v5.15 header fields with `advanced_mode`.
///
/// Anycubic v5.16 header table，在 v1/v5.15 header 字段基础上增加 `advanced_mode`。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct HeaderTableV516 {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub pixel_size_um: f32,
    pub layer_height_mm: f32,
    pub exposure_time: f32,
    pub wait_time_before_cure: f32,
    pub bottom_exposure_time: f32,
    pub bottom_layer_count: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub retract_speed: f32,
    pub volume_ml: f32,
    pub anti_aliasing: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub weight_g: f32,
    pub cost: f32,
    pub price_currency_symbol: [u8; 4],
    pub per_layer_settings: u32,
    pub print_time_sec: u32,
    pub transition_layer_count: u32,
    pub transition_layer_type: u32,
    pub advanced_mode: u32,
}

/// Anycubic v5.17 header table.
///
/// Extends v5.16 with software/model version fields.
///
/// Anycubic v5.17 header table，在 v5.16 基础上增加 software/model version 字段。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct HeaderTableV517 {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub pixel_size_um: f32,
    pub layer_height_mm: f32,
    pub exposure_time: f32,
    pub wait_time_before_cure: f32,
    pub bottom_exposure_time: f32,
    pub bottom_layer_count: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub retract_speed: f32,
    pub volume_ml: f32,
    pub anti_aliasing: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub weight_g: f32,
    pub cost: f32,
    pub price_currency_symbol: [u8; 4],
    pub per_layer_settings: u32,
    pub print_time_sec: u32,
    pub transition_layer_count: u32,
    pub transition_layer_type: u32,
    pub advanced_mode: u32,
    pub software_version: u32,
    pub model_version: u32,
}

/// Anycubic v5.18 header table.
///
/// Extends v5.17 with a second-preview enable flag.
///
/// Anycubic v5.18 header table，在 v5.17 基础上增加 second-preview enable flag。
#[derive(BinRead, BinWrite, Debug, Clone)]
#[brw(little)]
pub struct HeaderTableV518 {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub pixel_size_um: f32,
    pub layer_height_mm: f32,
    pub exposure_time: f32,
    pub wait_time_before_cure: f32,
    pub bottom_exposure_time: f32,
    pub bottom_layer_count: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub retract_speed: f32,
    pub volume_ml: f32,
    pub anti_aliasing: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub weight_g: f32,
    pub cost: f32,
    pub price_currency_symbol: [u8; 4],
    pub per_layer_settings: u32,
    pub print_time_sec: u32,
    pub transition_layer_count: u32,
    pub transition_layer_type: u32,
    pub advanced_mode: u32,
    pub software_version: u32,
    pub model_version: u32,
    pub preview2_enabled: u32,
}

/// Anycubic v5 layer image color table.
///
/// Declares grayscale usage and the palette-like grayscale slots expected by
/// Anycubic readers.
///
/// Anycubic v5 layer image color table，声明 grayscale 使用方式以及 Anycubic reader
/// 期望的 palette-like grayscale slots。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct LayerImageColorTable {
    pub use_full_greyscale: u32,
    pub grey_max_count: u32,
    pub grey: [u8; 16],
    pub unknown: u32,
}

/// Anycubic v5.16+ motion extension table.
///
/// Stores two-stage lift/retract values for bottom and normal layers.
///
/// Anycubic v5.16+ motion extension table，保存 bottom/normal layers 的两段式
/// lift/retract 参数。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct ExtraTable {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub bottom_lift_count: u32,
    pub bottom_lift_height1: f32,
    pub bottom_lift_speed1: f32,
    pub bottom_retract_speed2: f32,
    pub bottom_lift_height2: f32,
    pub bottom_lift_speed2: f32,
    pub bottom_retract_speed1: f32,
    pub normal_lift_count: u32,
    pub lift_height1: f32,
    pub lift_speed1: f32,
    pub retract_speed2: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub retract_speed1: f32,
}

/// Anycubic v5.16/v5.17 machine table.
///
/// Contains machine display facts and the layer image format string consumed by
/// Anycubic readers.
///
/// Anycubic v5.16/v5.17 machine table，包含机器 display facts 与 Anycubic reader 读取的
/// layer image format 字符串。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct MachineTableV516 {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub machine_name: [u8; 96],
    pub layer_image_format: [u8; 16],
    pub max_antialiasing_level: u32,
    pub property_fields: u32,
    pub display_width: f32,
    pub display_height: f32,
    pub machine_z: f32,
    pub max_file_version: u32,
    pub machine_background: u32,
}

/// Anycubic v5.18 machine table.
///
/// Extends v5.16/v5.17 machine metadata with explicit resolution fields and a
/// reserved tail.
///
/// Anycubic v5.18 machine table，在 v5.16/v5.17 machine metadata 基础上增加显式分辨率
/// 字段与 reserved tail。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct MachineTableV518 {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub machine_name: [u8; 96],
    pub layer_image_format: [u8; 16],
    pub max_antialiasing_level: u32,
    pub property_fields: u32,
    pub display_width: f32,
    pub display_height: f32,
    pub machine_z: f32,
    pub max_file_version: u32,
    pub machine_background: u32,
    pub resolution_x: u16,
    pub resolution_y: u16,
    pub reserved: [u8; 64],
}

/// Anycubic software table.
///
/// Anycubic software table。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct SoftwareTable {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub software_name: [u8; 32],
}

/// Anycubic model table.
///
/// Anycubic model table。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct ModelTable {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub model_name: [u8; 32],
}

/// Anycubic v5.18 sub-layer definition.
///
/// Points at one encoded sub-layer payload.
///
/// Anycubic v5.18 sub-layer definition，指向单个 encoded sub-layer payload。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct SubLayerDef {
    pub data_address: u32,
    pub data_length: u32,
    pub non_zero_pixel_count: u32,
    pub padding1: f32,
    pub padding2: f32,
    pub padding3: f32,
    pub padding4: f32,
    pub padding5: f32,
    pub padding6: f32,
    pub padding7: f32,
    pub padding8: f32,
}

/// Anycubic v5.18 second preview table.
///
/// Anycubic v5.18 second preview table。
#[derive(BinWrite, Debug, Clone)]
#[brw(little)]
pub struct Preview2Table {
    pub mark: [u8; 12],
    pub table_length: u32,
    pub resolution_x: u32,
    pub x_mark: [u8; 4],
    pub resolution_y: u32,
}
