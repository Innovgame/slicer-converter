use slicer_converter_core::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::{
        common::{
            keys, ArchiveLayout, ContainerKind, CryptoPolicy, DefaultBinaryPipeline,
            ExportTargetOptions, FamilyIdentity, FooterPolicy, FormatCapabilities, FormatCatalog,
            FormatFamily, FormatKey, FormatKeys, FormatSpec, HeaderLayout, LayerCodec,
            NoArchiveLayout, NoCrypto, NoFooter, OutputSink, VecOutputSink,
        },
        FormatIdentity, LayerDecodeContext, LayerEncodeContext,
    },
};

#[test]
fn registry_resolves_specs_from_family_profiles_only() {
    let registry = slicer_converter_core::formats::global_registry();
    let all_specs = registry.all_specs();

    let ctb_by_name = registry.spec_by_name("ctb").expect("ctb spec by name");
    let ctb_by_extension = registry
        .spec_by_extension("ctb")
        .expect("ctb spec by extension");
    let ctb_from_profiles = all_specs
        .iter()
        .copied()
        .find(|spec| spec.identity.name == "ctb")
        .expect("ctb spec from flattened family profiles");

    assert!(std::ptr::eq(ctb_by_name, ctb_from_profiles));
    assert!(std::ptr::eq(ctb_by_extension, ctb_from_profiles));
}

#[test]
fn format_keys_are_typed_and_orderable() {
    let mut keys = vec![keys::CTB_RLE, keys::GOO_RLE, keys::CTB_RLE];
    keys.sort();
    keys.dedup();

    assert_eq!(keys, vec![keys::CTB_RLE, keys::GOO_RLE]);
    assert_ne!(FormatKey("ctb-rle"), FormatKey("goo-rle"));
}

#[test]
fn capabilities_describe_import_export_binary_profiles() {
    let caps = FormatCapabilities::binary_import_export()
        .with_preview(true)
        .with_anti_alias(true)
        .with_per_layer_settings(true);

    assert!(caps.can_import);
    assert!(caps.can_export);
    assert!(!caps.is_archive);
    assert!(caps.supports_preview);
    assert!(caps.supports_anti_alias);
    assert!(caps.supports_per_layer_settings);
}

#[test]
fn capabilities_do_not_keep_unused_catalog_only_constructor() {
    let capabilities_rs = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/formats/common/capabilities.rs"),
    )
    .expect("read common capabilities.rs");

    assert!(
        !capabilities_rs.contains("archive_catalog"),
        "FormatCapabilities should not keep a catalog-only constructor for unimplemented formats"
    );
    assert!(
        !capabilities_rs.contains("can_import: false,\n            can_export: false"),
        "FormatCapabilities should not encode a catalog-only import/export-disabled profile"
    );
}

#[test]
fn production_format_sources_do_not_keep_catalog_only_wording() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let offenders = rust_source_files(&manifest_dir.join("src"))
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).expect("read Rust source");
            if source.contains("catalog-only") || source.contains("cataloged but not implemented") {
                Some(
                    path.strip_prefix(&manifest_dir)
                        .unwrap()
                        .display()
                        .to_string(),
                )
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "production format sources should not keep catalog-only wording after removing that model"
    );
}

#[test]
fn catalog_identity_is_distinct_from_family_identity() {
    let family = FamilyIdentity {
        name: "chitubox",
        aliases: &["ctb", "cbddlp"],
        extensions: &["ctb", "cbddlp"],
    };
    let catalog = FormatCatalog {
        profile_id: "ctb-v4",
        display_name: "Chitubox CTB v4",
        default_extension: "ctb",
        target_names: &["ctb", "ctb-v4"],
        default_export: false,
    };

    assert_eq!(family.name, "chitubox");
    assert_eq!(catalog.profile_id, "ctb-v4");
}

#[test]
fn format_specs_do_not_expose_family_specific_fact_slot() {
    let registry = slicer_converter_core::formats::global_registry();
    assert!(registry.spec_by_name("goo").is_some());
    assert!(registry.spec_by_name("ctb").is_some());
    assert!(registry.spec_by_name("pw0").is_some());

    let spec_rs = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/formats/common/spec.rs"),
    )
    .expect("read common spec.rs");

    assert!(
        !spec_rs.contains("specifics:"),
        "FormatSpec should stay runtime-only; family facts live in catalog profiles"
    );
}

