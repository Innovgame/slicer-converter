use serde::Deserialize;
use serde_json::json;

use crate::common::{FileMetadata, FileMetadataBuilder, PerLayerSettings};

use super::{calc_volumes::AnycubicZipCalcLayerVolumes, scene::AnycubicZipSceneManifest};

/// Parsed `settings.json` subset for Anycubic ZIP/PWSZ archives.
///
/// Only the machine section is needed for shared metadata; unknown JSON fields
/// are ignored for forward compatibility with slicer-generated archives.
///
/// Anycubic ZIP/PWSZ archive 中 `settings.json` 的已解析字段子集。共享 metadata 只需要
/// machine section；未知 JSON 字段会被忽略，以兼容 slicer 生成的新字段。
#[derive(Debug, Deserialize, Default)]
pub(super) struct AnycubicZipSettingsManifest {
    #[serde(default)]
    pub(super) machine_type: AnycubicZipMachineType,
    #[serde(default)]
    machine_extern: serde_json::Value,
}

impl AnycubicZipSettingsManifest {
    fn active_slicepara(&self) -> Option<&serde_json::Value> {
        let name = self
            .machine_extern
            .get("active_resins")?
            .as_array()?
            .first()?
            .as_str()?;
        for key in ["user_resins", "factory_resins"] {
            if let Some(resins) = self
                .machine_extern
                .get(key)
                .and_then(serde_json::Value::as_array)
            {
                if let Some(resin) = resins.iter().find(|resin| {
                    resin
                        .pointer("/property/name")
                        .and_then(serde_json::Value::as_str)
                        == Some(name)
                }) {
                    return resin.get("slicepara");
                }
            }
        }
        None
    }
}

/// Machine settings extracted from Anycubic ZIP `settings.json`.
///
/// 从 Anycubic ZIP `settings.json` 提取的 machine settings。
#[derive(Debug, Deserialize, Default)]
pub(super) struct AnycubicZipMachineType {
    #[serde(default)]
    name: String,
    #[serde(default)]
    res_x: u32,
    #[serde(default)]
    res_y: u32,
    #[serde(default)]
    print_xsize: f32,
    #[serde(default)]
    print_ysize: f32,
    #[serde(default)]
    print_zsize: f32,
    #[serde(default)]
    raster_antialiasing: u32,
}

/// Parsed `layers.json` controller.
///
/// The real JSON stores layer entries under `paras`; serde renames it into a
/// clearer Rust field.
///
/// 已解析的 `layers.json` controller。真实 JSON 将 layer entries 放在 `paras` 下；
/// serde 将其映射为更清晰的 Rust 字段。
#[derive(Debug, Deserialize, Default)]
pub(super) struct AnycubicZipLayersController {
    #[serde(default, rename = "paras")]
    pub(super) layers: Vec<AnycubicZipLayerManifest>,
}

/// Per-layer print settings from Anycubic ZIP `layers.json`.
///
/// Anycubic ZIP `layers.json` 中的单层打印参数。
#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct AnycubicZipLayerManifest {
    #[serde(default)]
    exposure_time: f32,
    #[serde(default)]
    layer_index: u32,
    #[serde(default)]
    layer_thickness: f32,
    #[serde(default)]
    zup_height: f32,
    #[serde(default)]
    zup_speed: f32,
}

/// Parsed print summary from Anycubic ZIP `print_info.json`.
///
/// Anycubic ZIP `print_info.json` 中的打印统计摘要。
#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct AnycubicZipPrintInfo {
    #[serde(default)]
    print_time: f32,
    #[serde(default, rename = "volume")]
    material_milliliters: f32,
    #[serde(default, rename = "weight")]
    material_grams: f32,
    #[serde(default)]
    cost: f32,
}

fn parse_print_time(value: f32) -> u32 {
    if value.is_finite() && value >= 0.0 && value <= u32::MAX as f32 {
        value.round() as u32
    } else {
        0
    }
}

/// Round a positive-ish float to four decimal places for JSON output.
///
/// 将正向浮点值四舍五入到 4 位小数，用于 JSON 输出。
pub(super) fn round_positive_4(value: f32) -> f32 {
    (value * 10000.0).round() / 10000.0
}

/// Return `value` when finite, otherwise `fallback`.
///
/// 当 `value` 为 finite 时返回它，否则返回 `fallback`。
pub(super) fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

