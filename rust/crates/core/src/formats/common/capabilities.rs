use serde::Serialize;

/// User-facing and routing-facing capability facts for a format profile.
///
/// These flags describe what the runtime should offer or filter. They are not a
/// substitute for codec/layout contracts: a profile can share codec keys while
/// exposing different import/export capabilities.
///
/// 面向用户与路由策略的格式能力事实。这些 flag 描述 runtime 应展示或过滤的能力，
/// 但不能替代 codec/layout 契约：不同 profile 可以复用 codec key，同时暴露不同的
/// import/export 能力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FormatCapabilities {
    /// Whether the registry may use this profile for import/detection.
    ///
    /// registry 是否可用该 profile 进行 import/detection。
    pub can_import: bool,
    /// Whether export resolvers may choose this profile.
    ///
    /// export resolver 是否可选择该 profile。
    pub can_export: bool,
    /// Whether the physical container is an archive rather than a single binary file.
    ///
    /// 物理容器是否为 archive，而不是单文件二进制。
    pub is_archive: bool,
    /// Whether the UI/runtime should expose layer preview support.
    ///
    /// UI/runtime 是否应暴露 layer preview 能力。
    pub supports_preview: bool,
    /// Whether the format has anti-alias or multi-bitplane layer semantics.
    ///
    /// 格式是否具有 anti-alias 或多 bitplane layer 语义。
    pub supports_anti_alias: bool,
    /// Whether layer-level exposure/motion settings are meaningful.
    ///
    /// layer 级 exposure/motion 设置是否有意义。
    pub supports_per_layer_settings: bool,
    /// Whether layer payloads may be encrypted.
    ///
    /// layer payload 是否可能加密。
    pub supports_encryption: bool,
    /// Whether opening requires full-document bytes instead of only header bytes.
    ///
    /// 打开文件时是否需要完整 document bytes，而不只是 header bytes。
    pub requires_full_document_parse: bool,
}

impl FormatCapabilities {
    /// Baseline capabilities for implemented single-file binary formats.
    ///
    /// 已实现单文件二进制格式的基础能力集。
    pub const fn binary_import_export() -> Self {
        Self {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: false,
            supports_anti_alias: false,
            supports_per_layer_settings: false,
            supports_encryption: false,
            requires_full_document_parse: false,
        }
    }

    /// Baseline capabilities for implemented archive formats.
    ///
    /// 已实现 archive 格式的基础能力集。
    pub const fn archive_import_export() -> Self {
        Self {
            is_archive: true,
            requires_full_document_parse: true,
            ..Self::binary_import_export()
        }
    }

    /// Capabilities for archive formats that can be opened but not exported yet.
    ///
    /// 可打开但暂不能导出的 archive 格式能力集。
    pub const fn archive_import_only() -> Self {
        Self {
            can_import: true,
            can_export: false,
            is_archive: true,
            supports_preview: false,
            supports_anti_alias: false,
            supports_per_layer_settings: false,
            supports_encryption: false,
            requires_full_document_parse: true,
        }
    }

    /// Sets preview support while preserving the rest of the capability facts.
    ///
    /// 设置 preview 支持，同时保留其它 capability facts。
    pub const fn with_preview(mut self, value: bool) -> Self {
        self.supports_preview = value;
        self
    }

    /// Sets anti-alias support while preserving the rest of the capability facts.
    ///
    /// 设置 anti-alias 支持，同时保留其它 capability facts。
    pub const fn with_anti_alias(mut self, value: bool) -> Self {
        self.supports_anti_alias = value;
        self
    }

    /// Sets per-layer settings support while preserving other capability facts.
    ///
    /// 设置 per-layer settings 支持，同时保留其它 capability facts。
    pub const fn with_per_layer_settings(mut self, value: bool) -> Self {
        self.supports_per_layer_settings = value;
        self
    }

    /// Sets encryption support while preserving the rest of the capability facts.
    ///
    /// 设置 encryption 支持，同时保留其它 capability facts。
    pub const fn with_encryption(mut self, value: bool) -> Self {
        self.supports_encryption = value;
        self
    }
}
