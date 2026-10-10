//! Core format specification structs.
//!
//! `FormatSpec` is the runtime composition root for one concrete profile. Keep
//! identity/catalog facts, stable implementation keys, and trait-object wiring
//! separate: identity answers "what can users request", catalog answers "what
//! should UI/API expose", keys answer "which implementation facts are stable",
//! and wiring answers "which code runs".
//!
//! 核心格式规格结构。`FormatSpec` 是单个具体 profile 的 runtime composition root。
//! identity/catalog facts、稳定 implementation keys 与 trait-object wiring 需要保持分离：
//! identity 回答“用户能请求什么”，catalog 回答“UI/API 应展示什么”，keys 回答“哪些实现事实稳定”，
//! wiring 回答“实际跑哪段代码”。

use super::{
    ArchiveLayout, ContainerKind, CryptoPolicy, FamilyIdentity, FooterPolicy, FormatCapabilities,
    FormatCatalog, FormatFamily, FormatKey, FormatPipeline, HeaderLayout, LayerCodec,
    ProfileResolver,
};

/// Aggregated lookup identity for one concrete slicer profile.
///
/// These values feed registry lookup and extension matching. User-facing labels
/// and export target descriptions belong in `FormatCatalog`, not here.
///
/// 单个具体 slicer profile 的聚合查找身份。这里的值用于 registry lookup 与 extension matching；
/// 面向用户的 label 和导出 target 描述属于 `FormatCatalog`，不属于这里。
#[derive(Debug, Clone, Copy)]
pub struct FormatIdentity {
    /// Canonical lowercase name, e.g. `"ctb"`, `"goo"`, `"pw0"`.
    pub name: &'static str,
    /// Alternate names accepted during name-based lookup (includes `name` itself
    /// conventionally for ergonomic `contains` checks).
    pub aliases: &'static [&'static str],
    /// File extensions supported by this format (lowercase, no leading dot).
    pub extensions: &'static [&'static str],
}

/// Stable implementation-key bundle for a concrete profile.
///
/// These keys let routing, architecture tests, and TS contract checks compare
/// implementation facts without depending on trait object identity.
///
/// 具体 profile 的稳定实现 key 集合。routing、架构测试和 TS 契约检查可用它比较实现事实，
/// 而不依赖 trait object identity。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatKeys {
    /// Stable family discriminator used by route keys and architecture guards.
    ///
    /// 稳定的 family 判别键，用于 route key 与架构护栏测试。
    pub family_key: FormatKey,
    /// Stable concrete-profile discriminator, finer than extension or family.
    ///
    /// 稳定的具体 profile 判别键，比扩展名或 family 更细。
    pub profile_key: FormatKey,
    /// Stable container-shape key: binary file or archive file.
    ///
    /// 稳定的容器形态键：单文件二进制或 archive。
    pub container_key: FormatKey,
    /// Header/layout implementation key for detecting compatible layout reuse.
    ///
    /// header/layout 实现键，用于识别哪些 profile 复用同一布局实现。
    pub header_key: FormatKey,
    /// Layer codec implementation key used by transcode routing.
    ///
    /// layer codec 实现键，供 transcode route 判定可复用路径。
    pub layer_codec_key: FormatKey,
    /// Pixel model key for formats whose payloads represent different pixel semantics.
    ///
    /// pixel model 键，用于区分 payload 像素语义不同的格式。
    pub pixel_model_key: FormatKey,
    /// Crypto implementation key; plain formats use `keys::NO_CRYPTO`.
    ///
    /// crypto 实现键；明文格式使用 `keys::NO_CRYPTO`。
    pub crypto_key: FormatKey,
    /// Footer implementation key; most formats use `keys::NO_FOOTER`.
    ///
    /// footer 实现键；多数格式使用 `keys::NO_FOOTER`。
    pub footer_key: FormatKey,
    /// Archive layout key; binary formats use `keys::NO_ARCHIVE`.
    ///
    /// archive layout 键；二进制格式使用 `keys::NO_ARCHIVE`。
    pub archive_key: FormatKey,
}

