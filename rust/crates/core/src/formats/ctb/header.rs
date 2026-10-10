use binrw::{BinRead, BinWrite};

/// Fixed byte size of the CTB v2+ slicer information prefix.
///
/// The machine name and v4/v5 extension blocks live outside this fixed prefix
/// and are reached through address/length fields.
///
/// CTB v2+ slicer information 固定前缀大小。machine name 与 v4/v5 扩展块不在这个
/// 固定结构体内，而是通过 address/length 字段引用。
pub const CTB_SLICER_INFO_FIXED_SIZE: u32 = 76;
/// Byte size of the standard CTB v4 disclaimer string block.
///
/// CTB v4 标准 disclaimer 字符串块大小。
pub const CTB_V4_DISCLAIMER_SIZE: u32 = 320;
/// Byte size of the GKtwo-specific CTB v4 disclaimer string block.
///
/// GKtwo CTB v4 disclaimer 字符串块大小。
pub const CTB_V4_GKTWO_DISCLAIMER_SIZE: u32 = 323;
/// Fixed byte size of the CTB v4 print-parameters extension block.
///
/// CTB v4 print parameters 扩展块固定大小。
pub const CTB_PRINT_PARAMS_V4_SIZE: u32 = 464;
/// Fixed byte size of the CTB v5 resin-parameters prefix.
///
/// Resin and machine strings follow this prefix and are referenced by offsets.
///
/// CTB v5 resin parameters 固定前缀大小；resin/machine 字符串跟在其后并通过 offset 引用。
pub const CTB_RESIN_PARAMS_FIXED_SIZE: u32 = 40;
/// Reserved payload size inside [`CtbPrintParamsV4`].
///
/// [`CtbPrintParamsV4`] 内部保留字段的字节数。
pub const CTB_V4_RESERVED_SIZE: usize = 380;

/// CTB file header, 128 bytes, little-endian.
///
/// The header stores global print settings plus offsets to optional preview,
/// print-parameter, slicer-info, and layer-table sections. Layer payload
/// offsets are patched after all layer RLE data has been encoded.
///
/// CTB 文件主头部（128 字节，小端序）。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbHeader {
    pub magic: u32,
    pub version: u32,
    pub bed_size_x: f32,
    pub bed_size_y: f32,
    pub bed_size_z: f32,
    pub _reserved1: u32,
    pub _reserved2: u32,
    pub total_height_mm: f32,
    pub layer_height_mm: f32,
    pub exposure_seconds: f32,
    pub bottom_exposure_seconds: f32,
    pub light_off_delay: f32,
    pub bottom_layer_count: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub preview_large_offset: u32,
    pub layer_table_offset: u32,
    pub layer_count: u32,
    pub preview_small_offset: u32,
    pub print_time_sec: u32,
    pub projector_type: u32,
    pub print_params_offset: u32,
    pub print_params_size: u32,
    pub anti_alias_level: u32,
    pub light_pwm: u16,
    pub bottom_light_pwm: u16,
    pub encryption_key: u32,
    pub slicer_info_offset: u32,
    pub slicer_info_size: u32,
    /// Padding/reserved fields to reach 128 bytes total (CTB v3 format).
    pub _reserved3: u32,
    pub _reserved4: u32,
    pub _reserved5: u32,
    pub _reserved6: u32,
}

/// CTB v2+ print-parameters block, 60 bytes.
///
/// This block carries motion/statistics values that older Chitubox readers
/// expect outside the main 128-byte header.
///
/// CTB v2+ 打印参数块（60 字节），保存旧 reader 期望在主 header 外读取的运动与统计字段。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbPrintParams {
    pub bottom_lift_height: f32,
    pub bottom_lift_speed: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub retract_speed: f32,
    pub volume_ml: f32,
    pub weight_g: f32,
    pub cost_dollars: f32,
    pub bottom_light_off_delay: f32,
    pub light_off_delay: f32,
    pub bottom_layer_count: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
    pub _pad4: u32,
}

