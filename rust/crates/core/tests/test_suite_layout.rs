#[test]
fn chitubox_profile_tests_are_split_from_family_registry_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_test_path = manifest_dir.join("tests/format_family_registry.rs");
    let split_test_path = manifest_dir.join("tests/chitubox_profiles.rs");
    let registry_source =
        std::fs::read_to_string(&registry_test_path).expect("read format_family_registry.rs");

    assert!(
        split_test_path.is_file(),
        "Chitubox profile behavior should live in tests/chitubox_profiles.rs"
    );

    for test_name in [
        "chitubox_family_exposes_distinct_spec_profiles_for_extension_targets",
        "chitubox_photon_profile_builds_cbddlp_magic_header",
        "chitubox_export_profiles_follow_uvtools_suffix_magic_version_matrix",
        "chitubox_family_exposes_special_profiles_with_current_import_export_capabilities",
        "chitubox_special_profiles_use_active_profile_keys",
        "all_uvtools_chitubox_binary_profiles_are_export_capable",
        "fdg_pipeline_parses_generated_fixture_and_decodes_gray_rle",
        "fdg_pipeline_encodes_parseable_document",
        "phz_pipeline_parses_generated_fixture_and_decodes_gray_rle",
        "phz_pipeline_encodes_parseable_document",
        "photons_pipeline_parses_generated_fixture_and_decodes_binary_rle",
        "photons_pipeline_encodes_parseable_document",
        "encrypted_ctb_pipeline_parses_aes_settings_and_layer_pointers",
        "encrypted_ctb_pipeline_encodes_parseable_v5_document",
        "chitubox_special_profile_magic_detection_does_not_reuse_plain_ctb_specs",
        "chitubox_family_exposes_gktwo_as_ctb_v4_profile",
        "chitubox_profiles_publish_concrete_strategy_keys",
        "chitubox_ctb_profiles_use_explicit_xor_crypto_policy",
        "chitubox_profile_detection_uses_magic_before_extension_aliases",
        "chitubox_profile_detection_splits_gktwo_magic_from_generic_ctb_v4",
        "chitubox_profile_detection_splits_ctb_versions_with_shared_magic",
        "chitubox_ctb_v4_detects_legacy_magic_version_4",
        "chitubox_header_layout_rejects_headers_outside_profile",
        "chitubox_header_build_uses_profile_magic_and_version",
        "chitubox_ctb_v4_layout_preserves_profile_identity_on_finalize",
        "chitubox_ctb_v4_layout_uses_extended_layer_table_stride",
        "chitubox_ctb_v4_parser_accepts_uvtools_layer_def_ex_blocks_between_layers",
        "chitubox_ctb_v4_layout_writes_print_parameters_v4_table",
        "chitubox_ctb_v5_layout_writes_resin_parameters_table",
        "chitubox_ctb_v4_layout_parses_slicer_and_print_parameters_v4",
        "chitubox_export_resolution_targets_concrete_binary_profile",
        "chitubox_export_resolution_targets_gktwo_profile",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !registry_source.contains(&marker),
            "{test_name} should live in tests/chitubox_profiles.rs, not format_family_registry.rs"
        );
    }
}

#[test]
fn archive_profile_tests_are_split_from_family_registry_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_test_path = manifest_dir.join("tests/format_family_registry.rs");
    let split_test_path = manifest_dir.join("tests/archive_profiles.rs");
    let registry_source =
        std::fs::read_to_string(&registry_test_path).expect("read format_family_registry.rs");

    assert!(
        split_test_path.is_file(),
        "archive profile behavior should live in tests/archive_profiles.rs"
    );

    for test_name in [
        "archive_formats_expose_archive_container_keys",
        "archive_specs_encode_documents_through_zip_pipeline",
        "archive_specs_can_encode_documents_from_streaming_layer_source",
        "png_archive_specs_publish_archive_strategy_keys",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !registry_source.contains(&marker),
            "{test_name} should live in tests/archive_profiles.rs, not format_family_registry.rs"
        );
    }
}

