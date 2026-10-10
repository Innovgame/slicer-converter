//! Chitubox-family profile declarations.
//!
//! This family contains several related but incompatible profile groups: CTB
//! v1-v5, GKtwo CTB variants, legacy bitplane formats, encrypted CTB, and ZIP
//! archive catalogs. The declarations keep resolver facts beside runtime
//! wiring so magic/version detection, export target resolution, and WASM catalog
//! facts remain aligned.
//!
//! Chitubox-family profile declarations。本 family 包含多组相关但不兼容的 profile：
//! CTB v1-v5、GKtwo CTB variants、legacy bitplane formats、encrypted CTB 与 ZIP
//! archive catalogs。声明把 resolver facts 与 runtime wiring 放在一起，确保 magic/version
//! detection、export target resolution 与 WASM catalog facts 保持一致。

use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, FamilyIdentity, FormatCapabilities, FormatFamily, FormatFamilySpec, FormatSpec,
        NoCrypto,
    },
    ctb,
    sl1::adapters::PngLayerCodec,
};

use super::{
    adapters::{
        CbddlpBitplaneLayerCodec, ChituboxHeaderLayout, ChituboxXorCryptoPolicy, CtbRleLayerCodec,
    },
    archive::{ChituboxZipArchiveLayout, ChituboxZipHeaderLayout},
    encrypted_ctb::{EncryptedCtbHeaderLayout, EncryptedCtbLayerCodec, EncryptedCtbPipeline},
    fdg::{FdgHeaderLayout, FdgLayerCodec},
    photons::{PhotonSHeaderLayout, PhotonSLayerCodec},
    phz::{PhzHeaderLayout, PhzLayerCodec},
    profile_kit::{
        define_chitubox_archive_profile, define_chitubox_ctb_profile,
        define_chitubox_ctb_profile_parts, define_chitubox_legacy_bitplane_profile,
        define_chitubox_legacy_bitplane_profile_parts, define_chitubox_profiles,
        define_chitubox_read_only_profiles,
    },
    resolver::ChituboxResolver,
    specifics::{ChituboxProfileKind, ChituboxSpec},
};

const CTB_V2_MAGICS: &[u32] = &[ctb::MAGIC_CTB];
const CTB_V4_MAGICS: &[u32] = &[ctb::MAGIC_CTB_V4, ctb::MAGIC_CTB];
const GKTWO_MAGICS: &[u32] = &[ctb::MAGIC_CTB_V4_GKTWO];
const CBDDLP_MAGICS: &[u32] = &[ctb::MAGIC_CBDDLP];
const PHOTON_MAGICS: &[u32] = CBDDLP_MAGICS;

define_chitubox_ctb_profile_parts!(
    header = CTB_V1_HEADER,
    facts = CTB_V1_SPECIFICS,
    kind = ChituboxProfileKind::Ctb,
    ctb_version = 1,
    machine_profile = None,
    allowed_magics = CTB_V2_MAGICS,
    allowed_versions = [1],
    build_magic = ctb::MAGIC_CTB,
    build_version = 1,
    layer_table_entry_size = 36,
);

define_chitubox_ctb_profile_parts!(
    header = CTB_V2_HEADER,
    facts = CTB_V2_SPECIFICS,
    kind = ChituboxProfileKind::Ctb,
    ctb_version = 2,
    machine_profile = None,
    allowed_magics = CTB_V2_MAGICS,
    allowed_versions = [2],
    build_magic = ctb::MAGIC_CTB,
    build_version = 2,
    layer_table_entry_size = 36,
);

define_chitubox_ctb_profile_parts!(
    header = CTB_V3_HEADER,
    facts = CTB_V3_SPECIFICS,
    kind = ChituboxProfileKind::Ctb,
    ctb_version = 3,
    machine_profile = None,
    allowed_magics = CTB_V2_MAGICS,
    allowed_versions = [3],
    build_magic = ctb::MAGIC_CTB,
    build_version = 3,
    layer_table_entry_size = 84,
);

