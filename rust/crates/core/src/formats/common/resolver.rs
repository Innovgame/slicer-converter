use crate::common::{ConvertError, FileMetadata};

use super::{FormatFamilySpec, FormatSpec};

/// Normalized export target lookup candidates.
///
/// It accepts either an explicit profile id or an output-like target such as
/// `model.v4.ctb`, then produces suffix candidates in most-specific-first order.
///
/// 归一化后的导出目标候选。它可以接收显式 profile id，也可以接收类似输出路径的 target
/// （例如 `model.v4.ctb`），再按从最具体到最宽泛的顺序生成 suffix candidates。
pub(crate) struct ExportTargetRequest {
    explicit: bool,
    candidates: Vec<String>,
}

impl ExportTargetRequest {
    /// Creates normalized candidates from an export target and optional explicit profile.
    ///
    /// Explicit profiles bypass filename suffix inference. Non-explicit targets
    /// keep the full basename first, then progressively drop leading dot
    /// segments so compound targets like `v4.ctb` can be matched.
    ///
    /// 从导出 target 与可选显式 profile 创建归一化候选。显式 profile 会跳过文件名后缀推断；
    /// 非显式 target 会先保留完整 basename，再逐步去掉前面的 dot segment，以匹配 `v4.ctb`
    /// 这类复合 target。
    pub(crate) fn new(target: &str, explicit_profile: Option<&str>) -> Self {
        if let Some(profile) = explicit_profile {
            return Self {
                explicit: true,
                candidates: vec![Self::normalize(profile)],
            };
        }

        let file_name = target
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(target)
            .trim_start_matches('.');
        let normalized = Self::normalize(file_name);
        let mut candidates = Vec::new();
        if !normalized.is_empty() {
            candidates.push(normalized.clone());
        }

        let parts = normalized.split('.').collect::<Vec<_>>();
        for index in 1..parts.len() {
            let suffix = parts[index..].join(".");
            if !suffix.is_empty() && !candidates.contains(&suffix) {
                candidates.push(suffix);
            }
        }

        Self {
            explicit: false,
            candidates,
        }
    }

    fn normalize(value: &str) -> String {
        value.trim_start_matches('.').to_ascii_lowercase()
    }

    /// Iterates candidates in resolution order.
    ///
    /// 按解析顺序遍历候选 target。
    pub(crate) fn candidates(&self) -> impl Iterator<Item = &str> {
        self.candidates.iter().map(String::as_str)
    }

    /// Returns the explicit profile candidate when the request came from an explicit hint.
    ///
    /// 当请求来自显式 hint 时，返回对应的 explicit profile candidate。
    pub(crate) fn explicit_profile_for<'a>(&self, candidate: &'a str) -> Option<&'a str> {
        self.explicit.then_some(candidate)
    }

    /// Returns whether a candidate can address the given family.
    ///
    /// 判断候选是否可指向给定 family。
    pub(crate) fn family_matches(&self, candidate: &str, family: &FormatFamilySpec) -> bool {
        family.matches_name(candidate)
    }
}

/// Export target options shared by family resolvers.
///
/// family resolver 共用的导出目标选项。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportTargetOptions<'a> {
    /// Explicit concrete profile id; skips suffix inference when present.
    ///
    /// 显式 concrete profile id；存在时跳过后缀推断。
    pub explicit_profile: Option<&'a str>,
    /// Optional format-level version requested by the caller.
    ///
    /// 调用方请求的可选格式级版本。
    pub format_version: Option<u32>,
}

impl<'a> ExportTargetOptions<'a> {
    pub const fn new() -> Self {
        Self {
            explicit_profile: None,
            format_version: None,
        }
    }

    pub const fn with_explicit_profile(mut self, explicit_profile: &'a str) -> Self {
        self.explicit_profile = Some(explicit_profile);
        self
    }

    pub const fn with_format_version(mut self, format_version: u32) -> Self {
        self.format_version = Some(format_version);
        self
    }
}

/// Profile resolver for a format family.
///
/// Families such as Chitubox and Anycubic expose multiple concrete profiles
/// under related names/extensions. A resolver owns profile detection from bytes
/// and export target resolution from filenames, explicit profile hints, and
/// optional metadata.
///
/// 格式 family 的 profile 解析器。Chitubox、Anycubic 这类 family 会在相近的名称/扩展名
/// 下暴露多个具体 profile；resolver 负责从文件字节检测 profile，也负责根据输出文件名、
/// 显式 profile hint 与可选 metadata 解析导出目标。
pub trait ProfileResolver: Sync {
    /// Detects the concrete profile for an input document.
    ///
    /// The resolver may use the extension/name hint to narrow candidates, but it
    /// must confirm with profile-specific header detection before returning a
    /// profile.
    ///
    /// 为输入 document 检测具体 profile。resolver 可以使用扩展名/名称 hint 缩小候选，
    /// 但返回 profile 前必须通过 profile-specific header detection 确认。
    fn detect_profile(
        &self,
        data: &[u8],
        hint: Option<&str>,
        profiles: &'static [&'static FormatSpec],
    ) -> Option<&'static FormatSpec>;

    /// Resolves the concrete profile to use for export.
    ///
    /// `target` is usually an output path or extension-like request. An explicit
    /// profile hint wins over suffix inference; family-specific resolvers may use
    /// metadata to choose a machine profile.
    ///
    /// 解析导出时使用的具体 profile。`target` 通常是输出路径或类似扩展名的请求。
    /// 显式 profile hint 优先于后缀推断；family-specific resolver 可以使用 metadata
    /// 选择机器 profile。
    fn resolve_export_profile(
        &self,
        target: &str,
        metadata: Option<&FileMetadata>,
        options: ExportTargetOptions<'_>,
        profiles: &'static [&'static FormatSpec],
    ) -> Result<&'static FormatSpec, ConvertError>;
}