/// Runtime wiring for one concrete import/export profile.
///
/// A `FormatSpec` is the registry-owned composition root for a slicer format
/// profile: it joins identity/catalog facts, file container shape, header
/// layout, layer codec, crypto/footer/archive policies, and the document
/// pipeline used by CLI and WASM entrypoints.
///
/// 单个具体导入/导出 profile 的运行期装配根。`FormatSpec` 把格式身份、catalog 能力、
/// 文件容器、header layout、layer codec、crypto/footer/archive 策略以及 document
/// pipeline 组合在一起，CLI 与 WASM 入口都通过它路由。
pub struct FormatSpec {
    /// Lookup identity accepted by CLI/WASM/user-facing format selectors.
    ///
    /// CLI/WASM/用户侧格式选择器使用的查找身份。
    pub identity: FormatIdentity,
    /// Broad format family for grouped profile resolution and catalog display.
    ///
    /// 粗粒度格式 family，用于 profile 分组解析与 catalog 展示。
    pub family: FormatFamily,
    /// User-facing catalog facts that should not drive low-level codec behavior.
    ///
    /// 面向用户/catalog 的事实，不应直接替代底层 codec/layout 契约。
    pub catalog: FormatCatalog,
    /// Physical container shape for routing document assembly.
    ///
    /// 物理容器形态，用于路由 document assembly。
    pub container: ContainerKind,
    /// Capability facts consumed by UI policy, export filtering, and runtime guards.
    ///
    /// 供 UI policy、导出过滤与 runtime guard 消费的能力事实。
    pub capabilities: FormatCapabilities,
    /// Stable implementation keys used by transcode route selection and drift tests.
    ///
    /// 稳定实现键，用于 transcode route 选择与漂移测试。
    pub keys: FormatKeys,
    /// File-level layout implementation for detection, metadata parsing, and header finalization.
    ///
    /// 文件级 layout 实现，负责检测、metadata 解析与 header finalize。
    pub header: &'static dyn HeaderLayout,
    /// Per-layer payload codec. It should not handle document layout or archive naming.
    ///
    /// 单层 payload codec；不应处理 document layout 或 archive naming。
    pub layer_codec: &'static dyn LayerCodec,
    /// Per-layer crypto policy applied outside `layer_codec`.
    ///
    /// 单层加密策略，在 `layer_codec` 外层执行。
    pub crypto: &'static dyn CryptoPolicy,
    /// Optional trailing-byte policy for binary containers.
    ///
    /// 二进制容器的可选尾部字节策略。
    pub footer: &'static dyn FooterPolicy,
    /// Archive entry naming and metadata policy. Binary profiles use `NoArchiveLayout`.
    ///
    /// archive entry 命名与 metadata 策略；二进制 profile 使用 `NoArchiveLayout`。
    pub archive: &'static dyn ArchiveLayout,
    /// File-level parse/assembly strategy tying all policies together.
    ///
    /// 文件级 parse/assembly 策略，将以上策略组合为完整文件流程。
    pub pipeline: &'static dyn FormatPipeline,
}

/// Registry entry for a format family and its concrete profiles.
///
/// A family spec owns the closed set of concrete profiles plus the resolver that
/// chooses among them for import detection and export target selection. New
/// profile variants should enter through this closed set so registry lookup,
/// catalog projection, and behavior tests stay aligned.
///
/// format family 的 registry 条目。它持有该 family 的具体 profile 闭合集合，以及用于
/// import detection/export target selection 的 resolver。新增 profile variant 应进入这个闭合集合，
/// 让 registry lookup、catalog projection 与行为测试保持一致。
pub struct FormatFamilySpec {
    /// Family lookup identity shared by its concrete profiles.
    ///
    /// family 层面的查找身份，由其具体 profiles 共享。
    pub identity: FamilyIdentity,
    /// Broad family enum value used by catalog and route grouping.
    ///
    /// catalog 与 route grouping 使用的 family 枚举值。
    pub family: FormatFamily,
    /// Concrete profiles in deterministic lookup order.
    ///
    /// Order is observable through fallback extension lookup and catalog output,
    /// so profile additions should be deliberate.
    ///
    /// 确定性查找顺序下的具体 profiles。顺序会影响 fallback extension lookup 与 catalog 输出，
    /// 因此新增 profile 时需要有意安排位置。
    pub profiles: &'static [&'static FormatSpec],
    /// Resolver responsible for selecting a concrete profile inside the family.
    ///
    /// The resolver is the only place that should encode family-specific
    /// detection/export selection rules.
    ///
    /// 负责在 family 内选择具体 profile 的 resolver。family-specific detection/export
    /// selection 规则应集中在这里。
    pub resolver: &'static dyn ProfileResolver,
}