define_chitubox_ctb_profile_parts!(
    header = CTB_V4_HEADER,
    facts = CTB_V4_SPECIFICS,
    kind = ChituboxProfileKind::Ctb,
    ctb_version = 4,
    machine_profile = None,
    allowed_magics = CTB_V4_MAGICS,
    allowed_versions = [4],
    build_magic = ctb::MAGIC_CTB_V4,
    build_version = 4,
    layer_table_entry_size = 84,
);

define_chitubox_ctb_profile_parts!(
    header = CTB_V5_HEADER,
    facts = CTB_V5_SPECIFICS,
    kind = ChituboxProfileKind::Ctb,
    ctb_version = 5,
    machine_profile = None,
    allowed_magics = CTB_V4_MAGICS,
    allowed_versions = [5],
    build_magic = ctb::MAGIC_CTB_V4,
    build_version = 5,
    layer_table_entry_size = 84,
);

define_chitubox_legacy_bitplane_profile_parts!(
    header = CBDDLP_V1_HEADER,
    facts = CBDDLP_V1_SPECIFICS,
    kind = ChituboxProfileKind::Cbddlp,
    ctb_version = 1,
    machine_profile = None,
    allowed_magics = CBDDLP_MAGICS,
    allowed_versions = [1],
    build_version = 1,
);

define_chitubox_legacy_bitplane_profile_parts!(
    header = CBDDLP_HEADER,
    facts = CBDDLP_SPECIFICS,
    kind = ChituboxProfileKind::Cbddlp,
    ctb_version = 2,
    machine_profile = None,
    allowed_magics = CBDDLP_MAGICS,
    allowed_versions = [2],
    build_version = 2,
);

define_chitubox_ctb_profile_parts!(
    header = GKTWO_V1_HEADER,
    facts = GKTWO_CTB_V1_SPECIFICS,
    kind = ChituboxProfileKind::GktwoCtb,
    ctb_version = 1,
    machine_profile = Some("gktwo"),
    allowed_magics = GKTWO_MAGICS,
    allowed_versions = [1],
    build_magic = ctb::MAGIC_CTB_V4_GKTWO,
    build_version = 1,
    layer_table_entry_size = 36,
);

define_chitubox_ctb_profile_parts!(
    header = GKTWO_V2_HEADER,
    facts = GKTWO_CTB_V2_SPECIFICS,
    kind = ChituboxProfileKind::GktwoCtb,
    ctb_version = 2,
    machine_profile = Some("gktwo"),
    allowed_magics = GKTWO_MAGICS,
    allowed_versions = [2],
    build_magic = ctb::MAGIC_CTB_V4_GKTWO,
    build_version = 2,
    layer_table_entry_size = 36,
);

define_chitubox_ctb_profile_parts!(
    header = GKTWO_V3_HEADER,
    facts = GKTWO_CTB_V3_SPECIFICS,
    kind = ChituboxProfileKind::GktwoCtb,
    ctb_version = 3,
    machine_profile = Some("gktwo"),
    allowed_magics = GKTWO_MAGICS,
    allowed_versions = [3],
    build_magic = ctb::MAGIC_CTB_V4_GKTWO,
    build_version = 3,
    layer_table_entry_size = 84,
);

define_chitubox_ctb_profile_parts!(
    header = GKTWO_V4_HEADER,
    facts = GKTWO_CTB_V4_SPECIFICS,
    kind = ChituboxProfileKind::GktwoCtb,
    ctb_version = 4,
    machine_profile = Some("gktwo"),
    allowed_magics = GKTWO_MAGICS,
    allowed_versions = [4],
    build_magic = ctb::MAGIC_CTB_V4_GKTWO,
    build_version = 4,
    layer_table_entry_size = 84,
);

define_chitubox_ctb_profile_parts!(
    header = GKTWO_V5_HEADER,
    facts = GKTWO_CTB_V5_SPECIFICS,
    kind = ChituboxProfileKind::GktwoCtb,
    ctb_version = 5,
    machine_profile = Some("gktwo"),
    allowed_magics = GKTWO_MAGICS,
    allowed_versions = [5],
    build_magic = ctb::MAGIC_CTB_V4_GKTWO,
    build_version = 5,
    layer_table_entry_size = 84,
);

