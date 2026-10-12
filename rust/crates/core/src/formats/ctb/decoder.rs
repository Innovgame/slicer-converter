use std::io::Cursor;

use binrw::BinRead;

use super::header::{
    CtbHeader, CtbLayerDef, CtbPreviewHeader, CtbPrintParams, CtbPrintParamsV4, CtbResinParameters,
    CtbSlicerInfo, CTB_PRINT_PARAMS_V4_SIZE, CTB_RESIN_PARAMS_FIXED_SIZE,
    CTB_SLICER_INFO_FIXED_SIZE,
};
use super::{MAGIC_CBDDLP, MAGIC_CTB, MAGIC_CTB_V4, MAGIC_CTB_V4_GKTWO};
use crate::common::{ConvertError, FileMetadata, FileMetadataBuilder, LayerOffset, Thumbnail};

/// Stateful CTB/CBDDLP header decoder.
///
/// The decoder caches the parsed main header and print-parameter block for
/// callers that need to inspect decode state after parsing. Layer image payloads
/// are still decoded by the stateless RLE helpers in [`super::rle`].
///
/// 有状态 CTB/CBDDLP header decoder。解析后会缓存 main header 与 print-parameter
/// block，便于调用方检查 decode state；layer image payload 仍由 [`super::rle`] 中的
/// stateless helper 解码。
pub struct CtbDecoder {
    header: Option<CtbHeader>,
    print_params: Option<CtbPrintParams>,
}

impl Default for CtbDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl CtbDecoder {
    /// Create an empty CTB decoder with no cached header state.
    ///
    /// 创建一个尚未缓存 header state 的 CTB decoder。
    pub fn new() -> Self {
        Self {
            header: None,
            print_params: None,
        }
    }