/// Return `value` when finite and positive, otherwise `fallback`.
///
/// 当 `value` 为 finite 且大于 0 时返回它，否则返回 `fallback`。
pub(super) fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

/// Resolve the metadata layer height used by ZIP manifests.
///
/// 为 ZIP manifests 解析 metadata layer height，缺失时回退到 0.05mm。
pub(super) fn metadata_layer_height(metadata: &FileMetadata) -> f32 {
    positive_or(metadata.slicing.layer_height_mm, 0.05)
}

/// Resolve the Z position for a layer.
///
/// Per-layer metadata wins when present; otherwise position is derived from the
/// shared layer height and zero-based layer index.
///
/// 解析单层 Z position。存在 per-layer metadata 时优先使用；否则按共享 layer height 与
/// 0-based layer index 推导。
pub(super) fn metadata_layer_position_z(metadata: &FileMetadata, layer_index: u32) -> f32 {
    metadata
        .per_layer_settings
        .get(layer_index as usize)
        .map(|settings| settings.position_z)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or_else(|| metadata_layer_height(metadata) * (layer_index as f32 + 1.0))
}

fn metadata_layer_exposure(metadata: &FileMetadata, layer_index: u32) -> f32 {
    metadata
        .per_layer_settings
        .get(layer_index as usize)
        .and_then(|settings| settings.exposure_time)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(if layer_index < metadata.slicing.bottom_layer_count {
            metadata.exposure.bottom_exposure_time
        } else {
            metadata.exposure.exposure_time
        })
}

fn metadata_layer_lift_height(metadata: &FileMetadata, layer_index: u32) -> f32 {
    metadata
        .per_layer_settings
        .get(layer_index as usize)
        .and_then(|settings| settings.lift_height)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(metadata.motion.lift_height)
}

fn metadata_layer_lift_speed(metadata: &FileMetadata, layer_index: u32) -> f32 {
    metadata
        .per_layer_settings
        .get(layer_index as usize)
        .and_then(|settings| settings.lift_speed)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(metadata.motion.lift_speed)
}

/// Build Anycubic ZIP `settings.json`.
///
/// The generated shape mirrors the fields consumed by UVTools and Anycubic
/// Photon Workshop for machine geometry and anti-aliasing metadata.
///
/// 构建 Anycubic ZIP `settings.json`。生成结构对齐 UVTools 与 Anycubic Photon Workshop
/// 消费的 machine geometry 与 anti-aliasing metadata 字段。
pub(super) fn build_settings_json(metadata: &FileMetadata) -> serde_json::Value {
    let width = metadata.printer.resolution_x;
    let height = metadata.printer.resolution_y;
    let display_width = positive_or(metadata.printer.display_width_mm, 1.0);
    let display_height = positive_or(metadata.printer.display_height_mm, 1.0);
    let pixel_width = if width > 0 {
        display_width * 1000.0 / width as f32
    } else {
        0.0
    };
    let pixel_height = if height > 0 {
        display_height * 1000.0 / height as f32
    } else {
        0.0
    };

    json!({
        "version": "3",
        "machine_type": {
            "version": "3",
            "name": if metadata.printer.machine_name.is_empty() { "Unknown" } else { &metadata.printer.machine_name },
            "key_suffix": "pwsz",
            "key_image_format": "pw0Img",
            "res_x": width,
            "res_y": height,
            "xy_pixel": round_positive_4(pixel_width),
            "xy_pixel_y": round_positive_4(pixel_height),
            "rotate_z": 0.0,
            "max_samples": 16,
            "property": 119,
            "print_xsize": display_width,
            "print_ysize": display_height,
            "print_zsize": finite_or(metadata.printer.machine_z_mm, 0.0),
            "raster_antialiasing": metadata.slicing.anti_alias_level.max(1)
        },
        "machine_extern": {
            "version": "3", "active_resins": ["slicer-converter"],
            "user_resins": [{
                "version": "2", "property": {"name": "slicer-converter"},
                "slicepara": {
                    "anti_count": metadata.slicing.anti_alias_level.max(1),
                    "bott_layers": metadata.slicing.bottom_layer_count,
                    "bott_time": metadata.exposure.bottom_exposure_time,
                    "bott_time_dual": metadata.exposure.bottom_exposure_time,
                    "exposure_time": metadata.exposure.exposure_time,
                    "zthick": metadata.slicing.layer_height_mm,
                    "zup_height": metadata.motion.lift_height + metadata.motion.lift_height2,
                    "zup_speed": metadata.motion.lift_speed / 60.0,
                    "zdown_speed": metadata.motion.retract_speed / 60.0,
                    "off_time": metadata.waits.before_cure,
                    "bott_off_time": metadata.waits.before_cure,
                    "wait_before_lift": metadata.waits.after_cure,
                    "bott_wait_before_lift": metadata.waits.after_cure,
                    "wait_after_lift": metadata.waits.after_lift,
                    "bott_wait_after_lift": metadata.waits.after_lift
                },
                "slice_extpara": {
                    "transition_layercount": metadata.slicing.transition_layer_count,
                    "multi_state_paras": {
                        "bott_0": {"height": metadata.motion.bottom_lift_height, "up_speed": metadata.motion.bottom_lift_speed / 60.0, "down_speed": metadata.motion.retract_speed / 60.0},
                        "normal_0": {"height": metadata.motion.lift_height, "up_speed": metadata.motion.lift_speed / 60.0, "down_speed": metadata.motion.retract_speed / 60.0},
                        "bott_1": {"height": metadata.motion.lift_height2, "up_speed": metadata.motion.lift_speed2 / 60.0, "down_speed": metadata.motion.retract_speed2 / 60.0},
                        "normal_1": {"height": metadata.motion.lift_height2, "up_speed": metadata.motion.lift_speed2 / 60.0, "down_speed": metadata.motion.retract_speed2 / 60.0}
                    }
                }
            }]
        }
    })
}