define_chitubox_legacy_bitplane_profile_parts!(
    header = PHOTON_HEADER,
    facts = PHOTON_SPECIFICS,
    kind = ChituboxProfileKind::Photon,
    ctb_version = 2,
    machine_profile = Some("photon"),
    allowed_magics = PHOTON_MAGICS,
    allowed_versions = [1, 2],
    build_version = 2,
);

static PHZ_HEADER: PhzHeaderLayout = PhzHeaderLayout;

static FDG_HEADER: FdgHeaderLayout = FdgHeaderLayout;

static PHOTONS_HEADER: PhotonSHeaderLayout = PhotonSHeaderLayout;

static CTB_RLE_CODEC: CtbRleLayerCodec = CtbRleLayerCodec;
static CBDDLP_BITPLANE_CODEC: CbddlpBitplaneLayerCodec = CbddlpBitplaneLayerCodec;
static PHZ_LAYER_CODEC: PhzLayerCodec = PhzLayerCodec;
static FDG_LAYER_CODEC: FdgLayerCodec = FdgLayerCodec;
static PHOTONS_LAYER_CODEC: PhotonSLayerCodec = PhotonSLayerCodec;
static CTB_ENCRYPTED_V4_HEADER: EncryptedCtbHeaderLayout =
    EncryptedCtbHeaderLayout { version: Some(4) };
static CTB_ENCRYPTED_V5_HEADER: EncryptedCtbHeaderLayout =
    EncryptedCtbHeaderLayout { version: Some(5) };
static CTB_ENCRYPTED_LAYER_CODEC: EncryptedCtbLayerCodec = EncryptedCtbLayerCodec;
static CTB_ENCRYPTED_PIPELINE: EncryptedCtbPipeline = EncryptedCtbPipeline;
static CHITUBOX_XOR_CRYPTO: ChituboxXorCryptoPolicy = ChituboxXorCryptoPolicy;
static CHITUBOX_ZIP_HEADER: ChituboxZipHeaderLayout = ChituboxZipHeaderLayout;
static CHITUBOX_ZIP_ARCHIVE: ChituboxZipArchiveLayout = ChituboxZipArchiveLayout;
static CHITUBOX_ZIP_PNG_CODEC: PngLayerCodec = PngLayerCodec;

static PHZ_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::Phz,
    ctb_version: Some(2),
    machine_profile: None,
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};
static FDG_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::Fdg,
    ctb_version: Some(2),
    machine_profile: Some("voxelab"),
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};
static PHOTONS_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::PhotonS,
    ctb_version: None,
    machine_profile: Some("photon-s"),
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};
static CTB_ENCRYPTED_V4_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::EncryptedCtb,
    ctb_version: Some(4),
    machine_profile: None,
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};
static CTB_ENCRYPTED_V5_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::EncryptedCtb,
    ctb_version: Some(5),
    machine_profile: None,
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};
static CHITUBOX_ZIP_SPECIFICS: ChituboxSpec = ChituboxSpec {
    kind: ChituboxProfileKind::Zip,
    ctb_version: None,
    machine_profile: None,
    allowed_magics: &[],
    allowed_versions: &[],
    build_version: None,
    layer_table_entry_size: None,
};