    /// Parse the CTB file header from the initial bytes of the file.
    ///
    /// Returns universal metadata and a layer offset table for streaming decode.
    /// This method accepts CBDDLP, CTB v1-v3, generic CTB v4/v5, and GKtwo magic
    /// values, then normalizes version-specific metadata into [`FileMetadata`].
    ///
    /// 从文件起始字节解析 CTB header，返回通用 metadata 与用于 streaming decode 的
    /// layer offset table。该方法接受 CBDDLP、CTB v1-v3、generic CTB v4/v5 与 GKtwo
    /// magic，并把版本差异归一到 [`FileMetadata`]。
    pub fn parse_header(
        &mut self,
        data: &[u8],
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        if data.len() < 128 {
            return Err(ConvertError::BufferTooSmall {
                need: 128,
                got: data.len(),
            });
        }

        let mut cursor = Cursor::new(data);
        let header = CtbHeader::read(&mut cursor)?;

        // Validate magic
        match header.magic {
            MAGIC_CBDDLP | MAGIC_CTB | MAGIC_CTB_V4 | MAGIC_CTB_V4_GKTWO => {}
            other => {
                return Err(ConvertError::InvalidMagic {
                    expected: MAGIC_CTB,
                    actual: other,
                });
            }
        }

        // Try to read print parameters if present
        let print_params = if header.print_params_offset > 0
            && (header.print_params_offset as usize + 60) <= data.len()
        {
            let mut params_cursor = Cursor::new(&data[header.print_params_offset as usize..]);
            CtbPrintParams::read(&mut params_cursor).ok()
        } else {
            None
        };

        let slicer_info = Self::read_slicer_info(data, &header);
        let print_params_v4 = slicer_info
            .as_ref()
            .and_then(|info| Self::read_print_params_v4(data, &header, info));
        let resin_params = print_params_v4
            .as_ref()
            .and_then(|params| Self::read_resin_params(data, &header, params));

        let slicer_machine_name = slicer_info
            .as_ref()
            .and_then(|info| {
                Self::read_string(
                    data,
                    info.machine_name_address as usize,
                    info.machine_name_size as usize,
                )
            })
            .unwrap_or_default();
        let resin_machine_name = resin_params
            .as_ref()
            .and_then(|resin| {
                Self::read_string(
                    data,
                    resin.machine_name_address as usize,
                    resin.machine_name_length as usize,
                )
            })
            .unwrap_or_default();
        let machine_name = if !resin_machine_name.trim().is_empty() {
            resin_machine_name
        } else {
            slicer_machine_name
        };

        // Read layer table
        let layer_table_offset = header.layer_table_offset as usize;
        let layer_count = header.layer_count as usize;

        // UVTools stores the v3+ LayerDefEx blocks before each layer payload.
        // The main LayerDef table itself remains a contiguous 36-byte table.
        let layer_entry_stride = 36usize;

        let table_end = layer_count
            .checked_mul(layer_entry_stride)
            .and_then(|size| layer_table_offset.checked_add(size))
            .ok_or_else(|| ConvertError::BinrwError("CTB layer table range overflows".into()))?;
        if table_end > data.len() {
            return Err(ConvertError::BufferTooSmall {
                need: table_end,
                got: data.len(),
            });
        }
        let mut layer_offsets = Vec::with_capacity(layer_count);
        for i in 0..layer_count {
            let offset = layer_table_offset + i * layer_entry_stride;
            let mut layer_cursor = Cursor::new(&data[offset..table_end]);
            let layer_def = CtbLayerDef::read(&mut layer_cursor)?;
            let abs_offset =
                (layer_def.page_number as u64) * 4_294_967_296 + layer_def.data_address as u64;
            layer_offsets.push(LayerOffset {
                offset: abs_offset,
                size: layer_def.data_size as u64,
            });
        }

        // Extract preview thumbnails
        let mut thumbnails = Vec::new();
        for preview_offset in [header.preview_large_offset, header.preview_small_offset] {
            if let Some(thumb) = Self::decode_preview(data, preview_offset) {
                thumbnails.push(thumb);
            }
        }

        // Build universal metadata
        let metadata = FileMetadataBuilder::new()
            .resolution(header.resolution_x, header.resolution_y)
            .display_size_mm(header.bed_size_x, header.bed_size_y)
            .machine_z_mm(header.bed_size_z)
            .machine_name(machine_name)
            .mirror(header.projector_type == 1, false)
            .layer_height_mm(header.layer_height_mm)
            .layer_count(header.layer_count)
            .bottom_layer_count(header.bottom_layer_count)
            .transition_layer_count(
                slicer_info
                    .as_ref()
                    .map_or(0, |info| info.transition_layer_count),
            )
            .anti_alias_level(anti_alias_level(&header, slicer_info.as_ref()))
            .encryption_key(header.encryption_key)
            .exposure(header.exposure_seconds, header.bottom_exposure_seconds)
            .light_off_delay(
                header.light_off_delay,
                print_params
                    .as_ref()
                    .map_or(0.0, |p| p.bottom_light_off_delay),
            )
            .light_pwm(header.light_pwm, header.bottom_light_pwm)
            .lift(
                first_stage_lift_height(
                    print_params.as_ref().map_or(5.0, |p| p.lift_height),
                    slicer_info.as_ref().map_or(0.0, |info| info.lift_height2),
                    header.version,
                ),
                print_params.as_ref().map_or(65.0, |p| p.lift_speed),
            )
            .bottom_lift(
                first_stage_lift_height(
                    print_params.as_ref().map_or(5.0, |p| p.bottom_lift_height),
                    slicer_info
                        .as_ref()
                        .map_or(0.0, |info| info.bottom_lift_height2),
                    header.version,
                ),
                print_params.as_ref().map_or(65.0, |p| p.bottom_lift_speed),
            )
            .retract(print_params.as_ref().map_or(150.0, |p| p.retract_speed))
            .lift2(
                slicer_info.as_ref().map_or(0.0, |info| info.lift_height2),
                slicer_info.as_ref().map_or(0.0, |info| info.lift_speed2),
            )
            .retract2(
                slicer_info
                    .as_ref()
                    .map_or(0.0, |info| info.retract_height2),
                slicer_info.as_ref().map_or(0.0, |info| info.retract_speed2),
            )
            .wait_times(
                print_params_v4
                    .as_ref()
                    .map_or(0.0, |params| params.rest_time_after_retract),
                print_params_v4
                    .as_ref()
                    .map_or(0.0, |params| params.rest_time_before_lift),
                print_params_v4.as_ref().map_or_else(
                    || {
                        slicer_info
                            .as_ref()
                            .map_or(0.0, |info| info.rest_time_after_lift)
                    },
                    |params| params.rest_time_after_lift,
                ),
            )
            .statistics(
                header.print_time_sec,
                print_params.as_ref().map_or(0.0, |p| p.volume_ml),
                print_params.as_ref().map_or(0.0, |p| p.weight_g),
                print_params.as_ref().map_or(0.0, |p| p.cost_dollars),
            )
            .thumbnails(thumbnails)
            .build()?;

        self.header = Some(header);
        self.print_params = print_params;

        Ok((metadata, layer_offsets))
    }

