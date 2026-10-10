//! Anycubic family profile declarations.
//!
//! The macro calls below intentionally keep machine facts, runtime wiring, and
//! catalog identity next to each other. Generated `FormatSpec` values enter the
//! runtime registry through `PROFILES`, while generated `CatalogProfile` values
//! enter the WASM/TypeScript catalog through `CATALOG_PROFILES`.
//!
//! Anycubic family profile declarations。下面的宏调用刻意把 machine facts、runtime
//! wiring 与 catalog identity 放在一起。生成的 `FormatSpec` 通过 `PROFILES` 进入
//! runtime registry，生成的 `CatalogProfile` 通过 `CATALOG_PROFILES` 进入 WASM/TypeScript catalog。

use crate::formats::{
    catalog::CatalogProfile,
    common::{
        keys, FamilyIdentity, FamilyProfile, FormatCapabilities, FormatFamily, FormatFamilySpec,
        FormatSpec,
    },
};

use super::{
    archive::{AnycubicZipArchiveLayout, AnycubicZipHeaderLayout, AnycubicZipLayerCodec},
    codecs::{Pw0Codec, PwsCodec},
    layouts::{V1Layout, V516Layout, V517Layout, V518Layout},
    profile_kit::{define_anycubic_archive_profile, define_anycubic_profiles},
    resolver::AnycubicResolver,
    specifics::{
        AnycubicArchiveSpec, AnycubicMachineId, AnycubicSpec, AnycubicTableLayout, AnycubicVersion,
    },
};

