use slicer_converter_core::common::FileMetadata;
use slicer_converter_core::formats::ctb::encode_layer_data;
use slicer_converter_wasm::{
    build_archive_metadata_entries, decode_encode_layer_stats, describe_archive_layer_entries,
    diagnose_export, list_supported_formats,
};

#[test]
fn supported_format_infos_include_unified_capabilities_when_available() {
    let json = list_supported_formats().expect("supported formats JSON");
    let infos: serde_json::Value = serde_json::from_str(&json).expect("supported formats value");
    let infos = infos.as_array().expect("supported formats array");

    let ctb = format_info(infos, "ctb");
    assert_eq!(ctb["display_name"], "Chitubox CTB v2");
    assert_eq!(
        string_array(&ctb["target_names"]),
        vec!["ctb", "ctb-v2", "v2.ctb"]
    );
    assert_eq!(ctb["capabilities"]["can_import"], true);
    assert_eq!(ctb["capabilities"]["supports_anti_alias"], true);
    assert_eq!(ctb["capabilities"]["supports_encryption"], true);

    let pw0 = format_info(infos, "pw0");
    assert_eq!(pw0["capabilities"]["can_export"], true);
    assert_eq!(pw0["capabilities"]["supports_preview"], true);

    let sl1 = format_info(infos, "sl1");
    assert_eq!(sl1["capabilities"]["is_archive"], true);
    assert_eq!(sl1["capabilities"]["requires_full_document_parse"], true);
}

#[test]
fn supported_format_infos_expose_anycubic_archive_extension_matrix() {
    let json = list_supported_formats().expect("supported formats JSON");
    let infos: serde_json::Value = serde_json::from_str(&json).expect("supported formats value");
    let infos = infos.as_array().expect("supported formats array");
    let archive_targets = ["anycubic-zip", "pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"];
    let archive_extensions = ["pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"];

    let anycubic_zip = format_info(infos, "anycubic-zip");
    assert_eq!(anycubic_zip["family"], "anycubic");
    assert_eq!(anycubic_zip["is_zip"], true);
    assert_eq!(anycubic_zip["default_extension"], "pm4u");
    assert_eq!(anycubic_zip["capabilities"]["can_import"], true);
    assert_eq!(anycubic_zip["capabilities"]["can_export"], true);
    assert_eq!(
        string_array(&anycubic_zip["aliases"]),
        archive_targets.to_vec()
    );
    assert_eq!(
        string_array(&anycubic_zip["extensions"]),
        archive_extensions.to_vec()
    );
    assert_eq!(
        string_array(&anycubic_zip["target_names"]),
        archive_targets.to_vec()
    );

    let specifics = &anycubic_zip["specifics"];
    assert_eq!(specifics["family"], "anycubic-archive");
    assert_eq!(specifics["layer_entry_prefix"], "layer_images/layer_");
    assert_eq!(
        string_array(&specifics["layer_image_extensions"]),
        vec!["pw0Img", "pwszImg"]
    );
    assert_eq!(
        string_array(&specifics["required_manifest_entries"]),
        vec!["anycubic_photon_resins.pwsp", "layers_controller.conf"]
    );
}

#[test]
fn diagnose_export_warns_for_anycubic_machine_profile_mismatch() {
    let metadata = FileMetadata::builder()
        .machine_name("ELEGOO Mars 5 Ultra(1)")
        .resolution(8520, 4320)
        .display_size_mm(55.44, 98.637)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let metadata_json = serde_json::to_string(&metadata).expect("metadata json");

    let json = diagnose_export(&metadata_json, "pw0").expect("diagnostics JSON");
    let diagnostics: serde_json::Value = serde_json::from_str(&json).expect("diagnostics value");
    let warnings = diagnostics["warnings"].as_array().expect("warnings");

    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["code"], "ANYCUBIC_MACHINE_PROFILE_MISMATCH");
    assert_eq!(warnings[0]["severity"], "warning");
    assert_eq!(
        warnings[0]["source"]["display_name"],
        "ELEGOO Mars 5 Ultra(1)"
    );
    assert_eq!(warnings[0]["target"]["format"], "pw0");
    assert_eq!(warnings[0]["target"]["display_name"], "Photon Zero");
    let message = warnings[0]["message"].as_str().expect("message");
    assert!(message.contains("Source machine ELEGOO Mars 5 Ultra(1)"));
    assert!(message.contains("target machine Photon Zero"));
    assert!(message.contains("without layer resampling"));
}