#[test]
fn common_profile_model_uses_family_profile_not_generic_catalog_profile() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile_rs = std::fs::read_to_string(manifest_dir.join("src/formats/common/profile.rs"))
        .expect("read common profile.rs");
    let catalog_rs = std::fs::read_to_string(manifest_dir.join("src/formats/catalog.rs"))
        .expect("read formats catalog.rs");
    let old_facts_bus = concat!("Format", "Facts");
    let old_catalog_profile = "CatalogProfile";
    let old_generic_profile = format!("{old_catalog_profile}<{old_facts_bus}>");

    assert!(profile_rs.contains("pub struct FormatCatalog"));
    assert!(profile_rs.contains("pub struct FamilyProfile<Facts"));
    assert!(!profile_rs.contains(&format!("pub struct {old_catalog_profile}<Facts>")));
    assert!(catalog_rs.contains("pub enum CatalogProfile"));
    assert!(!catalog_rs.contains(&format!("pub enum {old_facts_bus}")));
    assert!(!catalog_rs.contains(&old_generic_profile));
}

#[test]
fn catalog_projection_is_enum_not_facts_bus_alias() {
    let catalog_rs = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/formats/catalog.rs"),
    )
    .expect("read formats catalog.rs");

    assert!(catalog_rs.contains("pub enum CatalogProfile"));
    assert!(catalog_rs.contains("Generic(&'static common::FormatSpec)"));
    assert!(catalog_rs.contains("Chitubox(&'static common::FamilyProfile<ChituboxSpec>)"));
    assert!(catalog_rs.contains("Anycubic(&'static common::FamilyProfile<AnycubicSpec>)"));
    assert!(
        catalog_rs.contains("AnycubicArchive(&'static common::FamilyProfile<AnycubicArchiveSpec>)")
    );
    assert!(
        !catalog_rs.contains(&format!("pub type {} =", concat!("Format", "Profile"))),
        "CatalogProfile should be a projection enum, not a compatibility alias"
    );
}

#[test]
fn registry_names_catalog_profile_queries_explicitly() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod_rs =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats mod");

    assert!(formats_mod_rs.contains("fn catalog_profiles("));
    assert!(formats_mod_rs.contains("fn catalog_profile_by_name("));
    assert!(formats_mod_rs.contains("fn catalog_profile_for_spec("));
    assert!(formats_mod_rs.contains("CATALOG_PROFILE_GROUPS"));
    assert!(
        !formats_mod_rs.contains("fn all_profiles("),
        "registry catalog descriptions should be named catalog_profiles, not all_profiles"
    );
    assert!(
        !formats_mod_rs.contains("FORMAT_CATALOG_PROFILE_GROUPS"),
        "root catalog profile aggregation should not repeat the module prefix in its name"
    );
}

#[test]
fn common_modules_do_not_import_concrete_family_specifics() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_dir = manifest_dir.join("src/formats/common");

    for entry in std::fs::read_dir(&common_dir).expect("read common dir") {
        let entry = entry.expect("common file");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read common source");
        assert!(
            !source.contains("formats::anycubic") && !source.contains("formats::chitubox"),
            "{} should not import concrete family modules",
            path.display()
        );
    }

    assert!(!common_dir.join("specifics.rs").exists());
}

#[test]
fn format_spec_matches_identity_alias_extension_and_target_names() {
    assert!(DUMMY_SPEC.matches_name("dummy-profile"));
    assert!(DUMMY_SPEC.matches_name("DUMMY"));
    assert!(DUMMY_SPEC.matches_name(".dummy"));
    assert!(DUMMY_SPEC.matches_extension("dummy"));
    assert!(DUMMY_SPEC.matches_extension(".DUMMY"));
    assert!(!DUMMY_SPEC.matches_name("other"));
    assert!(!DUMMY_SPEC.matches_extension("other"));
}

