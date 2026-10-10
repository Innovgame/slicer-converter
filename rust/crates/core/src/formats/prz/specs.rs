use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, ContainerKind, DefaultBinaryPipeline, FamilyIdentity, FormatCapabilities,
        FormatCatalog, FormatFamily, FormatFamilySpec, FormatKey, FormatKeys, FormatSpec,
        HintScopedProfileResolver, NoArchiveLayout, NoCrypto,
    },
    FormatIdentity,
};

use super::super::goo::adapters::{
    GooFamilyFooterPolicy, GooFamilyHeaderLayout, GooFamilyLayerCodec,
};

static PRZ_HEADER: GooFamilyHeaderLayout = GooFamilyHeaderLayout;
static PRZ_LAYER_CODEC: GooFamilyLayerCodec = GooFamilyLayerCodec {
    use_color_difference_compression: false,
};
static PRZ_FOOTER: GooFamilyFooterPolicy = GooFamilyFooterPolicy;

/// Runtime spec for Phrozen PRZ v3 binary files.
///
/// PRZ reuses the GOO-family header/footer and RLE codec shape, but keeps its
/// own family/profile identity so detection and catalog output stay explicit.
///
/// Phrozen PRZ v3 binary file 的运行期 spec。PRZ 复用 GOO-family header/footer 与
/// RLE codec 形态，但保留独立 family/profile identity，避免 detection 与 catalog 输出含糊。
pub static PRZ_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "prz",
        aliases: &["prz"],
        extensions: &["prz"],
    },
    family: FormatFamily::Phrozen,
    catalog: FormatCatalog {
        profile_id: "phrozen-prz-v3",
        display_name: "Phrozen PRZ v3",
        default_extension: "prz",
        target_names: &["prz"],
        default_export: true,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities::binary_import_export().with_preview(true),
    keys: FormatKeys {
        family_key: keys::FAMILY_PHROZEN,
        profile_key: FormatKey("profile/phrozen-prz-v3"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_GOO_FAMILY,
        layer_codec_key: keys::GOO_RLE,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::GOO_FAMILY_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &PRZ_HEADER,
    layer_codec: &PRZ_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &PRZ_FOOTER,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

/// Runtime profiles published by the Phrozen PRZ family.
///
/// Phrozen PRZ family 暴露给 runtime registry 的 profiles。
pub static PROFILES: &[&FormatSpec] = &[&PRZ_SPEC];

/// Catalog wrapper for the generic PRZ profile.
///
/// generic PRZ profile 的 catalog wrapper。
pub static PRZ_PROFILE: CatalogProfile = CatalogProfile::Generic(&PRZ_SPEC);

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// 导出到 WASM/TypeScript format catalog 的 PRZ profiles。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[&PRZ_PROFILE];

static RESOLVER: HintScopedProfileResolver = HintScopedProfileResolver;

/// Family-level registry entry for Phrozen PRZ.
///
/// Phrozen PRZ 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "phrozen",
        aliases: &["phrozen", "prz"],
        extensions: &["prz"],
    },
    family: FormatFamily::Phrozen,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
