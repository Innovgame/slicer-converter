//! GOO binary header and layer-table records.
//!
//! GOO mixes byte-order conventions: most numeric fields are big-endian, while
//! fixed string byte arrays are written as raw bytes. The header also embeds two
//! fixed-size RGB565 previews before the layer definition table.
//!
//! GOO header 混合了不同二进制约定：大部分数字字段是 big-endian，固定字符串字段按
//! raw bytes 写入；header 还在 layer definition table 前嵌入两张固定尺寸 RGB565 preview。

use binrw::{BinRead, BinWrite};

use crate::formats::common::binary::{read_be_u32, BinaryReader, BinaryWriter};

/// GOO file header before the layer definition table.
///
/// The serialized header contains fixed metadata fields, two raw RGB565 preview
/// images, print settings, and `layer_def_address`, which points to the first
/// [`GooLayerDef`] record.
///
/// GOO 文件的 layer definition table 前置 header。序列化后包含固定 metadata 字段、
/// 两张 raw RGB565 preview、打印参数，以及指向第一条 [`GooLayerDef`] 的
/// `layer_def_address`。
#[derive(Debug, Clone)]
pub struct GooHeader {
    pub version: [u8; 4], // "V3.0"
    pub magic: [u8; 8],
    pub software_name: [u8; 32],
    pub software_version: [u8; 24],
    pub file_create_time: [u8; 24],
    pub machine_name: [u8; 32],
    pub machine_type: [u8; 32],
    pub profile_name: [u8; 32],
    pub anti_aliasing_level: u16,
    pub grey_level: u16,
    pub blur_level: u16,
    pub small_preview_565: Vec<u8>, // 26912 bytes (116×116×2)
    pub big_preview_565: Vec<u8>,   // 168200 bytes (290×290×2)
    pub layer_count: u32,
    pub resolution_x: u16,
    pub resolution_y: u16,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub display_width_mm: f32,
    pub display_height_mm: f32,
    pub machine_z_mm: f32,
    pub layer_height_mm: f32,
    pub exposure_time: f32,
    pub delay_mode: u8,
    pub light_off_delay: f32,
    // NOTE: UVtools has NO separate BottomLightOffDelay field in the file;
    // it reuses LightOffDelay for both.
    pub bottom_wait_time_after_cure: f32,
    pub bottom_wait_time_after_lift: f32,
    pub bottom_wait_time_before_cure: f32,
    pub wait_time_after_cure: f32,
    pub wait_time_after_lift: f32,
    pub wait_time_before_cure: f32,
    pub bottom_exposure_time: f32,
    pub bottom_layer_count: u32,
    // Lift/retract fields — order matches UVtools exactly:
    // Bottom tier-1, Normal tier-1, Bottom tier-2, Normal tier-2
    pub bottom_lift_height: f32,
    pub bottom_lift_speed: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub bottom_retract_height: f32,
    pub bottom_retract_speed: f32,
    pub retract_height: f32,
    pub retract_speed: f32,
    pub bottom_lift_height2: f32,
    pub bottom_lift_speed2: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub bottom_retract_height2: f32,
    pub bottom_retract_speed2: f32,
    pub retract_height2: f32,
    pub retract_speed2: f32,
    pub bottom_light_pwm: u16,
    pub light_pwm: u16,
    pub per_layer_settings: bool,
    pub print_time_sec: u32,
    pub volume_mm3: f32,
    pub material_grams: f32,
    pub material_cost: f32,
    pub currency_symbol: [u8; 8],
    pub layer_def_address: u32,
    pub gray_scale_level: u8,
    pub transition_layer_count: u16,
}