/// CTB v2+ slicer information fixed block. The machine name string follows at
/// `machine_name_address` and is not part of this fixed-size struct.
///
/// CTB v2+ slicer information 固定块。machine name 字符串位于 `machine_name_address`，
/// 不属于这个固定大小结构体。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbSlicerInfo {
    pub bottom_lift_height2: f32,
    pub bottom_lift_speed2: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub retract_height2: f32,
    pub retract_speed2: f32,
    pub rest_time_after_lift: f32,
    pub machine_name_address: u32,
    pub machine_name_size: u32,
    pub anti_alias_flag: u8,
    pub padding: u16,
    pub per_layer_settings: u8,
    pub modified_timestamp_minutes: u32,
    pub anti_alias_level: u32,
    pub software_version: u32,
    pub rest_time_after_retract: f32,
    pub rest_time_after_lift2: f32,
    pub transition_layer_count: u32,
    pub print_parameters_v4_address: u32,
    pub padding2: u32,
    pub padding3: u32,
}

/// CTB v4+ print parameters block. The disclaimer string is stored separately
/// and referenced through `disclaimer_address` / `disclaimer_length`.
///
/// For CTB v5 this block also points at the resin-parameters prefix.
///
/// CTB v4+ print parameters 扩展块。disclaimer 字符串单独存放，并通过
/// `disclaimer_address` / `disclaimer_length` 引用；CTB v5 还会从这里引用 resin 参数块。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbPrintParamsV4 {
    pub bottom_retract_speed: f32,
    pub bottom_retract_speed2: f32,
    pub padding1: u32,
    pub four1: f32,
    pub padding2: u32,
    pub four2: f32,
    pub rest_time_after_retract: f32,
    pub rest_time_after_lift: f32,
    pub rest_time_before_lift: f32,
    pub bottom_retract_height2: f32,
    pub unknown1: f32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub last_layer_index: u32,
    pub padding3: u32,
    pub padding4: u32,
    pub padding5: u32,
    pub padding6: u32,
    pub disclaimer_address: u32,
    pub disclaimer_length: u32,
    pub resin_parameters_address: u32,
    pub reserved: [u8; CTB_V4_RESERVED_SIZE],
}

/// Fixed prefix of the CTB v5 resin parameters block. Variable-length strings
/// follow according to the address/length pairs.
///
/// CTB v5 resin parameters 固定前缀。可变长字符串按 address/length 字段跟随在后续区域。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbResinParameters {
    pub padding1: u32,
    pub resin_color_b: u8,
    pub resin_color_g: u8,
    pub resin_color_r: u8,
    pub resin_color_a: u8,
    pub machine_name_address: u32,
    pub resin_type_length: u32,
    pub resin_type_address: u32,
    pub resin_name_length: u32,
    pub resin_name_address: u32,
    pub machine_name_length: u32,
    pub resin_density: f32,
    pub padding2: u32,
}

/// Per-layer definition, 36 bytes per entry in the main layer table.
///
/// `data_address` and `data_size` are placeholders during initial header
/// construction and are finalized once encoded layer sizes are known.
///
/// 主图层表条目（每条 36 字节），描述单层打印参数与数据偏移。初始构建时
/// `data_address` / `data_size` 是占位符，等待 encoded layer sizes 确定后回填。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbLayerDef {
    pub position_z: f32,
    pub exposure_time: f32,
    pub light_off_seconds: f32,
    pub data_address: u32,
    pub data_size: u32,
    pub page_number: u32,
    pub table_size: u32,
    pub _reserved1: u32,
    pub _reserved2: u32,
}

/// Extended per-layer settings for CTB v3+ style layer records.
///
/// UVTools-compatible CTB v4/v5 files keep a contiguous 36-byte main table and
/// place each extended record immediately before the matching layer payload.
///
/// CTB v3+ 扩展图层设置。UVTools-compatible CTB v4/v5 文件保留连续的 36 字节主表，
/// 并把每个 extended record 放在对应 layer payload 之前。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbLayerDefEx {
    pub total_size: u32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub retract_speed: f32,
    pub retract_height2: f32,
    pub retract_speed2: f32,
    pub rest_time_before_lift: f32,
    pub rest_time_after_lift: f32,
    pub rest_time_after_retract: f32,
    pub light_pwm: f32,
}

/// Preview image header, 32 bytes.
///
/// The image payload is CTB RGB15 RLE and starts at `image_offset`.
///
/// 预览图头部（32 字节），记录预览图分辨率和数据偏移；payload 为 CTB RGB15 RLE。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub struct CtbPreviewHeader {
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub image_offset: u32,
    pub image_length: u32,
    pub _reserved1: u32,
    pub _reserved2: u32,
    pub _reserved3: u32,
    pub _reserved4: u32,
}
