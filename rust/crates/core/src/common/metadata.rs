//! Normalized metadata shared by all format families.
//!
//! Format parsers translate native headers, manifests, and layer tables into
//! these structs before returning data to CLI/WASM callers. The serde layout is
//! intentionally stable: nested concern structs are flattened so older
//! TypeScript callers still see the historical field names.
//!
//! 所有格式 family 共享的归一化 metadata。格式 parser 会先把 native header、manifest
//! 与 layer table 转成这些结构，再返回给 CLI/WASM 调用方。serde layout 刻意保持稳定：
//! 各 concern struct 通过 flatten 展开，旧 TypeScript 调用方仍能看到历史字段名。

use serde::{Deserialize, Serialize};

use super::error::ConvertError;

fn default_aa_level() -> u32 {
    1
}

/// Top-level result from `parse_header`, returned as JSON to JS.
///
/// `parse_header` 返回的顶层结果，以 JSON 形式传递到 JS 端。
#[derive(Serialize, Deserialize, Debug)]
pub struct ParseResult {
    /// Detected concrete format/profile name.
    ///
    /// 检测出的具体 format/profile 名称。
    pub format: String,
    /// Normalized file metadata.
    ///
    /// 归一化文件 metadata。
    pub metadata: FileMetadata,
    /// Encoded layer byte ranges in source document order.
    ///
    /// 源 document 顺序下的 encoded layer byte ranges。
    pub layers: Vec<LayerOffset>,
}

/// Byte offset and size of a single layer's data within the source file.
/// The JS side uses these to call `File.slice(offset, offset + size)` on demand.
///
/// 单个图层在源文件中的字节偏移量与大小。
/// JS 端调用 `File.slice(offset, offset + size)` 按需读取图层数据。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayerOffset {
    /// Byte offset in the source document.
    ///
    /// 源 document 中的 byte offset。
    pub offset: u64,
    /// Encoded payload size in bytes.
    ///
    /// encoded payload 字节大小。
    pub size: u64,
}

/// Wait-time parameters.
///
/// 打印机等待时间参数。
///
/// JSON schema 通过 `#[serde(rename)]` 保持与旧平铺字段 (`wait_time_*`) 完全兼容，
/// 不产生 breaking change。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct WaitTimes {
    /// Wait before curing starts.
    ///
    /// 固化开始前等待时间。
    #[serde(rename = "wait_time_before_cure")]
    pub before_cure: f32,
    /// Wait after curing completes.
    ///
    /// 固化完成后等待时间。
    #[serde(rename = "wait_time_after_cure")]
    pub after_cure: f32,
    /// Wait after lift motion completes.
    ///
    /// 抬升动作完成后等待时间。
    #[serde(rename = "wait_time_after_lift")]
    pub after_lift: f32,
}

/// Print statistics.
///
/// 打印统计字段。
///
/// 字段名与旧 JSON schema 完全一致，配合 `#[serde(flatten)]` 保持兼容。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PrintStatistics {
    /// Estimated print time in seconds.
    ///
    /// 预计打印时间，单位秒。
    pub print_time_sec: u32,
    /// Estimated resin volume in milliliters.
    ///
    /// 预计树脂体积，单位毫升。
    pub volume_ml: f32,
    /// Estimated resin weight in grams.
    ///
    /// 预计树脂重量，单位克。
    pub weight_g: f32,
    /// Estimated material cost in the source format currency.
    ///
    /// 源格式币种下的预计材料成本。
    pub cost: f32,
}

/// Exposure plan.
///
/// 曝光参数。字段名保持原样，JSON schema 无变化。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExposurePlan {
    /// Normal-layer exposure time in seconds.
    ///
    /// 普通层曝光时间，单位秒。
    pub exposure_time: f32,
    /// Bottom-layer exposure time in seconds.
    ///
    /// 底层曝光时间，单位秒。
    pub bottom_exposure_time: f32,
    /// Normal-layer light-off delay in seconds.
    ///
    /// 普通层 light-off delay，单位秒。
    pub light_off_delay: f32,
    /// Bottom-layer light-off delay in seconds.
    ///
    /// 底层 light-off delay，单位秒。
    pub bottom_light_off_delay: f32,
    /// Normal-layer light PWM value.
    ///
    /// 普通层光源 PWM 值。
    pub light_pwm: u16,
    /// Bottom-layer light PWM value.
    ///
    /// 底层光源 PWM 值。
    pub bottom_light_pwm: u16,
}