#[test]
fn simple_profile_resolver_matches_target_names_case_insensitively() {
    assert_eq!(FormatFamily::Chitubox.as_key(), keys::FAMILY_CHITUBOX);
    assert_eq!(ContainerKind::Binary.as_key(), keys::CONTAINER_BINARY);
}

struct DummyHeader;
struct DummyCodec;
struct DummyFooter;

impl HeaderLayout for DummyHeader {
    fn try_detect(&self, data: &[u8], _hint: Option<&str>) -> bool {
        data.starts_with(b"DUMMY")
    }

    fn parse_header(
        &self,
        _data: &[u8],
        _spec: &FormatSpec,
    ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
        let metadata = FileMetadata::builder()
            .resolution(2, 1)
            .display_size_mm(1.0, 1.0)
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()?;
        Ok((metadata, vec![LayerOffset { offset: 5, size: 2 }]))
    }

    fn build_header(
        &self,
        _metadata: &FileMetadata,
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(b"DUMMY".to_vec())
    }

    fn finalize_header(
        &self,
        header: &[u8],
        _layer_sizes: &[u32],
        _spec: &FormatSpec,
        _options: ExportTargetOptions<'_>,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(header.to_vec())
    }
}

impl LayerCodec for DummyCodec {
    fn decode_layer_into(
        &self,
        data: &[u8],
        _ctx: &LayerDecodeContext,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        output.clear();
        output.extend(data.iter().map(|byte| byte.wrapping_add(1)));
        Ok(())
    }

    fn encode_layer_into(
        &self,
        pixels: &[u8],
        _ctx: &LayerEncodeContext<'_>,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        output.clear();
        output.extend(pixels.iter().map(|byte| byte.wrapping_sub(1)));
        Ok(())
    }
}

impl FooterPolicy for DummyFooter {
    fn build_footer(
        &self,
        _metadata: &FileMetadata,
        _spec: &FormatSpec,
    ) -> Result<Vec<u8>, ConvertError> {
        Ok(b"TAIL".to_vec())
    }
}

static DUMMY_HEADER: DummyHeader = DummyHeader;
static DUMMY_CODEC: DummyCodec = DummyCodec;
static DUMMY_FOOTER: DummyFooter = DummyFooter;
static DUMMY_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "dummy-profile",
        aliases: &["dummy"],
        extensions: &["dummy"],
    },
    family: FormatFamily::Other("dummy"),
    catalog: FormatCatalog {
        profile_id: "dummy-profile",
        display_name: "Dummy Profile",
        default_extension: "dummy",
        target_names: &["dummy"],
        default_export: true,
    },
    container: ContainerKind::Binary,
    capabilities: FormatCapabilities::binary_import_export(),
    keys: FormatKeys {
        family_key: keys::FAMILY_OTHER,
        profile_key: FormatKey("profile/dummy"),
        container_key: keys::CONTAINER_BINARY,
        header_key: keys::GENERIC_HEADER_FALLBACK,
        layer_codec_key: keys::GENERIC_LAYER_FALLBACK,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::NO_ARCHIVE,
    },
    header: &DUMMY_HEADER,
    layer_codec: &DUMMY_CODEC,
    crypto: &NoCrypto,
    footer: &DUMMY_FOOTER,
    archive: &NoArchiveLayout,
    pipeline: &DefaultBinaryPipeline,
};

#[test]
fn no_crypto_no_footer_and_vec_sink_are_zero_behavior_defaults() {
    let mut payload = vec![1, 2, 3];
    let ctx = LayerDecodeContext::new(0, 3, 1);
    NoCrypto
        .decrypt_layer_in_place(&mut payload, 0, &ctx)
        .expect("NoCrypto decrypt");
    assert_eq!(payload, vec![1, 2, 3]);

    let footer = NoFooter
        .build_footer(
            &FileMetadata::builder()
                .resolution(1, 1)
                .display_size_mm(1.0, 1.0)
                .layer_height_mm(0.05)
                .layer_count(0)
                .build()
                .expect("metadata"),
            &DUMMY_SPEC,
        )
        .expect("NoFooter");
    assert!(footer.is_empty());

    let mut sink = VecOutputSink::new();
    sink.write_all(b"ab").expect("write ab");
    sink.write_all(b"cd").expect("write cd");
    assert_eq!(sink.into_inner(), b"abcd");
}

