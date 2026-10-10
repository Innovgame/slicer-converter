use super::{
    anycubic::specifics::{AnycubicArchiveSpec, AnycubicSpec, AnycubicVersion},
    chitubox::specifics::{ChituboxProfileKind, ChituboxSpec},
    common,
    creality::specifics::CrealityCxdLpSpec,
};
use serde::Serialize;

/// Serializable format profile facts exposed to TypeScript/WASM callers.
///
/// This is the catalog-facing view of `FormatSpec`: it includes names,
/// extensions, capabilities, and optional family-specific facts, but it does not
/// expose runtime trait objects.
///
/// 暴露给 TypeScript/WASM 调用方的可序列化格式 profile 信息。它是 `FormatSpec` 的
/// catalog-facing 视图：包含名称、扩展名、能力和可选 family-specific facts，但不暴露运行期
/// trait object。
#[derive(Serialize)]
pub struct SupportedFormatInfo {
    name: String,
    display_name: String,
    family: &'static str,
    profile_id: String,
    aliases: Vec<String>,
    extensions: Vec<String>,
    default_extension: String,
    target_names: Vec<String>,
    is_zip: bool,
    capabilities: common::FormatCapabilities,
    specifics: SupportedFormatSpecifics,
}

/// Family-specific catalog payload attached to `SupportedFormatInfo`.
///
/// Generic profiles use `None`; richer families expose enough facts for UI and
/// contract tests to understand concrete variants without importing Rust-only
/// runtime policies.
///
/// `SupportedFormatInfo` 携带的 family-specific catalog payload。通用 profile 使用
/// `None`；更复杂的 family 会暴露足够事实，让 UI 与契约测试理解具体 variant，而不需要导入
/// Rust-only runtime policy。
#[derive(Serialize)]
#[serde(tag = "family", rename_all = "lowercase")]
pub enum SupportedFormatSpecifics {
    /// No family-specific catalog payload.
    ///
    /// 没有 family-specific catalog payload。
    None,
    /// Chitubox-family profile facts.
    ///
    /// Chitubox-family profile facts。
    Chitubox {
        kind: &'static str,
        ctb_version: Option<u32>,
        machine_profile: Option<&'static str>,
        allowed_magics: Vec<u32>,
        allowed_versions: Vec<u32>,
        build_version: Option<u32>,
        layer_table_entry_size: Option<u32>,
    },
    /// Anycubic binary profile facts.
    ///
    /// Anycubic binary profile facts。
    Anycubic {
        machine_id: super::anycubic::AnycubicMachineId,
        display_name: &'static str,
        extension: &'static str,
        default_version: AnycubicVersion,
        supported_versions: Vec<AnycubicVersion>,
        table_layout: &'static str,
        resolution_x: u32,
        resolution_y: u32,
        display_width_mm: f32,
        display_height_mm: f32,
        machine_z_mm: f32,
    },
    /// Anycubic archive profile facts.
    ///
    /// Anycubic archive profile facts。
    #[serde(rename = "anycubic-archive")]
    AnycubicArchive {
        settings_manifest_entry: &'static str,
        layers_manifest_entry: &'static str,
        layer_entry_prefix: &'static str,
        default_layer_image_extension: &'static str,
        layer_image_extensions: Vec<&'static str>,
        required_manifest_entries: Vec<&'static str>,
    },
    /// Creality CXDLP profile facts.
    ///
    /// Creality CXDLP profile facts。
    Creality {
        variant: &'static str,
        version: u32,
        extension: &'static str,
        magic: &'static str,
        checksum: &'static str,
        layer_definition_size: u32,
        layer_parameter_size: u32,
        preview_count: u32,
        supports_layer_encryption: bool,
    },
}

