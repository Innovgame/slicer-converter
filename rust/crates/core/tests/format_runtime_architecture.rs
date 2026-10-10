use slicer_converter_core::formats::common::keys;

#[test]
fn removed_runtime_bridge_is_absent_from_format_modules() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_mod_path = manifest_dir.join("src/formats/common/mod.rs");
    let common_adapter_path = manifest_dir.join("src/formats/common/adapter.rs");

    let common_mod = std::fs::read_to_string(&common_mod_path).expect("read common mod.rs");

    assert!(
        common_mod.contains("pub mod spec;"),
        "common module should expose the FormatSpec kernel"
    );
    assert!(
        !common_adapter_path.exists(),
        "runtime bridge module should be removed once no spec uses it"
    );
}

#[test]
fn layer_context_types_live_in_common_context_module() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats");
    let common_mod = std::fs::read_to_string(manifest_dir.join("src/formats/common/mod.rs"))
        .expect("read common mod.rs");

    assert!(
        common_mod.contains("pub mod context;"),
        "common module should own layer context types"
    );
    for context_type in [
        "pub struct LayerOptions",
        "pub struct LayerDecodeContext",
        "pub struct LayerEncodeContext",
    ] {
        assert!(
            !formats_mod.contains(context_type),
            "formats facade should re-export {context_type}, not define it"
        );
    }
}

#[test]
fn format_identity_lives_with_format_spec_kernel() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats");
    let spec_source = std::fs::read_to_string(manifest_dir.join("src/formats/common/spec.rs"))
        .expect("read common spec.rs");
    let common_mod = std::fs::read_to_string(manifest_dir.join("src/formats/common/mod.rs"))
        .expect("read common mod.rs");

    assert!(
        !formats_mod.contains("pub struct FormatIdentity"),
        "formats facade should re-export FormatIdentity, not define it"
    );
    assert!(
        spec_source.contains("pub struct FormatIdentity"),
        "FormatIdentity should live beside FormatSpec"
    );
    assert!(
        common_mod.contains("FormatIdentity"),
        "common module should re-export FormatIdentity"
    );
}

#[test]
fn metadata_builder_keeps_unchecked_escape_hatch_crate_private() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let metadata_source = std::fs::read_to_string(manifest_dir.join("src/common/metadata.rs"))
        .expect("read common metadata.rs");

    assert!(
        metadata_source.contains("pub fn build(self)"),
        "validated metadata construction should stay public"
    );
    assert!(
        metadata_source.contains("pub(crate) fn build_unchecked(self)"),
        "unchecked metadata construction should be reserved for in-crate parsers"
    );
    assert!(
        !metadata_source.contains("pub fn build_unchecked(self)"),
        "unchecked metadata construction should not remain on the external Rust API"
    );
}

#[test]
fn metadata_comments_describe_stable_schema_not_refactor_steps() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let metadata_source = std::fs::read_to_string(manifest_dir.join("src/common/metadata.rs"))
        .expect("read common metadata.rs");

    for stale_marker in ["T1.2", "concern slice", "Phase ", "Step 2", "本 Step"] {
        assert!(
            !metadata_source.contains(stale_marker),
            "metadata.rs should not keep completed refactor planning marker {stale_marker}"
        );
    }
}

#[test]
fn cross_format_document_tests_use_pipeline_not_tuple_parse_wrapper() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cross_format_source = std::fs::read_to_string(manifest_dir.join("tests/cross_format.rs"))
        .expect("read cross_format tests");

    for stale_fragment in [".parse_header(data)", "fmt.parse_header("] {
        assert!(
            !cross_format_source.contains(stale_fragment),
            "cross-format document tests should parse through FormatPipeline, not tuple wrapper fragment {stale_fragment}"
        );
    }
}

#[test]
fn fixture_document_tests_use_pipeline_not_tuple_parse_wrapper() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_source = std::fs::read_to_string(manifest_dir.join("tests/fixture_files.rs"))
        .expect("read fixture tests");

    for stale_fragment in [".parse_header(data)", "fmt.parse_header("] {
        assert!(
            !fixture_source.contains(stale_fragment),
            "fixture document tests should parse through FormatPipeline, not tuple wrapper fragment {stale_fragment}"
        );
    }
}

#[test]
fn format_spec_does_not_keep_tuple_parse_header_wrapper() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composed_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/composed.rs"))
            .expect("read composed format spec");

    assert!(
        !composed_source.contains("pub fn parse_header("),
        "FormatSpec should expose document parsing through FormatPipeline, not an older tuple wrapper"
    );
    assert!(
        !composed_source.contains("tuple-shaped API"),
        "FormatSpec docs should not keep stale tuple wrapper compatibility wording"
    );
}

#[test]
fn common_pipeline_does_not_keep_unused_catalog_only_placeholder() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/pipeline.rs"))
            .expect("read common pipeline.rs");
    let common_mod_source = std::fs::read_to_string(manifest_dir.join("src/formats/common/mod.rs"))
        .expect("read common mod.rs");

    assert!(
        !pipeline_source.contains("CatalogOnlyPipeline"),
        "common pipeline should not keep an unused catalog-only placeholder"
    );
    assert!(
        !common_mod_source.contains("CatalogOnlyPipeline"),
        "common module should not re-export an unused catalog-only pipeline"
    );
}