define_chitubox_ctb_profile!(
    spec = CTB_V1_SPEC,
    profile = CTB_V1_PROFILE,
    catalog_profile = CTB_V1_CATALOG_PROFILE,
    identity = {
        name = "ctb-v1",
        aliases = ["ctb-v1", "v1.ctb"],
        extensions = ["v1.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-v1",
        display_name = "Chitubox CTB v1",
        default_extension = "ctb",
        target_names = ["ctb-v1", "v1.ctb"],
        default_export = false,
    },
    facts = &CTB_V1_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V1,
        header = &CTB_V1_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = CTB_V2_SPEC,
    profile = CTB_V2_PROFILE,
    catalog_profile = CTB_V2_CATALOG_PROFILE,
    identity = {
        name = "ctb",
        aliases = ["ctb", "ctb-v2", "v2.ctb"],
        extensions = ["ctb", "v2.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-v2",
        display_name = "Chitubox CTB v2",
        target_names = ["ctb", "ctb-v2", "v2.ctb"],
        default_export = true,
    },
    facts = &CTB_V2_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V2,
        header = &CTB_V2_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = CTB_V3_SPEC,
    profile = CTB_V3_PROFILE,
    catalog_profile = CTB_V3_CATALOG_PROFILE,
    identity = {
        name = "ctb-v3",
        aliases = ["ctb-v3", "v3.ctb"],
        extensions = ["ctb", "v3.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-v3",
        display_name = "Chitubox CTB v3",
        target_names = ["ctb-v3", "v3.ctb"],
        default_export = false,
    },
    facts = &CTB_V3_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true)
            .with_per_layer_settings(true),
        header_key = keys::HEADER_CHITU_CTB_V3,
        header = &CTB_V3_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = CTB_V4_SPEC,
    profile = CTB_V4_PROFILE,
    catalog_profile = CTB_V4_CATALOG_PROFILE,
    identity = {
        name = "ctb-v4",
        aliases = ["ctb-v4", "v4.ctb"],
        extensions = ["ctb", "v4.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-v4",
        display_name = "Chitubox CTB v4",
        target_names = ["ctb-v4", "v4.ctb"],
        default_export = false,
    },
    facts = &CTB_V4_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V4,
        header = &CTB_V4_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = CTB_V5_SPEC,
    profile = CTB_V5_PROFILE,
    catalog_profile = CTB_V5_CATALOG_PROFILE,
    identity = {
        name = "ctb-v5",
        aliases = ["ctb-v5", "v5.ctb"],
        extensions = ["ctb", "v5.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-v5",
        display_name = "Chitubox CTB v5",
        target_names = ["ctb-v5", "v5.ctb"],
        default_export = false,
    },
    facts = &CTB_V5_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true)
            .with_per_layer_settings(true),
        header_key = keys::HEADER_CHITU_CTB_V5,
        header = &CTB_V5_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = GKTWO_CTB_V1_SPEC,
    profile = GKTWO_CTB_V1_PROFILE,
    catalog_profile = GKTWO_CTB_V1_CATALOG_PROFILE,
    identity = {
        name = "gktwo-ctb-v1",
        aliases = ["gktwo-ctb-v1", "gktwo-v1.ctb"],
        extensions = ["gktwo-v1.ctb"],
    },
    catalog = {
        profile_id = "chitubox-gktwo-ctb-v1",
        display_name = "GKtwo CTB v1",
        default_extension = "gktwo.ctb",
        target_names = ["gktwo-ctb-v1", "gktwo-v1.ctb"],
        default_export = false,
    },
    facts = &GKTWO_CTB_V1_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V1,
        header = &GKTWO_V1_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = GKTWO_CTB_V2_SPEC,
    profile = GKTWO_CTB_V2_PROFILE,
    catalog_profile = GKTWO_CTB_V2_CATALOG_PROFILE,
    identity = {
        name = "gktwo-ctb-v2",
        aliases = ["gktwo-ctb-v2", "gktwo-v2.ctb"],
        extensions = ["gktwo-v2.ctb"],
    },
    catalog = {
        profile_id = "chitubox-gktwo-ctb-v2",
        display_name = "GKtwo CTB v2",
        default_extension = "gktwo.ctb",
        target_names = ["gktwo-ctb-v2", "gktwo-v2.ctb"],
        default_export = false,
    },
    facts = &GKTWO_CTB_V2_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V2,
        header = &GKTWO_V2_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = GKTWO_CTB_V3_SPEC,
    profile = GKTWO_CTB_V3_PROFILE,
    catalog_profile = GKTWO_CTB_V3_CATALOG_PROFILE,
    identity = {
        name = "gktwo-ctb-v3",
        aliases = ["gktwo-ctb-v3", "gktwo-v3.ctb"],
        extensions = ["gktwo-v3.ctb"],
    },
    catalog = {
        profile_id = "chitubox-gktwo-ctb-v3",
        display_name = "GKtwo CTB v3",
        default_extension = "gktwo.ctb",
        target_names = ["gktwo-ctb-v3", "gktwo-v3.ctb"],
        default_export = false,
    },
    facts = &GKTWO_CTB_V3_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true)
            .with_per_layer_settings(true),
        header_key = keys::HEADER_CHITU_CTB_V3,
        header = &GKTWO_V3_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = GKTWO_CTB_SPEC,
    profile = GKTWO_CTB_PROFILE,
    catalog_profile = GKTWO_CTB_CATALOG_PROFILE,
    identity = {
        name = "gktwo-ctb",
        aliases = ["gktwo-ctb", "gktwo.ctb"],
        extensions = ["ctb", "gktwo.ctb"],
    },
    catalog = {
        profile_id = "chitubox-gktwo-ctb-v4",
        display_name = "GKtwo CTB v4",
        default_extension = "gktwo.ctb",
        target_names = ["gktwo-ctb", "gktwo.ctb"],
        default_export = false,
    },
    facts = &GKTWO_CTB_V4_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true),
        header_key = keys::HEADER_CHITU_CTB_V4,
        header = &GKTWO_V4_HEADER,
    },
);

