use serde::{Deserialize, Serialize};

use crate::common::{FileMetadata, FileMetadataBuilder};

/// VDT `manifest.json` root structure.
///
/// VDT archives keep machine, print, advanced, and per-layer settings in one
/// JSON manifest. Serde defaults are part of the compatibility contract: older
/// or partial manifests can still be imported.
///
/// VDT ZIP 包内 `manifest.json` 的根结构，包含 machine、print、advanced 与 per-layer
/// settings。Serde defaults 是兼容契约的一部分，用于读取旧版或字段不完整的 manifest。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VdtManifest {
    #[serde(default)]
    pub application_name: String,
    #[serde(default = "default_file_version")]
    pub file_version: u32,
    #[serde(default)]
    pub machine: VdtMachine,
    #[serde(default)]
    pub advanced_parameters: VdtAdvanced,
    #[serde(default)]
    pub print: VdtPrint,
    #[serde(default)]
    pub layers: Vec<VdtLayer>,
}

fn default_file_version() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// VDT machine configuration.
///
/// Contains display resolution, physical screen size, Z height, mirror flags,
/// and optional notes.
///
/// VDT machine 配置，包含显示分辨率、物理屏幕尺寸、Z 高度、镜像选项与 notes。
pub struct VdtMachine {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub resolution_x: u32,
    #[serde(default)]
    pub resolution_y: u32,
    #[serde(default)]
    pub lcd_width: f64,
    #[serde(default)]
    pub lcd_height: f64,
    #[serde(default)]
    pub z_height: f64,
    #[serde(default)]
    pub x_mirror: bool,
    #[serde(default)]
    pub y_mirror: bool,
    #[serde(default)]
    pub notes: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// VDT advanced print parameters.
///
/// The upstream field is spelled `antialasing_level` in observed manifests, so
/// the Rust field keeps that spelling for serde compatibility.
///
/// VDT advanced print parameters，包含抗锯齿等级和光强 PWM 设置。实测 manifest 中字段名为
/// `antialasing_level`，Rust 字段保留该拼写以保证 serde 兼容。
pub struct VdtAdvanced {
    #[serde(default = "default_aa")]
    pub antialasing_level: u32,
    #[serde(default = "default_pwm")]
    pub bottom_light_pwm: u16,
    #[serde(default = "default_pwm")]
    pub light_pwm: u16,
}

fn default_aa() -> u32 {
    1
}
fn default_pwm() -> u16 {
    255
}

impl Default for VdtAdvanced {
    fn default() -> Self {
        Self {
            antialasing_level: 1,
            bottom_light_pwm: 255,
            light_pwm: 255,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// VDT file-level print settings.
///
/// Per-layer overrides can appear in [`VdtLayer`], but these values provide the
/// default exposure, lift/retract, wait-time, and material/statistics settings.
///
/// VDT file-level print settings。Per-layer overrides 可出现在 [`VdtLayer`] 中；
/// 这些字段提供默认曝光、lift/retract、wait-time 与材料/统计设置。
pub struct VdtPrint {
    #[serde(default = "default_layer_thickness")]
    pub layer_thickness: f64,
    #[serde(default)]
    pub bottom_layers: u32,
    #[serde(default)]
    pub transition_layers: u32,
    #[serde(default = "default_exposure")]
    pub bottom_exposure_time: f64,
    #[serde(default = "default_exposure")]
    pub exposure_time: f64,
    #[serde(default)]
    pub bottom_light_off_delay: f64,
    #[serde(default)]
    pub light_off_delay: f64,
    #[serde(default = "default_lift")]
    pub bottom_lift_distance: f64,
    #[serde(default = "default_lift_speed")]
    pub bottom_lift_speed: f64,
    #[serde(default = "default_lift")]
    pub lift_distance: f64,
    #[serde(default = "default_lift_speed")]
    pub lift_speed: f64,
    #[serde(default = "default_retract_speed")]
    pub retract_speed: f64,
    #[serde(default)]
    pub bottom_lift_distance2: f64,
    #[serde(default)]
    pub bottom_lift_speed2: f64,
    #[serde(default)]
    pub lift_distance2: f64,
    #[serde(default)]
    pub lift_speed2: f64,
    #[serde(default)]
    pub retract_distance2: f64,
    #[serde(default)]
    pub retract_speed2: f64,
    #[serde(default)]
    pub bottom_retract_speed: f64,
    #[serde(default)]
    pub bottom_retract_distance2: f64,
    #[serde(default)]
    pub bottom_retract_speed2: f64,
    #[serde(default)]
    pub wait_time_before_cure: f64,
    #[serde(default)]
    pub wait_time_after_cure: f64,
    #[serde(default)]
    pub wait_time_after_lift: f64,
    #[serde(default)]
    pub volume_ml: f64,
    #[serde(default)]
    pub weight_g: f64,
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub print_time: f64,
}

fn default_layer_thickness() -> f64 {
    0.05
}
fn default_exposure() -> f64 {
    2.0
}
fn default_lift() -> f64 {
    5.0
}
fn default_lift_speed() -> f64 {
    60.0
}
fn default_retract_speed() -> f64 {
    150.0
}

fn round_layer_value(value: f32) -> f64 {
    ((value as f64) * 10_000.0).round() / 10_000.0
}

impl Default for VdtPrint {
    fn default() -> Self {
        Self {
            layer_thickness: 0.05,
            bottom_layers: 0,
            transition_layers: 0,
            bottom_exposure_time: 30.0,
            exposure_time: 2.0,
            bottom_light_off_delay: 0.0,
            light_off_delay: 0.0,
            bottom_lift_distance: 5.0,
            bottom_lift_speed: 60.0,
            lift_distance: 5.0,
            lift_speed: 60.0,
            retract_speed: 150.0,
            bottom_lift_distance2: 0.0,
            bottom_lift_speed2: 0.0,
            lift_distance2: 0.0,
            lift_speed2: 0.0,
            retract_distance2: 0.0,
            retract_speed2: 0.0,
            bottom_retract_speed: 150.0,
            bottom_retract_distance2: 0.0,
            bottom_retract_speed2: 0.0,
            wait_time_before_cure: 0.0,
            wait_time_after_cure: 0.0,
            wait_time_after_lift: 0.0,
            volume_ml: 0.0,
            weight_g: 0.0,
            cost: 0.0,
            print_time: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// VDT per-layer settings entry.
///
/// The layer list is also used to infer layer count on import because ZIP entry
/// names alone do not carry print timing.
///
/// VDT 单层设置条目，包含图层高度、曝光时间及每层独立的 lift/retract 参数。导入时也会用
/// layer list 推断 layer count，因为 ZIP entry name 本身不携带打印时序。
pub struct VdtLayer {
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub exposure_time: f64,
    #[serde(default)]
    pub lift_distance: f64,
    #[serde(default)]
    pub lift_speed: f64,
    #[serde(default)]
    pub retract_speed: f64,
    #[serde(default)]
    pub light_off_delay: f64,
    #[serde(default)]
    pub lift_distance2: f64,
    #[serde(default)]
    pub lift_speed2: f64,
    #[serde(default)]
    pub retract_distance2: f64,
    #[serde(default)]
    pub retract_speed2: f64,
    #[serde(default)]
    pub wait_time_before_cure: f64,
    #[serde(default)]
    pub wait_time_after_cure: f64,
    #[serde(default)]
    pub wait_time_after_lift: f64,
}

// ── Conversion ──────────────────────────────────────────────

impl VdtManifest {
    /// Build a `VdtManifest` from shared file metadata.
    ///
    /// Layer entries are intentionally left empty here; archive assembly fills
    /// them once the final layer count and ordering are known.
    ///
    /// 从共享 [`FileMetadata`] 构建 VDT manifest JSON 根结构。这里有意不填充 layer entries；
    /// archive assembly 会在最终 layer count 与顺序确定后写入。
    pub fn from_metadata(meta: &FileMetadata) -> Self {
        VdtManifest {
            application_name: "slicer-converter".into(),
            file_version: 1,
            machine: VdtMachine {
                name: meta.printer.machine_name.clone(),
                resolution_x: meta.printer.resolution_x,
                resolution_y: meta.printer.resolution_y,
                lcd_width: meta.printer.display_width_mm as f64,
                lcd_height: meta.printer.display_height_mm as f64,
                z_height: meta.printer.machine_z_mm as f64,
                x_mirror: meta.printer.mirror_x,
                y_mirror: meta.printer.mirror_y,
                notes: String::new(),
            },
            advanced_parameters: VdtAdvanced {
                antialasing_level: meta.slicing.anti_alias_level,
                bottom_light_pwm: meta.exposure.bottom_light_pwm,
                light_pwm: meta.exposure.light_pwm,
            },
            print: VdtPrint {
                layer_thickness: meta.slicing.layer_height_mm as f64,
                bottom_layers: meta.slicing.bottom_layer_count,
                transition_layers: meta.slicing.transition_layer_count,
                bottom_exposure_time: meta.exposure.bottom_exposure_time as f64,
                exposure_time: meta.exposure.exposure_time as f64,
                bottom_light_off_delay: meta.exposure.bottom_light_off_delay as f64,
                light_off_delay: meta.exposure.light_off_delay as f64,
                bottom_lift_distance: meta.motion.bottom_lift_height as f64,
                bottom_lift_speed: meta.motion.bottom_lift_speed as f64,
                lift_distance: meta.motion.lift_height as f64,
                lift_speed: meta.motion.lift_speed as f64,
                retract_speed: meta.motion.retract_speed as f64,
                bottom_lift_distance2: meta.motion.lift_height2 as f64,
                bottom_lift_speed2: meta.motion.lift_speed2 as f64,
                lift_distance2: meta.motion.lift_height2 as f64,
                lift_speed2: meta.motion.lift_speed2 as f64,
                retract_distance2: meta.motion.retract_height2 as f64,
                retract_speed2: meta.motion.retract_speed2 as f64,
                bottom_retract_speed: meta.motion.retract_speed as f64,
                bottom_retract_distance2: meta.motion.retract_height2 as f64,
                bottom_retract_speed2: meta.motion.retract_speed2 as f64,
                wait_time_before_cure: meta.waits.before_cure as f64,
                wait_time_after_cure: meta.waits.after_cure as f64,
                wait_time_after_lift: meta.waits.after_lift as f64,
                volume_ml: meta.stats.volume_ml as f64,
                weight_g: meta.stats.weight_g as f64,
                cost: meta.stats.cost as f64,
                print_time: meta.stats.print_time_sec as f64,
            },
            layers: Vec::new(), // Layers are filled during encode
        }
    }

    /// Build a `VdtManifest` with one UVTools-compatible layer record per entry.
    ///
    /// UVTools derives VDT `LayerCount` from `layers.length` before loading PNG
    /// entries, so exported archives must keep this array aligned with the ZIP
    /// layer payload count.
    ///
    /// 构建带有逐层记录的 `VdtManifest`。UVTools 会先用 `layers.length` 推导 VDT
    /// `LayerCount`，再加载 PNG entries，因此导出的 archive 必须让该数组与 ZIP
    /// layer payload 数量一致。
    pub fn from_metadata_with_layers(meta: &FileMetadata, total_layers: u32) -> Self {
        let mut manifest = Self::from_metadata(meta);
        manifest.layers = (0..total_layers)
            .map(|layer_index| VdtLayer::from_metadata(meta, layer_index))
            .collect();
        manifest
    }

    /// Convert a VDT manifest into shared [`FileMetadata`].
    ///
    /// This normalizes JSON defaults and manifest spelling quirks into the
    /// format-agnostic metadata model used by the transcoding pipeline.
    ///
    /// 将 VDT manifest 转换为共享 [`FileMetadata`]，把 JSON defaults 与 manifest 字段拼写差异
    /// 归一到 transcoding pipeline 使用的 format-agnostic metadata model。
    pub fn to_metadata(&self) -> FileMetadata {
        FileMetadataBuilder::new()
            .resolution(self.machine.resolution_x, self.machine.resolution_y)
            .display_size_mm(
                self.machine.lcd_width as f32,
                self.machine.lcd_height as f32,
            )
            .machine_z_mm(self.machine.z_height as f32)
            .machine_name(self.machine.name.clone())
            .mirror(self.machine.x_mirror, self.machine.y_mirror)
            .layer_height_mm(self.print.layer_thickness as f32)
            .layer_count(self.layers.len() as u32)
            .bottom_layer_count(self.print.bottom_layers)
            .transition_layer_count(self.print.transition_layers)
            .anti_alias_level(self.advanced_parameters.antialasing_level.max(1))
            .exposure(
                self.print.exposure_time as f32,
                self.print.bottom_exposure_time as f32,
            )
            .light_off_delay(
                self.print.light_off_delay as f32,
                self.print.bottom_light_off_delay as f32,
            )
            .light_pwm(
                self.advanced_parameters.light_pwm,
                self.advanced_parameters.bottom_light_pwm,
            )
            .lift(
                self.print.lift_distance as f32,
                self.print.lift_speed as f32,
            )
            .lift2(
                self.print.lift_distance2 as f32,
                self.print.lift_speed2 as f32,
            )
            .bottom_lift(
                self.print.bottom_lift_distance as f32,
                self.print.bottom_lift_speed as f32,
            )
            .retract(self.print.retract_speed as f32)
            .retract2(
                self.print.retract_distance2 as f32,
                self.print.retract_speed2 as f32,
            )
            .wait_times(
                self.print.wait_time_before_cure as f32,
                self.print.wait_time_after_cure as f32,
                self.print.wait_time_after_lift as f32,
            )
            .statistics(
                self.print.print_time as u32,
                self.print.volume_ml as f32,
                self.print.weight_g as f32,
                self.print.cost as f32,
            )
            .build_unchecked()
    }
}

impl VdtLayer {
    fn from_metadata(meta: &FileMetadata, layer_index: u32) -> Self {
        let is_bottom = layer_index < meta.slicing.bottom_layer_count;
        let layer_settings = meta.per_layer_settings.get(layer_index as usize);

        Self {
            height: round_layer_value(
                layer_settings
                    .map(|settings| settings.position_z)
                    .unwrap_or_else(|| (layer_index + 1) as f32 * meta.slicing.layer_height_mm),
            ),
            exposure_time: round_layer_value(
                layer_settings
                    .and_then(|settings| settings.exposure_time)
                    .unwrap_or(if is_bottom {
                        meta.exposure.bottom_exposure_time
                    } else {
                        meta.exposure.exposure_time
                    }),
            ),
            lift_distance: round_layer_value(
                layer_settings
                    .and_then(|settings| settings.lift_height)
                    .unwrap_or(if is_bottom {
                        meta.motion.bottom_lift_height
                    } else {
                        meta.motion.lift_height
                    }),
            ),
            lift_speed: round_layer_value(
                layer_settings
                    .and_then(|settings| settings.lift_speed)
                    .unwrap_or(if is_bottom {
                        meta.motion.bottom_lift_speed
                    } else {
                        meta.motion.lift_speed
                    }),
            ),
            retract_speed: round_layer_value(
                layer_settings
                    .and_then(|settings| settings.retract_speed)
                    .unwrap_or(meta.motion.retract_speed),
            ),
            light_off_delay: round_layer_value(if is_bottom {
                meta.exposure.bottom_light_off_delay
            } else {
                meta.exposure.light_off_delay
            }),
            lift_distance2: round_layer_value(meta.motion.lift_height2),
            lift_speed2: round_layer_value(meta.motion.lift_speed2),
            retract_distance2: round_layer_value(meta.motion.retract_height2),
            retract_speed2: round_layer_value(meta.motion.retract_speed2),
            wait_time_before_cure: round_layer_value(meta.waits.before_cure),
            wait_time_after_cure: round_layer_value(meta.waits.after_cure),
            wait_time_after_lift: round_layer_value(meta.waits.after_lift),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VdtManifest;
    use crate::common::FileMetadataBuilder;

    #[test]
    fn vdt_print_profile_keeps_bottom_second_stage_motion() {
        let metadata = FileMetadataBuilder::new()
            .resolution(16, 16)
            .display_size_mm(68.04, 120.96)
            .layer_height_mm(0.05)
            .layer_count(1)
            .lift2(1.5, 40.0)
            .retract2(2.0, 80.0)
            .build()
            .expect("motion metadata");
        let manifest = VdtManifest::from_metadata(&metadata);

        assert_eq!(manifest.print.bottom_lift_distance2, 1.5);
        assert_eq!(manifest.print.bottom_lift_speed2, 40.0);
        assert_eq!(manifest.print.bottom_retract_distance2, 2.0);
        assert_eq!(manifest.print.bottom_retract_speed2, 80.0);
        assert_eq!(
            manifest.print.lift_distance2,
            manifest.print.bottom_lift_distance2
        );
        assert_eq!(
            manifest.print.retract_distance2,
            manifest.print.bottom_retract_distance2
        );
    }
}