#[test]
fn goo_family_runtime_uses_format_specs_not_concrete_format_wrappers() {
    let goo = slicer_converter_core::formats::get_format_spec_by_name("goo").expect("goo spec");
    let prz = slicer_converter_core::formats::get_format_spec_by_name("prz").expect("prz spec");

    assert_eq!(goo.identity.name, "goo");
    assert_eq!(goo.keys.header_key, keys::HEADER_GOO_FAMILY);
    assert_eq!(goo.keys.layer_codec_key, keys::GOO_RLE);
    assert!(goo.capabilities.can_import);
    assert!(goo.capabilities.can_export);

    assert_eq!(prz.identity.name, "prz");
    assert_eq!(prz.keys.header_key, keys::HEADER_GOO_FAMILY);
    assert_eq!(prz.keys.layer_codec_key, keys::GOO_RLE);
    assert!(prz.capabilities.can_import);
    assert!(prz.capabilities.can_export);
}

#[test]
fn anycubic_family_runtime_uses_format_specs_not_concrete_format_wrappers() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let anycubic_mod_path = manifest_dir.join("src/formats/anycubic/mod.rs");
    let anycubic_format_path = manifest_dir.join("src/formats/anycubic/format.rs");

    let anycubic_mod = std::fs::read_to_string(&anycubic_mod_path).expect("read anycubic mod.rs");
    let pw0 = slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0 spec");
    let pm5s = slicer_converter_core::formats::get_format_spec_by_name("pm5s").expect("pm5s spec");

    assert!(anycubic_mod.contains("pub mod specs;"));
    assert!(!anycubic_format_path.exists());
    assert!(
        !anycubic_mod.contains("pub use decoder::parse_anycubic_v1_header_data"),
        "Anycubic facade should not publicly re-export the v1 parser helper"
    );
    assert_eq!(pw0.keys.header_key, keys::HEADER_ANYCUBIC_V1);
    assert_eq!(pw0.keys.layer_codec_key, keys::ANYCUBIC_PW0_RLE);
    assert_eq!(pm5s.keys.header_key, keys::HEADER_ANYCUBIC_V518);
    assert_eq!(pm5s.keys.layer_codec_key, keys::ANYCUBIC_PW0_RLE);
}

#[test]
fn public_format_api_exposes_specs_and_family_registry() {
    let registry = slicer_converter_core::formats::global_registry();
    let names = registry
        .all_specs()
        .iter()
        .map(|spec| spec.identity.name)
        .collect::<std::collections::BTreeSet<_>>();

    assert!(names.contains("ctb"));
    assert!(names.contains("goo"));
    assert!(names.contains("pw0"));
    assert!(names.contains("pm5s"));
    assert!(registry.spec_by_extension("ctb").is_some());
    assert!(registry.spec_by_extension("goo").is_some());
}

#[test]
fn registry_lookup_delegates_identity_matching_to_format_spec() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats");
    let resolver_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/resolver.rs"))
            .expect("read resolver");

    assert!(
        formats_mod.contains("spec.matches_name(name)"),
        "registry name lookup should use FormatSpec identity matching"
    );
    assert!(
        formats_mod.contains("spec.matches_extension(extension)"),
        "registry extension lookup should use FormatSpec extension matching"
    );
    assert!(
        resolver_source.contains("spec.matches_name(request)"),
        "profile resolvers should use FormatSpec identity matching for scoped requests"
    );

    for source in [formats_mod.as_str(), resolver_source.as_str()] {
        assert!(
            !source.contains("spec.profile.target_names.iter()"),
            "registry/resolver code should not duplicate target-name matching"
        );
        assert!(
            !source.contains("spec.identity.extensions"),
            "registry/resolver code should not duplicate extension matching"
        );
    }
}

#[test]
fn registry_detection_delegates_profile_detection_to_family_resolvers() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats");
    let detect_spec_body = formats_mod
        .split("fn detect_spec(")
        .nth(1)
        .expect("detect_spec method exists")
        .split("/// Lookup a concrete profile spec")
        .next()
        .expect("detect_spec body is bounded");
    let compact_detect_spec_body = detect_spec_body.split_whitespace().collect::<String>();

    assert!(
        compact_detect_spec_body
            .contains("family.resolver.detect_profile(data,extension_hint,family.profiles)"),
        "registry detection should delegate profile detection to each family resolver"
    );
    assert!(
        !detect_spec_body.contains(".header.try_detect("),
        "registry detection should not duplicate per-profile header detection"
    );
}

#[test]
fn registry_export_target_family_matching_uses_family_identity_helper() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let formats_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/mod.rs")).expect("read formats");
    let resolver_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/resolver.rs"))
            .expect("read resolver");

    assert!(
        !formats_mod.contains("struct ExportTargetRequest"),
        "formats facade should not own export target request normalization"
    );

    assert!(
        resolver_source.contains("family.matches_name(candidate)"),
        "export target request should delegate family identity matching to FormatFamilySpec"
    );
    assert!(
        !resolver_source.contains("family.identity.aliases")
            && !resolver_source.contains("family.identity.extensions"),
        "export target request should not duplicate family alias/extension matching"
    );
}

#[test]
fn cli_export_target_selection_delegates_to_format_registry() {
    let main_source = std::fs::read_to_string(
        rust_workspace_root()
            .join("crates")
            .join("cli")
            .join("src")
            .join("main.rs"),
    )
    .expect("read CLI main.rs");
    let target_selection_body = main_source
        .split("fn target_format_for_output(")
        .nth(1)
        .expect("target_format_for_output exists")
        .split("fn ensure_format_can_import(")
        .next()
        .expect("target_format_for_output body is bounded");

    assert!(
        target_selection_body.contains(".resolve_export_spec("),
        "CLI output target selection should delegate export profile resolution to the registry"
    );
    for hardcoded_profile in ["ctb-v3", "ctb-v4", "ctb-v5", "gktwo.ctb"] {
        assert!(
            !target_selection_body.contains(hardcoded_profile),
            "CLI output target selection should not hard-code {hardcoded_profile}"
        );
    }
}

fn rust_workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("core crate should live in rust/crates/core")
        .to_path_buf()
}