#[test]
fn anycubic_zip_tests_are_split_from_family_registry_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_test_path = manifest_dir.join("tests/format_family_registry.rs");
    let split_test_path = manifest_dir.join("tests/anycubic_zip_profiles.rs");
    let registry_source =
        std::fs::read_to_string(&registry_test_path).expect("read format_family_registry.rs");

    assert!(
        split_test_path.is_file(),
        "Anycubic Zip behavior should live in tests/anycubic_zip_profiles.rs"
    );

    for test_name in [
        "anycubic_zip_pipeline_parses_manifests_and_pw0_layers",
        "anycubic_zip_archive_export_builds_printer_native_entries",
        "anycubic_zip_pipeline_parses_scene_and_calc_manifests",
        "anycubic_zip_layer_codec_decodes_pwsz_contour_layers",
        "anycubic_zip_layer_codec_decodes_pwsz_contour_holes",
        "anycubic_zip_layer_codec_unions_overlapping_pwsz_positive_contours",
        "anycubic_zip_layer_codec_decodes_pwsz_interleaved_contour_records",
        "anycubic_zip_layer_codec_decodes_pwsz_single_point_contours",
        "anycubic_zip_layer_codec_decodes_dense_pwsz_contours_quickly",
        "anycubic_zip_real_pwszimg_decode_area_tracks_scene_manifest_when_configured",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !registry_source.contains(&marker),
            "{test_name} should live in tests/anycubic_zip_profiles.rs, not format_family_registry.rs"
        );
    }
}

#[test]
fn chitubox_zip_tests_are_split_from_family_registry_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_test_path = manifest_dir.join("tests/format_family_registry.rs");
    let split_test_path = manifest_dir.join("tests/chitubox_zip_profiles.rs");
    let registry_source =
        std::fs::read_to_string(&registry_test_path).expect("read format_family_registry.rs");

    assert!(
        split_test_path.is_file(),
        "Chitubox Zip behavior should live in tests/chitubox_zip_profiles.rs"
    );

    for test_name in ["chitubox_zip_pipeline_parses_gcode_metadata_and_png_layers"] {
        let marker = format!("fn {test_name}");
        assert!(
            !registry_source.contains(&marker),
            "{test_name} should live in tests/chitubox_zip_profiles.rs, not format_family_registry.rs"
        );
    }
}

#[test]
fn anycubic_profile_tests_are_split_from_family_registry_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_test_path = manifest_dir.join("tests/format_family_registry.rs");
    let split_test_path = manifest_dir.join("tests/anycubic_profiles.rs");
    let registry_source =
        std::fs::read_to_string(&registry_test_path).expect("read format_family_registry.rs");

    assert!(
        split_test_path.is_file(),
        "Anycubic binary profile behavior should live in tests/anycubic_profiles.rs"
    );

    for test_name in [
        "anycubic_family_exposes_all_existing_machine_profiles",
        "anycubic_profiles_use_explicit_no_footer_policy",
        "anycubic_profiles_publish_concrete_strategy_keys",
        "anycubic_strategy_adapters_accept_direct_layout_and_codec_slots",
        "anycubic_spec_slots_match_direct_execution_behavior",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !registry_source.contains(&marker),
            "{test_name} should live in tests/anycubic_profiles.rs, not format_family_registry.rs"
        );
    }
}

#[test]
fn transcode_architecture_tests_are_split_from_format_spec_kernel_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel_test_path = manifest_dir.join("tests/format_spec_kernel.rs");
    let split_test_path = manifest_dir.join("tests/transcode_architecture.rs");
    let kernel_source =
        std::fs::read_to_string(&kernel_test_path).expect("read format_spec_kernel.rs");

    assert!(
        split_test_path.is_file(),
        "transcode architecture source guards should live in tests/transcode_architecture.rs"
    );

    for test_name in [
        "transcode_registry_lookup_is_route_first",
        "layer_transcode_request_does_not_duplicate_route_key_source",
        "transcode_route_constructors_are_spec_only",
        "transcode_registry_entries_declare_specs_not_precomputed_route_keys",
        "layer_transcoders_do_not_own_route_selection",
        "transcode_engine_uses_route_descriptor_for_dispatch",
        "transcode_engine_derives_route_descriptor_from_specs",
        "public_transcode_entrypoints_build_route_descriptors_before_layer_requests",
        "public_transcode_entrypoints_do_not_pass_route_descriptors_to_engine",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !kernel_source.contains(&marker),
            "{test_name} should live in tests/transcode_architecture.rs, not format_spec_kernel.rs"
        );
    }
}