define_chitubox_ctb_profile!(
    spec = GKTWO_CTB_V5_SPEC,
    profile = GKTWO_CTB_V5_PROFILE,
    catalog_profile = GKTWO_CTB_V5_CATALOG_PROFILE,
    identity = {
        name = "gktwo-ctb-v5",
        aliases = ["gktwo-ctb-v5", "gktwo-v5.ctb"],
        extensions = ["gktwo-v5.ctb"],
    },
    catalog = {
        profile_id = "chitubox-gktwo-ctb-v5",
        display_name = "GKtwo CTB v5",
        default_extension = "gktwo.ctb",
        target_names = ["gktwo-ctb-v5", "gktwo-v5.ctb"],
        default_export = false,
    },
    facts = &GKTWO_CTB_V5_SPECIFICS,
    runtime = ctb {
        capabilities = FormatCapabilities::binary_import_export()
            .with_preview(true)
            .with_anti_alias(true)
            .with_encryption(true)
            .with_per_layer_settings(true),
        header_key = keys::HEADER_CHITU_CTB_V5,
        header = &GKTWO_V5_HEADER,
    },
);

define_chitubox_legacy_bitplane_profile!(
    spec = CBDDLP_V1_SPEC,
    profile = CBDDLP_V1_PROFILE,
    catalog_profile = CBDDLP_V1_CATALOG_PROFILE,
    identity = {
        name = "cbddlp-v1",
        aliases = ["cbddlp-v1"],
        extensions = ["cbddlp-v1"],
    },
    catalog = {
        profile_id = "chitubox-cbddlp-v1",
        display_name = "Chitubox CBDDLP v1",
        default_extension = "cbddlp",
        target_names = ["cbddlp-v1"],
        default_export = false,
    },
    facts = &CBDDLP_V1_SPECIFICS,
    runtime = legacy_bitplane {
        header = &CBDDLP_V1_HEADER,
    },
);

define_chitubox_legacy_bitplane_profile!(
    spec = CBDDLP_SPEC,
    profile = CBDDLP_PROFILE,
    catalog_profile = CBDDLP_CATALOG_PROFILE,
    identity = {
        name = "cbddlp",
        aliases = ["cbddlp"],
        extensions = ["cbddlp"],
    },
    catalog = {
        profile_id = "chitubox-cbddlp",
        display_name = "Chitubox CBDDLP v2",
        target_names = ["cbddlp", "cbddlp-v2"],
        default_export = false,
    },
    facts = &CBDDLP_SPECIFICS,
    runtime = legacy_bitplane {
        header = &CBDDLP_HEADER,
    },
);

define_chitubox_legacy_bitplane_profile!(
    spec = PHOTON_SPEC,
    profile = PHOTON_PROFILE,
    catalog_profile = PHOTON_CATALOG_PROFILE,
    identity = {
        name = "photon",
        aliases = ["photon"],
        extensions = ["photon"],
    },
    catalog = {
        profile_id = "chitubox-photon",
        display_name = "Anycubic Photon",
        target_names = ["photon"],
        default_export = false,
    },
    facts = &PHOTON_SPECIFICS,
    runtime = legacy_bitplane {
        header = &PHOTON_HEADER,
    },
);