/// Catalog wrapper pairing a runtime `FormatSpec` with optional typed facts.
///
/// The runtime registry uses `FormatSpec`; catalog generation needs the typed
/// family facts as well, so each family publishes one of these wrappers.
///
/// catalog wrapper 将运行期 `FormatSpec` 与可选 typed facts 配对。运行期 registry 使用
/// `FormatSpec`；catalog 生成还需要 typed family facts，因此每个 family 会发布这种 wrapper。
#[derive(Clone, Copy)]
pub enum CatalogProfile {
    /// Profile without extra family-specific facts.
    ///
    /// 没有额外 family-specific facts 的 profile。
    Generic(&'static common::FormatSpec),
    /// Chitubox profile with typed facts.
    ///
    /// 带 typed facts 的 Chitubox profile。
    Chitubox(&'static common::FamilyProfile<ChituboxSpec>),
    /// Anycubic binary profile with typed facts.
    ///
    /// 带 typed facts 的 Anycubic binary profile。
    Anycubic(&'static common::FamilyProfile<AnycubicSpec>),
    /// Anycubic archive profile with typed facts.
    ///
    /// 带 typed facts 的 Anycubic archive profile。
    AnycubicArchive(&'static common::FamilyProfile<AnycubicArchiveSpec>),
    /// Creality profile with typed facts.
    ///
    /// 带 typed facts 的 Creality profile。
    Creality(&'static common::FamilyProfile<CrealityCxdLpSpec>),
}

impl CatalogProfile {
    /// Returns the runtime spec behind this catalog wrapper.
    ///
    /// 返回该 catalog wrapper 背后的运行期 spec。
    pub const fn spec(self) -> &'static common::FormatSpec {
        match self {
            Self::Generic(spec) => spec,
            Self::Chitubox(profile) => profile.spec,
            Self::Anycubic(profile) => profile.spec,
            Self::AnycubicArchive(profile) => profile.spec,
            Self::Creality(profile) => profile.spec,
        }
    }

    /// Returns Chitubox-specific facts when this is a Chitubox profile.
    ///
    /// 当该 profile 属于 Chitubox 时返回 Chitubox-specific facts。
    pub const fn as_chitubox(self) -> Option<&'static ChituboxSpec> {
        match self {
            Self::Chitubox(profile) => Some(profile.facts),
            _ => None,
        }
    }

    /// Returns Anycubic binary facts when this is an Anycubic binary profile.
    ///
    /// 当该 profile 属于 Anycubic binary 时返回对应 facts。
    pub const fn as_anycubic(self) -> Option<&'static AnycubicSpec> {
        match self {
            Self::Anycubic(profile) => Some(profile.facts),
            _ => None,
        }
    }

    /// Returns Anycubic archive facts when this is an Anycubic archive profile.
    ///
    /// 当该 profile 属于 Anycubic archive 时返回对应 facts。
    pub const fn as_anycubic_archive(self) -> Option<&'static AnycubicArchiveSpec> {
        match self {
            Self::AnycubicArchive(profile) => Some(profile.facts),
            _ => None,
        }
    }

    /// Returns Creality-specific facts when this is a Creality profile.
    ///
    /// 当该 profile 属于 Creality 时返回 Creality-specific facts。
    pub const fn as_creality(self) -> Option<&'static CrealityCxdLpSpec> {
        match self {
            Self::Creality(profile) => Some(profile.facts),
            _ => None,
        }
    }
}

/// Builds serializable supported-format entries from catalog profiles.
///
/// The order is preserved from the registry input so UI callers can rely on the
/// same family/profile ordering as the Rust registry.
///
/// 从 catalog profiles 构建可序列化的 supported-format entries。输出顺序保留 registry 输入顺序，
/// 让 UI 调用方可依赖与 Rust registry 一致的 family/profile 排序。
pub fn supported_format_infos(
    profiles: &'static [&'static CatalogProfile],
) -> Vec<SupportedFormatInfo> {
    profiles
        .iter()
        .map(|profile| {
            let spec = profile.spec();
            let id = spec.identity;
            SupportedFormatInfo {
                name: id.name.to_string(),
                display_name: spec.catalog.display_name.to_string(),
                family: format_family_name(spec.family),
                profile_id: spec.catalog.profile_id.to_string(),
                aliases: id
                    .aliases
                    .iter()
                    .map(|alias| (*alias).to_string())
                    .collect(),
                extensions: id
                    .extensions
                    .iter()
                    .map(|extension| (*extension).to_string())
                    .collect(),
                default_extension: spec.catalog.default_extension.to_string(),
                target_names: spec
                    .catalog
                    .target_names
                    .iter()
                    .map(|target| (*target).to_string())
                    .collect(),
                is_zip: spec.capabilities.is_archive,
                capabilities: spec.capabilities,
                specifics: supported_format_specifics(*profile),
            }
        })
        .collect()
}