/// Width of the embedded small preview, in pixels.
///
/// 内嵌 small preview 宽度（像素）。
pub const SMALL_PREVIEW_W: u32 = 116;
/// Height of the embedded small preview, in pixels.
///
/// 内嵌 small preview 高度（像素）。
pub const SMALL_PREVIEW_H: u32 = 116;
/// Byte length of the embedded small RGB565 preview.
///
/// 内嵌 small RGB565 preview 字节长度。
pub const SMALL_PREVIEW_SIZE: usize = (SMALL_PREVIEW_W * SMALL_PREVIEW_H * 2) as usize; // 26912
/// Width of the embedded large preview, in pixels.
///
/// 内嵌 large preview 宽度（像素）。
pub const BIG_PREVIEW_W: u32 = 290;
/// Height of the embedded large preview, in pixels.
///
/// 内嵌 large preview 高度（像素）。
pub const BIG_PREVIEW_H: u32 = 290;
/// Byte length of the embedded large RGB565 preview.
///
/// 内嵌 large RGB565 preview 字节长度。
pub const BIG_PREVIEW_SIZE: usize = (BIG_PREVIEW_W * BIG_PREVIEW_H * 2) as usize; // 168200
const HEADER_FIXED_SIZE_WITH_DELIMITERS: usize = 365;

/// GOO per-layer definition record, all big-endian.
///
/// Each record precedes one encoded RLE payload. `data_length` counts the RLE
/// payload bytes only; the trailing layer delimiter is written after the payload
/// and is not included in this length.
///
/// GOO 单层定义表条目（全 big-endian），位于对应 encoded RLE payload 之前。
/// `data_length` 只统计 RLE payload 字节数，不包含 payload 后追加的 layer delimiter。
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(big)]
pub struct GooLayerDef {
    pub pause_flag: u16,
    pub pause_position_z: f32,
    pub position_z: f32,
    pub exposure_time: f32,
    pub light_off_delay: f32,
    pub wait_time_after_cure: f32,
    pub wait_time_after_lift: f32,
    pub wait_time_before_cure: f32,
    pub lift_height: f32,
    pub lift_speed: f32,
    pub lift_height2: f32,
    pub lift_speed2: f32,
    pub retract_height: f32,
    pub retract_speed: f32,
    pub retract_height2: f32,
    pub retract_speed2: f32,
    pub light_pwm: u16,
    pub delimiter: [u8; 2],
    pub data_length: u32,
}