define_chitubox_read_only_profiles!(
    spec = PHZ_SPEC,
    profile = PHZ_PROFILE,
    catalog_profile = PHZ_CATALOG_PROFILE,
    identity = {
        name = "phz",
        aliases = ["phz"],
        extensions = ["phz"],
    },
    catalog = {
        profile_id = "chitubox-phz",
        display_name = "Chitubox PHZ",
        target_names = ["phz"],
    },
    facts = &PHZ_SPECIFICS,
    runtime = binary {
        capabilities = FormatCapabilities {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: true,
            supports_anti_alias: false,
            supports_per_layer_settings: true,
            supports_encryption: true,
            requires_full_document_parse: false,
        },
        header_key = keys::HEADER_CHITU_PHZ,
        layer_codec_key = keys::PHZ_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        crypto_key = keys::PHZ_XOR,
        header = &PHZ_HEADER,
        layer_codec = &PHZ_LAYER_CODEC,
        crypto = &NoCrypto,
    },
);

define_chitubox_read_only_profiles!(
    spec = FDG_SPEC,
    profile = FDG_PROFILE,
    catalog_profile = FDG_CATALOG_PROFILE,
    identity = {
        name = "fdg",
        aliases = ["fdg"],
        extensions = ["fdg"],
    },
    catalog = {
        profile_id = "chitubox-fdg",
        display_name = "Voxelab FDG",
        target_names = ["fdg"],
    },
    facts = &FDG_SPECIFICS,
    runtime = binary {
        capabilities = FormatCapabilities {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: true,
            supports_anti_alias: false,
            supports_per_layer_settings: true,
            supports_encryption: true,
            requires_full_document_parse: false,
        },
        header_key = keys::HEADER_CHITU_FDG,
        layer_codec_key = keys::FDG_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        crypto_key = keys::FDG_XOR,
        header = &FDG_HEADER,
        layer_codec = &FDG_LAYER_CODEC,
        crypto = &NoCrypto,
    },
);

define_chitubox_read_only_profiles!(
    spec = PHOTONS_SPEC,
    profile = PHOTONS_PROFILE,
    catalog_profile = PHOTONS_CATALOG_PROFILE,
    identity = {
        name = "photons",
        aliases = ["photons", "photon-s"],
        extensions = ["photons"],
    },
    catalog = {
        profile_id = "chitubox-photons",
        display_name = "Anycubic PhotonS",
        target_names = ["photons", "photon-s"],
    },
    facts = &PHOTONS_SPECIFICS,
    runtime = binary {
        capabilities = FormatCapabilities {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: true,
            supports_anti_alias: false,
            supports_per_layer_settings: false,
            supports_encryption: false,
            requires_full_document_parse: true,
        },
        header_key = keys::HEADER_CHITU_PHOTONS,
        layer_codec_key = keys::PHOTONS_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        crypto_key = keys::NO_CRYPTO,
        header = &PHOTONS_HEADER,
        layer_codec = &PHOTONS_LAYER_CODEC,
        crypto = &NoCrypto,
    },
);