impl Default for ExposurePlan {
    fn default() -> Self {
        Self {
            exposure_time: 2.0,
            bottom_exposure_time: 30.0,
            light_off_delay: 0.0,
            bottom_light_off_delay: 0.0,
            light_pwm: 255,
            bottom_light_pwm: 255,
        }
    }
}

/// Motion plan.
///
/// 运动参数：lift / retract 9 字段。
/// 字段名保持原样，JSON schema 无变化。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MotionPlan {
    /// First-stage normal lift height in millimeters.
    ///
    /// 普通层第一段抬升高度，单位毫米。
    pub lift_height: f32,
    /// First-stage normal lift speed.
    ///
    /// 普通层第一段抬升速度。
    pub lift_speed: f32,
    /// Optional second-stage normal lift height in millimeters.
    ///
    /// 普通层可选第二段抬升高度，单位毫米。
    pub lift_height2: f32,
    /// Optional second-stage normal lift speed.
    ///
    /// 普通层可选第二段抬升速度。
    pub lift_speed2: f32,
    /// Normal retract speed.
    ///
    /// 普通层回退速度。
    pub retract_speed: f32,
    /// Optional second-stage retract height in millimeters.
    ///
    /// 可选第二段回退高度，单位毫米。
    pub retract_height2: f32,
    /// Optional second-stage retract speed.
    ///
    /// 可选第二段回退速度。
    pub retract_speed2: f32,
    /// Bottom-layer first-stage lift height in millimeters.
    ///
    /// 底层第一段抬升高度，单位毫米。
    pub bottom_lift_height: f32,
    /// Bottom-layer first-stage lift speed.
    ///
    /// 底层第一段抬升速度。
    pub bottom_lift_speed: f32,
}

impl Default for MotionPlan {
    fn default() -> Self {
        Self {
            lift_height: 5.0,
            lift_speed: 65.0,
            lift_height2: 0.0,
            lift_speed2: 0.0,
            retract_speed: 150.0,
            retract_height2: 0.0,
            retract_speed2: 0.0,
            bottom_lift_height: 5.0,
            bottom_lift_speed: 65.0,
        }
    }
}

/// Printer spec.
///
/// 打印机物理参数。JSON schema 保持原字段名。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PrinterSpec {
    /// Native LCD resolution on X axis.
    ///
    /// LCD X 方向原生分辨率。
    pub resolution_x: u32,
    /// Native LCD resolution on Y axis.
    ///
    /// LCD Y 方向原生分辨率。
    pub resolution_y: u32,
    /// Physical LCD width in millimeters.
    ///
    /// LCD 物理宽度，单位毫米。
    pub display_width_mm: f32,
    /// Physical LCD height in millimeters.
    ///
    /// LCD 物理高度，单位毫米。
    pub display_height_mm: f32,
    /// Printable Z height in millimeters.
    ///
    /// 可打印 Z 高度，单位毫米。
    pub machine_z_mm: f32,
    /// Source-format machine name.
    ///
    /// 源格式中的机器名称。
    pub machine_name: String,
    /// Whether the format requests X-axis mirroring.
    ///
    /// 格式是否要求 X 方向镜像。
    #[serde(default)]
    pub mirror_x: bool,
    /// Whether the format requests Y-axis mirroring.
    ///
    /// 格式是否要求 Y 方向镜像。
    #[serde(default)]
    pub mirror_y: bool,
}

/// Slicing spec.
///
/// 切片参数。JSON schema 保持原字段名。
/// `anti_alias_level` 保持 `default = "default_aa_level"` 默认值 1；
/// `encryption_key` 通过 `#[serde(default)]` 回退到 0。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SlicingSpec {
    /// Layer height in millimeters.
    ///
    /// 层高，单位毫米。
    pub layer_height_mm: f32,
    /// Total logical layer count.
    ///
    /// 逻辑层总数。
    pub layer_count: u32,
    /// Number of bottom layers.
    ///
    /// 底层数量。
    pub bottom_layer_count: u32,
    /// Number of transition layers after bottom layers.
    ///
    /// 底层之后的过渡层数量。
    pub transition_layer_count: u32,
    /// Anti-alias level; `1` means no extra AA bitplanes.
    ///
    /// anti-alias 等级；`1` 表示没有额外 AA bitplanes。
    #[serde(default = "default_aa_level")]
    pub anti_alias_level: u32,
    /// Format-specific encryption key; `0` means unencrypted for current formats.
    ///
    /// 格式特定 encryption key；当前格式中 `0` 表示未加密。
    #[serde(default)]
    pub encryption_key: u32,
}