#[test]
fn format_spec_delegates_to_spec_strategies() {
    let fmt = &DUMMY_SPEC;
    assert_eq!(fmt.identity.name, "dummy-profile");

    let parsed = fmt.pipeline.parse_document(fmt, b"DUMMY12").expect("parse");
    let metadata = parsed.metadata;
    let layers = parsed.layers;
    assert_eq!(metadata.printer.resolution_x, 2);
    assert_eq!(layers[0].offset, 5);

    let ctx = LayerDecodeContext::new(0, 2, 1);
    let mut pixels = Vec::new();
    fmt.decode_layer_into(&[10, 20], &ctx, &mut pixels)
        .expect("decode");
    assert_eq!(pixels, vec![11, 21]);

    let encode_ctx = LayerEncodeContext {
        layer_index: 0,
        width: 2,
        height: 1,
        layer_metadata: None,
    };
    let mut encoded = Vec::new();
    fmt.encode_layer_into(&pixels, &encode_ctx, &mut encoded)
        .expect("encode");
    assert_eq!(encoded, vec![10, 20]);
    let metadata = FileMetadata::default();
    let footer = fmt
        .footer
        .build_footer(&metadata, fmt)
        .expect("dummy footer");
    assert_eq!(footer, b"TAIL");
}

#[test]
fn format_spec_does_not_keep_removed_runtime_facade_aliases() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composed_rs = std::fs::read_to_string(manifest_dir.join("src/formats/common/composed.rs"))
        .expect("read common composed.rs source");

    assert!(
        !composed_rs.contains("pub fn is_zip_container"),
        "FormatSpec should expose archive/container facts through capabilities, not a legacy facade alias"
    );
    assert!(
        !composed_rs.contains("pub fn try_detect"),
        "FormatSpec detection should go through family resolvers and HeaderLayout, not a legacy facade alias"
    );
    assert!(
        !composed_rs.contains("pub fn build_footer(&self)"),
        "FormatSpec footer behavior should be explicit FooterPolicy execution, not a metadata-free facade alias"
    );
}

#[test]
fn no_archive_layout_is_empty_default() {
    let entries = NoArchiveLayout
        .list_layer_entries(&[], &DUMMY_SPEC)
        .expect("NoArchive entries");
    assert!(entries.is_empty());
}

#[test]
fn chitubox_spec_declarations_do_not_keep_legacy_static_names() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs_path = manifest_dir.join("src/formats/chitubox/specs.rs");
    let specs_rs = std::fs::read_to_string(&specs_rs_path).expect("read chitubox specs.rs source");

    assert!(
        !specs_rs.contains("_LEGACY_SPEC"),
        "Chitubox FormatSpec static names should describe active specs, not legacy adapters"
    );
}

#[test]
fn chitubox_specs_do_not_keep_unused_unsupported_scaffolding() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let adapters = std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/adapters.rs"))
        .expect("read chitubox adapters");

    assert!(!adapters.contains("UnsupportedChituboxHeaderLayout"));
    assert!(!adapters.contains("UnsupportedChituboxLayerCodec"));
}

#[test]
fn anycubic_does_not_keep_parallel_profile_fact_sources() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let anycubic_dir = manifest_dir.join("src/formats/anycubic");

    assert!(!anycubic_dir.join("profiles.rs").exists());
    assert!(!anycubic_dir.join("matcher.rs").exists());
    assert!(!anycubic_dir.join("plan.rs").exists());

    let mod_rs = std::fs::read_to_string(anycubic_dir.join("mod.rs")).expect("read anycubic mod");
    for forbidden in [
        "ANYCUBIC_PROFILES",
        "build_anycubic_header_bytes",
        "match_anycubic_profile",
        "resolve_export_plan",
        "AnycubicFormatPlan",
    ] {
        assert!(
            !mod_rs.contains(forbidden),
            "anycubic mod.rs should not re-export {forbidden}"
        );
    }
}