#[test]
fn diagnose_export_returns_no_warnings_for_matching_anycubic_machine_profile() {
    let metadata = FileMetadata::builder()
        .resolution(480, 854)
        .display_size_mm(55.4, 98.63)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let metadata_json = serde_json::to_string(&metadata).expect("metadata json");

    let json = diagnose_export(&metadata_json, "pw0").expect("diagnostics JSON");
    let diagnostics: serde_json::Value = serde_json::from_str(&json).expect("diagnostics value");

    assert_eq!(
        diagnostics["warnings"].as_array().expect("warnings").len(),
        0
    );
}

#[test]
fn diagnose_export_warns_when_photons_export_resamples_layers() {
    let metadata = FileMetadata::builder()
        .resolution(64, 64)
        .display_size_mm(68.04, 120.96)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let metadata_json = serde_json::to_string(&metadata).expect("metadata json");

    let json = diagnose_export(&metadata_json, "photons").expect("diagnostics JSON");
    let diagnostics: serde_json::Value = serde_json::from_str(&json).expect("diagnostics value");
    let warnings = diagnostics["warnings"].as_array().expect("warnings");

    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["code"], "PHOTONS_MACHINE_GEOMETRY_REWRITE");
    assert_eq!(warnings[0]["target"]["resolution_x"], 1440);
    assert_eq!(warnings[0]["target"]["resolution_y"], 2560);
    let message = warnings[0]["message"].as_str().expect("message");
    assert!(message.contains("resamples layers from 64x64 to 1440x2560"));
}

#[test]
fn supported_format_infos_expose_creality_cxdlpv4_specifics() {
    let json = list_supported_formats().expect("supported formats JSON");
    let infos: serde_json::Value = serde_json::from_str(&json).expect("supported formats value");
    let infos = infos.as_array().expect("supported formats array");

    let cxdlp = format_info(infos, "cxdlp");
    assert_eq!(cxdlp["family"], "creality");
    assert_eq!(cxdlp["is_zip"], false);
    assert_eq!(cxdlp["capabilities"]["can_import"], true);
    assert_eq!(cxdlp["capabilities"]["can_export"], false);
    assert_eq!(cxdlp["capabilities"]["supports_preview"], true);
    assert_eq!(cxdlp["capabilities"]["supports_per_layer_settings"], false);
    assert_eq!(cxdlp["capabilities"]["supports_encryption"], false);
    assert_eq!(string_array(&cxdlp["extensions"]), vec!["cxdlp"]);

    let specifics = &cxdlp["specifics"];
    assert_eq!(specifics["family"], "creality");
    assert_eq!(specifics["variant"], "cxdlp");
    assert_eq!(specifics["version"], 3);
    assert_eq!(specifics["extension"], "cxdlp");
    assert_eq!(specifics["magic"], "CXSW3DV2");
    assert_eq!(specifics["checksum"], "crc32");
    assert_eq!(specifics["layer_definition_size"], 8);
    assert_eq!(specifics["layer_parameter_size"], 8);
    assert_eq!(specifics["preview_count"], 3);
    assert_eq!(specifics["supports_layer_encryption"], false);

    let cxdlpv4 = format_info(infos, "cxdlpv4");
    assert_eq!(cxdlpv4["family"], "creality");
    assert_eq!(cxdlpv4["is_zip"], false);
    assert_eq!(cxdlpv4["capabilities"]["can_import"], true);
    assert_eq!(cxdlpv4["capabilities"]["can_export"], true);
    assert_eq!(cxdlpv4["capabilities"]["supports_preview"], true);
    assert_eq!(cxdlpv4["capabilities"]["supports_per_layer_settings"], true);
    assert_eq!(cxdlpv4["capabilities"]["supports_encryption"], true);
    assert_eq!(string_array(&cxdlpv4["extensions"]), vec!["cxdlpv4"]);

    let specifics = &cxdlpv4["specifics"];
    assert_eq!(specifics["family"], "creality");
    assert_eq!(specifics["variant"], "cxdlpv4");
    assert_eq!(specifics["version"], 4);
    assert_eq!(specifics["extension"], "cxdlpv4");
    assert_eq!(specifics["magic"], "CXSW3DV2");
    assert_eq!(specifics["checksum"], "crc32");
    assert_eq!(specifics["layer_definition_size"], 40);
    assert_eq!(specifics["layer_parameter_size"], 44);
    assert_eq!(specifics["preview_count"], 2);
    assert_eq!(specifics["supports_layer_encryption"], true);
}

