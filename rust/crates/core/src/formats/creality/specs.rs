use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, ContainerKind, DefaultBinaryPipeline, FamilyIdentity, FamilyProfile,
        FormatCapabilities, FormatCatalog, FormatFamily, FormatFamilySpec, FormatKey, FormatKeys,
        FormatSpec, HintScopedProfileResolver, NoArchiveLayout, NoCrypto, NoFooter,
    },
    FormatIdentity,
};

use super::{
    cxdlp::{
        CrealityCxdLpLayerCodec, CrealityCxdLpLegacyHeaderLayout, CrealityCxdLpLegacyLayerCodec,
        CrealityCxdLpV1HeaderLayout, CrealityCxdLpV4HeaderLayout, CrealityCxdLpV4Pipeline,
    },
    specifics::CrealityCxdLpSpec,
};

static CXDLP_V1_SPECIFICS: CrealityCxdLpSpec = CrealityCxdLpSpec {
    variant: "cxdlp-v1",
    version: 1,
    extension: "v1.cxdlp",
    magic: "CXSW3DV2",
    checksum: "none",
    layer_definition_size: 8,
    layer_parameter_size: 8,
    preview_count: 3,
    supports_layer_encryption: false,
};

static CXDLP_V3_SPECIFICS: CrealityCxdLpSpec = CrealityCxdLpSpec {
    variant: "cxdlp",
    version: 3,
    extension: "cxdlp",
    magic: "CXSW3DV2",
    checksum: "crc32",
    layer_definition_size: 8,
    layer_parameter_size: 8,
    preview_count: 3,
    supports_layer_encryption: false,
};

static CXDLP_V2_SPECIFICS: CrealityCxdLpSpec = CrealityCxdLpSpec {
    variant: "cxdlp-v2",
    version: 2,
    extension: "cxdlp",
    magic: "CXSW3DV2",
    checksum: "xor8",
    layer_definition_size: 8,
    layer_parameter_size: 8,
    preview_count: 3,
    supports_layer_encryption: false,
};

static CXDLP_V4_SPECIFICS: CrealityCxdLpSpec = CrealityCxdLpSpec {
    variant: "cxdlpv4",
    version: 4,
    extension: "cxdlpv4",
    magic: "CXSW3DV2",
    checksum: "crc32",
    layer_definition_size: 40,
    layer_parameter_size: 44,
    preview_count: 2,
    supports_layer_encryption: true,
};

static CXDLP_V3_HEADER: CrealityCxdLpLegacyHeaderLayout = CrealityCxdLpLegacyHeaderLayout {
    specifics: &CXDLP_V3_SPECIFICS,
};
static CXDLP_V3_LAYER_CODEC: CrealityCxdLpLegacyLayerCodec = CrealityCxdLpLegacyLayerCodec {
    specifics: &CXDLP_V3_SPECIFICS,
};
static CXDLP_V1_HEADER: CrealityCxdLpV1HeaderLayout = CrealityCxdLpV1HeaderLayout {
    specifics: &CXDLP_V1_SPECIFICS,
};
static CXDLP_V1_LAYER_CODEC: CrealityCxdLpLegacyLayerCodec = CrealityCxdLpLegacyLayerCodec {
    specifics: &CXDLP_V1_SPECIFICS,
};
static CXDLP_V2_HEADER: CrealityCxdLpLegacyHeaderLayout = CrealityCxdLpLegacyHeaderLayout {
    specifics: &CXDLP_V2_SPECIFICS,
};
static CXDLP_V2_LAYER_CODEC: CrealityCxdLpLegacyLayerCodec = CrealityCxdLpLegacyLayerCodec {
    specifics: &CXDLP_V2_SPECIFICS,
};
static CXDLP_V4_HEADER: CrealityCxdLpV4HeaderLayout = CrealityCxdLpV4HeaderLayout {
    specifics: &CXDLP_V4_SPECIFICS,
};
static CXDLP_V4_LAYER_CODEC: CrealityCxdLpLayerCodec = CrealityCxdLpLayerCodec {
    specifics: &CXDLP_V4_SPECIFICS,
};
static CXDLP_V4_PIPELINE: CrealityCxdLpV4Pipeline = CrealityCxdLpV4Pipeline;