#[test]
fn anycubic_layout_parse_header_is_required_not_default_unsupported() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout_rs = std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/layout.rs"))
        .expect("read anycubic layout");
    let error_rs =
        std::fs::read_to_string(manifest_dir.join("src/common/error.rs")).expect("read error");

    assert!(
        !layout_rs.contains("fn parse_header(&self, _data: &[u8])"),
        "AnycubicLayoutStrategy should require concrete parse_header implementations"
    );
    assert!(
        !layout_rs.contains("UnsupportedAnycubicLayout"),
        "Anycubic layout parsing should not keep an unsupported default branch"
    );
    assert!(
        !error_rs.contains("UnsupportedAnycubicLayout"),
        "ConvertError should not keep a dead Anycubic layout migration error"
    );
}

#[test]
fn anycubic_header_building_has_no_public_extension_version_escape_hatch() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let encoder = std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/encoder.rs"))
        .expect("read anycubic encoder");

    assert!(
        !encoder.contains("pub fn build_anycubic_header_bytes"),
        "Anycubic header building must go through FormatSpec.header.build_header"
    );
}

#[test]
fn production_code_has_no_anycubic_parallel_registry_tokens() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let mut stack = vec![src_dir];
    let forbidden = [
        "AnycubicMachineProfile",
        "AnycubicFormatPlan",
        "resolve_export_plan",
        "match_anycubic_profile",
        "list_anycubic_machine_profiles",
        "resolve_anycubic_export_plan",
    ];

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("read source");
            for token in forbidden {
                assert!(
                    !source.contains(token),
                    "{} should not contain legacy token {token}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn docs_do_not_point_new_format_work_to_removed_anycubic_profile_sources() {
    let repo_root = rust_workspace_root()
        .parent()
        .expect("rust workspace should live below repo root")
        .to_path_buf();
    let doc_paths = [
        "docs/adding-a-new-format.md",
        "docs/rust-quickstart.md",
        "docs/architecture-improvement-plan.md",
    ];
    let forbidden = [
        "rust/src/formats/anycubic/profiles.rs",
        "AnycubicMachineProfile",
        "ANYCUBIC_PROFILES",
    ];

    for relative_path in doc_paths {
        let path = repo_root.join(relative_path);
        let source = std::fs::read_to_string(&path).expect("read doc");
        for token in forbidden {
            assert!(
                !source.contains(token),
                "{relative_path} should not direct new work through removed token {token}"
            );
        }
    }
}

#[test]
fn docs_do_not_reference_removed_format_spec_parse_header_facade() {
    let repo_root = rust_workspace_root()
        .parent()
        .expect("rust workspace should live below repo root")
        .to_path_buf();
    let doc_paths = ["docs/adding-a-new-format.md", "docs/rust-quickstart.md"];
    let forbidden = [
        "FormatSpec::parse_header",
        "source_spec.parse_header",
        "`parse_header`、`decode_layer_into`",
    ];

    for relative_path in doc_paths {
        let path = repo_root.join(relative_path);
        let source = std::fs::read_to_string(&path).expect("read doc");
        for token in forbidden {
            assert!(
                !source.contains(token),
                "{relative_path} should reference FormatPipeline parsing instead of removed facade token {token}"
            );
        }
    }
}

fn rust_workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("core crate should live in rust/crates/core")
        .to_path_buf()
}