define_chitubox_read_only_profiles!(
    spec = CTB_ENCRYPTED_V4_SPEC,
    profile = CTB_ENCRYPTED_V4_PROFILE,
    catalog_profile = CTB_ENCRYPTED_V4_CATALOG_PROFILE,
    identity = {
        name = "encrypted-ctb-v4",
        aliases = ["encrypted-ctb-v4", "encrypted-v4.ctb"],
        extensions = ["ctb", "encrypted-v4.ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-encrypted-v4",
        display_name = "Chitubox CTB (Encrypted) v4",
        default_extension = "encrypted.ctb",
        target_names = ["encrypted-ctb-v4", "encrypted-v4.ctb"],
    },
    facts = &CTB_ENCRYPTED_V4_SPECIFICS,
    runtime = binary {
        capabilities = FormatCapabilities {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: false,
            supports_anti_alias: false,
            supports_per_layer_settings: true,
            supports_encryption: true,
            requires_full_document_parse: true,
        },
        header_key = keys::HEADER_CHITU_CTB_ENCRYPTED,
        layer_codec_key = keys::CTB_ENCRYPTED_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        crypto_key = keys::CHITU_AES_BLOCK,
        header = &CTB_ENCRYPTED_V4_HEADER,
        layer_codec = &CTB_ENCRYPTED_LAYER_CODEC,
        crypto = &NoCrypto,
        pipeline = &CTB_ENCRYPTED_PIPELINE,
    },
);

define_chitubox_read_only_profiles!(
    spec = CTB_ENCRYPTED_V5_SPEC,
    profile = CTB_ENCRYPTED_V5_PROFILE,
    catalog_profile = CTB_ENCRYPTED_V5_CATALOG_PROFILE,
    identity = {
        name = "encrypted-ctb",
        aliases = ["encrypted-ctb", "encrypted-ctb-v5", "encrypted.ctb", "encrypted-v5.ctb"],
        extensions = ["encrypted.ctb", "encrypted-v5.ctb", "ctb"],
    },
    catalog = {
        profile_id = "chitubox-ctb-encrypted-v5",
        display_name = "Chitubox CTB (Encrypted) v5",
        target_names = ["encrypted-ctb", "encrypted-ctb-v5", "encrypted.ctb", "encrypted-v5.ctb"],
    },
    facts = &CTB_ENCRYPTED_V5_SPECIFICS,
    runtime = binary {
        capabilities = FormatCapabilities {
            can_import: true,
            can_export: true,
            is_archive: false,
            supports_preview: false,
            supports_anti_alias: false,
            supports_per_layer_settings: true,
            supports_encryption: true,
            requires_full_document_parse: true,
        },
        header_key = keys::HEADER_CHITU_CTB_ENCRYPTED,
        layer_codec_key = keys::CTB_ENCRYPTED_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        crypto_key = keys::CHITU_AES_BLOCK,
        header = &CTB_ENCRYPTED_V5_HEADER,
        layer_codec = &CTB_ENCRYPTED_LAYER_CODEC,
        crypto = &NoCrypto,
        pipeline = &CTB_ENCRYPTED_PIPELINE,
    },
);

define_chitubox_archive_profile!(
    spec = CHITUBOX_ZIP_SPEC,
    profile = CHITUBOX_ZIP_PROFILE,
    catalog_profile = CHITUBOX_ZIP_CATALOG_PROFILE,
    identity = {
        name = "chitubox-zip",
        aliases = ["chitubox-zip", "chituzip"],
        extensions = ["zip"],
    },
    catalog = {
        profile_id = "chitubox-zip-archive-catalog",
        display_name = "Chitubox Zip archive catalog",
        target_names = ["chitubox-zip"],
        default_export = false,
    },
    facts = &CHITUBOX_ZIP_SPECIFICS,
    runtime = archive {
        capabilities = FormatCapabilities::archive_import_export().with_preview(true),
        header_key = keys::HEADER_CHITUBOX_ZIP,
        layer_codec_key = keys::PNG_GRAYSCALE,
        pixel_model_key = keys::PIXEL_PNG_GRAYSCALE,
        archive_key = keys::ZIP_ARCHIVE,
        header = &CHITUBOX_ZIP_HEADER,
        layer_codec = &CHITUBOX_ZIP_PNG_CODEC,
        archive = &CHITUBOX_ZIP_ARCHIVE,
    },
);

/// Runtime profiles published by the Chitubox family.
///
/// The order matters for deterministic catalog output and fallback extension
/// lookup; profile detection still uses magic/version facts before extension
/// aliases when bytes are available.
///
/// Chitubox family 暴露给 runtime registry 的 profiles。顺序影响确定性的 catalog 输出与
/// fallback extension lookup；当存在文件字节时，profile detection 仍优先使用 magic/version
/// facts，再考虑 extension aliases。
pub static PROFILES: &[&FormatSpec] = &[
    &CTB_V1_SPEC,
    &CTB_V2_SPEC,
    &CTB_V3_SPEC,
    &CTB_V4_SPEC,
    &CTB_V5_SPEC,
    &GKTWO_CTB_V1_SPEC,
    &GKTWO_CTB_V2_SPEC,
    &GKTWO_CTB_V3_SPEC,
    &GKTWO_CTB_SPEC,
    &GKTWO_CTB_V5_SPEC,
    &CBDDLP_V1_SPEC,
    &CBDDLP_SPEC,
    &PHOTON_SPEC,
    &PHZ_SPEC,
    &FDG_SPEC,
    &PHOTONS_SPEC,
    &CTB_ENCRYPTED_V5_SPEC,
    &CTB_ENCRYPTED_V4_SPEC,
    &CHITUBOX_ZIP_SPEC,
];

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// These wrappers preserve Chitubox-specific facts such as profile kind,
/// allowed magics, versions, and layer table entry sizes.
///
/// 导出到 WASM/TypeScript format catalog 的 Chitubox profiles。这些 wrapper 保留
/// profile kind、allowed magics、versions 与 layer table entry sizes 等 Chitubox-specific facts。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[
    &CTB_V1_CATALOG_PROFILE,
    &CTB_V2_CATALOG_PROFILE,
    &CTB_V3_CATALOG_PROFILE,
    &CTB_V4_CATALOG_PROFILE,
    &CTB_V5_CATALOG_PROFILE,
    &GKTWO_CTB_V1_CATALOG_PROFILE,
    &GKTWO_CTB_V2_CATALOG_PROFILE,
    &GKTWO_CTB_V3_CATALOG_PROFILE,
    &GKTWO_CTB_CATALOG_PROFILE,
    &GKTWO_CTB_V5_CATALOG_PROFILE,
    &CBDDLP_V1_CATALOG_PROFILE,
    &CBDDLP_CATALOG_PROFILE,
    &PHOTON_CATALOG_PROFILE,
    &PHZ_CATALOG_PROFILE,
    &FDG_CATALOG_PROFILE,
    &PHOTONS_CATALOG_PROFILE,
    &CTB_ENCRYPTED_V5_CATALOG_PROFILE,
    &CTB_ENCRYPTED_V4_CATALOG_PROFILE,
    &CHITUBOX_ZIP_CATALOG_PROFILE,
];

