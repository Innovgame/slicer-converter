use super::{
    ArchiveLayout, ContainerKind, CryptoPolicy, FooterPolicy, FormatCapabilities, FormatCatalog,
    FormatFamily, FormatIdentity, FormatKeys, FormatPipeline, FormatSpec, HeaderLayout, LayerCodec,
};

/// Compile-time identity descriptor for a concrete format profile.
///
/// This keeps the human-facing names together while allowing profile modules to
/// build `FormatSpec` values in `const` contexts.
///
/// 具体格式 profile 的编译期 identity 描述。它把名称、别名和扩展名集中在一起，同时让
/// profile 模块仍然可以在 `const` 上下文中组装 `FormatSpec`。
#[derive(Debug, Clone, Copy)]
pub struct ProfileIdentityDescriptor {
    /// Canonical profile name used by lookup helpers.
    ///
    /// lookup helper 使用的规范 profile name。
    pub name: &'static str,
    /// Accepted profile aliases.
    ///
    /// 可接受的 profile aliases。
    pub aliases: &'static [&'static str],
    /// File extensions accepted for this profile.
    ///
    /// 该 profile 接受的文件扩展名。
    pub extensions: &'static [&'static str],
}

/// Catalog-facing metadata for a profile.
///
/// The catalog fields are intentionally separated from `ProfileIdentityDescriptor`:
/// identity is used for detection and matching, while catalog metadata controls
/// what the UI/API exposes as export targets.
///
/// 面向 catalog 的 profile 元数据。这里特意与 `ProfileIdentityDescriptor` 分开：identity
/// 用于识别和匹配，catalog metadata 用于决定 UI/API 对外展示哪些导出目标。
#[derive(Debug, Clone, Copy)]
pub struct ProfileCatalogDescriptor {
    /// Stable id exposed through catalog APIs.
    ///
    /// 通过 catalog API 暴露的稳定 id。
    pub profile_id: &'static str,
    /// Human-readable name shown by UI/catalog callers.
    ///
    /// UI/catalog 调用方展示的人类可读名称。
    pub display_name: &'static str,
    /// Default filename extension exposed to catalog callers.
    ///
    /// 暴露给 catalog 调用方的默认文件名扩展名。
    pub default_extension: &'static str,
    /// Export target aliases accepted by resolvers.
    ///
    /// resolver 接受的导出 target aliases。
    pub target_names: &'static [&'static str],
    /// Whether this profile is the default export target for the family.
    ///
    /// 该 profile 是否为 family 的默认导出目标。
    pub default_export: bool,
}

/// Runtime wiring for a concrete profile.
///
/// These fields bind the static format facts to the implementations that do the
/// actual parse, encode, archive, crypto, and footer work. Keeping wiring in one
/// struct makes profile definitions compact without hiding which runtime
/// contracts a format participates in.
///
/// 具体 profile 的运行期接线。这里把静态格式事实绑定到真正执行 parse、encode、archive、
/// crypto 和 footer 的实现上；集中成一个结构能减少 profile 定义的重复，同时不隐藏格式参与了哪些
/// 运行期契约。
#[derive(Clone, Copy)]
pub struct RuntimeProfileWiring {
    /// Physical container shape selected for this profile.
    ///
    /// 该 profile 选择的物理容器形态。
    pub container: ContainerKind,
    /// User-facing and routing-facing capability facts.
    ///
    /// 面向用户与路由策略的能力事实。
    pub capabilities: FormatCapabilities,
    /// Stable implementation keys for route and drift checks.
    ///
    /// route 与 drift check 使用的稳定实现 key。
    pub keys: FormatKeys,
    /// File-level header/layout implementation.
    ///
    /// 文件级 header/layout 实现。
    pub header: &'static dyn HeaderLayout,
    /// Per-layer payload codec implementation.
    ///
    /// 逐层 payload codec 实现。
    pub layer_codec: &'static dyn LayerCodec,
    /// Per-layer crypto policy.
    ///
    /// 逐层 crypto 策略。
    pub crypto: &'static dyn CryptoPolicy,
    /// Binary footer policy.
    ///
    /// 二进制 footer 策略。
    pub footer: &'static dyn FooterPolicy,
    /// Archive layout policy.
    ///
    /// archive layout 策略。
    pub archive: &'static dyn ArchiveLayout,
    /// Document-level parse/assembly pipeline.
    ///
    /// document 级 parse/assembly pipeline。
    pub pipeline: &'static dyn FormatPipeline,
}

/// Build a full `FormatSpec` from profile descriptors and runtime wiring.
///
/// This helper is deliberately a `const fn` so static profile tables can stay
/// declarative and allocation-free. It should not add behavior; format-specific
/// behavior belongs in the referenced policies and pipeline implementations.
///
/// 从 profile descriptor 和 runtime wiring 组装完整的 `FormatSpec`。该 helper 故意保持为
/// `const fn`，让静态 profile table 维持声明式且无分配；它不应承载行为，具体格式行为应留在被引用的
/// policy 和 pipeline 实现中。
pub const fn format_spec_from_profile_parts(
    identity: ProfileIdentityDescriptor,
    family: FormatFamily,
    catalog: ProfileCatalogDescriptor,
    runtime: RuntimeProfileWiring,
) -> FormatSpec {
    FormatSpec {
        identity: FormatIdentity {
            name: identity.name,
            aliases: identity.aliases,
            extensions: identity.extensions,
        },
        family,
        catalog: FormatCatalog {
            profile_id: catalog.profile_id,
            display_name: catalog.display_name,
            default_extension: catalog.default_extension,
            target_names: catalog.target_names,
            default_export: catalog.default_export,
        },
        container: runtime.container,
        capabilities: runtime.capabilities,
        keys: runtime.keys,
        header: runtime.header,
        layer_codec: runtime.layer_codec,
        crypto: runtime.crypto,
        footer: runtime.footer,
        archive: runtime.archive,
        pipeline: runtime.pipeline,
    }
}