#[test]
fn wasm_architecture_tests_are_split_from_format_spec_kernel_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel_test_path = manifest_dir.join("tests/format_spec_kernel.rs");
    let split_test_path = rust_workspace_root()
        .join("crates")
        .join("wasm")
        .join("tests")
        .join("wasm_api_architecture.rs");
    let kernel_source =
        std::fs::read_to_string(&kernel_test_path).expect("read format_spec_kernel.rs");

    assert!(
        split_test_path.is_file(),
        "wasm API architecture source guards should live in crates/wasm/tests/wasm_api_architecture.rs"
    );

    for test_name in [
        "wasm_decode_encode_export_stays_thin_facade",
        "wasm_binary_exports_return_js_uint8array",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !kernel_source.contains(&marker),
            "{test_name} should live in crates/wasm/tests/wasm_api_architecture.rs, not format_spec_kernel.rs"
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

#[test]
fn format_runtime_architecture_tests_are_split_from_format_spec_kernel_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel_test_path = manifest_dir.join("tests/format_spec_kernel.rs");
    let split_test_path = manifest_dir.join("tests/format_runtime_architecture.rs");
    let kernel_source =
        std::fs::read_to_string(&kernel_test_path).expect("read format_spec_kernel.rs");

    assert!(
        split_test_path.is_file(),
        "format runtime architecture source guards should live in tests/format_runtime_architecture.rs"
    );

    for test_name in [
        "removed_runtime_bridge_is_absent_from_format_modules",
        "goo_family_runtime_uses_format_specs_not_concrete_format_wrappers",
        "anycubic_family_runtime_uses_format_specs_not_concrete_format_wrappers",
        "public_format_api_exposes_specs_and_family_registry",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !kernel_source.contains(&marker),
            "{test_name} should live in tests/format_runtime_architecture.rs, not format_spec_kernel.rs"
        );
    }
}

#[test]
fn archive_architecture_tests_are_split_from_format_spec_kernel_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel_test_path = manifest_dir.join("tests/format_spec_kernel.rs");
    let split_test_path = manifest_dir.join("tests/archive_architecture.rs");
    let kernel_source =
        std::fs::read_to_string(&kernel_test_path).expect("read format_spec_kernel.rs");

    assert!(
        split_test_path.is_file(),
        "archive architecture source guards should live in tests/archive_architecture.rs"
    );

    for test_name in [
        "anycubic_zip_archive_has_focused_modules",
        "chitubox_zip_archive_has_focused_modules",
        "archive_output_has_no_legacy_whole_zip_builders",
        "vdt_archive_entry_policy_is_factored_out_of_decoder_and_adapter_glue",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !kernel_source.contains(&marker),
            "{test_name} should live in tests/archive_architecture.rs, not format_spec_kernel.rs"
        );
    }
}

#[test]
fn format_spec_kernel_keeps_suite_layout_guards_out_of_kernel_suite() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kernel_test_path = manifest_dir.join("tests/format_spec_kernel.rs");
    let kernel_source =
        std::fs::read_to_string(&kernel_test_path).expect("read format_spec_kernel.rs");

    for test_name in [
        "chitubox_profile_tests_are_split_from_family_registry_suite",
        "archive_profile_tests_are_split_from_family_registry_suite",
        "anycubic_zip_tests_are_split_from_family_registry_suite",
        "chitubox_zip_tests_are_split_from_family_registry_suite",
        "anycubic_profile_tests_are_split_from_family_registry_suite",
        "transcode_architecture_tests_are_split_from_format_spec_kernel_suite",
        "wasm_architecture_tests_are_split_from_format_spec_kernel_suite",
        "format_runtime_architecture_tests_are_split_from_format_spec_kernel_suite",
        "archive_architecture_tests_are_split_from_format_spec_kernel_suite",
    ] {
        let marker = format!("fn {test_name}");
        assert!(
            !kernel_source.contains(&marker),
            "{test_name} should live in tests/test_suite_layout.rs, not format_spec_kernel.rs"
        );
    }
}
