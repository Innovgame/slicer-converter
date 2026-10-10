pub mod anycubic;
pub mod catalog;
pub mod chitubox;
pub mod common;
pub mod creality;
pub mod ctb;
pub mod diagnostics;
pub mod goo;
pub mod prz;
pub mod sl1;
pub mod vdt;

use std::sync::OnceLock;

use crate::common::{ConvertError, FileMetadata};

pub use common::FormatIdentity;
pub use common::{LayerDecodeContext, LayerEncodeContext, LayerOptions};
pub use diagnostics::diagnose_export;

// ── Format Registry ─────────────────────────────────────────

/// Built-in format families in deterministic detection/catalog order.
///
/// Families own their concrete profile specs and resolver policy; this slice is
/// the root of the runtime registry graph.
///
/// 内置 format families，顺序同时用于 detection 与 catalog。family 持有其具体 profile specs
/// 与 resolver policy；这个 slice 是运行期 registry graph 的根。
pub const FORMAT_FAMILIES: &[&common::FormatFamilySpec] = &[
    &chitubox::FAMILY,
    &anycubic::FAMILY,
    &goo::FAMILY,
    &prz::FAMILY,
    &creality::FAMILY,
    &sl1::FAMILY,
    &vdt::FAMILY,
];

/// Catalog profile groups aligned with `FORMAT_FAMILIES`.
///
/// These wrappers carry optional family-specific facts for WASM/TS catalog
/// serialization while reusing the same runtime `FormatSpec` references.
///
/// 与 `FORMAT_FAMILIES` 对齐的 catalog profile groups。这些 wrapper 携带可选
/// family-specific facts 供 WASM/TS catalog 序列化，同时复用同一组运行期 `FormatSpec` 引用。
pub const CATALOG_PROFILE_GROUPS: &[&[&catalog::CatalogProfile]] = &[
    chitubox::specs::CATALOG_PROFILES,
    anycubic::specs::CATALOG_PROFILES,
    goo::specs::CATALOG_PROFILES,
    prz::specs::CATALOG_PROFILES,
    creality::specs::CATALOG_PROFILES,
    sl1::specs::CATALOG_PROFILES,
    vdt::specs::CATALOG_PROFILES,
];

