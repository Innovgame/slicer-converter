use serde::Serialize;

use crate::common::FileMetadata;

use super::{AnycubicLayerCodec, AnycubicLayoutStrategy};

/// Anycubic binary file version.
///
/// Anycubic 二进制文件版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnycubicVersion {
    /// Legacy v1 layout.
    ///
    /// 旧版 v1 layout。
    V1,
    /// V5.15 layout.
    ///
    /// V5.15 layout。
    V515,
    /// V5.16 layout.
    ///
    /// V5.16 layout。
    V516,
    /// V5.17 layout.
    ///
    /// V5.17 layout。
    V517,
    /// V5.18 layout.
    ///
    /// V5.18 layout。
    V518,
}

impl AnycubicVersion {
    /// Returns the numeric version stored in Anycubic headers.
    ///
    /// 返回 Anycubic header 中存储的数字版本。
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V515 => 515,
            Self::V516 => 516,
            Self::V517 => 517,
            Self::V518 => 518,
        }
    }

    /// Parses a numeric Anycubic file version.
    ///
    /// 解析数字形式的 Anycubic 文件版本。
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::V1),
            515 => Some(Self::V515),
            516 => Some(Self::V516),
            517 => Some(Self::V517),
            518 => Some(Self::V518),
            _ => None,
        }
    }

    /// Returns the UVTools-compatible numeric version label.
    ///
    /// 返回与 UVTools 兼容的数字版本 label。
    pub const fn label(self) -> &'static str {
        match self {
            Self::V1 => "1",
            Self::V515 => "515",
            Self::V516 => "516",
            Self::V517 => "517",
            Self::V518 => "518",
        }
    }

    /// Returns the lowercase catalog/version label.
    ///
    /// 返回小写 catalog/version label。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V515 => "v515",
            Self::V516 => "v516",
            Self::V517 => "v517",
            Self::V518 => "v518",
        }
    }
}

impl Serialize for AnycubicVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.as_u32())
    }
}

/// Anycubic binary table layout family.
///
/// Multiple versions share most fields but differ in file mark and table sizes;
/// this enum names the concrete layout strategy used by a profile.
///
/// Anycubic 二进制 table layout family。多个版本共享大部分字段，但 file mark 与 table size
/// 不同；该枚举标识 profile 使用的具体 layout strategy。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnycubicTableLayout {
    /// V1 table layout.
    ///
    /// V1 table layout。
    V1,
    /// V5.15 table layout.
    ///
    /// V5.15 table layout。
    V515,
    /// V5.16 table layout.
    ///
    /// V5.16 table layout。
    V516,
    /// V5.17 table layout.
    ///
    /// V5.17 table layout。
    V517,
    /// V5.18 table layout.
    ///
    /// V5.18 table layout。
    V518,
}

impl AnycubicTableLayout {
    /// Returns the catalog label for this table layout.
    ///
    /// 返回该 table layout 的 catalog label。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "V1",
            Self::V515 => "V515",
            Self::V516 => "V516",
            Self::V517 => "V517",
            Self::V518 => "V518",
        }
    }
}

/// Returns the table layout used by a file version.
///
/// 返回指定文件版本使用的 table layout。
pub const fn layout_for_version(version: AnycubicVersion) -> AnycubicTableLayout {
    match version {
        AnycubicVersion::V1 => AnycubicTableLayout::V1,
        AnycubicVersion::V515 => AnycubicTableLayout::V515,
        AnycubicVersion::V516 => AnycubicTableLayout::V516,
        AnycubicVersion::V517 => AnycubicTableLayout::V517,
        AnycubicVersion::V518 => AnycubicTableLayout::V518,
    }
}

/// Stable machine id for known Anycubic printer profiles.
///
/// 已知 Anycubic printer profile 的稳定 machine id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnycubicMachineId {
    PhotonS,
    PhotonZero,
    PhotonX,
    PhotonUltra,
    PhotonD2,
    PhotonMono,
    PhotonMonoSE,
    PhotonMono4,
    PhotonMono4K,
    PhotonMonoX,
    PhotonMonoX2,
    PhotonMonoSQ,
    PhotonM3,
    PhotonM3Max,
    PhotonM3Premium,
    PhotonMono2,
    PhotonMonoM5,
    PhotonMonoM5s,
    PhotonMonoM5sPro,
    PhotonMonoX6KM3Plus,
    PhotonMonoX6Ks,
}

