use crate::{
    common::{ConvertError, FileMetadata},
    formats::common::{
        ensure_export_capable, ExportTargetOptions, FormatSpec, ProfileResolver,
        SimpleProfileResolver,
    },
};

use super::{specs, AnycubicSpec, AnycubicVersion};

/// Anycubic profile resolver.
///
/// Exporting to the broad `anycubic` target may infer a concrete machine
/// profile from metadata resolution and machine-name aliases.
///
/// Anycubic profile resolver。导出到宽泛的 `anycubic` target 时，可根据 metadata 分辨率与
/// machine-name aliases 推断具体机器 profile。
pub struct AnycubicResolver;

impl ProfileResolver for AnycubicResolver {
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
        options: ExportTargetOptions<'_>,
        profiles: &'static [&'static FormatSpec],
    ) -> Result<&'static FormatSpec, ConvertError> {
        let requested = options.explicit_profile.unwrap_or(target);
        if let Some(spec) = profiles
            .iter()
            .copied()
            .find(|spec| spec.matches_name(requested))
        {
            return ensure_anycubic_version_supported(ensure_export_capable(spec)?, options);
        }

        if options.explicit_profile.is_some() {
            return Err(ConvertError::UnknownFormat);
        }

        if target.eq_ignore_ascii_case("anycubic") {
            if let Some(metadata) = metadata {
                return ensure_anycubic_version_supported(
                    resolve_by_metadata(metadata, profiles)?,
                    options,
                );
            }
        }

        ensure_anycubic_version_supported(
            SimpleProfileResolver::new()
                .resolve_export_profile(target, metadata, options, profiles)?,
            options,
        )
    }
}

fn resolve_by_metadata(
    metadata: &FileMetadata,
    profiles: &'static [&'static FormatSpec],
) -> Result<&'static FormatSpec, ConvertError> {
    // Resolution is the hard filter because Anycubic extensions are machine
    // families, not only container/codec variants. Display size and aliases are
    // ranking signals among machines with the same pixel dimensions.
    let candidates = profiles
        .iter()
        .copied()
        .filter_map(|spec| anycubic_facts_for_spec(spec).map(|anycubic| (spec, anycubic)))
        .filter(|(spec, _)| spec.capabilities.can_export)
        .filter(|(_, anycubic)| anycubic.matches_dimensions(metadata))
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(ConvertError::NoMatchingAnycubicProfile {
            resolution_x: metadata.printer.resolution_x,
            resolution_y: metadata.printer.resolution_y,
            display_width_mm: metadata.printer.display_width_mm,
            display_height_mm: metadata.printer.display_height_mm,
        });
    }

    let mut ranked = candidates
        .into_iter()
        .map(|(spec, anycubic)| {
            // Prefer an alias match when the source metadata names a known
            // machine, then use display-size distance to break ties between
            // same-resolution profiles.
            let display_distance = (anycubic.machine.display_width_mm
                - metadata.printer.display_width_mm)
                .abs()
                + (anycubic.machine.display_height_mm - metadata.printer.display_height_mm).abs();
            let alias_score = alias_score(anycubic, &metadata.printer.machine_name);
            (spec, anycubic, alias_score, display_distance)
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| left.3.total_cmp(&right.3))
            .then_with(|| {
                left.1
                    .machine
                    .display_name
                    .cmp(right.1.machine.display_name)
            })
    });

    let best = ranked[0];
    let tied = ranked
        .iter()
        .filter(|candidate| candidate.2 == best.2 && candidate.3 == best.3)
        .collect::<Vec<_>>();

    if tied.len() > 1 {
        return Err(ConvertError::AmbiguousAnycubicProfile {
            resolution_x: metadata.printer.resolution_x,
            resolution_y: metadata.printer.resolution_y,
            candidates: tied
                .iter()
                .map(|candidate| candidate.1.machine.display_name)
                .collect(),
        });
    }

    Ok(best.0)
}

fn anycubic_facts_for_spec(spec: &'static FormatSpec) -> Option<&'static AnycubicSpec> {
    specs::ANYCUBIC_PROFILES
        .iter()
        .copied()
        .find(|profile| std::ptr::eq(profile.spec, spec))
        .map(|profile| profile.facts)
}

fn ensure_anycubic_version_supported(
    spec: &'static FormatSpec,
    options: ExportTargetOptions<'_>,
) -> Result<&'static FormatSpec, ConvertError> {
    let Some(version) = options.format_version else {
        return Ok(spec);
    };
    let Some(anycubic) = anycubic_facts_for_spec(spec) else {
        return Ok(spec);
    };
    let Some(parsed_version) = AnycubicVersion::from_u32(version) else {
        return Err(unsupported_anycubic_version(anycubic, version));
    };
    if anycubic.supports_version(parsed_version) {
        Ok(spec)
    } else {
        Err(unsupported_anycubic_version(
            anycubic,
            parsed_version.as_u32(),
        ))
    }
}

fn unsupported_anycubic_version(anycubic: &AnycubicSpec, version: u32) -> ConvertError {
    let supported = anycubic
        .supported_versions
        .iter()
        .map(|version| version.label())
        .collect::<Vec<_>>()
        .join(", ");
    ConvertError::UnsupportedFormatVersion {
        target: anycubic.extension.to_string(),
        version,
        supported,
    }
}

fn alias_score(specifics: &AnycubicSpec, machine_name: &str) -> usize {
    if machine_name.trim().is_empty() {
        return 0;
    }

    let haystack = machine_name.to_ascii_lowercase();
    specifics
        .machine
        .machine_name_aliases
        .iter()
        .filter_map(|alias| haystack.contains(*alias).then_some(alias.len()))
        .max()
        .unwrap_or(0)
}
