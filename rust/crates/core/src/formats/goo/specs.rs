use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, ContainerKind, DefaultBinaryPipeline, FamilyIdentity, FormatCapabilities,
        FormatCatalog, FormatFamily, FormatFamilySpec, FormatKey, FormatKeys, FormatSpec,
        HintScopedProfileResolver, NoArchiveLayout, NoCrypto,
    },
    FormatIdentity,
};

use super::adapters::{GooFamilyFooterPolicy, GooFamilyHeaderLayout, GooFamilyLayerCodec};

static GOO_HEADER: GooFamilyHeaderLayout = GooFamilyHeaderLayout;
static GOO_LAYER_CODEC: GooFamilyLayerCodec = GooFamilyLayerCodec {
    use_color_difference_compression: true,
};
static GOO_FOOTER: GooFamilyFooterPolicy = GooFamilyFooterPolicy;

/// Runtime spec for GOO v3 binary files.
///
/// GOO uses the GOO-family header/footer and enables color-difference RLE
/// compression, unlike PRZ which shares the container shape but disables that
/// compression mode.
///
/// GOO v3 binary file 的运行期 spec。GOO 使用 GOO-family header/footer，并启用
/// color-difference RLE compression；PRZ 虽共享 container 形态，但关闭该压缩模式。
pub static GOO_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "goo",
        aliases: &["goo"],
        extensions: &["goo"],
    },
    family: FormatFamily::Goo,
    catalog: FormatCatalog {
        profile_id: "goo-v3",
        display_name: "GOO v3",
        default_extension: "goo",
        target_names: &["goo"],
        default_export: true,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities::binary_import_export().with_preview(true),
    keys: FormatKeys {
        family_key: keys::FAMILY_GOO,
        profile_key: FormatKey("profile/goo-v3"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_GOO_FAMILY,
        layer_codec_key: keys::GOO_RLE,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::GOO_FAMILY_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &GOO_HEADER,
    layer_codec: &GOO_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &GOO_FOOTER,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

/// Runtime profiles published by the GOO family.
///
/// GOO family 暴露给 runtime registry 的 profiles。
pub static PROFILES: &[&FormatSpec] = &[&GOO_SPEC];

/// Catalog wrapper for the generic GOO profile.
///
/// generic GOO profile 的 catalog wrapper。
pub static GOO_PROFILE: CatalogProfile = CatalogProfile::Generic(&GOO_SPEC);

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// 导出到 WASM/TypeScript format catalog 的 GOO profiles。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[&GOO_PROFILE];

static RESOLVER: HintScopedProfileResolver = HintScopedProfileResolver;

/// Family-level registry entry for GOO.
///
/// GOO 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "goo",
        aliases: &["goo"],
        extensions: &["goo"],
    },
    family: FormatFamily::Goo,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