/// Build Anycubic ZIP `layers.json`.
///
/// Layer entries are generated in zero-based order and include exposure, layer
/// thickness, and lift settings used to reconstruct per-layer metadata on import.
///
/// 构建 Anycubic ZIP `layers.json`。layer entries 按 0-based 顺序生成，包含曝光、层厚与
/// lift settings，导入时用于还原 per-layer metadata。
pub(super) fn build_layers_json(metadata: &FileMetadata, total_layers: u32) -> serde_json::Value {
    let layer_height = metadata_layer_height(metadata);
    let layers = (0..total_layers)
        .map(|layer_index| {
            json!({
                "exposure_time": metadata_layer_exposure(metadata, layer_index),
                "layer_index": layer_index,
                "layer_minheight": layer_height * layer_index as f32,
                "layer_thickness": layer_height,
                "zup_height": metadata_layer_lift_height(metadata, layer_index),
                "zup_speed": metadata_layer_lift_speed(metadata, layer_index)
            })
        })
        .collect::<Vec<_>>();

    json!({
        "count": total_layers,
        "paras": layers
    })
}

/// Build Anycubic ZIP `lcd_function.json`.
///
/// This mostly carries feature flags and model-processing placeholders required
/// by Anycubic readers.
///
/// 构建 Anycubic ZIP `lcd_function.json`。该文件主要携带 Anycubic reader 需要的功能开关与
/// model-processing placeholders。
pub(super) fn build_lcd_function_json() -> serde_json::Value {
    json!({
        "models_processed_info": {
            "models": [{
                "auto_support": 0,
                "hollow": 0,
                "makeronline_sourceid": -1,
                "manual_support": 0,
                "name": "",
                "punch": 0,
                "source_from": 0
            }],
            "scene_models_from": 0,
            "slice_paras_process": 0,
            "software_version": "slicer-converter"
        },
        "rerf_function": {
            "enable": false,
            "model_name": "",
            "model_type": 1,
            "partition_num": 0
        }
    })
}

/// Build Anycubic ZIP `print_info.json`.
///
/// 构建 Anycubic ZIP `print_info.json`。
pub(super) fn build_print_info_json(
    metadata: &FileMetadata,
    total_layers: u32,
) -> serde_json::Value {
    json!({
        "model_layers_count": total_layers,
        "cost": finite_or(metadata.stats.cost, 0.0),
        "currency": "",
        "print_time": metadata.stats.print_time_sec as f32,
        "volume": finite_or(metadata.stats.volume_ml, 0.0),
        "weight": finite_or(metadata.stats.weight_g, 0.0),
        "sub_estimated_infos": [{
            "cost": finite_or(metadata.stats.cost, 0.0),
            "currency": "",
            "material_milliliters": finite_or(metadata.stats.volume_ml, 0.0),
            "material_grams": finite_or(metadata.stats.weight_g, 0.0),
            "print_time": metadata.stats.print_time_sec as f32
        }]
    })
}