impl Default for SlicingSpec {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.05,
            layer_count: 0,
            bottom_layer_count: 0,
            transition_layer_count: 0,
            anti_alias_level: 1,
            encryption_key: 0,
        }
    }
}

/// Universal file metadata — the common denominator across CTB/SL1/GOO.
///
/// 跨格式的通用文件元数据，是 CTB、SL1、GOO 等格式元数据字段的最小公共子集。
///
/// 外部 crate 禁止使用结构体字面量直接构造，必须通过 [`FileMetadataBuilder`]
/// 或 [`FileMetadata::builder`] 创建。`#[non_exhaustive]` 在编译期硬约束此规则，
/// 同时允许未来无破坏性地添加新字段。

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[non_exhaustive]
pub struct FileMetadata {
    /// Printer and display facts.
    ///
    /// 打印机与显示屏事实。
    #[serde(flatten)]
    pub printer: PrinterSpec,

    /// Slicing geometry and layer-count facts.
    ///
    /// 切片几何与层数事实。
    #[serde(flatten)]
    pub slicing: SlicingSpec,

    /// Exposure timing and PWM facts.
    ///
    /// 曝光时间与 PWM 事实。
    #[serde(flatten)]
    pub exposure: ExposurePlan,

    /// Lift/retract motion facts.
    ///
    /// 抬升/回退运动事实。
    #[serde(flatten)]
    pub motion: MotionPlan,

    /// Wait-time facts.
    ///
    /// 等待时间事实。
    #[serde(flatten)]
    pub waits: WaitTimes,

    /// Estimated print statistics.
    ///
    /// 预计打印统计。
    #[serde(flatten)]
    pub stats: PrintStatistics,

    // Per-layer overrides (optional)
    /// Optional per-layer overrides.
    ///
    /// 可选逐层覆盖参数。
    #[serde(default)]
    pub per_layer_settings: Vec<PerLayerSettings>,

    // Thumbnails (extracted from source format)
    /// Embedded thumbnails extracted from or written to source formats.
    ///
    /// 从源格式提取或写入源格式的嵌入缩略图。
    #[serde(default)]
    pub thumbnails: Vec<Thumbnail>,
}

/// Owned layer-encode metadata used at crate boundaries (wasm JSON bridge).
///
/// 跨 FFI 边界反序列化用的 owned 版本；内部编码路径应优先使用
/// [`LayerEncodeMetadata<'a>`] 借用视图，避免每层 clone。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OwnedLayerEncodeMetadata {
    /// Owned printer facts.
    ///
    /// owned printer facts。
    #[serde(flatten)]
    pub printer: PrinterSpec,
    /// Owned slicing facts.
    ///
    /// owned slicing facts。
    #[serde(flatten)]
    pub slicing: SlicingSpec,
    /// Owned exposure facts.
    ///
    /// owned exposure facts。
    #[serde(flatten)]
    pub exposure: ExposurePlan,
    /// Owned motion facts.
    ///
    /// owned motion facts。
    #[serde(flatten)]
    pub motion: MotionPlan,
    /// Owned wait-time facts.
    ///
    /// owned wait-time facts。
    #[serde(flatten)]
    pub waits: WaitTimes,
}

impl OwnedLayerEncodeMetadata {
    /// Build a borrowed view referencing this owned snapshot.
    ///
    /// 构建引用该 owned snapshot 的 borrowed view。
    pub fn as_view(&self) -> LayerEncodeMetadata<'_> {
        LayerEncodeMetadata {
            printer: &self.printer,
            slicing: &self.slicing,
            exposure: &self.exposure,
            motion: &self.motion,
            waits: &self.waits,
        }
    }
}

impl From<&FileMetadata> for OwnedLayerEncodeMetadata {
    fn from(metadata: &FileMetadata) -> Self {
        Self {
            printer: metadata.printer.clone(),
            slicing: metadata.slicing.clone(),
            exposure: metadata.exposure.clone(),
            motion: metadata.motion.clone(),
            waits: metadata.waits.clone(),
        }
    }
}