    /// Decode a CTB preview image at the given file offset.
    fn decode_preview(data: &[u8], offset: u32) -> Option<Thumbnail> {
        if offset == 0 {
            return None;
        }
        let off = offset as usize;
        if off + 32 > data.len() {
            return None;
        }

        let mut cursor = Cursor::new(&data[off..]);
        let preview_header = CtbPreviewHeader::read(&mut cursor).ok()?;

        let img_len = preview_header.image_length as usize;
        let abs_img_off = preview_header.image_offset as usize;

        if abs_img_off + img_len > data.len() {
            return None;
        }

        let rle_data = &data[abs_img_off..abs_img_off + img_len];
        let w = preview_header.resolution_x;
        let h = preview_header.resolution_y;

        super::preview::decode_rgb15_rle(rle_data, w, h)
            .ok()
            .map(|pixels_rgba| Thumbnail {
                width: w,
                height: h,
                pixels_rgba,
            })
    }

    fn read_slicer_info(data: &[u8], header: &CtbHeader) -> Option<CtbSlicerInfo> {
        if header.slicer_info_offset == 0
            || header.slicer_info_size < CTB_SLICER_INFO_FIXED_SIZE
            || header.version < 2
        {
            return None;
        }
        let offset = header.slicer_info_offset as usize;
        if offset + CTB_SLICER_INFO_FIXED_SIZE as usize > data.len() {
            return None;
        }
        let mut cursor = Cursor::new(&data[offset..]);
        CtbSlicerInfo::read(&mut cursor).ok()
    }

    fn read_print_params_v4(
        data: &[u8],
        header: &CtbHeader,
        slicer_info: &CtbSlicerInfo,
    ) -> Option<CtbPrintParamsV4> {
        if header.version < 4 || slicer_info.print_parameters_v4_address == 0 {
            return None;
        }
        let offset = slicer_info.print_parameters_v4_address as usize;
        if offset + CTB_PRINT_PARAMS_V4_SIZE as usize > data.len() {
            return None;
        }
        let mut cursor = Cursor::new(&data[offset..]);
        CtbPrintParamsV4::read(&mut cursor).ok()
    }

    fn read_resin_params(
        data: &[u8],
        header: &CtbHeader,
        print_params_v4: &CtbPrintParamsV4,
    ) -> Option<CtbResinParameters> {
        if header.version < 5 || print_params_v4.resin_parameters_address == 0 {
            return None;
        }
        let offset = print_params_v4.resin_parameters_address as usize;
        if offset + CTB_RESIN_PARAMS_FIXED_SIZE as usize > data.len() {
            return None;
        }
        let mut cursor = Cursor::new(&data[offset..]);
        CtbResinParameters::read(&mut cursor).ok()
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
}

fn first_stage_lift_height(stored_total: f32, second_stage: f32, version: u32) -> f32 {
    // UVTools v4+ stores the sum in PrintParameters and the second stage in SlicerInfo.
    if version >= 4 {
        (stored_total - second_stage).max(0.0)
    } else {
        stored_total
    }
}

fn anti_alias_level(header: &CtbHeader, slicer_info: Option<&CtbSlicerInfo>) -> u32 {
    // UVTools reads CTB anti-alias from SlicerInfo and forces the header field to 1.
    // CBDDLP keeps the level in the header.
    if header.magic != MAGIC_CBDDLP {
        if let Some(info) = slicer_info {
            return info.anti_alias_level.max(1);
        }
    }
    header.anti_alias_level.max(1)
}
