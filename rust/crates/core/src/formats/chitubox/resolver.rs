use crate::{
    common::{ConvertError, FileMetadata},
    formats::common::{FormatSpec, ProfileResolver, SimpleProfileResolver},
};

/// Chitubox-family resolver.
///
/// Chitubox currently uses the shared simple resolver; the dedicated type keeps
/// the family boundary explicit and leaves room for future machine/profile
/// selection rules.
///
/// Chitubox-family resolver。当前复用 shared simple resolver；保留专用类型是为了显式表达
/// family 边界，并为后续 machine/profile selection 规则留位置。
pub struct ChituboxResolver;

impl ProfileResolver for ChituboxResolver {
    fn detect_profile(
        &self,
        data: &[u8],
        hint: Option<&str>,
        profiles: &'static [&'static FormatSpec],
    ) -> Option<&'static FormatSpec> {
        SimpleProfileResolver::new().detect_profile(data, hint, profiles)
    }

    fn resolve_export_profile(
        &self,
        target: &str,
        metadata: Option<&FileMetadata>,
        options: crate::formats::common::ExportTargetOptions<'_>,
        profiles: &'static [&'static FormatSpec],
    ) -> Result<&'static FormatSpec, ConvertError> {
        SimpleProfileResolver::new().resolve_export_profile(target, metadata, options, profiles)
    }
}
