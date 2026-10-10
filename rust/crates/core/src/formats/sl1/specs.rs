use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, ContainerKind, DefaultArchivePipeline, FamilyIdentity, FormatCapabilities,
        FormatCatalog, FormatFamily, FormatFamilySpec, FormatKey, FormatKeys, FormatSpec,
        HintScopedProfileResolver, NoCrypto, NoFooter,
    },
    FormatIdentity,
};

use super::adapters::{PngLayerCodec, Sl1ArchiveHeaderLayout, Sl1ArchiveLayout};

static SL1_HEADER: Sl1ArchiveHeaderLayout = Sl1ArchiveHeaderLayout;
static SL1_ARCHIVE: Sl1ArchiveLayout = Sl1ArchiveLayout;
static PNG_CODEC: PngLayerCodec = PngLayerCodec;

/// Runtime spec for Prusa SL1/SL1S ZIP archives.
///
/// The archive layout owns the SL1 config/entry naming, while layer pixels are
/// encoded as PNG grayscale images.
///
/// Prusa SL1/SL1S ZIP archive 的运行期 spec。archive layout 负责 SL1 config/entry
/// 命名，图层像素以 PNG grayscale image 编码。
pub static SL1_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "sl1",
        aliases: &["sl1", "sl1s"],
        extensions: &["sl1", "sl1s"],
    },
    family: FormatFamily::Sl1,
    catalog: FormatCatalog {
        profile_id: "prusa-sl1-archive",
        display_name: "Prusa SL1 archive",
        default_extension: "sl1",
        target_names: &["sl1", "sl1s"],
        default_export: true,
    },
    container: ContainerKind::Archive,
    capabilities: FormatCapabilities::archive_import_export().with_preview(true),
    keys: FormatKeys {
        family_key: keys::FAMILY_SL1,
        profile_key: FormatKey("profile/prusa-sl1-archive"),
        container_key: keys::CONTAINER_ARCHIVE,
        header_key: keys::HEADER_SL1_ARCHIVE,
        layer_codec_key: keys::PNG_GRAYSCALE,
        pixel_model_key: keys::PIXEL_PNG_GRAYSCALE,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::ZIP_ARCHIVE,
    },
    header: &SL1_HEADER,
    layer_codec: &PNG_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &SL1_ARCHIVE,
    pipeline: &DefaultArchivePipeline,
};

/// Runtime profiles published by the SL1 family.
///
/// SL1 family 暴露给 runtime registry 的 profiles。
pub static PROFILES: &[&FormatSpec] = &[&SL1_SPEC];

/// Catalog wrapper for the generic SL1 profile.
///
/// generic SL1 profile 的 catalog wrapper。
pub static SL1_PROFILE: CatalogProfile = CatalogProfile::Generic(&SL1_SPEC);

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// 导出到 WASM/TypeScript format catalog 的 SL1 profiles。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[&SL1_PROFILE];

static RESOLVER: HintScopedProfileResolver = HintScopedProfileResolver;

/// Family-level registry entry for Prusa SL1/SL1S archives.
///
/// Prusa SL1/SL1S archive 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "sl1",
        aliases: &["sl1", "sl1s", "prusa"],
        extensions: &["sl1", "sl1s"],
    },
    family: FormatFamily::Sl1,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