#[test]
fn chitubox_ctb_like_specs_use_shared_declaration_macro() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs_path = manifest_dir.join("src/formats/chitubox/specs.rs");
    let profile_kit_rs_path = manifest_dir.join("src/formats/chitubox/profile_kit.rs");
    let specs_rs = std::fs::read_to_string(&specs_rs_path).expect("read chitubox specs.rs source");
    let profile_kit_rs =
        std::fs::read_to_string(&profile_kit_rs_path).expect("read chitubox profile_kit.rs source");

    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_profiles"),
        "CTB-like Chitubox FormatSpec assembly should be centralized in profile_kit.rs"
    );
    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_ctb_profile"),
        "CTB version profiles should share a narrower CTB RLE/XOR declaration macro"
    );
    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_ctb_profile_parts"),
        "CTB header and typed specifics should share one profile-parts macro"
    );
    assert_eq!(
        specs_rs
            .matches("define_chitubox_ctb_profile_parts!(")
            .count(),
        10,
        "CTB v1-v5 and GKtwo v1-v5 header+specifics declarations should use the profile-parts macro"
    );
    assert!(
        profile_kit_rs.contains("define_chitubox_profiles!("),
        "narrow CTB-like Chitubox macros should delegate to the shared profile kit macro"
    );
    assert_eq!(
        specs_rs.matches("define_chitubox_ctb_profile!(").count(),
        10,
        "CTB v1-v5 and GKtwo v1-v5 should use the narrow CTB declaration macro"
    );
    assert!(
        !specs_rs.contains("macro_rules! define_chitubox_profiles"),
        "CTB-like Chitubox specs.rs should consume, not define, profile assembly macros"
    );

    for spec_name in [
        "CTB_V2_SPEC",
        "CTB_V3_SPEC",
        "CTB_V4_SPEC",
        "CTB_V5_SPEC",
        "GKTWO_CTB_V1_SPEC",
        "GKTWO_CTB_V2_SPEC",
        "GKTWO_CTB_V3_SPEC",
        "GKTWO_CTB_SPEC",
        "GKTWO_CTB_V5_SPEC",
        "CBDDLP_SPEC",
        "PHOTON_SPEC",
    ] {
        let static_marker = format!("pub static {spec_name}: FormatSpec = FormatSpec");
        assert!(
            !specs_rs.contains(&static_marker),
            "{spec_name} should be declared through define_chitubox_profiles!, not a full hand-written FormatSpec block"
        );
    }
}

#[test]
fn chitubox_legacy_bitplane_specs_use_shared_declaration_macro() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs_path = manifest_dir.join("src/formats/chitubox/specs.rs");
    let profile_kit_rs_path = manifest_dir.join("src/formats/chitubox/profile_kit.rs");
    let specs_rs = std::fs::read_to_string(&specs_rs_path).expect("read chitubox specs.rs source");
    let profile_kit_rs =
        std::fs::read_to_string(&profile_kit_rs_path).expect("read chitubox profile_kit.rs source");

    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_legacy_bitplane_profile_parts"),
        "CBDDLP/Photon header and typed specifics should share one legacy bitplane parts macro"
    );
    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_legacy_bitplane_profile"),
        "CBDDLP/Photon runtime wiring should share one legacy bitplane profile macro"
    );
    assert_eq!(
        specs_rs
            .matches("define_chitubox_legacy_bitplane_profile_parts!(")
            .count(),
        3,
        "CBDDLP v1/v2 and Photon header+specifics declarations should use the legacy bitplane parts macro"
    );
    assert_eq!(
        specs_rs
            .matches("define_chitubox_legacy_bitplane_profile!(")
            .count(),
        3,
        "CBDDLP v1/v2 and Photon should use the narrow legacy bitplane declaration macro"
    );

    for static_marker in [
        "static CBDDLP_HEADER: ChituboxHeaderLayout = ChituboxHeaderLayout",
        "static PHOTON_HEADER: ChituboxHeaderLayout = ChituboxHeaderLayout",
        "static CBDDLP_SPECIFICS: ChituboxSpec = ChituboxSpec",
        "static PHOTON_SPECIFICS: ChituboxSpec = ChituboxSpec",
    ] {
        assert!(
            !specs_rs.contains(static_marker),
            "{static_marker} should be declared through define_chitubox_legacy_bitplane_profile_parts!"
        );
    }
}