#[test]
fn supported_format_infos_expose_chitubox_version_specifics() {
    let json = list_supported_formats().expect("supported formats JSON");
    let infos: serde_json::Value = serde_json::from_str(&json).expect("supported formats value");
    let infos = infos.as_array().expect("supported formats array");

    let ctb_v1 = format_info(infos, "ctb-v1");
    assert_eq!(ctb_v1["family"], "chitubox");
    assert_eq!(ctb_v1["is_zip"], false);
    assert_eq!(ctb_v1["capabilities"]["can_export"], true);
    assert_eq!(ctb_v1["default_extension"], "ctb");
    assert_eq!(string_array(&ctb_v1["extensions"]), vec!["v1.ctb"]);

    let specifics = &ctb_v1["specifics"];
    assert_eq!(specifics["family"], "chitubox");
    assert_eq!(specifics["kind"], "ctb");
    assert_eq!(specifics["ctb_version"], 1);
    assert_eq!(specifics["build_version"], 1);
    assert_eq!(number_array(&specifics["allowed_versions"]), vec![1]);
    assert_eq!(specifics["layer_table_entry_size"], 36);

    let cbddlp_v1 = format_info(infos, "cbddlp-v1");
    assert_eq!(cbddlp_v1["family"], "chitubox");
    assert_eq!(cbddlp_v1["is_zip"], false);
    assert_eq!(cbddlp_v1["capabilities"]["can_export"], true);
    assert_eq!(cbddlp_v1["default_extension"], "cbddlp");

    let specifics = &cbddlp_v1["specifics"];
    assert_eq!(specifics["family"], "chitubox");
    assert_eq!(specifics["kind"], "cbddlp");
    assert_eq!(specifics["ctb_version"], 1);
    assert_eq!(specifics["build_version"], 1);
    assert_eq!(number_array(&specifics["allowed_versions"]), vec![1]);
    assert_eq!(specifics["layer_table_entry_size"], 36);

    let cbddlp = format_info(infos, "cbddlp");
    assert_eq!(cbddlp["family"], "chitubox");
    assert_eq!(cbddlp["is_zip"], false);
    assert_eq!(cbddlp["capabilities"]["can_export"], true);

    let specifics = &cbddlp["specifics"];
    assert_eq!(specifics["family"], "chitubox");
    assert_eq!(specifics["kind"], "cbddlp");
    assert_eq!(specifics["ctb_version"], 2);
    assert_eq!(specifics["build_version"], 2);
    assert_eq!(number_array(&specifics["allowed_versions"]), vec![2]);
    assert_eq!(specifics["layer_table_entry_size"], 36);

    for (name, version, stride) in [
        ("gktwo-ctb-v1", 1, 36),
        ("gktwo-ctb-v2", 2, 36),
        ("gktwo-ctb-v3", 3, 84),
        ("gktwo-ctb", 4, 84),
        ("gktwo-ctb-v5", 5, 84),
    ] {
        let gktwo = format_info(infos, name);
        assert_eq!(gktwo["family"], "chitubox");
        assert_eq!(gktwo["is_zip"], false);
        assert_eq!(gktwo["capabilities"]["can_export"], true);
        assert_eq!(gktwo["default_extension"], "gktwo.ctb");

        let specifics = &gktwo["specifics"];
        assert_eq!(specifics["family"], "chitubox");
        assert_eq!(specifics["kind"], "gktwo-ctb");
        assert_eq!(specifics["machine_profile"], "gktwo");
        assert_eq!(specifics["ctb_version"], version);
        assert_eq!(specifics["build_version"], version);
        assert_eq!(number_array(&specifics["allowed_versions"]), vec![version]);
        assert_eq!(specifics["layer_table_entry_size"], stride);
    }

    let chitubox_zip = format_info(infos, "chitubox-zip");
    assert_eq!(chitubox_zip["family"], "chitubox");
    assert_eq!(chitubox_zip["is_zip"], true);
    assert_eq!(chitubox_zip["capabilities"]["can_import"], true);
    assert_eq!(chitubox_zip["capabilities"]["can_export"], true);

    let specifics = &chitubox_zip["specifics"];
    assert_eq!(specifics["family"], "chitubox");
    assert_eq!(specifics["kind"], "zip");
    assert_eq!(specifics["ctb_version"], serde_json::Value::Null);
    assert_eq!(specifics["build_version"], serde_json::Value::Null);
    assert_eq!(
        number_array(&specifics["allowed_versions"]),
        Vec::<u64>::new()
    );
    assert_eq!(specifics["layer_table_entry_size"], serde_json::Value::Null);

    for (name, kind, ctb_version) in [
        ("phz", "phz", Some(2)),
        ("fdg", "fdg", Some(2)),
        ("photons", "photons", None),
        ("encrypted-ctb-v4", "encrypted-ctb", Some(4)),
        ("encrypted-ctb", "encrypted-ctb", Some(5)),
    ] {
        let special = format_info(infos, name);
        assert_eq!(special["family"], "chitubox");
        assert_eq!(special["is_zip"], false);
        assert_eq!(special["capabilities"]["can_import"], true);
        assert_eq!(special["capabilities"]["can_export"], true);
        if name.starts_with("encrypted-ctb") {
            assert_eq!(special["default_extension"], "encrypted.ctb");
        }
        if name == "photons" {
            assert_eq!(
                special["capabilities"]["requires_full_document_parse"],
                true
            );
        }

        let specifics = &special["specifics"];
        assert_eq!(specifics["family"], "chitubox");
        assert_eq!(specifics["kind"], kind);
        match ctb_version {
            Some(version) => assert_eq!(specifics["ctb_version"], version),
            None => assert_eq!(specifics["ctb_version"], serde_json::Value::Null),
        }
    }
}