static RESOLVER: ChituboxResolver = ChituboxResolver;

/// Family-level registry entry for Chitubox-compatible formats.
///
/// Chitubox-compatible formats 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "chitubox",
        aliases: &[
            "ctb",
            "ctb-v1",
            "v1.ctb",
            "ctb-v2",
            "ctb-v3",
            "v3.ctb",
            "ctb-v4",
            "v4.ctb",
            "ctb-v5",
            "v5.ctb",
            "gktwo-ctb",
            "gktwo-ctb-v1",
            "gktwo-v1.ctb",
            "gktwo-ctb-v2",
            "gktwo-v2.ctb",
            "gktwo-ctb-v3",
            "gktwo-v3.ctb",
            "gktwo.ctb",
            "gktwo-ctb-v5",
            "gktwo-v5.ctb",
            "cbddlp-v1",
            "cbddlp",
            "photon",
            "phz",
            "fdg",
            "photons",
            "photon-s",
            "encrypted-ctb-v4",
            "encrypted-v4.ctb",
            "encrypted-ctb",
            "encrypted-ctb-v5",
            "encrypted.ctb",
            "encrypted-v5.ctb",
            "chitubox-zip",
            "chituzip",
        ],
        extensions: &[
            "ctb",
            "v1.ctb",
            "v3.ctb",
            "v4.ctb",
            "v5.ctb",
            "gktwo-v1.ctb",
            "gktwo-v2.ctb",
            "gktwo-v3.ctb",
            "gktwo.ctb",
            "gktwo-v5.ctb",
            "cbddlp",
            "photon",
            "phz",
            "fdg",
            "photons",
            "encrypted-v4.ctb",
            "encrypted.ctb",
            "encrypted-v5.ctb",
            "zip",
        ],
    },
    family: FormatFamily::Chitubox,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