/// Physical machine facts used for Anycubic export target inference.
///
/// 用于 Anycubic 导出目标推断的物理机器事实。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnycubicMachineFacts {
    /// Stable machine id.
    ///
    /// 稳定 machine id。
    pub machine_id: AnycubicMachineId,
    /// Human-readable machine name.
    ///
    /// 面向人的机器名称。
    pub display_name: &'static str,
    /// Lowercase aliases used to match source metadata machine names.
    ///
    /// 用于匹配源 metadata machine name 的小写 aliases。
    pub machine_name_aliases: &'static [&'static str],
    /// Native X resolution.
    ///
    /// X 方向原生分辨率。
    pub resolution_x: u32,
    /// Native Y resolution.
    ///
    /// Y 方向原生分辨率。
    pub resolution_y: u32,
    /// Physical display width in millimeters.
    ///
    /// 物理显示宽度，单位毫米。
    pub display_width_mm: f32,
    /// Physical display height in millimeters.
    ///
    /// 物理显示高度，单位毫米。
    pub display_height_mm: f32,
    /// Printable Z height in millimeters.
    ///
    /// 可打印 Z 高度，单位毫米。
    pub machine_z_mm: f32,
}

/// Anycubic binary profile facts.
///
/// These facts connect catalog/machine identity to the runtime layout strategy
/// and layer codec used by the corresponding `FormatSpec`.
///
/// Anycubic 二进制 profile facts。它把 catalog/machine identity 连接到对应 `FormatSpec`
/// 使用的运行期 layout strategy 与 layer codec。
#[derive(Clone, Copy)]
pub struct AnycubicSpec {
    /// Physical machine facts.
    ///
    /// 物理机器事实。
    pub machine: AnycubicMachineFacts,
    /// Preferred file extension.
    ///
    /// 首选文件扩展名。
    pub extension: &'static str,
    /// Supported file versions for this profile.
    ///
    /// 该 profile 支持的文件版本。
    pub supported_versions: &'static [AnycubicVersion],
    /// Default export version.
    ///
    /// 默认导出版本。
    pub default_version: AnycubicVersion,
    /// Runtime table layout family.
    ///
    /// 运行期 table layout family。
    pub table_layout: AnycubicTableLayout,
    /// Binary layout implementation.
    ///
    /// 二进制 layout 实现。
    pub layout: &'static dyn AnycubicLayoutStrategy,
    /// Layer codec implementation.
    ///
    /// layer codec 实现。
    pub codec: &'static dyn AnycubicLayerCodec,
}

/// Anycubic ZIP/archive profile facts.
///
/// Anycubic ZIP/archive profile facts。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnycubicArchiveSpec {
    /// Settings manifest entry name.
    ///
    /// settings manifest entry 名称。
    pub settings_manifest_entry: &'static str,
    /// Layers manifest entry name.
    ///
    /// layers manifest entry 名称。
    pub layers_manifest_entry: &'static str,
    /// Prefix used by layer image entries.
    ///
    /// layer image entry 使用的前缀。
    pub layer_entry_prefix: &'static str,
    /// Default layer image extension.
    ///
    /// 默认 layer image 扩展名。
    pub default_layer_image_extension: &'static str,
    /// Accepted layer image extensions.
    ///
    /// 可接受的 layer image 扩展名。
    pub layer_image_extensions: &'static [&'static str],
    /// Manifest entries required for import.
    ///
    /// import 所需 manifest entries。
    pub required_manifest_entries: &'static [&'static str],
}

impl AnycubicSpec {
    /// Returns whether this profile supports the requested file version.
    ///
    /// 判断该 profile 是否支持请求的文件版本。
    pub fn supports_version(&self, version: AnycubicVersion) -> bool {
        self.supported_versions.contains(&version)
    }

    /// Returns whether metadata resolution matches this machine profile.
    ///
    /// 判断 metadata 分辨率是否匹配该机器 profile。
    pub fn matches_dimensions(&self, metadata: &FileMetadata) -> bool {
        metadata.printer.resolution_x == self.machine.resolution_x
            && metadata.printer.resolution_y == self.machine.resolution_y
    }
}

impl std::fmt::Debug for AnycubicSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnycubicSpec")
            .field("machine", &self.machine)
            .field("extension", &self.extension)
            .field("supported_versions", &self.supported_versions)
            .field("default_version", &self.default_version)
            .field("table_layout", &self.table_layout)
            .finish_non_exhaustive()
    }
}