#[test]
fn decode_encode_layer_stats_serializes_fast_path_stats() {
    let width = 8;
    let height = 1;
    let pixels = vec![0, 0, 32, 32, 128, 128, 255, 255];
    let ctb = encode_layer_data(&pixels, 0, width, height).expect("encode CTB");

    let json = decode_encode_layer_stats(&ctb, 0, "ctb", "goo", width, height, 1, 0, None, None)
        .expect("stats JSON");
    let stats: serde_json::Value = serde_json::from_str(&json).expect("stats JSON value");

    assert_eq!(stats["strategy_attempts"], 1);
    assert_eq!(stats["strategy_declines"], 0);
    assert_eq!(stats["fast_path_applied"], true);
    assert_eq!(stats["fallback_used"], false);
}

#[test]
fn archive_entry_plan_api_uses_rust_archive_layout_rules() {
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");
    let metadata_json = serde_json::to_string(&metadata).expect("metadata json");

    let metadata_entries =
        build_archive_metadata_entries(&metadata_json, "sl1", "demo", 2).expect("metadata entries");
    let metadata_entries: serde_json::Value =
        serde_json::from_str(&metadata_entries).expect("metadata entries json");
    let metadata_entries = metadata_entries.as_array().expect("metadata array");
    assert_eq!(metadata_entries[0]["name"], "config.ini");
    assert_eq!(metadata_entries[0]["compression_method"], 8);
    assert_eq!(metadata_entries[1]["name"], "prusaslicer.ini");

    let layer_entries = describe_archive_layer_entries("sl1", "demo", 0, 2).expect("layer entries");
    let layer_entries: serde_json::Value =
        serde_json::from_str(&layer_entries).expect("layer entries json");
    assert_eq!(layer_entries[0]["name"], "demo_00000.png");
    assert_eq!(layer_entries[0]["compression_method"], 8);
    assert_eq!(layer_entries[1]["name"], "demo_00001.png");

    let vdt_entries =
        describe_archive_layer_entries("vdt", "ignored", 0, 2).expect("vdt layer entries");
    let vdt_entries: serde_json::Value =
        serde_json::from_str(&vdt_entries).expect("vdt layer entries json");
    assert_eq!(vdt_entries[0]["name"], "0.png");
    assert_eq!(vdt_entries[1]["name"], "1.png");

    let vdt_metadata_entries =
        build_archive_metadata_entries(&metadata_json, "vdt", "ignored", 2).expect("vdt metadata");
    let vdt_metadata_entries: serde_json::Value =
        serde_json::from_str(&vdt_metadata_entries).expect("vdt metadata json");
    let manifest_bytes = vdt_metadata_entries[0]["data"]
        .as_array()
        .expect("manifest byte array")
        .iter()
        .map(|value| value.as_u64().expect("byte") as u8)
        .collect::<Vec<_>>();
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).expect("manifest json");
    assert_eq!(manifest["layers"].as_array().expect("layers").len(), 2);
}

fn format_info<'a>(infos: &'a [serde_json::Value], name: &str) -> &'a serde_json::Value {
    infos
        .iter()
        .find(|info| info["name"] == name)
        .unwrap_or_else(|| panic!("{name} should be listed"))
}

fn string_array(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .expect("string array")
        .iter()
        .map(|value| value.as_str().expect("string value"))
        .collect()
}

fn number_array(value: &serde_json::Value) -> Vec<u64> {
    value
        .as_array()
        .expect("number array")
        .iter()
        .map(|value| value.as_u64().expect("number value"))
        .collect()
}