/// Borrowed layer-encode view — shared across all encoders without per-layer clones.
///
/// 逐层编码的借用视图，消除 `From<&FileMetadata>` 旧路径里逐字段 memcpy 的开销。
/// `Copy` + 生命周期确保在 backing `FileMetadata` 或 `OwnedLayerEncodeMetadata`
/// 存活期间可自由传递。
#[derive(Debug, Clone, Copy)]
pub struct LayerEncodeMetadata<'a> {
    /// Borrowed printer facts.
    ///
    /// 借用的 printer facts。
    pub printer: &'a PrinterSpec,
    /// Borrowed slicing facts.
    ///
    /// 借用的 slicing facts。
    pub slicing: &'a SlicingSpec,
    /// Borrowed exposure facts.
    ///
    /// 借用的 exposure facts。
    pub exposure: &'a ExposurePlan,
    /// Borrowed motion facts.
    ///
    /// 借用的 motion facts。
    pub motion: &'a MotionPlan,
    /// Borrowed wait-time facts.
    ///
    /// 借用的 wait-time facts。
    pub waits: &'a WaitTimes,
}

/// Per-layer parameter overrides.
///
/// 逐层参数覆盖字段。各字段均为可选，未设置时回落到全局元数据。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerLayerSettings {
    /// Layer Z position in millimeters.
    ///
    /// layer Z 位置，单位毫米。
    pub position_z: f32,
    /// Optional per-layer exposure override.
    ///
    /// 可选逐层曝光覆盖。
    #[serde(default)]
    pub exposure_time: Option<f32>,
    /// Optional per-layer lift-height override.
    ///
    /// 可选逐层抬升高度覆盖。
    #[serde(default)]
    pub lift_height: Option<f32>,
    /// Optional per-layer lift-speed override.
    ///
    /// 可选逐层抬升速度覆盖。
    #[serde(default)]
    pub lift_speed: Option<f32>,
    /// Optional per-layer retract-speed override.
    ///
    /// 可选逐层回退速度覆盖。
    #[serde(default)]
    pub retract_speed: Option<f32>,
    /// Optional per-layer light PWM override.
    ///
    /// 可选逐层光源 PWM 覆盖。
    #[serde(default)]
    pub light_pwm: Option<u16>,
}

/// Thumbnail image embedded in the slicer file.
///
/// 浏览切片文件中嵌入的缩略图图像。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thumbnail {
    /// Thumbnail width in pixels.
    ///
    /// thumbnail 像素宽度。
    pub width: u32,
    /// Thumbnail height in pixels.
    ///
    /// thumbnail 像素高度。
    pub height: u32,
    /// Row-major RGBA pixels.
    ///
    /// 行优先 RGBA 像素。
    pub pixels_rgba: Vec<u8>,
}

// ── FileMetadataBuilder ──────────────────────────────────────
//
// 统一的 `FileMetadata` 构造入口。链式 setter + `build()` 不变量校验，
// 避免 33 字段 struct literal 散落在各 format 里导致字段漂移。
//
// 设计原则：
//  * 语义分组 setter（resolution / display_size / exposure / lift / ...）
//    映射 `FileMetadata` 的 7 个逻辑组，调用方不用关心字段排序。
//  * `build()` 只校验"已被显式设置 且 取值不自洽"的情况；默认零值不触发报错，
//    保证旧构造路径（`FileMetadata { .. }`）保持等价。
//  * 不变量违反统一走 `ConvertError::MetadataValidation`，避免每个 format
//    各自 panic / 沉默接受非法值。
//
// `#[non_exhaustive]` keeps external crates on the builder path while in-crate
// parsers can still populate grouped metadata directly when mirroring source files.

/// Builder for normalized file metadata.
///
/// It groups setters by metadata concern and validates cross-field invariants in
/// `build`.
///
/// 归一化文件 metadata 的 builder。它按 metadata concern 组织 setter，并在 `build` 中校验跨字段不变量。
#[derive(Debug, Clone, Default)]
pub struct FileMetadataBuilder {
    inner: FileMetadata,
}

