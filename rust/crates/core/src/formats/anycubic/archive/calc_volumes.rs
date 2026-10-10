use crate::{common::ConvertError, common::FileMetadata, formats::common::binary::BinaryReader};

use super::{
    manifest::{metadata_layer_height, metadata_layer_position_z, round_positive_4},
    scene::{
        push_f32, push_u32, read_manifest_f32_for_entry, read_manifest_u32_for_entry,
        skip_manifest_bytes_for_entry, zip_binary_parse_error_for_entry,
    },
};

/// Material estimates parsed from Anycubic ZIP `calc_layer_volumes.data`.
///
/// Anycubic ZIP `calc_layer_volumes.data` 中解析出的材料估算。
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AnycubicZipCalcLayerVolumes {
    pub(super) material_milliliters: f32,
    pub(super) material_grams: f32,
}

/// Build Anycubic ZIP `calc_layer_volumes.data`.
///
/// The file stores one raw volume value per layer. Photon Workshop uses a raw
/// volume unit that is converted back to milliliters with the constants mirrored
/// by [`parse_calc_layer_volumes`].
///
/// 构建 Anycubic ZIP `calc_layer_volumes.data`。该文件每层保存一个 raw volume value；
/// Photon Workshop 使用的 raw volume 单位会由 [`parse_calc_layer_volumes`] 中的常量转换回
/// milliliters。
pub(super) fn build_calc_layer_volumes(metadata: &FileMetadata, total_layers: u32) -> Vec<u8> {
    const MAGIC: u32 = 0x20251024;
    const RAW_VOLUME_TO_MILLILITERS: f32 = 0.0011;

    let mut bytes = Vec::new();
    push_u32(&mut bytes, MAGIC);
    push_f32(&mut bytes, 0.2);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, total_layers);

    let raw_volume_total = if metadata.stats.volume_ml.is_finite() && metadata.stats.volume_ml > 0.0
    {
        metadata.stats.volume_ml / RAW_VOLUME_TO_MILLILITERS
    } else {
        0.0
    };
    let raw_volume_per_layer = if total_layers > 0 {
        raw_volume_total / total_layers as f32
    } else {
        0.0
    };

    for layer_index in 0..total_layers {
        let z = if layer_index == 0 {
            0.0
        } else {
            metadata_layer_position_z(metadata, layer_index - 1)
        };
        push_u32(&mut bytes, layer_index);
        push_f32(&mut bytes, round_positive_4(z));
        push_f32(&mut bytes, metadata_layer_height(metadata));
        for _ in 0..8 {
            push_u32(&mut bytes, 0);
        }
        push_f32(&mut bytes, raw_volume_per_layer);
    }

    bytes
}

/// Parse material estimates from Anycubic ZIP `calc_layer_volumes.data`.
///
/// The parser sums raw per-layer volumes and converts them to milliliters and
/// grams. This acts as a fallback when `print_info.json` lacks material totals.
///
/// 从 Anycubic ZIP `calc_layer_volumes.data` 解析材料估算。parser 会累加每层 raw volume，
/// 并转换为 milliliters 与 grams；当 `print_info.json` 缺少材料统计时作为 fallback。
pub(super) fn parse_calc_layer_volumes(
    data: &[u8],
) -> Result<AnycubicZipCalcLayerVolumes, ConvertError> {
    const ENTRY: &str = "calc_layer_volumes.data";
    const MAGIC: u32 = 0x20251024;
    const RAW_VOLUME_TO_MILLILITERS: f32 = 0.0011;
    const RAW_VOLUME_TO_GRAMS: f32 = 0.00132;

    let mut reader = BinaryReader::new(data);
    let magic = read_manifest_u32_for_entry(&mut reader, ENTRY)?;
    if magic != MAGIC {
        return Err(zip_binary_parse_error_for_entry(ENTRY, 0, "invalid magic"));
    }
    let _base_step = read_manifest_f32_for_entry(&mut reader, ENTRY)?;
    let _one = read_manifest_u32_for_entry(&mut reader, ENTRY)?;
    let entry_count = read_manifest_u32_for_entry(&mut reader, ENTRY)?;
    let mut raw_volume_sum = 0.0f32;
    for _ in 0..entry_count {
        let _layer_ref = read_manifest_u32_for_entry(&mut reader, ENTRY)?;
        let _z = read_manifest_f32_for_entry(&mut reader, ENTRY)?;
        let _thickness = read_manifest_f32_for_entry(&mut reader, ENTRY)?;
        skip_manifest_bytes_for_entry(&mut reader, 8 * 4, ENTRY)?;
        raw_volume_sum += read_manifest_f32_for_entry(&mut reader, ENTRY)?;
    }

    Ok(AnycubicZipCalcLayerVolumes {
        material_milliliters: round_positive_3(raw_volume_sum * RAW_VOLUME_TO_MILLILITERS),
        material_grams: round_positive_3(raw_volume_sum * RAW_VOLUME_TO_GRAMS),
    })
}

fn round_positive_3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}
