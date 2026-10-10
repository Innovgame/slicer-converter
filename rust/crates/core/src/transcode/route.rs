use crate::formats::common::{FormatKey, FormatSpec};

/// Compact key used to choose layer-level transcode strategies.
///
/// The key intentionally focuses on the payload-transform dimensions: container,
/// layer codec, pixel model, and crypto. Family/profile/header/archive facts are
/// kept in `TranscodeRouteDescriptor` for diagnostics and tests.
///
/// 用于选择逐层转码策略的紧凑 key。它只关注 payload transform 维度：container、layer codec、
/// pixel model 与 crypto；family/profile/header/archive 事实保留在
/// `TranscodeRouteDescriptor` 中供诊断和测试使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TranscodeRouteKey {
    /// Source container implementation key.
    ///
    /// 源 container 实现 key。
    pub source_container: FormatKey,
    /// Target container implementation key.
    ///
    /// 目标 container 实现 key。
    pub target_container: FormatKey,
    /// Source layer codec implementation key.
    ///
    /// 源 layer codec 实现 key。
    pub source_layer_codec: FormatKey,
    /// Target layer codec implementation key.
    ///
    /// 目标 layer codec 实现 key。
    pub target_layer_codec: FormatKey,
    /// Source decoded pixel model key.
    ///
    /// 源 decoded pixel model key。
    pub source_pixel_model: FormatKey,
    /// Target decoded pixel model key.
    ///
    /// 目标 decoded pixel model key。
    pub target_pixel_model: FormatKey,
    /// Source crypto policy key.
    ///
    /// 源 crypto policy key。
    pub source_crypto: FormatKey,
    /// Target crypto policy key.
    ///
    /// 目标 crypto policy key。
    pub target_crypto: FormatKey,
}

/// Expanded route descriptor used by diagnostics and architecture tests.
///
/// It embeds the compact `route_key` plus the broader facts that should not
/// affect fast-path lookup but are useful when explaining or validating a route.
///
/// 用于诊断和架构测试的展开 route descriptor。它包含紧凑的 `route_key`，以及不应影响快路径查找、
/// 但有助于解释或验证 route 的更宽泛事实。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TranscodeRouteDescriptor {
    /// Compact fast-path lookup key.
    ///
    /// 紧凑快路径查找 key。
    pub route_key: TranscodeRouteKey,
    /// Source family key.
    ///
    /// 源 family key。
    pub source_family_key: FormatKey,
    /// Target family key.
    ///
    /// 目标 family key。
    pub target_family_key: FormatKey,
    /// Source concrete profile key.
    ///
    /// 源具体 profile key。
    pub source_profile_key: FormatKey,
    /// Target concrete profile key.
    ///
    /// 目标具体 profile key。
    pub target_profile_key: FormatKey,
    /// Source header/layout key.
    ///
    /// 源 header/layout key。
    pub source_header_key: FormatKey,
    /// Target header/layout key.
    ///
    /// 目标 header/layout key。
    pub target_header_key: FormatKey,
    /// Source archive layout key.
    ///
    /// 源 archive layout key。
    pub source_archive_key: FormatKey,
    /// Target archive layout key.
    ///
    /// 目标 archive layout key。
    pub target_archive_key: FormatKey,
}

impl TranscodeRouteKey {
    /// Builds a compact route key from source and target specs.
    ///
    /// 从源/目标 spec 构建紧凑 route key。
    pub const fn from_specs(source: &FormatSpec, target: &FormatSpec) -> Self {
        Self {
            source_container: source.keys.container_key,
            target_container: target.keys.container_key,
            source_layer_codec: source.keys.layer_codec_key,
            target_layer_codec: target.keys.layer_codec_key,
            source_pixel_model: source.keys.pixel_model_key,
            target_pixel_model: target.keys.pixel_model_key,
            source_crypto: source.keys.crypto_key,
            target_crypto: target.keys.crypto_key,
        }
    }
}

impl TranscodeRouteDescriptor {
    /// Builds a full route descriptor from source and target specs.
    ///
    /// 从源/目标 spec 构建完整 route descriptor。
    pub fn from_specs(source: &FormatSpec, target: &FormatSpec) -> Self {
        Self {
            route_key: TranscodeRouteKey::from_specs(source, target),
            source_family_key: source.keys.family_key,
            target_family_key: target.keys.family_key,
            source_profile_key: source.keys.profile_key,
            target_profile_key: target.keys.profile_key,
            source_header_key: source.keys.header_key,
            target_header_key: target.keys.header_key,
            source_archive_key: source.keys.archive_key,
            target_archive_key: target.keys.archive_key,
        }
    }
}
