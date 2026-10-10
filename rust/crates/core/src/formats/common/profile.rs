use super::{keys, spec::FormatSpec, FormatKey};

/// Broad slicer format family.
///
/// Families group concrete profiles that share detection/export policy, but
/// they are intentionally coarser than `FormatSpec`.
///
/// 粗粒度切片格式 family。family 用来分组共享 detection/export policy 的具体 profiles，
/// 但它故意比 `FormatSpec` 更粗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatFamily {
    /// Chitubox binary and Chitubox-derived profiles.
    ///
    /// Chitubox 二进制及其派生 profile。
    Chitubox,
    /// Anycubic binary/archive profiles.
    ///
    /// Anycubic 二进制或 archive profile。
    Anycubic,
    /// Goo-family binary profiles.
    ///
    /// Goo-family 二进制 profile。
    Goo,
    /// Phrozen-family profiles.
    ///
    /// Phrozen-family profile。
    Phrozen,
    /// Prusa SL1 archive profile.
    ///
    /// Prusa SL1 archive profile。
    Sl1,
    /// VDT archive profile.
    ///
    /// VDT archive profile。
    Vdt,
    /// Creality CXDLP profiles.
    ///
    /// Creality CXDLP profile。
    Creality,
    /// Extension point for cataloged families outside the built-in set.
    ///
    /// 内置集合之外的 catalog family 扩展点。
    Other(&'static str),
}

impl FormatFamily {
    /// Returns the stable key for this broad family.
    ///
    /// 返回该粗粒度 family 的稳定 key。
    pub const fn as_key(self) -> FormatKey {
        match self {
            Self::Chitubox => keys::FAMILY_CHITUBOX,
            Self::Anycubic => keys::FAMILY_ANYCUBIC,
            Self::Goo => keys::FAMILY_GOO,
            Self::Phrozen => keys::FAMILY_PHROZEN,
            Self::Sl1 => keys::FAMILY_SL1,
            Self::Vdt => keys::FAMILY_VDT,
            Self::Creality => keys::FAMILY_CREALITY,
            Self::Other(_) => keys::FAMILY_OTHER,
        }
    }
}

/// Physical top-level container shape.
///
/// This controls which document pipeline is appropriate; it does not by itself
/// determine the layer codec.
///
/// 顶层物理容器形态。它决定适合的 document pipeline，但不单独决定 layer codec。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    /// Single-file binary container.
    ///
    /// 单文件二进制容器。
    Binary,
    /// Archive container, currently ZIP-based for implemented formats.
    ///
    /// archive 容器；当前已实现格式以 ZIP 为主。
    Archive,
}

impl ContainerKind {
    /// Returns the stable key for this container shape.
    ///
    /// 返回该容器形态的稳定 key。
    pub const fn as_key(self) -> FormatKey {
        match self {
            Self::Binary => keys::CONTAINER_BINARY,
            Self::Archive => keys::CONTAINER_ARCHIVE,
        }
    }
}

/// Lookup identity shared by a format family.
///
/// family 层面的查找身份。
#[derive(Debug, Clone, Copy)]
pub struct FamilyIdentity {
    /// Canonical family name.
    ///
    /// family 规范名。
    pub name: &'static str,
    /// Accepted family aliases.
    ///
    /// 可接受的 family 别名。
    pub aliases: &'static [&'static str],
    /// Extensions that may route into this family before profile detection.
    ///
    /// profile detection 前可路由到该 family 的扩展名。
    pub extensions: &'static [&'static str],
}

/// User-facing catalog facts for one concrete profile.
///
/// These values support selection UI and export target names; runtime behavior
/// should be driven by `FormatSpec` fields and keys.
///
/// 单个具体 profile 的面向用户 catalog 事实。这些值支持选择 UI 与导出 target names；
/// runtime 行为应由 `FormatSpec` 字段和 keys 驱动。
#[derive(Debug, Clone, Copy)]
pub struct FormatCatalog {
    /// Stable catalog/profile id exposed to callers.
    ///
    /// 暴露给调用方的稳定 catalog/profile id。
    pub profile_id: &'static str,
    /// Human-readable display name.
    ///
    /// 面向人的显示名称。
    pub display_name: &'static str,
    /// Default filename extension for exports, lowercase and without a leading dot.
    ///
    /// 导出文件名默认扩展名，小写且不带前导点。
    pub default_extension: &'static str,
    /// Export target names accepted in addition to identity aliases/extensions.
    ///
    /// 除 identity aliases/extensions 之外可接受的导出 target names。
    pub target_names: &'static [&'static str],
    /// Whether this profile is the family default when no more specific target is supplied.
    ///
    /// 未提供更具体 target 时，该 profile 是否为 family 默认导出目标。
    pub default_export: bool,
}

/// Pairing between a runtime profile and typed family-specific facts.
///
/// 运行期 profile 与 family-specific typed facts 的配对。
#[derive(Clone, Copy)]
pub struct FamilyProfile<Facts: 'static> {
    /// Concrete runtime profile.
    ///
    /// 具体运行期 profile。
    pub spec: &'static FormatSpec,
    /// Static family-specific facts for UI/catalog/resolver use.
    ///
    /// 供 UI/catalog/resolver 使用的静态 family-specific facts。
    pub facts: &'static Facts,
}