/// Runtime spec for legacy Creality CXDLP v3 files.
///
/// This profile is import-only because the current writer targets the newer v4
/// document layout and checksum model.
///
/// legacy Creality CXDLP v3 文件的运行期 spec。该 profile 当前仅 import，因为 writer
/// 面向更新的 v4 document layout 与 checksum model。
pub static CXDLP_V3_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "cxdlp",
        aliases: &["cxdlp", "creality-cxdlp"],
        extensions: &["cxdlp"],
    },
    family: FormatFamily::Creality,
    catalog: FormatCatalog {
        profile_id: "creality-cxdlp-v3-read-only",
        display_name: "Creality CXDLP v3 read-only profile",
        default_extension: "cxdlp",
        target_names: &["cxdlp", "creality-cxdlp"],
        default_export: false,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities {
        can_import: true,
        can_export: false,
        is_archive: false,
        supports_preview: true,
        supports_anti_alias: true,
        supports_per_layer_settings: false,
        supports_encryption: false,
        requires_full_document_parse: true,
    },
    keys: FormatKeys {
        family_key: keys::FAMILY_CREALITY,
        profile_key: FormatKey("profile/creality-cxdlp-v3-read-only"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_CREALITY_CXDLP,
        layer_codec_key: keys::CREALITY_CXDLP_LINES,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &CXDLP_V3_HEADER,
    layer_codec: &CXDLP_V3_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

/// Runtime spec for legacy Creality CXDLP v1 files.
///
/// This profile is import-only. v1 uses the same line layer codec as newer
/// legacy CXDLP files but has its own shorter document header without a
/// version field.
///
/// legacy Creality CXDLP v1 文件的运行期 spec。该 profile 当前仅 import。v1 复用
/// 较新的 legacy CXDLP line layer codec，但 document header 更短且没有 version 字段。
pub static CXDLP_V1_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "cxdlp-v1",
        aliases: &["cxdlp-v1", "creality-cxdlp-v1", "v1.cxdlp"],
        extensions: &["v1.cxdlp"],
    },
    family: FormatFamily::Creality,
    catalog: FormatCatalog {
        profile_id: "creality-cxdlp-v1-read-only",
        display_name: "Creality CXDLP v1 read-only profile",
        default_extension: "v1.cxdlp",
        target_names: &["cxdlp-v1", "creality-cxdlp-v1", "v1.cxdlp"],
        default_export: false,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities {
        can_import: true,
        can_export: false,
        is_archive: false,
        supports_preview: true,
        supports_anti_alias: true,
        supports_per_layer_settings: false,
        supports_encryption: false,
        requires_full_document_parse: true,
    },
    keys: FormatKeys {
        family_key: keys::FAMILY_CREALITY,
        profile_key: FormatKey("profile/creality-cxdlp-v1-read-only"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_CREALITY_CXDLP,
        layer_codec_key: keys::CREALITY_CXDLP_LINES,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &CXDLP_V1_HEADER,
    layer_codec: &CXDLP_V1_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

/// Runtime spec for legacy Creality CXDLP v2 files.
///
/// This profile is import-only; its document layout matches the legacy line
/// codec while checksum handling differs from v3.
///
/// legacy Creality CXDLP v2 文件的运行期 spec。该 profile 当前仅 import；其 document
/// layout 复用 legacy line codec，但 checksum 处理与 v3 不同。
pub static CXDLP_V2_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "cxdlp-v2",
        aliases: &["cxdlp-v2", "creality-cxdlp-v2"],
        extensions: &["cxdlp"],
    },
    family: FormatFamily::Creality,
    catalog: FormatCatalog {
        profile_id: "creality-cxdlp-v2-read-only",
        display_name: "Creality CXDLP v2 read-only profile",
        default_extension: "cxdlp",
        target_names: &["cxdlp-v2", "creality-cxdlp-v2"],
        default_export: false,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities {
        can_import: true,
        can_export: false,
        is_archive: false,
        supports_preview: true,
        supports_anti_alias: true,
        supports_per_layer_settings: false,
        supports_encryption: false,
        requires_full_document_parse: true,
    },
    keys: FormatKeys {
        family_key: keys::FAMILY_CREALITY,
        profile_key: FormatKey("profile/creality-cxdlp-v2-read-only"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_CREALITY_CXDLP,
        layer_codec_key: keys::CREALITY_CXDLP_LINES,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &CXDLP_V2_HEADER,
    layer_codec: &CXDLP_V2_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

/// Runtime spec for Creality CXDLP v4 files.
///
/// CXDLP v4 has a custom pipeline because document header/layer definition
/// assembly depends on encoded layer sizes and final data addresses.
///
/// Creality CXDLP v4 文件的运行期 spec。CXDLP v4 使用自定义 pipeline，因为 document
/// header/layer definition 组装依赖已编码图层大小与最终 data address。
pub static CXDLP_V4_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "cxdlpv4",
        aliases: &["cxdlpv4", "creality-cxdlpv4"],
        extensions: &["cxdlpv4"],
    },
    family: FormatFamily::Creality,
    catalog: FormatCatalog {
        profile_id: "creality-cxdlpv4",
        display_name: "Creality CXDLPv4 profile",
        default_extension: "cxdlpv4",
        target_names: &["cxdlpv4", "creality-cxdlpv4"],
        default_export: false,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities {
        can_import: true,
        can_export: true,
        is_archive: false,
        supports_preview: true,
        supports_anti_alias: true,
        supports_per_layer_settings: true,
        supports_encryption: true,
        requires_full_document_parse: true,
    },
    keys: FormatKeys {
        family_key: keys::FAMILY_CREALITY,
        profile_key: FormatKey("profile/creality-cxdlpv4"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::HEADER_CREALITY_CXDLP_V4,
        layer_codec_key: keys::CREALITY_CXDLP_RLE,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &CXDLP_V4_HEADER,
    layer_codec: &CXDLP_V4_LAYER_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &NoArchiveLayout,
    pipeline: &CXDLP_V4_PIPELINE,
};

/// Catalog/runtime pairing for the CXDLP v3 profile.
///
/// CXDLP v3 profile 的 catalog/runtime 配对。
pub static CXDLP_V3_PROFILE: FamilyProfile<CrealityCxdLpSpec> = FamilyProfile {
    spec: &CXDLP_V3_SPEC,
    facts: &CXDLP_V3_SPECIFICS,
};

/// Catalog/runtime pairing for the CXDLP v1 profile.
///
/// CXDLP v1 profile 的 catalog/runtime 配对。
pub static CXDLP_V1_PROFILE: FamilyProfile<CrealityCxdLpSpec> = FamilyProfile {
    spec: &CXDLP_V1_SPEC,
    facts: &CXDLP_V1_SPECIFICS,
};

/// Catalog/runtime pairing for the CXDLP v2 profile.
///
/// CXDLP v2 profile 的 catalog/runtime 配对。
pub static CXDLP_V2_PROFILE: FamilyProfile<CrealityCxdLpSpec> = FamilyProfile {
    spec: &CXDLP_V2_SPEC,
    facts: &CXDLP_V2_SPECIFICS,
};

/// Catalog/runtime pairing for the CXDLP v4 profile.
///
/// CXDLP v4 profile 的 catalog/runtime 配对。
pub static CXDLP_V4_PROFILE: FamilyProfile<CrealityCxdLpSpec> = FamilyProfile {
    spec: &CXDLP_V4_SPEC,
    facts: &CXDLP_V4_SPECIFICS,
};

/// Catalog wrapper for the CXDLP v3 profile.
///
/// CXDLP v3 profile 的 catalog wrapper。
pub static CREALITY_CXDLP_V3_CATALOG_PROFILE: CatalogProfile =
    CatalogProfile::Creality(&CXDLP_V3_PROFILE);

/// Catalog wrapper for the CXDLP v1 profile.
///
/// CXDLP v1 profile 的 catalog wrapper。
pub static CREALITY_CXDLP_V1_CATALOG_PROFILE: CatalogProfile =
    CatalogProfile::Creality(&CXDLP_V1_PROFILE);

/// Catalog wrapper for the CXDLP v2 profile.
///
/// CXDLP v2 profile 的 catalog wrapper。
pub static CREALITY_CXDLP_V2_CATALOG_PROFILE: CatalogProfile =
    CatalogProfile::Creality(&CXDLP_V2_PROFILE);

/// Catalog wrapper for the CXDLP v4 profile.
///
/// CXDLP v4 profile 的 catalog wrapper。
pub static CREALITY_CXDLP_V4_CATALOG_PROFILE: CatalogProfile =
    CatalogProfile::Creality(&CXDLP_V4_PROFILE);

/// Typed Creality profiles used by family-specific catalog generation.
///
/// family-specific catalog 生成使用的 typed Creality profiles。
pub static CREALITY_PROFILES: &[&FamilyProfile<CrealityCxdLpSpec>] = &[
    &CXDLP_V3_PROFILE,
    &CXDLP_V2_PROFILE,
    &CXDLP_V4_PROFILE,
    &CXDLP_V1_PROFILE,
];

/// Runtime profiles published by the Creality family.
///
/// Creality family 暴露给 runtime registry 的 profiles。
pub static PROFILES: &[&FormatSpec] = &[
    &CXDLP_V3_SPEC,
    &CXDLP_V2_SPEC,
    &CXDLP_V4_SPEC,
    &CXDLP_V1_SPEC,
];

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// 导出到 WASM/TypeScript format catalog 的 Creality profiles。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[
    &CREALITY_CXDLP_V3_CATALOG_PROFILE,
    &CREALITY_CXDLP_V2_CATALOG_PROFILE,
    &CREALITY_CXDLP_V4_CATALOG_PROFILE,
    &CREALITY_CXDLP_V1_CATALOG_PROFILE,
];

static RESOLVER: HintScopedProfileResolver = HintScopedProfileResolver;

/// Family-level registry entry for Creality CXDLP variants.
///
/// Creality CXDLP variants 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "creality",
        aliases: &["creality", "halot", "cxdlp", "v1.cxdlp", "cxdlpv4"],
        extensions: &["cxdlp", "v1.cxdlp", "cxdlpv4"],
    },
    family: FormatFamily::Creality,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