/// Build Anycubic ZIP `software_info.json`.
///
/// 构建 Anycubic ZIP `software_info.json`。
pub(super) fn build_software_info_json() -> serde_json::Value {
    json!({
        "mark": "slicer-converter",
        "opengl": "",
        "os": std::env::consts::OS,
        "Version": env!("CARGO_PKG_VERSION")
    })
}

fn sorted_layers(layers: &AnycubicZipLayersController) -> Vec<&AnycubicZipLayerManifest> {
    let mut sorted = layers.layers.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|layer| layer.layer_index);
    sorted
}

/// Merge parsed Anycubic ZIP manifests into shared [`FileMetadata`].
///
/// Scene and calc-volume files are optional supplemental sources: scene data can
/// refine layer Z positions, while calc volumes can fill material estimates when
/// `print_info.json` is missing those values.
///
/// 将已解析的 Anycubic ZIP manifests 合并为共享 [`FileMetadata`]。scene 与 calc-volume
/// 文件是可选补充源：scene 可修正 layer Z positions，calc volumes 可在 `print_info.json`
/// 缺失材料估算时补齐。
pub(super) fn metadata_from_manifests(
    settings: &AnycubicZipSettingsManifest,
    layers: &AnycubicZipLayersController,
    print_info: Option<&AnycubicZipPrintInfo>,
    scene: Option<&AnycubicZipSceneManifest>,
    calc_volumes: Option<AnycubicZipCalcLayerVolumes>,
) -> FileMetadata {
    let sorted = sorted_layers(layers);
    let first_layer = sorted.first().copied().cloned().unwrap_or_default();
    let mut position_z = 0.0f32;
    let per_layer_settings = sorted
        .iter()
        .map(|layer| {
            let thickness = if layer.layer_thickness > 0.0 {
                layer.layer_thickness
            } else {
                first_layer.layer_thickness
            };
            position_z += thickness;
            let scene_position_z = scene
                .and_then(|scene| scene.layer_defs.get(layer.layer_index as usize))
                .map(|layer_def| layer_def.height)
                .filter(|height| *height > 0.0);
            PerLayerSettings {
                position_z: scene_position_z.unwrap_or(position_z),
                exposure_time: (layer.exposure_time > 0.0).then_some(layer.exposure_time),
                lift_height: (layer.zup_height > 0.0).then_some(layer.zup_height),
                lift_speed: (layer.zup_speed > 0.0).then_some(layer.zup_speed),
                retract_speed: None,
                light_pwm: None,
            }
        })
        .collect::<Vec<_>>();
    let print_info = print_info.cloned().unwrap_or_default();
    let calc_volumes = calc_volumes.unwrap_or_default();
    let material_milliliters = if print_info.material_milliliters > 0.0 {
        print_info.material_milliliters
    } else {
        calc_volumes.material_milliliters
    };
    let material_grams = if print_info.material_grams > 0.0 {
        print_info.material_grams
    } else {
        calc_volumes.material_grams
    };
    let machine = &settings.machine_type;

    let slice = settings.active_slicepara();
    let bottom_count = slice
        .and_then(|value| value.get("bott_layers"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let exposure = |key: &str, fallback: f32| {
        slice
            .and_then(|value| value.get(key))
            .and_then(serde_json::Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value as f32)
            .unwrap_or(fallback)
    };

    FileMetadataBuilder::new()
        .bottom_layer_count(bottom_count)
        .machine_name(machine.name.clone())
        .resolution(machine.res_x, machine.res_y)
        .display_size_mm(machine.print_xsize, machine.print_ysize)
        .machine_z_mm(machine.print_zsize)
        .layer_height_mm(first_layer.layer_thickness)
        .layer_count(layers.layers.len() as u32)
        .anti_alias_level(machine.raster_antialiasing.max(1))
        .exposure(
            exposure("exposure_time", first_layer.exposure_time),
            exposure("bott_time", first_layer.exposure_time),
        )
        .lift(first_layer.zup_height, first_layer.zup_speed)
        .statistics(
            parse_print_time(print_info.print_time),
            material_milliliters,
            material_grams,
            print_info.cost,
        )
        .per_layer_settings(per_layer_settings)
        .build_unchecked()
}