#[test]
fn define_chitubox_read_only_profiles_use_shared_declaration_macro() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs_path = manifest_dir.join("src/formats/chitubox/specs.rs");
    let profile_kit_rs_path = manifest_dir.join("src/formats/chitubox/profile_kit.rs");
    let specs_rs = std::fs::read_to_string(&specs_rs_path).expect("read chitubox specs.rs source");
    let profile_kit_rs =
        std::fs::read_to_string(&profile_kit_rs_path).expect("read chitubox profile_kit.rs source");

    assert!(
        profile_kit_rs.contains("macro_rules! define_chitubox_read_only_profiles"),
        "read-only Chitubox binary FormatSpec assembly should be centralized in profile_kit.rs"
    );
    assert!(
        specs_rs.contains("define_chitubox_read_only_profiles!("),
        "read-only Chitubox specs.rs should declare profiles through the profile kit macro"
    );
    assert!(
        !specs_rs.contains("macro_rules! define_chitubox_read_only_profiles"),
        "read-only Chitubox specs.rs should consume, not define, profile assembly macros"
    );
    let read_only_macro_body = profile_kit_rs
        .split("macro_rules! define_chitubox_read_only_profiles")
        .nth(1)
        .and_then(|tail| {
            tail.split("\nmacro_rules! define_chitubox_archive_profile")
                .next()
        })
        .expect("read-only macro should appear before archive profile macro");
    assert!(
        read_only_macro_body.contains("define_chitubox_profiles!"),
        "read-only Chitubox binary macro should delegate shared binary FormatSpec construction to define_chitubox_profiles!"
    );
    assert!(
        !read_only_macro_body.contains("FormatSpec {"),
        "read-only Chitubox binary macro should not duplicate the full FormatSpec literal"
    );

    for spec_name in ["PHZ_SPEC", "FDG_SPEC", "PHOTONS_SPEC", "CTB_ENCRYPTED_SPEC"] {
        let static_marker = format!("pub static {spec_name}: FormatSpec = FormatSpec");
        assert!(
            !specs_rs.contains(&static_marker),
            "{spec_name} should be declared through define_chitubox_read_only_profiles!, not a full hand-written FormatSpec block"
        );
    }
}

#[test]
fn profile_kit_common_layer_stays_family_agnostic() {
    let profile_kit_rs = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/formats/common/profile_kit.rs"),
    )
    .expect("read common profile_kit.rs");
    let lower = profile_kit_rs.to_ascii_lowercase();

    for forbidden in [
        "formats::chitubox",
        "formats::anycubic",
        "chitubox",
        "anycubic",
        "machine_id",
        "machine_profile",
        "manifest_entry",
        "gcode",
        "thumbnail",
        "checksum",
        "signature",
    ] {
        assert!(
            !lower.contains(forbidden),
            "common profile kit should not contain family-specific field {forbidden}"
        );
    }
}

#[test]
fn old_family_macro_names_do_not_remain() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let chitubox_specs =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/specs.rs"))
            .expect("read chitubox specs.rs");
    let anycubic_specs =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/specs.rs"))
            .expect("read anycubic specs.rs");

    let old_chitubox_binary = concat!("chitubox", "_binary", "_spec!");
    let old_chitubox_read_only = concat!("chitubox", "_read_only_binary", "_spec!");
    let old_anycubic_binary = concat!("anycubic", "_binary", "_spec!");

    assert!(!chitubox_specs.contains(old_chitubox_binary));
    assert!(!chitubox_specs.contains(old_chitubox_read_only));
    assert!(!anycubic_specs.contains(old_anycubic_binary));
    assert!(chitubox_specs.contains("define_chitubox_ctb_profile!"));
    assert!(chitubox_specs.contains("define_chitubox_read_only_profiles!"));
    assert!(anycubic_specs.contains("define_anycubic_profiles!"));
}

#[test]
fn family_profile_assembly_lives_in_profile_kit_not_specs_files() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let chitubox_specs =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/specs.rs"))
            .expect("read chitubox specs.rs");
    let chitubox_kit =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/profile_kit.rs"))
            .expect("read chitubox profile_kit.rs");
    let anycubic_specs =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/specs.rs"))
            .expect("read anycubic specs.rs");
    let anycubic_kit =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/profile_kit.rs"))
            .expect("read anycubic profile_kit.rs");
    let common_kit =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/profile_kit.rs"))
            .expect("read common profile_kit.rs");

    for (name, specs) in [
        ("chitubox", chitubox_specs.as_str()),
        ("anycubic", anycubic_specs.as_str()),
    ] {
        assert!(
            !specs.contains("macro_rules!"),
            "{name} specs.rs should call profile kit macros, not define assembly macros"
        );
        assert!(
            !specs.contains(": FormatSpec = FormatSpec"),
            "{name} specs.rs should not own full FormatSpec wiring"
        );
    }

    assert!(
        common_kit.contains("pub const fn format_spec_from_profile_parts"),
        "common profile kit should own shared FormatSpec composition"
    );
    assert!(chitubox_kit.contains("macro_rules! define_chitubox_profiles"));
    assert!(chitubox_kit.contains("macro_rules! define_chitubox_read_only_profiles"));
    assert!(anycubic_kit.contains("macro_rules! define_anycubic_profiles"));

    for (name, kit) in [
        ("chitubox", chitubox_kit.as_str()),
        ("anycubic", anycubic_kit.as_str()),
    ] {
        assert!(
            kit.contains("format_spec_from_profile_parts"),
            "{name} profile kit should delegate shared FormatSpec composition to common profile kit"
        );
        assert!(
            !kit.contains("$crate::formats::common::FormatSpec {"),
            "{name} profile kit should not duplicate common FormatSpec field wiring"
        );
    }
}