impl FileMetadataBuilder {
    /// Creates a builder initialized with default metadata.
    ///
    /// 创建以默认 metadata 初始化的 builder。
    pub fn new() -> Self {
        Self {
            inner: FileMetadata::default(),
        }
    }

    /// Creates a builder from an existing metadata value.
    ///
    /// 从已有 metadata 值创建 builder。
    pub fn from_metadata(metadata: FileMetadata) -> Self {
        Self { inner: metadata }
    }

    // —— Printer ——
    /// Sets native LCD resolution.
    ///
    /// 设置 LCD 原生分辨率。
    pub fn resolution(mut self, x: u32, y: u32) -> Self {
        self.inner.printer.resolution_x = x;
        self.inner.printer.resolution_y = y;
        self
    }

    /// Sets physical display size in millimeters.
    ///
    /// 设置物理显示尺寸，单位毫米。
    pub fn display_size_mm(mut self, width: f32, height: f32) -> Self {
        self.inner.printer.display_width_mm = width;
        self.inner.printer.display_height_mm = height;
        self
    }

    /// Sets printable Z height in millimeters.
    ///
    /// 设置可打印 Z 高度，单位毫米。
    pub fn machine_z_mm(mut self, z: f32) -> Self {
        self.inner.printer.machine_z_mm = z;
        self
    }

    /// Sets source-format machine name.
    ///
    /// 设置源格式机器名称。
    pub fn machine_name(mut self, name: impl Into<String>) -> Self {
        self.inner.printer.machine_name = name.into();
        self
    }

    /// Sets axis mirror flags.
    ///
    /// 设置坐标轴镜像标记。
    pub fn mirror(mut self, x: bool, y: bool) -> Self {
        self.inner.printer.mirror_x = x;
        self.inner.printer.mirror_y = y;
        self
    }

    // —— Slicing ——
    /// Sets layer height in millimeters.
    ///
    /// 设置层高，单位毫米。
    pub fn layer_height_mm(mut self, v: f32) -> Self {
        self.inner.slicing.layer_height_mm = v;
        self
    }

    /// Sets total logical layer count.
    ///
    /// 设置逻辑层总数。
    pub fn layer_count(mut self, n: u32) -> Self {
        self.inner.slicing.layer_count = n;
        self
    }

    /// Sets bottom layer count.
    ///
    /// 设置底层数量。
    pub fn bottom_layer_count(mut self, n: u32) -> Self {
        self.inner.slicing.bottom_layer_count = n;
        self
    }

    /// Sets transition layer count.
    ///
    /// 设置过渡层数量。
    pub fn transition_layer_count(mut self, n: u32) -> Self {
        self.inner.slicing.transition_layer_count = n;
        self
    }

    /// Sets anti-alias level.
    ///
    /// 设置 anti-alias 等级。
    pub fn anti_alias_level(mut self, v: u32) -> Self {
        self.inner.slicing.anti_alias_level = v;
        self
    }

    /// Sets format-specific encryption key.
    ///
    /// 设置格式特定 encryption key。
    pub fn encryption_key(mut self, k: u32) -> Self {
        self.inner.slicing.encryption_key = k;
        self
    }

    // —— Exposure ——
    /// Sets normal and bottom exposure times.
    ///
    /// 设置普通层与底层曝光时间。
    pub fn exposure(mut self, normal: f32, bottom: f32) -> Self {
        self.inner.exposure.exposure_time = normal;
        self.inner.exposure.bottom_exposure_time = bottom;
        self
    }

    /// Sets normal and bottom light-off delays.
    ///
    /// 设置普通层与底层 light-off delay。
    pub fn light_off_delay(mut self, normal: f32, bottom: f32) -> Self {
        self.inner.exposure.light_off_delay = normal;
        self.inner.exposure.bottom_light_off_delay = bottom;
        self
    }

    /// Sets normal and bottom light PWM values.
    ///
    /// 设置普通层与底层光源 PWM 值。
    pub fn light_pwm(mut self, normal: u16, bottom: u16) -> Self {
        self.inner.exposure.light_pwm = normal;
        self.inner.exposure.bottom_light_pwm = bottom;
        self
    }

    // —— Motion ——
    /// Sets first-stage normal lift height and speed.
    ///
    /// 设置普通层第一段抬升高度与速度。
    pub fn lift(mut self, height: f32, speed: f32) -> Self {
        self.inner.motion.lift_height = height;
        self.inner.motion.lift_speed = speed;
        self
    }