fn all_format_specs() -> &'static [&'static common::FormatSpec] {
    static ALL_FORMAT_SPECS: OnceLock<Box<[&'static common::FormatSpec]>> = OnceLock::new();
    ALL_FORMAT_SPECS
        .get_or_init(|| {
            FORMAT_FAMILIES
                .iter()
                .flat_map(|family| family.profiles.iter().copied())
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .as_ref()
}

fn catalog_format_profiles() -> &'static [&'static catalog::CatalogProfile] {
    static CATALOG_FORMAT_PROFILES: OnceLock<Box<[&'static catalog::CatalogProfile]>> =
        OnceLock::new();
    CATALOG_FORMAT_PROFILES
        .get_or_init(|| {
            CATALOG_PROFILE_GROUPS
                .iter()
                .flat_map(|profiles| profiles.iter().copied())
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .as_ref()
}

// ── Registry service ─────────────────────────────────────────

/// Abstraction over a format registry: name/extension lookup + magic detection.
///
/// 对格式注册表的抽象接口：按名称/扩展名查找、基于魔数字节自动识别。
/// lib.rs 与上层应用通过 `&'static dyn FormatRegistry` 依赖本 trait，
/// 便于在测试中注入自定义 registry 或扩展第三方格式。
pub trait FormatRegistry: Sync {
    /// List unified format families used by the internal `FormatSpec` kernel.
    ///
    /// 返回内部 `FormatSpec` 内核使用的统一 format families。
    fn families(&self) -> &'static [&'static common::FormatFamilySpec] {
        &[]
    }

    /// Flatten all concrete profile specs from the unified family registry.
    ///
    /// 从统一 family registry 展平全部 concrete profile specs。
    fn all_specs(&self) -> &'static [&'static common::FormatSpec];

    /// Flatten all concrete profile descriptions for catalog/WASM-facing facts.
    ///
    /// 展平面向 catalog/WASM 的 concrete profile descriptions。
    fn catalog_profiles(&self) -> &'static [&'static catalog::CatalogProfile] {
        &[]
    }

    /// Lookup a concrete profile description by profile/format name, alias, extension, or target name.
    ///
    /// 通过 profile/format name、alias、extension 或 target name 查找 concrete profile description。
    fn catalog_profile_by_name(&self, name: &str) -> Option<&'static catalog::CatalogProfile> {
        self.catalog_profiles()
            .iter()
            .copied()
            .find(|profile| profile.spec().matches_name(name))
    }

    /// Lookup a concrete profile description for an already-resolved runtime spec.
    ///
    /// 根据已解析的 runtime spec 查找对应 concrete profile description。
    fn catalog_profile_for_spec(
        &self,
        spec: &'static common::FormatSpec,
    ) -> Option<&'static catalog::CatalogProfile> {
        self.catalog_profiles()
            .iter()
            .copied()
            .find(|profile| std::ptr::eq(profile.spec(), spec))
    }

    /// Detect a concrete profile spec from the unified family registry.
    ///
    /// Detection lets family resolvers inspect bytes first and uses the extension
    /// hint as fallback, which keeps shared extensions from overriding stronger
    /// magic/header evidence.
    ///
    /// 从统一 family registry 检测 concrete profile spec。检测流程允许 family resolver 先检查字节，
    /// 再把 extension hint 作为 fallback，避免共享扩展名覆盖更强的 magic/header 证据。
    fn detect_spec(
        &self,
        data: &[u8],
        extension_hint: Option<&str>,
    ) -> Option<&'static common::FormatSpec> {
        let families = self.families();
        if !families.is_empty() {
            let hinted = extension_hint.and_then(|hint| self.spec_by_extension(hint));
            let detected = families.iter().find_map(|family| {
                family
                    .resolver
                    .detect_profile(data, extension_hint, family.profiles)
            });
            return detected.or(hinted);
        }

        let hinted = extension_hint.and_then(|hint| self.spec_by_extension(hint));
        common::ProfileResolver::detect_profile(
            &common::SimpleProfileResolver::new(),
            data,
            extension_hint,
            self.all_specs(),
        )
        .or(hinted)
    }

    /// Lookup a concrete profile spec by profile/format name, alias, extension, or target name.
    ///
    /// 通过 profile/format name、alias、extension 或 target name 查找 concrete profile spec。
    fn spec_by_name(&self, name: &str) -> Option<&'static common::FormatSpec> {
        self.all_specs()
            .iter()
            .copied()
            .find(|spec| spec.matches_name(name))
    }

    /// Lookup a concrete profile spec by file extension.
    ///
    /// 通过文件扩展名查找 concrete profile spec。
    fn spec_by_extension(&self, extension: &str) -> Option<&'static common::FormatSpec> {
        self.all_specs()
            .iter()
            .copied()
            .find(|spec| spec.matches_extension(extension))
    }

    /// Resolve an export target through family-specific profile selection.
    ///
    /// `target` may be a path, extension-like request, or family/profile alias.
    /// `options.explicit_profile` bypasses suffix inference and must resolve directly.
    ///
    /// 通过 family-specific profile selection 解析导出目标。`target` 可以是路径、类似扩展名的请求或
    /// family/profile alias；`options.explicit_profile` 会跳过后缀推断并要求直接解析成功。
    fn resolve_export_spec(
        &self,
        target: &str,
        metadata: Option<&FileMetadata>,
        options: common::ExportTargetOptions<'_>,
    ) -> Result<&'static common::FormatSpec, ConvertError> {
        let families = self.families();
        if !families.is_empty() {
            let request = common::ExportTargetRequest::new(target, options.explicit_profile);
            let mut matched_family_error = None;
            for candidate in request.candidates() {
                for family in families
                    .iter()
                    .copied()
                    .filter(|family| request.family_matches(candidate, family))
                {
                    match family.resolver.resolve_export_profile(
                        candidate,
                        metadata,
                        common::ExportTargetOptions {
                            explicit_profile: request.explicit_profile_for(candidate),
                            format_version: options.format_version,
                        },
                        family.profiles,
                    ) {
                        Ok(spec) => return common::ensure_export_capable(spec),
                        Err(error) => matched_family_error = Some(error),
                    }
                }
            }
            if let Some(error) = matched_family_error {
                return Err(error);
            }
            return Err(ConvertError::UnknownFormat);
        }

        let spec = common::ProfileResolver::resolve_export_profile(
            &common::SimpleProfileResolver::new(),
            target,
            metadata,
            options,
            self.all_specs(),
        )?;
        common::ensure_export_capable(spec)
    }
}

/// Built-in registry backed by the unified family profile graph.
///
/// 由统一 family profile graph 支撑的内置 registry。
pub struct BuiltinRegistry;

impl FormatRegistry for BuiltinRegistry {
    fn families(&self) -> &'static [&'static common::FormatFamilySpec] {
        FORMAT_FAMILIES
    }

    fn all_specs(&self) -> &'static [&'static common::FormatSpec] {
        all_format_specs()
    }

    fn catalog_profiles(&self) -> &'static [&'static catalog::CatalogProfile] {
        catalog_format_profiles()
    }
}

static BUILTIN_REGISTRY: BuiltinRegistry = BuiltinRegistry;

/// Returns the process-wide built-in format registry.
///
/// 获取进程级内置格式注册表；上层代码可通过该 `&'static dyn FormatRegistry`
/// 进行所有查找/检测。runtime `FormatSpec` 和 catalog `CatalogProfile`
/// 均从 family-owned declarations 展平派生。
pub fn global_registry() -> &'static dyn FormatRegistry {
    &BUILTIN_REGISTRY
}

// ── Spec lookup helpers ──────────────────────────────────────

/// Get the registered [`common::FormatSpec`] by format/profile name.
///
/// 按格式名称字符串获取对应的 `FormatSpec`。
/// 如果该名称不在注册表中则返回 `Err(ConvertError::UnknownFormat)`。
pub fn get_format_spec_by_name(name: &str) -> Result<&'static common::FormatSpec, ConvertError> {
    global_registry()
        .spec_by_name(name)
        .ok_or(ConvertError::UnknownFormat)
}

/// Detect a registered [`common::FormatSpec`] from magic bytes or extension hint.
///
/// 通过魔数字节或文件扩展名提示自动识别切片文件格式。
/// 扩展名提示优先用于消歧共享魔数字节的格式（如 GOO/PRZ、PW0/PWS）。
pub fn detect_format_spec(
    data: &[u8],
    extension_hint: Option<&str>,
) -> Result<&'static common::FormatSpec, ConvertError> {
    global_registry()
        .detect_spec(data, extension_hint)
        .ok_or(ConvertError::UnknownFormat)
}