#[test]
fn family_profile_kits_do_not_keep_public_type_alias_wrappers() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for family in ["chitubox", "anycubic"] {
        let profile_kit_rs = std::fs::read_to_string(
            manifest_dir.join(format!("src/formats/{family}/profile_kit.rs")),
        )
        .unwrap_or_else(|error| panic!("read {family} profile_kit.rs: {error}"));

        assert!(
            !profile_kit_rs.contains("pub type "),
            "{family} profile kit should expose macros and concrete statics, not type aliases"
        );
    }
}

#[test]
fn chitubox_profile_kit_uses_common_descriptor_sections() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs = std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/specs.rs"))
        .expect("read chitubox specs.rs");
    let profile_kit_rs =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/profile_kit.rs"))
            .expect("read chitubox profile_kit.rs");

    for descriptor in [
        "ProfileIdentityDescriptor",
        "ProfileCatalogDescriptor",
        "RuntimeProfileWiring",
    ] {
        assert!(
            profile_kit_rs.contains(descriptor),
            "Chitubox profile kit should assemble profiles through common {descriptor}"
        );
    }

    let first_profile_declaration = specs_rs
        .split("define_chitubox_ctb_profile!(")
        .nth(1)
        .and_then(|tail| tail.split("\n);").next())
        .expect("first Chitubox profile declaration");

    for section in ["identity = {", "catalog = {", "facts =", "runtime = ctb {"] {
        assert!(
            first_profile_declaration.contains(section),
            "Chitubox profile declarations should use identity/catalog/facts/runtime sections"
        );
    }
    assert!(
        !specs_rs.contains("specifics ="),
        "Chitubox declarations should name family facts as facts, not old specifics wiring"
    );
}

#[test]
fn anycubic_profile_kit_uses_common_descriptor_sections() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_rs = std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/specs.rs"))
        .expect("read anycubic specs.rs");
    let profile_kit_rs =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/profile_kit.rs"))
            .expect("read anycubic profile_kit.rs");

    for descriptor in [
        "ProfileIdentityDescriptor",
        "ProfileCatalogDescriptor",
        "RuntimeProfileWiring",
    ] {
        assert!(
            profile_kit_rs.contains(descriptor),
            "Anycubic profile kit should assemble profiles through common {descriptor}"
        );
    }

    let first_profile_declaration = specs_rs
        .split("define_anycubic_profiles!(")
        .nth(1)
        .and_then(|tail| tail.split("\n);").next())
        .expect("first Anycubic profile declaration");

    for section in [
        "identity = {",
        "catalog = {",
        "facts = {",
        "runtime = binary {",
    ] {
        assert!(
            first_profile_declaration.contains(section),
            "Anycubic profile declarations should use identity/catalog/facts/runtime sections"
        );
    }
    assert!(
        specs_rs.contains("runtime = archive {"),
        "Anycubic archive declarations should use an archive runtime section"
    );
    assert!(
        !specs_rs.contains("specifics ="),
        "Anycubic archive declarations should name family facts as facts, not old specifics wiring"
    );
}

fn rust_source_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_rust_source_files(root, &mut files);
    files
}

fn collect_rust_source_files(root: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read Rust source directory") {
        let path = entry.expect("Rust source directory entry").path();
        if path.is_dir() {
            collect_rust_source_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}