    /// Sets optional second-stage normal lift height and speed.
    ///
    /// 设置普通层可选第二段抬升高度与速度。
    pub fn lift2(mut self, height: f32, speed: f32) -> Self {
        self.inner.motion.lift_height2 = height;
        self.inner.motion.lift_speed2 = speed;
        self
    }

    /// Sets bottom-layer first-stage lift height and speed.
    ///
    /// 设置底层第一段抬升高度与速度。
    pub fn bottom_lift(mut self, height: f32, speed: f32) -> Self {
        self.inner.motion.bottom_lift_height = height;
        self.inner.motion.bottom_lift_speed = speed;
        self
    }

    /// Sets normal retract speed.
    ///
    /// 设置普通层回退速度。
    pub fn retract(mut self, speed: f32) -> Self {
        self.inner.motion.retract_speed = speed;
        self
    }

    /// Sets optional second-stage retract height and speed.
    ///
    /// 设置可选第二段回退高度与速度。
    pub fn retract2(mut self, height: f32, speed: f32) -> Self {
        self.inner.motion.retract_height2 = height;
        self.inner.motion.retract_speed2 = speed;
        self
    }

    // —— Wait times ——
    /// Sets wait times around cure and lift phases.
    ///
    /// 设置固化与抬升阶段前后的等待时间。
    pub fn wait_times(mut self, before_cure: f32, after_cure: f32, after_lift: f32) -> Self {
        self.inner.waits = WaitTimes {
            before_cure,
            after_cure,
            after_lift,
        };
        self
    }

    // —— Statistics ——
    /// Sets estimated print statistics.
    ///
    /// 设置预计打印统计。
    pub fn statistics(
        mut self,
        print_time_sec: u32,
        volume_ml: f32,
        weight_g: f32,
        cost: f32,
    ) -> Self {
        self.inner.stats = PrintStatistics {
            print_time_sec,
            volume_ml,
            weight_g,
            cost,
        };
        self
    }

    // —— Collections ——
    /// Replaces per-layer override settings.
    ///
    /// 替换逐层覆盖参数。
    pub fn per_layer_settings(mut self, settings: Vec<PerLayerSettings>) -> Self {
        self.inner.per_layer_settings = settings;
        self
    }

    /// Replaces embedded thumbnails.
    ///
    /// 替换嵌入缩略图。
    pub fn thumbnails(mut self, thumbnails: Vec<Thumbnail>) -> Self {
        self.inner.thumbnails = thumbnails;
        self
    }

    // —— Finalize ——
    /// Validates and returns the built metadata.
    ///
    /// Validation checks only cross-field invariants that are meaningful after
    /// callers explicitly provide data, so the all-default value remains a valid
    /// placeholder for import paths that fill metadata gradually.
    ///
    /// 校验并返回构建完成的 metadata。校验只检查调用方显式提供数据后才有意义的跨字段不变量，
    /// 因此全默认值仍是合法占位，方便 import 路径逐步填充 metadata。
    pub fn build(self) -> Result<FileMetadata, ConvertError> {
        validate_metadata(&self.inner)?;
        Ok(self.inner)
    }

    /// Returns the built metadata without validation for in-crate parsers.
    ///
    /// This is an internal escape hatch for parsers that still mirror source
    /// files with incomplete or inconsistent metadata. New code should prefer
    /// `build`.
    ///
    /// 为 crate 内 parser 跳过校验直接返回，用于仍需镜像不完整或不一致源 metadata 的导入路径。
    /// 新代码应优先使用 `build`。
    pub(crate) fn build_unchecked(self) -> FileMetadata {
        self.inner
    }
}

impl FileMetadata {
    /// Creates a metadata builder.
    ///
    /// 链式构造器入口。等价于 `FileMetadataBuilder::new()`，便于发现。
    pub fn builder() -> FileMetadataBuilder {
        FileMetadataBuilder::new()
    }