impl GooHeader {
    /// Serialize the mixed-layout GOO header to bytes.
    ///
    /// `binrw` is still used for the pure big-endian layer records, but the main
    /// header is written manually because previews and fixed strings are
    /// interleaved with numeric fields.
    ///
    /// 将 mixed-layout GOO header 序列化为字节流。纯 big-endian 的 layer records 仍可用
    /// `binrw`，但主 header 中 preview、固定字符串与数字字段交错，因此这里手动写入。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            HEADER_FIXED_SIZE_WITH_DELIMITERS
                + self.small_preview_565.len()
                + self.big_preview_565.len(),
        );
        let mut writer = BinaryWriter::new(&mut buf);

        writer.write_bytes(&self.version);
        writer.write_bytes(&self.magic);
        writer.write_bytes(&self.software_name);
        writer.write_bytes(&self.software_version);
        writer.write_bytes(&self.file_create_time);
        writer.write_bytes(&self.machine_name);
        writer.write_bytes(&self.machine_type);
        writer.write_bytes(&self.profile_name);
        writer.write_be_u16(self.anti_aliasing_level);
        writer.write_be_u16(self.grey_level);
        writer.write_be_u16(self.blur_level);

        // Small preview + delimiter
        writer.write_bytes(&self.small_preview_565);
        writer.write_bytes(&super::DELIMITER);

        // Big preview + delimiter
        writer.write_bytes(&self.big_preview_565);
        writer.write_bytes(&super::DELIMITER);

        writer.write_be_u32(self.layer_count);
        writer.write_be_u16(self.resolution_x);
        writer.write_be_u16(self.resolution_y);
        writer.write_u8(self.mirror_x as u8);
        writer.write_u8(self.mirror_y as u8);
        writer.write_be_f32(self.display_width_mm);
        writer.write_be_f32(self.display_height_mm);
        writer.write_be_f32(self.machine_z_mm);
        writer.write_be_f32(self.layer_height_mm);
        writer.write_be_f32(self.exposure_time);
        writer.write_u8(self.delay_mode);
        writer.write_be_f32(self.light_off_delay);
        writer.write_be_f32(self.bottom_wait_time_after_cure);
        writer.write_be_f32(self.bottom_wait_time_after_lift);
        writer.write_be_f32(self.bottom_wait_time_before_cure);
        writer.write_be_f32(self.wait_time_after_cure);
        writer.write_be_f32(self.wait_time_after_lift);
        writer.write_be_f32(self.wait_time_before_cure);
        writer.write_be_f32(self.bottom_exposure_time);
        writer.write_be_u32(self.bottom_layer_count);
        writer.write_be_f32(self.bottom_lift_height);
        writer.write_be_f32(self.bottom_lift_speed);
        writer.write_be_f32(self.lift_height);
        writer.write_be_f32(self.lift_speed);
        writer.write_be_f32(self.bottom_retract_height);
        writer.write_be_f32(self.bottom_retract_speed);
        writer.write_be_f32(self.retract_height);
        writer.write_be_f32(self.retract_speed);
        writer.write_be_f32(self.bottom_lift_height2);
        writer.write_be_f32(self.bottom_lift_speed2);
        writer.write_be_f32(self.lift_height2);
        writer.write_be_f32(self.lift_speed2);
        writer.write_be_f32(self.bottom_retract_height2);
        writer.write_be_f32(self.bottom_retract_speed2);
        writer.write_be_f32(self.retract_height2);
        writer.write_be_f32(self.retract_speed2);
        writer.write_be_u16(self.bottom_light_pwm);
        writer.write_be_u16(self.light_pwm);
        writer.write_u8(self.per_layer_settings as u8);
        writer.write_be_u32(self.print_time_sec);
        writer.write_be_f32(self.volume_mm3);
        writer.write_be_f32(self.material_grams);
        writer.write_be_f32(self.material_cost);
        writer.write_bytes(&self.currency_symbol);
        writer.write_be_u32(self.layer_def_address);
        writer.write_u8(self.gray_scale_level);
        writer.write_be_u16(self.transition_layer_count);

        buf
    }

    /// Parse a GOO header from the beginning of a file buffer.
    ///
    /// The parser validates the GOO magic bytes and reads the fixed preview
    /// payloads eagerly so metadata consumers can expose thumbnails without
    /// seeking elsewhere.
    ///
    /// 从文件 buffer 起始位置解析 GOO header。parser 会校验 GOO magic bytes，并直接读取固定
    /// preview payload，方便 metadata consumer 不额外 seek 就能暴露 thumbnails。
    pub fn from_bytes(data: &[u8]) -> Result<Self, crate::common::ConvertError> {
        if data.len() < 400 {
            return Err(crate::common::ConvertError::BufferTooSmall {
                need: 400,
                got: data.len(),
            });
        }

        let mut reader = BinaryReader::new(data);

        let version = reader.read_bytes::<4>()?;
        let magic = reader.read_bytes::<8>()?;

        // Validate magic
        if &magic != super::MAGIC_BYTES {
            return Err(crate::common::ConvertError::InvalidMagic {
                expected: 0x0744_4C50,
                actual: read_be_u32(&magic, 0)?,
            });
        }

        let software_name = reader.read_bytes::<32>()?;
        let software_version = reader.read_bytes::<24>()?;
        let file_create_time = reader.read_bytes::<24>()?;
        let machine_name = reader.read_bytes::<32>()?;
        let machine_type = reader.read_bytes::<32>()?;
        let profile_name = reader.read_bytes::<32>()?;

        let anti_aliasing_level = reader.read_be_u16()?;
        let grey_level = reader.read_be_u16()?;
        let blur_level = reader.read_be_u16()?;

        // Small preview
        let small_preview_565 = reader.read_range(SMALL_PREVIEW_SIZE)?.to_vec();
        reader.skip(2)?; // delimiter

        // Big preview
        let big_preview_565 = reader.read_range(BIG_PREVIEW_SIZE)?.to_vec();
        reader.skip(2)?; // delimiter

        let layer_count = reader.read_be_u32()?;
        let resolution_x = reader.read_be_u16()?;
        let resolution_y = reader.read_be_u16()?;
        let mirror_x = reader.read_u8()? != 0;
        let mirror_y = reader.read_u8()? != 0;
        let display_width_mm = reader.read_be_f32()?;
        let display_height_mm = reader.read_be_f32()?;
        let machine_z_mm = reader.read_be_f32()?;
        let layer_height_mm = reader.read_be_f32()?;
        let exposure_time = reader.read_be_f32()?;
        let delay_mode = reader.read_u8()?;
        let light_off_delay = reader.read_be_f32()?;

        let bottom_wait_time_after_cure = reader.read_be_f32()?;
        let bottom_wait_time_after_lift = reader.read_be_f32()?;
        let bottom_wait_time_before_cure = reader.read_be_f32()?;
        let wait_time_after_cure = reader.read_be_f32()?;
        let wait_time_after_lift = reader.read_be_f32()?;
        let wait_time_before_cure = reader.read_be_f32()?;
        let bottom_exposure_time = reader.read_be_f32()?;
        let bottom_layer_count = reader.read_be_u32()?;

        let bottom_lift_height = reader.read_be_f32()?;
        let bottom_lift_speed = reader.read_be_f32()?;
        let lift_height = reader.read_be_f32()?;
        let lift_speed = reader.read_be_f32()?;
        let bottom_retract_height = reader.read_be_f32()?;
        let bottom_retract_speed = reader.read_be_f32()?;
        let retract_height = reader.read_be_f32()?;
        let retract_speed = reader.read_be_f32()?;
        let bottom_lift_height2 = reader.read_be_f32()?;
        let bottom_lift_speed2 = reader.read_be_f32()?;
        let lift_height2 = reader.read_be_f32()?;
        let lift_speed2 = reader.read_be_f32()?;
        let bottom_retract_height2 = reader.read_be_f32()?;
        let bottom_retract_speed2 = reader.read_be_f32()?;
        let retract_height2 = reader.read_be_f32()?;
        let retract_speed2 = reader.read_be_f32()?;

        let bottom_light_pwm = reader.read_be_u16()?;
        let light_pwm = reader.read_be_u16()?;
        let per_layer_settings = reader.read_u8()? != 0;
        let print_time_sec = reader.read_be_u32()?;
        let volume_mm3 = reader.read_be_f32()?;
        let material_grams = reader.read_be_f32()?;
        let material_cost = reader.read_be_f32()?;

        let currency_symbol = reader.read_bytes::<8>()?;

        let layer_def_address = reader.read_be_u32()?;
        let gray_scale_level = reader.read_u8()?;
        let transition_layer_count = reader.read_be_u16()?;

        Ok(Self {
            version,
            magic,
            software_name,
            software_version,
            file_create_time,
            machine_name,
            machine_type,
            profile_name,
            anti_aliasing_level,
            grey_level,
            blur_level,
            small_preview_565,
            big_preview_565,
            layer_count,
            resolution_x,
            resolution_y,
            mirror_x,
            mirror_y,
            display_width_mm,
            display_height_mm,
            machine_z_mm,
            layer_height_mm,
            exposure_time,
            delay_mode,
            light_off_delay,
            bottom_wait_time_after_cure,
            bottom_wait_time_after_lift,
            bottom_wait_time_before_cure,
            wait_time_after_cure,
            wait_time_after_lift,
            wait_time_before_cure,
            bottom_exposure_time,
            bottom_layer_count,
            bottom_lift_height,
            bottom_lift_speed,
            lift_height,
            lift_speed,
            bottom_retract_height,
            bottom_retract_speed,
            retract_height,
            retract_speed,
            bottom_lift_height2,
            bottom_lift_speed2,
            lift_height2,
            lift_speed2,
            bottom_retract_height2,
            bottom_retract_speed2,
            retract_height2,
            retract_speed2,
            bottom_light_pwm,
            light_pwm,
            per_layer_settings,
            print_time_sec,
            volume_mm3,
            material_grams,
            material_cost,
            currency_symbol,
            layer_def_address,
            gray_scale_level,
            transition_layer_count,
        })
    }

    /// Extract the machine name as a trimmed UTF-8 string.
    ///
    /// Fixed GOO strings are NUL-padded byte arrays. Invalid UTF-8 is decoded
    /// lossily so real-world files with vendor-specific bytes can still be read.
    ///
    /// 将固定长度、NUL-padded 的 GOO machine name 提取为 trimmed UTF-8 字符串。
    /// 遇到非 UTF-8 字节时使用 lossy decode，以兼容厂商文件。
    pub fn machine_name_str(&self) -> String {
        String::from_utf8_lossy(&self.machine_name)
            .trim_end_matches('\0')
            .trim()
            .to_string()
    }
}