fn format_family_name(family: common::FormatFamily) -> &'static str {
    match family {
        common::FormatFamily::Chitubox => "chitubox",
        common::FormatFamily::Anycubic => "anycubic",
        common::FormatFamily::Goo => "goo",
        common::FormatFamily::Phrozen => "phrozen",
        common::FormatFamily::Sl1 => "sl1",
        common::FormatFamily::Vdt => "vdt",
        common::FormatFamily::Creality => "creality",
        common::FormatFamily::Other(name) => name,
    }
}

fn supported_format_specifics(profile: &CatalogProfile) -> SupportedFormatSpecifics {
    match profile {
        CatalogProfile::Chitubox(profile) => {
            let chitubox = profile.facts;
            SupportedFormatSpecifics::Chitubox {
                kind: chitubox_kind_name(chitubox.kind),
                ctb_version: chitubox.ctb_version,
                machine_profile: chitubox.machine_profile,
                allowed_magics: chitubox.allowed_magics.to_vec(),
                allowed_versions: chitubox.allowed_versions.to_vec(),
                build_version: chitubox.build_version,
                layer_table_entry_size: chitubox.layer_table_entry_size,
            }
        }
        CatalogProfile::Anycubic(profile) => {
            let anycubic = profile.facts;
            SupportedFormatSpecifics::Anycubic {
                machine_id: anycubic.machine.machine_id,
                display_name: anycubic.machine.display_name,
                extension: anycubic.extension,
                default_version: anycubic.default_version,
                supported_versions: anycubic.supported_versions.to_vec(),
                table_layout: anycubic.table_layout.as_str(),
                resolution_x: anycubic.machine.resolution_x,
                resolution_y: anycubic.machine.resolution_y,
                display_width_mm: anycubic.machine.display_width_mm,
                display_height_mm: anycubic.machine.display_height_mm,
                machine_z_mm: anycubic.machine.machine_z_mm,
            }
        }
        CatalogProfile::AnycubicArchive(profile) => {
            let anycubic_archive = profile.facts;
            SupportedFormatSpecifics::AnycubicArchive {
                settings_manifest_entry: anycubic_archive.settings_manifest_entry,
                layers_manifest_entry: anycubic_archive.layers_manifest_entry,
                layer_entry_prefix: anycubic_archive.layer_entry_prefix,
                default_layer_image_extension: anycubic_archive.default_layer_image_extension,
                layer_image_extensions: anycubic_archive.layer_image_extensions.to_vec(),
                required_manifest_entries: anycubic_archive.required_manifest_entries.to_vec(),
            }
        }
        CatalogProfile::Creality(profile) => {
            let creality = profile.facts;
            SupportedFormatSpecifics::Creality {
                variant: creality.variant,
                version: creality.version,
                extension: creality.extension,
                magic: creality.magic,
                checksum: creality.checksum,
                layer_definition_size: creality.layer_definition_size,
                layer_parameter_size: creality.layer_parameter_size,
                preview_count: creality.preview_count,
                supports_layer_encryption: creality.supports_layer_encryption,
            }
        }
        CatalogProfile::Generic(_) => SupportedFormatSpecifics::None,
    }
}

fn chitubox_kind_name(kind: ChituboxProfileKind) -> &'static str {
    match kind {
        ChituboxProfileKind::Ctb => "ctb",
        ChituboxProfileKind::GktwoCtb => "gktwo-ctb",
        ChituboxProfileKind::Cbddlp => "cbddlp",
        ChituboxProfileKind::Photon => "photon",
        ChituboxProfileKind::Phz => "phz",
        ChituboxProfileKind::Fdg => "fdg",
        ChituboxProfileKind::PhotonS => "photons",
        ChituboxProfileKind::EncryptedCtb => "encrypted-ctb",
        ChituboxProfileKind::Zip => "zip",
    }
}