define_anycubic_profiles!(
    facts_static = PW0_ANYCUBIC,
    header_adapter = PW0_HEADER,
    codec_adapter = PW0_CODEC,
    spec = PW0_SPEC,
    profile = PW0_PROFILE,
    catalog_profile = PW0_CATALOG_PROFILE,
    identity = {
        name = "pw0",
        aliases = ["pw0"],
        extensions = ["pw0"],
    },
    catalog = {
        profile_id = "anycubic-pw0",
        display_name = "Photon Zero",
        target_names = ["pw0"],
        default_export = true,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonZero,
        display_name = "Photon Zero",
        aliases = ["photon zero"],
        resolution = (480, 854),
        display_mm = (55.4, 98.63),
        machine_z_mm = 150.0,
        supported_versions = [AnycubicVersion::V1],
        default_version = AnycubicVersion::V1,
        table_layout = AnycubicTableLayout::V1,
    },
    runtime = binary {
        layout = V1Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V1,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWS_ANYCUBIC,
    header_adapter = PWS_HEADER,
    codec_adapter = PWS_CODEC,
    spec = PWS_SPEC,
    profile = PWS_PROFILE,
    catalog_profile = PWS_CATALOG_PROFILE,
    identity = {
        name = "pws",
        aliases = ["pws"],
        extensions = ["pws"],
    },
    catalog = {
        profile_id = "anycubic-pws",
        display_name = "Photon S",
        target_names = ["pws"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonS,
        display_name = "Photon S",
        aliases = ["photon s"],
        resolution = (1440, 2560),
        display_mm = (68.04, 120.96),
        machine_z_mm = 165.0,
        supported_versions = [AnycubicVersion::V1],
        default_version = AnycubicVersion::V1,
        table_layout = AnycubicTableLayout::V1,
    },
    runtime = binary {
        layout = V1Layout,
        codec = PwsCodec,
        header_key = keys::HEADER_ANYCUBIC_V1,
        layer_codec_key = keys::ANYCUBIC_PWS_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWX_ANYCUBIC,
    header_adapter = PWX_HEADER,
    codec_adapter = PWX_CODEC,
    spec = PWX_SPEC,
    profile = PWX_PROFILE,
    catalog_profile = PWX_CATALOG_PROFILE,
    identity = {
        name = "pwx",
        aliases = ["pwx"],
        extensions = ["pwx"],
    },
    catalog = {
        profile_id = "anycubic-pwx",
        display_name = "Photon X",
        target_names = ["pwx"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonX,
        display_name = "Photon X",
        aliases = ["photon x"],
        resolution = (2560, 1600),
        display_mm = (192.0, 120.0),
        machine_z_mm = 165.0,
        supported_versions = [AnycubicVersion::V1],
        default_version = AnycubicVersion::V1,
        table_layout = AnycubicTableLayout::V1,
    },
    runtime = binary {
        layout = V1Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V1,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = DLP_ANYCUBIC,
    header_adapter = DLP_HEADER,
    codec_adapter = DLP_CODEC,
    spec = DLP_SPEC,
    profile = DLP_PROFILE,
    catalog_profile = DLP_CATALOG_PROFILE,
    identity = {
        name = "dlp",
        aliases = ["dlp"],
        extensions = ["dlp"],
    },
    catalog = {
        profile_id = "anycubic-dlp",
        display_name = "Photon Ultra",
        target_names = ["dlp"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonUltra,
        display_name = "Photon Ultra",
        aliases = ["photon ultra"],
        resolution = (1280, 720),
        display_mm = (102.4, 57.6),
        machine_z_mm = 165.0,
        supported_versions = [
        AnycubicVersion::V1,
        AnycubicVersion::V515,
        AnycubicVersion::V516
    ],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWMO_ANYCUBIC,
    header_adapter = PWMO_HEADER,
    codec_adapter = PWMO_CODEC,
    spec = PWMO_SPEC,
    profile = PWMO_PROFILE,
    catalog_profile = PWMO_CATALOG_PROFILE,
    identity = {
        name = "pwmo",
        aliases = ["pwmo"],
        extensions = ["pwmo"],
    },
    catalog = {
        profile_id = "anycubic-pwmo",
        display_name = "Photon Mono",
        target_names = ["pwmo"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMono,
        display_name = "Photon Mono",
        aliases = ["photon mono"],
        resolution = (1620, 2560),
        display_mm = (82.62, 130.56),
        machine_z_mm = 165.0,
        supported_versions = [
        AnycubicVersion::V1,
        AnycubicVersion::V515,
        AnycubicVersion::V516
    ],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWMS_ANYCUBIC,
    header_adapter = PWMS_HEADER,
    codec_adapter = PWMS_CODEC,
    spec = PWMS_SPEC,
    profile = PWMS_PROFILE,
    catalog_profile = PWMS_CATALOG_PROFILE,
    identity = {
        name = "pwms",
        aliases = ["pwms"],
        extensions = ["pwms"],
    },
    catalog = {
        profile_id = "anycubic-pwms",
        display_name = "Photon Mono SE",
        target_names = ["pwms"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoSE,
        display_name = "Photon Mono SE",
        aliases = ["photon mono se", "mono se"],
        resolution = (1620, 2560),
        display_mm = (82.62, 130.56),
        machine_z_mm = 160.0,
        supported_versions = [
        AnycubicVersion::V1,
        AnycubicVersion::V515,
        AnycubicVersion::V516
    ],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWMA_ANYCUBIC,
    header_adapter = PWMA_HEADER,
    codec_adapter = PWMA_CODEC,
    spec = PWMA_SPEC,
    profile = PWMA_PROFILE,
    catalog_profile = PWMA_CATALOG_PROFILE,
    identity = {
        name = "pwma",
        aliases = ["pwma"],
        extensions = ["pwma"],
    },
    catalog = {
        profile_id = "anycubic-pwma",
        display_name = "Photon Mono 4K",
        target_names = ["pwma"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMono4K,
        display_name = "Photon Mono 4K",
        aliases = ["photon mono 4k", "mono 4k"],
        resolution = (3840, 2400),
        display_mm = (134.4, 84.0),
        machine_z_mm = 165.0,
        supported_versions = [AnycubicVersion::V515, AnycubicVersion::V516],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWMX_ANYCUBIC,
    header_adapter = PWMX_HEADER,
    codec_adapter = PWMX_CODEC,
    spec = PWMX_SPEC,
    profile = PWMX_PROFILE,
    catalog_profile = PWMX_CATALOG_PROFILE,
    identity = {
        name = "pwmx",
        aliases = ["pwmx"],
        extensions = ["pwmx"],
    },
    catalog = {
        profile_id = "anycubic-pwmx",
        display_name = "Photon Mono X",
        target_names = ["pwmx"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoX,
        display_name = "Photon Mono X",
        aliases = ["photon mono x", "mono x"],
        resolution = (3840, 2400),
        display_mm = (192.0, 120.0),
        machine_z_mm = 245.0,
        supported_versions = [
        AnycubicVersion::V1,
        AnycubicVersion::V515,
        AnycubicVersion::V516
    ],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PMSQ_ANYCUBIC,
    header_adapter = PMSQ_HEADER,
    codec_adapter = PMSQ_CODEC,
    spec = PMSQ_SPEC,
    profile = PMSQ_PROFILE,
    catalog_profile = PMSQ_CATALOG_PROFILE,
    identity = {
        name = "pmsq",
        aliases = ["pmsq"],
        extensions = ["pmsq"],
    },
    catalog = {
        profile_id = "anycubic-pmsq",
        display_name = "Photon Mono SQ",
        target_names = ["pmsq"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoSQ,
        display_name = "Photon Mono SQ",
        aliases = ["photon mono sq", "mono sq"],
        resolution = (2400, 2560),
        display_mm = (120.0, 128.0),
        machine_z_mm = 200.0,
        supported_versions = [
        AnycubicVersion::V1,
        AnycubicVersion::V515,
        AnycubicVersion::V516
    ],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM3_ANYCUBIC,
    header_adapter = PM3_HEADER,
    codec_adapter = PM3_CODEC,
    spec = PM3_SPEC,
    profile = PM3_PROFILE,
    catalog_profile = PM3_CATALOG_PROFILE,
    identity = {
        name = "pm3",
        aliases = ["pm3"],
        extensions = ["pm3"],
    },
    catalog = {
        profile_id = "anycubic-pm3",
        display_name = "Photon M3",
        target_names = ["pm3"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonM3,
        display_name = "Photon M3",
        aliases = ["photon m3"],
        resolution = (4096, 2560),
        display_mm = (163.84, 102.4),
        machine_z_mm = 180.0,
        supported_versions = [AnycubicVersion::V515, AnycubicVersion::V516],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM3M_ANYCUBIC,
    header_adapter = PM3M_HEADER,
    codec_adapter = PM3M_CODEC,
    spec = PM3M_SPEC,
    profile = PM3M_PROFILE,
    catalog_profile = PM3M_CATALOG_PROFILE,
    identity = {
        name = "pm3m",
        aliases = ["pm3m"],
        extensions = ["pm3m"],
    },
    catalog = {
        profile_id = "anycubic-pm3m",
        display_name = "Photon M3 Max",
        target_names = ["pm3m"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonM3Max,
        display_name = "Photon M3 Max",
        aliases = ["photon m3 max", "m3 max"],
        resolution = (6480, 3600),
        display_mm = (298.08, 165.6),
        machine_z_mm = 300.0,
        supported_versions = [AnycubicVersion::V515, AnycubicVersion::V516],
        default_version = AnycubicVersion::V516,
        table_layout = AnycubicTableLayout::V516,
    },
    runtime = binary {
        layout = V516Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V516,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = DL2P_ANYCUBIC,
    header_adapter = DL2P_HEADER,
    codec_adapter = DL2P_CODEC,
    spec = DL2P_SPEC,
    profile = DL2P_PROFILE,
    catalog_profile = DL2P_CATALOG_PROFILE,
    identity = {
        name = "dl2p",
        aliases = ["dl2p"],
        extensions = ["dl2p"],
    },
    catalog = {
        profile_id = "anycubic-dl2p",
        display_name = "Photon D2",
        target_names = ["dl2p"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonD2,
        display_name = "Photon D2",
        aliases = ["photon d2"],
        resolution = (2560, 1440),
        display_mm = (130.56, 73.44),
        machine_z_mm = 165.0,
        supported_versions = [
        AnycubicVersion::V515,
        AnycubicVersion::V516,
        AnycubicVersion::V517
    ],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PMX2_ANYCUBIC,
    header_adapter = PMX2_HEADER,
    codec_adapter = PMX2_CODEC,
    spec = PMX2_SPEC,
    profile = PMX2_PROFILE,
    catalog_profile = PMX2_CATALOG_PROFILE,
    identity = {
        name = "pmx2",
        aliases = ["pmx2"],
        extensions = ["pmx2"],
    },
    catalog = {
        profile_id = "anycubic-pmx2",
        display_name = "Photon Mono X2",
        target_names = ["pmx2"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoX2,
        display_name = "Photon Mono X2",
        aliases = ["photon mono x2", "mono x2"],
        resolution = (4096, 2560),
        display_mm = (196.61, 122.88),
        machine_z_mm = 200.0,
        supported_versions = [
        AnycubicVersion::V515,
        AnycubicVersion::V516,
        AnycubicVersion::V517
    ],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PWMB_ANYCUBIC,
    header_adapter = PWMB_HEADER,
    codec_adapter = PWMB_CODEC,
    spec = PWMB_SPEC,
    profile = PWMB_PROFILE,
    catalog_profile = PWMB_CATALOG_PROFILE,
    identity = {
        name = "pwmb",
        aliases = ["pwmb"],
        extensions = ["pwmb"],
    },
    catalog = {
        profile_id = "anycubic-pwmb",
        display_name = "Photon Mono X 6K / M3 Plus",
        target_names = ["pwmb"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoX6KM3Plus,
        display_name = "Photon Mono X 6K / M3 Plus",
        aliases = ["photon mono x 6k", "m3 plus", "mono x 6k"],
        resolution = (5760, 3600),
        display_mm = (198.15, 123.84),
        machine_z_mm = 245.0,
        supported_versions = [
        AnycubicVersion::V515,
        AnycubicVersion::V516,
        AnycubicVersion::V517
    ],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM3R_ANYCUBIC,
    header_adapter = PM3R_HEADER,
    codec_adapter = PM3R_CODEC,
    spec = PM3R_SPEC,
    profile = PM3R_PROFILE,
    catalog_profile = PM3R_CATALOG_PROFILE,
    identity = {
        name = "pm3r",
        aliases = ["pm3r"],
        extensions = ["pm3r"],
    },
    catalog = {
        profile_id = "anycubic-pm3r",
        display_name = "Photon M3 Premium",
        target_names = ["pm3r"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonM3Premium,
        display_name = "Photon M3 Premium",
        aliases = ["photon m3 premium", "m3 premium"],
        resolution = (7680, 4320),
        display_mm = (218.88, 123.12),
        machine_z_mm = 250.0,
        supported_versions = [
        AnycubicVersion::V515,
        AnycubicVersion::V516,
        AnycubicVersion::V517
    ],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM3N_ANYCUBIC,
    header_adapter = PM3N_HEADER,
    codec_adapter = PM3N_CODEC,
    spec = PM3N_SPEC,
    profile = PM3N_PROFILE,
    catalog_profile = PM3N_CATALOG_PROFILE,
    identity = {
        name = "pm3n",
        aliases = ["pm3n"],
        extensions = ["pm3n"],
    },
    catalog = {
        profile_id = "anycubic-pm3n",
        display_name = "Photon Mono 2",
        target_names = ["pm3n"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMono2,
        display_name = "Photon Mono 2",
        aliases = ["photon mono 2", "mono 2"],
        resolution = (4096, 2560),
        display_mm = (143.36, 89.6),
        machine_z_mm = 165.0,
        supported_versions = [AnycubicVersion::V517],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM4N_ANYCUBIC,
    header_adapter = PM4N_HEADER,
    codec_adapter = PM4N_CODEC,
    spec = PM4N_SPEC,
    profile = PM4N_PROFILE,
    catalog_profile = PM4N_CATALOG_PROFILE,
    identity = {
        name = "pm4n",
        aliases = ["pm4n"],
        extensions = ["pm4n"],
    },
    catalog = {
        profile_id = "anycubic-pm4n",
        display_name = "Photon Mono 4",
        target_names = ["pm4n"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMono4,
        display_name = "Photon Mono 4",
        aliases = ["photon mono 4", "photon mono 4 ultra", "mono 4 ultra"],
        resolution = (9024, 5120),
        display_mm = (153.408, 87.04),
        machine_z_mm = 165.0,
        supported_versions = [AnycubicVersion::V517],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PX6S_ANYCUBIC,
    header_adapter = PX6S_HEADER,
    codec_adapter = PX6S_CODEC,
    spec = PX6S_SPEC,
    profile = PX6S_PROFILE,
    catalog_profile = PX6S_CATALOG_PROFILE,
    identity = {
        name = "px6s",
        aliases = ["px6s"],
        extensions = ["px6s"],
    },
    catalog = {
        profile_id = "anycubic-px6s",
        display_name = "Photon Mono X 6Ks",
        target_names = ["px6s"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoX6Ks,
        display_name = "Photon Mono X 6Ks",
        aliases = ["photon mono x 6ks", "mono x 6ks"],
        resolution = (5760, 3600),
        display_mm = (195.84, 122.4),
        machine_z_mm = 200.0,
        supported_versions = [AnycubicVersion::V517],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM5_ANYCUBIC,
    header_adapter = PM5_HEADER,
    codec_adapter = PM5_CODEC,
    spec = PM5_SPEC,
    profile = PM5_PROFILE,
    catalog_profile = PM5_CATALOG_PROFILE,
    identity = {
        name = "pm5",
        aliases = ["pm5"],
        extensions = ["pm5"],
    },
    catalog = {
        profile_id = "anycubic-pm5",
        display_name = "Photon Mono M5",
        target_names = ["pm5"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoM5,
        display_name = "Photon Mono M5",
        aliases = ["photon mono m5", "mono m5"],
        resolution = (11520, 5120),
        display_mm = (218.88, 122.88),
        machine_z_mm = 200.0,
        supported_versions = [AnycubicVersion::V517],
        default_version = AnycubicVersion::V517,
        table_layout = AnycubicTableLayout::V517,
    },
    runtime = binary {
        layout = V517Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V517,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = PM5S_ANYCUBIC,
    header_adapter = PM5S_HEADER,
    codec_adapter = PM5S_CODEC,
    spec = PM5S_SPEC,
    profile = PM5S_PROFILE,
    catalog_profile = PM5S_CATALOG_PROFILE,
    identity = {
        name = "pm5s",
        aliases = ["pm5s"],
        extensions = ["pm5s"],
    },
    catalog = {
        profile_id = "anycubic-pm5s",
        display_name = "Photon Mono M5s",
        target_names = ["pm5s"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoM5s,
        display_name = "Photon Mono M5s",
        aliases = ["photon mono m5s", "mono m5s"],
        resolution = (11520, 5120),
        display_mm = (218.88, 122.88),
        machine_z_mm = 200.0,
        supported_versions = [AnycubicVersion::V518],
        default_version = AnycubicVersion::V518,
        table_layout = AnycubicTableLayout::V518,
    },
    runtime = binary {
        layout = V518Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V518,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

define_anycubic_profiles!(
    facts_static = M5SP_ANYCUBIC,
    header_adapter = M5SP_HEADER,
    codec_adapter = M5SP_CODEC,
    spec = M5SP_SPEC,
    profile = M5SP_PROFILE,
    catalog_profile = M5SP_CATALOG_PROFILE,
    identity = {
        name = "m5sp",
        aliases = ["m5sp"],
        extensions = ["m5sp"],
    },
    catalog = {
        profile_id = "anycubic-m5sp",
        display_name = "Photon Mono M5s Pro",
        target_names = ["m5sp"],
        default_export = false,
    },
    facts = {
        machine_id = AnycubicMachineId::PhotonMonoM5sPro,
        display_name = "Photon Mono M5s Pro",
        aliases = ["photon mono m5s pro", "mono m5s pro"],
        resolution = (13312, 5120),
        display_mm = (223.6416, 126.976),
        machine_z_mm = 200.0,
        supported_versions = [AnycubicVersion::V518],
        default_version = AnycubicVersion::V518,
        table_layout = AnycubicTableLayout::V518,
    },
    runtime = binary {
        layout = V518Layout,
        codec = Pw0Codec,
        header_key = keys::HEADER_ANYCUBIC_V518,
        layer_codec_key = keys::ANYCUBIC_PW0_RLE,
        capabilities = FormatCapabilities::binary_import_export().with_preview(true),
    },
);

static ANYCUBIC_ZIP_SPECIFICS: AnycubicArchiveSpec = AnycubicArchiveSpec {
    // Photon Workshop ZIP/PWSZ archives use vendor-specific entry names that do
    // not match their public file extension. Keep these names centralized so
    // archive layout and catalog wiring stay in sync.
    settings_manifest_entry: "anycubic_photon_resins.pwsp",
    layers_manifest_entry: "layers_controller.conf",
    layer_entry_prefix: "layer_images/layer_",
    default_layer_image_extension: "pw0Img",
    layer_image_extensions: &["pw0Img", "pwszImg"],
    required_manifest_entries: &["anycubic_photon_resins.pwsp", "layers_controller.conf"],
};
static ANYCUBIC_ZIP_HEADER: AnycubicZipHeaderLayout = AnycubicZipHeaderLayout {
    specifics: &ANYCUBIC_ZIP_SPECIFICS,
};
static ANYCUBIC_ZIP_ARCHIVE: AnycubicZipArchiveLayout = AnycubicZipArchiveLayout {
    specifics: &ANYCUBIC_ZIP_SPECIFICS,
};
static ANYCUBIC_ZIP_CODEC: AnycubicZipLayerCodec = AnycubicZipLayerCodec;

define_anycubic_archive_profile!(
    spec = ANYCUBIC_ZIP_SPEC,
    profile = ANYCUBIC_ZIP_PROFILE,
    catalog_profile = ANYCUBIC_ZIP_CATALOG_PROFILE,
    identity = {
        name = "anycubic-zip",
        aliases = ["anycubic-zip", "pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"],
        extensions = ["pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"],
    },
    catalog = {
        profile_id = "anycubic-zip-archive-catalog",
        display_name = "Anycubic Zip archive catalog",
        target_names = ["anycubic-zip", "pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"],
        default_export = false,
    },
    facts = &ANYCUBIC_ZIP_SPECIFICS,
    runtime = archive {
        capabilities = FormatCapabilities::archive_import_export().with_preview(true),
        header_key = keys::HEADER_ANYCUBIC_ZIP,
        layer_codec_key = keys::ANYCUBIC_ZIP_RLE,
        pixel_model_key = keys::PIXEL_GRAYSCALE_8,
        archive_key = keys::ZIP_ARCHIVE,
        header = &ANYCUBIC_ZIP_HEADER,
        layer_codec = &ANYCUBIC_ZIP_CODEC,
        archive = &ANYCUBIC_ZIP_ARCHIVE,
    },
);

/// Typed Anycubic binary profiles used by family-specific catalog generation.
///
/// family-specific catalog 生成使用的 typed Anycubic binary profiles。
pub static ANYCUBIC_PROFILES: &[&FamilyProfile<AnycubicSpec>] = &[
    &PW0_PROFILE,
    &PWS_PROFILE,
    &PWX_PROFILE,
    &DLP_PROFILE,
    &PWMO_PROFILE,
    &PWMS_PROFILE,
    &PWMA_PROFILE,
    &PWMX_PROFILE,
    &PMSQ_PROFILE,
    &PM3_PROFILE,
    &PM3M_PROFILE,
    &DL2P_PROFILE,
    &PMX2_PROFILE,
    &PWMB_PROFILE,
    &PM3R_PROFILE,
    &PM3N_PROFILE,
    &PM4N_PROFILE,
    &PX6S_PROFILE,
    &PM5_PROFILE,
    &PM5S_PROFILE,
    &M5SP_PROFILE,
];

/// Typed Anycubic archive profiles used by family-specific catalog generation.
///
/// family-specific catalog 生成使用的 typed Anycubic archive profiles。
pub static ANYCUBIC_ARCHIVE_PROFILES: &[&FamilyProfile<AnycubicArchiveSpec>] =
    &[&ANYCUBIC_ZIP_PROFILE];

/// Runtime profiles published by the Anycubic family.
///
/// This includes binary machine profiles and the Anycubic ZIP archive profile.
///
/// Anycubic family 暴露给 runtime registry 的 profiles，包含 binary machine profiles
/// 与 Anycubic ZIP archive profile。
pub static PROFILES: &[&FormatSpec] = &[
    &PW0_SPEC,
    &PWS_SPEC,
    &PWX_SPEC,
    &DLP_SPEC,
    &PWMO_SPEC,
    &PWMS_SPEC,
    &PWMA_SPEC,
    &PWMX_SPEC,
    &PMSQ_SPEC,
    &PM3_SPEC,
    &PM3M_SPEC,
    &DL2P_SPEC,
    &PMX2_SPEC,
    &PWMB_SPEC,
    &PM3R_SPEC,
    &PM3N_SPEC,
    &PM4N_SPEC,
    &PX6S_SPEC,
    &PM5_SPEC,
    &PM5S_SPEC,
    &M5SP_SPEC,
    &ANYCUBIC_ZIP_SPEC,
];

/// Catalog profiles exported to the WASM/TypeScript format catalog.
///
/// The order mirrors `PROFILES` so UI callers see the same family/profile
/// ordering as the runtime registry.
///
/// 导出到 WASM/TypeScript format catalog 的 profiles。顺序与 `PROFILES` 对齐，让 UI
/// 调用方看到与 runtime registry 一致的 family/profile 顺序。
pub static CATALOG_PROFILES: &[&CatalogProfile] = &[
    &PW0_CATALOG_PROFILE,
    &PWS_CATALOG_PROFILE,
    &PWX_CATALOG_PROFILE,
    &DLP_CATALOG_PROFILE,
    &PWMO_CATALOG_PROFILE,
    &PWMS_CATALOG_PROFILE,
    &PWMA_CATALOG_PROFILE,
    &PWMX_CATALOG_PROFILE,
    &PMSQ_CATALOG_PROFILE,
    &PM3_CATALOG_PROFILE,
    &PM3M_CATALOG_PROFILE,
    &DL2P_CATALOG_PROFILE,
    &PMX2_CATALOG_PROFILE,
    &PWMB_CATALOG_PROFILE,
    &PM3R_CATALOG_PROFILE,
    &PM3N_CATALOG_PROFILE,
    &PM4N_CATALOG_PROFILE,
    &PX6S_CATALOG_PROFILE,
    &PM5_CATALOG_PROFILE,
    &PM5S_CATALOG_PROFILE,
    &M5SP_CATALOG_PROFILE,
    &ANYCUBIC_ZIP_CATALOG_PROFILE,
];

static RESOLVER: AnycubicResolver = AnycubicResolver;

/// Family-level registry entry for Anycubic binary and ZIP variants.
///
/// Anycubic binary 与 ZIP variants 的 family-level registry entry。
pub static FAMILY: FormatFamilySpec = FormatFamilySpec {
    identity: FamilyIdentity {
        name: "anycubic",
        aliases: &[
            "pw0",
            "pws",
            "pwx",
            "dlp",
            "pwmo",
            "pwms",
            "pwma",
            "pwmx",
            "pmsq",
            "pm3",
            "pm3m",
            "dl2p",
            "pmx2",
            "pwmb",
            "pm3r",
            "pm3n",
            "pm4n",
            "px6s",
            "pm5",
            "pm5s",
            "m5sp",
            "anycubic-zip",
            "pm4u",
            "pm7",
            "pm7m",
            "pwsz",
            "pp1",
            "pp1m",
        ],
        extensions: &[
            "pw0", "pws", "pwx", "dlp", "pwmo", "pwms", "pwma", "pwmx", "pmsq", "pm3", "pm3m",
            "dl2p", "pmx2", "pwmb", "pm3r", "pm3n", "pm4n", "px6s", "pm5", "pm5s", "m5sp", "pm4u",
            "pm7", "pm7m", "pwsz", "pp1", "pp1m",
        ],
    },
    family: FormatFamily::Anycubic,
    profiles: PROFILES,
    resolver: &RESOLVER,
};