/// Rejects registry profiles that are visible but not exportable.
///
/// Import-only profiles may be discoverable, but export resolution must stop
/// before returning them as writable targets.
///
/// 拒绝可见但不可导出的 registry profile。import-only profile 可以被发现，但 export
/// resolution 不能把它们返回为可写目标。
pub(crate) fn ensure_export_capable(
    spec: &'static FormatSpec,
) -> Result<&'static FormatSpec, ConvertError> {
    if spec.capabilities.can_export {
        Ok(spec)
    } else {
        Err(ConvertError::UnknownFormat)
    }
}

#[cfg(test)]
mod tests {
    use super::ExportTargetRequest;

    #[test]
    fn export_target_requests_normalize_paths_suffixes_and_explicit_profiles() {
        let request = ExportTargetRequest::new("/tmp/model.v4.ctb", None);
        assert_eq!(
            request.candidates().collect::<Vec<_>>(),
            vec!["model.v4.ctb", "v4.ctb", "ctb"]
        );

        let dotted = ExportTargetRequest::new(".ctb", None);
        assert_eq!(dotted.candidates().collect::<Vec<_>>(), vec!["ctb"]);

        let explicit = ExportTargetRequest::new("model.ctb", Some("ctb-v5"));
        assert_eq!(explicit.candidates().collect::<Vec<_>>(), vec!["ctb-v5"]);
        assert_eq!(explicit.explicit_profile_for("ctb-v5"), Some("ctb-v5"));
    }
}

/// Default resolver for families without special profile-scoping rules.
///
/// It tries hinted extension detection first, then falls back to profile header
/// detection order. For export it accepts direct profile names and otherwise
/// returns the default exportable profile.
///
/// 没有特殊 profile 范围规则的 family 使用的默认 resolver。import 时先尝试 hint extension
/// detection，再按 profile 顺序做 header detection；export 时接受直接 profile name，否则返回默认可导出
/// profile。
pub struct SimpleProfileResolver;

impl SimpleProfileResolver {
    /// Creates a stateless simple resolver.
    ///
    /// 创建无状态 simple resolver。
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SimpleProfileResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileResolver for SimpleProfileResolver {
    fn detect_profile(
        &self,
        data: &[u8],
        hint: Option<&str>,
        profiles: &'static [&'static FormatSpec],
    ) -> Option<&'static FormatSpec> {
        let hinted = hint.and_then(|hint| {
            profiles
                .iter()
                .copied()
                .find(|spec| spec.matches_extension(hint))
        });

        if let Some(spec) = hinted {
            if spec.header.try_detect(data, hint) {
                return Some(spec);
            }
        }

        profiles
            .iter()
            .copied()
            .find(|spec| spec.header.try_detect(data, hint))
    }

    fn resolve_export_profile(
        &self,
        target: &str,
        _metadata: Option<&FileMetadata>,
        options: ExportTargetOptions<'_>,
        profiles: &'static [&'static FormatSpec],
    ) -> Result<&'static FormatSpec, ConvertError> {
        let requested = options.explicit_profile.unwrap_or(target);
        let resolved = profiles
            .iter()
            .copied()
            .find(|spec| spec.matches_name(requested));
        if let Some(spec) = resolved {
            return ensure_export_capable(spec);
        }
        if options.explicit_profile.is_some() {
            return Err(ConvertError::UnknownFormat);
        }

        profiles
            .iter()
            .copied()
            .find(|spec| spec.catalog.default_export && spec.capabilities.can_export)
            .ok_or(ConvertError::UnknownFormat)
    }
}

/// Resolver that refuses hints outside the family/profile namespace.
///
/// This prevents broad extension aliases from letting one family detect another
/// family's bytes after an unrelated hint was supplied.
///
/// 会拒绝 family/profile namespace 之外 hint 的 resolver。它避免宽泛扩展名 alias 在传入无关
/// hint 后仍让某个 family 去检测其它 family 的字节。
pub struct HintScopedProfileResolver;

impl HintScopedProfileResolver {
    fn matches_profile_request(request: &str, spec: &FormatSpec) -> bool {
        spec.matches_name(request)
    }

    fn matches_family_request(request: &str, profiles: &'static [&'static FormatSpec]) -> bool {
        profiles
            .iter()
            .copied()
            .any(|spec| Self::matches_profile_request(request, spec))
    }
}

impl ProfileResolver for HintScopedProfileResolver {
    fn detect_profile(
        &self,
        data: &[u8],
        hint: Option<&str>,
        profiles: &'static [&'static FormatSpec],
    ) -> Option<&'static FormatSpec> {
        if let Some(hint) = hint {
            if !Self::matches_family_request(hint, profiles) {
                return None;
            }
        }

        SimpleProfileResolver::new().detect_profile(data, hint, profiles)
    }

    fn resolve_export_profile(
        &self,
        target: &str,
        metadata: Option<&FileMetadata>,
        options: ExportTargetOptions<'_>,
        profiles: &'static [&'static FormatSpec],
    ) -> Result<&'static FormatSpec, ConvertError> {
        let requested = options.explicit_profile.unwrap_or(target);
        if !Self::matches_family_request(requested, profiles) {
            return Err(ConvertError::UnknownFormat);
        }

        SimpleProfileResolver::new().resolve_export_profile(target, metadata, options, profiles)
    }
}