    /// Borrowed view used by per-layer encoders.
    ///
    /// The view excludes thumbnails and parse-only statistics, keeping the
    /// per-layer encode path focused on printer, slicing, exposure, motion, and
    /// wait-time facts.
    ///
    /// 供逐层 encoder 使用的 borrowed view。它排除 thumbnails 与 parse-only statistics，
    /// 只保留 printer、slicing、exposure、motion 与 wait-time facts，避免每层 owned clone。
    pub fn as_layer_encode_metadata(&self) -> LayerEncodeMetadata<'_> {
        LayerEncodeMetadata {
            printer: &self.printer,
            slicing: &self.slicing,
            exposure: &self.exposure,
            motion: &self.motion,
            waits: &self.waits,
        }
    }
}

fn validate_metadata(m: &FileMetadata) -> Result<(), ConvertError> {
    let mut issues: Vec<String> = Vec::new();

    // 分辨率与物理尺寸必须成对非零：一旦设置了 resolution_x，其余三项都要就位。
    let has_resolution = m.printer.resolution_x != 0 || m.printer.resolution_y != 0;
    let has_display_size = m.printer.display_width_mm > 0.0 || m.printer.display_height_mm > 0.0;
    if has_resolution || has_display_size {
        if m.printer.resolution_x == 0 || m.printer.resolution_y == 0 {
            issues.push(format!(
                "resolution must be positive on both axes (got {}x{})",
                m.printer.resolution_x, m.printer.resolution_y
            ));
        }
        if m.printer.display_width_mm <= 0.0 || m.printer.display_height_mm <= 0.0 {
            issues.push(format!(
                "display size must be positive on both axes (got {}x{} mm)",
                m.printer.display_width_mm, m.printer.display_height_mm
            ));
        }
    }

    // 图层数与图层高度必须一致化。
    if m.slicing.layer_count > 0 && m.slicing.layer_height_mm <= 0.0 {
        issues.push(format!(
            "layer_height_mm must be positive when layer_count > 0 (got {} mm)",
            m.slicing.layer_height_mm
        ));
    }
    if m.slicing.bottom_layer_count > m.slicing.layer_count {
        issues.push(format!(
            "bottom_layer_count ({}) must not exceed layer_count ({})",
            m.slicing.bottom_layer_count, m.slicing.layer_count
        ));
    }
    if m.slicing.transition_layer_count
        > m.slicing
            .layer_count
            .saturating_sub(m.slicing.bottom_layer_count)
    {
        issues.push(format!(
            "transition_layer_count ({}) must not exceed layer_count - bottom_layer_count ({})",
            m.slicing.transition_layer_count,
            m.slicing
                .layer_count
                .saturating_sub(m.slicing.bottom_layer_count)
        ));
    }

    // AA level 至少为 1（0 没有物理意义）。
    if m.slicing.anti_alias_level == 0 {
        issues.push("anti_alias_level must be >= 1".to_string());
    }

    // 缩略图像素缓冲大小必须匹配 width * height * 4。
    for (idx, t) in m.thumbnails.iter().enumerate() {
        if t.width == 0 || t.height == 0 {
            // 空缩略图视为占位：跳过像素校验。
            if !t.pixels_rgba.is_empty() {
                issues.push(format!(
                    "thumbnails[{idx}] has zero dimension but non-empty pixel buffer"
                ));
            }
            continue;
        }
        let expected = (t.width as usize)
            .saturating_mul(t.height as usize)
            .saturating_mul(4);
        if t.pixels_rgba.len() != expected {
            issues.push(format!(
                "thumbnails[{idx}] size mismatch: expected {expected} bytes ({}x{} RGBA), got {}",
                t.width,
                t.height,
                t.pixels_rgba.len()
            ));
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ConvertError::MetadataValidation(issues.join("; ")))
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn default_builder_passes_validation() {
        let md = FileMetadata::builder().build().expect("default must pass");
        assert_eq!(md.slicing.layer_count, 0);
        assert_eq!(md.slicing.anti_alias_level, 1);
    }

    #[test]
    fn typical_ctb_metadata_builds() {
        let md = FileMetadata::builder()
            .resolution(4920, 2880)
            .display_size_mm(218.88, 122.88)
            .machine_z_mm(200.0)
            .machine_name("Mock Saturn 3")
            .layer_height_mm(0.05)
            .layer_count(100)
            .bottom_layer_count(5)
            .transition_layer_count(3)
            .exposure(2.5, 30.0)
            .lift(6.0, 60.0)
            .bottom_lift(6.0, 60.0)
            .retract(150.0)
            .light_pwm(255, 255)
            .build()
            .expect("valid metadata");
        assert_eq!(md.printer.resolution_x, 4920);
        assert_eq!(md.slicing.layer_count, 100);
        assert_eq!(md.printer.machine_name, "Mock Saturn 3");
    }

    #[test]
    fn resolution_requires_display_size() {
        let err = FileMetadata::builder()
            .resolution(4920, 2880)
            .build()
            .expect_err("must fail without display size");
        match err {
            ConvertError::MetadataValidation(msg) => {
                assert!(msg.contains("display size"), "unexpected message: {msg}");
            }
            _ => panic!("expected MetadataValidation"),
        }
    }

    #[test]
    fn display_size_requires_resolution() {
        let err = FileMetadata::builder()
            .display_size_mm(218.88, 122.88)
            .build()
            .expect_err("must fail without resolution");
        match err {
            ConvertError::MetadataValidation(msg) => {
                assert!(msg.contains("resolution"), "unexpected message: {msg}");
            }
            _ => panic!("expected MetadataValidation"),
        }
    }

    #[test]
    fn bottom_cannot_exceed_total_layer_count() {
        let err = FileMetadata::builder()
            .layer_height_mm(0.05)
            .layer_count(10)
            .bottom_layer_count(20)
            .build()
            .expect_err("bottom > total must fail");
        assert!(matches!(err, ConvertError::MetadataValidation(_)));
    }

    #[test]
    fn transition_cannot_exceed_remaining_layers() {
        let err = FileMetadata::builder()
            .layer_height_mm(0.05)
            .layer_count(10)
            .bottom_layer_count(5)
            .transition_layer_count(6) // 10 - 5 = 5 上限
            .build()
            .expect_err("transition too large must fail");
        assert!(matches!(err, ConvertError::MetadataValidation(_)));
    }

    #[test]
    fn zero_anti_alias_rejected() {
        let err = FileMetadata::builder()
            .anti_alias_level(0)
            .build()
            .expect_err("aa=0 must fail");
        assert!(matches!(err, ConvertError::MetadataValidation(_)));
    }

    #[test]
    fn layer_count_requires_positive_height() {
        let err = FileMetadata::builder()
            .layer_count(50)
            .layer_height_mm(0.0)
            .build()
            .expect_err("layer_count>0 with height=0 must fail");
        assert!(matches!(err, ConvertError::MetadataValidation(_)));
    }

    #[test]
    fn thumbnail_size_mismatch_rejected() {
        let err = FileMetadata::builder()
            .thumbnails(vec![Thumbnail {
                width: 2,
                height: 2,
                pixels_rgba: vec![0; 10], // 应为 16
            }])
            .build()
            .expect_err("size mismatch must fail");
        if let ConvertError::MetadataValidation(msg) = err {
            assert!(msg.contains("thumbnails[0]"), "unexpected: {msg}");
        } else {
            panic!("expected MetadataValidation");
        }
    }

    #[test]
    fn empty_thumbnail_allowed() {
        let md = FileMetadata::builder()
            .thumbnails(vec![Thumbnail {
                width: 0,
                height: 0,
                pixels_rgba: vec![],
            }])
            .build()
            .expect("empty placeholder is valid");
        assert_eq!(md.thumbnails.len(), 1);
    }

    #[test]
    fn build_unchecked_skips_validation() {
        // 过渡 API：允许不合法值通过，仅限迁移期使用。
        let md = FileMetadata::builder()
            .resolution(4920, 2880) // 缺 display_size_mm
            .build_unchecked();
        assert_eq!(md.printer.resolution_x, 4920);
    }

    #[test]
    fn from_metadata_preserves_fields() {
        let original = FileMetadata {
            printer: PrinterSpec {
                machine_name: "Mock".to_string(),
                ..PrinterSpec::default()
            },
            slicing: SlicingSpec {
                layer_count: 42,
                layer_height_mm: 0.05,
                anti_alias_level: 2,
                ..SlicingSpec::default()
            },
            ..FileMetadata::default()
        };
        let md = FileMetadataBuilder::from_metadata(original.clone())
            .machine_name("Patched")
            .build()
            .expect("valid");
        assert_eq!(md.printer.machine_name, "Patched");
        assert_eq!(md.slicing.layer_count, 42);
        assert_eq!(md.slicing.anti_alias_level, 2);
    }
}
