use slicer_converter_core::common::FileMetadata;
use slicer_converter_core::formats::anycubic::{
    adapters::{AnycubicHeaderLayoutAdapter, AnycubicLayerCodecAdapter},
    AnycubicMachineId, AnycubicSpec, AnycubicVersion,
};
use slicer_converter_core::formats::{
    self,
    common::{keys, ExportTargetOptions, FormatFamily, HeaderLayout, LayerCodec},
    LayerDecodeContext, LayerEncodeContext,
};

fn anycubic(name: &str) -> &'static AnycubicSpec {
    formats::global_registry()
        .catalog_profile_by_name(name)
        .unwrap_or_else(|| panic!("{name} profile"))
        .as_anycubic()
        .unwrap_or_else(|| panic!("{name} anycubic specifics"))
}

fn le_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .expect("u32 slice should be in bounds"),
    )
}

#[test]
fn anycubic_versions_serialize_as_uvtools_numeric_ids() {
    assert_eq!(serde_json::to_string(&AnycubicVersion::V1).unwrap(), "1");
    assert_eq!(
        serde_json::to_string(&AnycubicVersion::V515).unwrap(),
        "515"
    );
    assert_eq!(
        serde_json::to_string(&AnycubicVersion::V516).unwrap(),
        "516"
    );
    assert_eq!(
        serde_json::to_string(&AnycubicVersion::V517).unwrap(),
        "517"
    );
    assert_eq!(
        serde_json::to_string(&AnycubicVersion::V518).unwrap(),
        "518"
    );
    assert_eq!(AnycubicVersion::from_u32(515), Some(AnycubicVersion::V515));
    assert_eq!(AnycubicVersion::from_u32(999), None);
    assert_eq!(AnycubicVersion::V518.label(), "518");
}

#[test]
fn anycubic_profiles_catalog_versions_serialize_as_uvtools_numeric_ids() {
    let infos = slicer_converter_core::formats::catalog::supported_format_infos(
        slicer_converter_core::formats::global_registry().catalog_profiles(),
    );
    let json = serde_json::to_value(&infos).expect("catalog json");
    let pwms = json
        .as_array()
        .expect("catalog entries")
        .iter()
        .find(|entry| {
            entry["specifics"]["family"] == "anycubic" && entry["specifics"]["extension"] == "pwms"
        })
        .expect("pwms profile");

    assert_eq!(pwms["specifics"]["default_version"], serde_json::json!(516));
    assert_eq!(
        pwms["specifics"]["supported_versions"],
        serde_json::json!([1, 515, 516])
    );
}

#[test]
fn anycubic_registry_matches_uvtools_extension_matrix() {
    let pw0 = anycubic("pw0");
    assert_eq!(pw0.machine.machine_id, AnycubicMachineId::PhotonZero);
    assert_eq!(pw0.default_version, AnycubicVersion::V1);
    assert_eq!(pw0.supported_versions, &[AnycubicVersion::V1]);
    assert_eq!(pw0.machine.resolution_x, 480);
    assert_eq!(pw0.machine.resolution_y, 854);

    let pws = anycubic("pws");
    assert_eq!(pws.machine.machine_id, AnycubicMachineId::PhotonS);
    assert_eq!(pws.default_version, AnycubicVersion::V1);
    assert_eq!(pws.supported_versions, &[AnycubicVersion::V1]);

    let pwmx = anycubic("pwmx");
    assert_eq!(pwmx.machine.machine_id, AnycubicMachineId::PhotonMonoX);
    assert_eq!(pwmx.default_version, AnycubicVersion::V516);
    assert_eq!(
        pwmx.supported_versions,
        &[
            AnycubicVersion::V1,
            AnycubicVersion::V515,
            AnycubicVersion::V516,
        ]
    );

    let pm3r = anycubic("pm3r");
    assert_eq!(pm3r.machine.machine_id, AnycubicMachineId::PhotonM3Premium);
    assert_eq!(pm3r.default_version, AnycubicVersion::V517);
    assert_eq!(pm3r.machine.resolution_x, 7680);
    assert_eq!(pm3r.machine.resolution_y, 4320);
    assert_eq!(
        pm3r.supported_versions,
        &[
            AnycubicVersion::V515,
            AnycubicVersion::V516,
            AnycubicVersion::V517,
        ]
    );

    let pm3n = anycubic("pm3n");
    assert_eq!(pm3n.machine.machine_id, AnycubicMachineId::PhotonMono2);
    assert_eq!(pm3n.default_version, AnycubicVersion::V517);
    assert_eq!(pm3n.machine.resolution_x, 4096);
    assert_eq!(pm3n.machine.resolution_y, 2560);

    let pm5 = anycubic("pm5");
    assert_eq!(pm5.machine.machine_id, AnycubicMachineId::PhotonMonoM5);
    assert_eq!(pm5.default_version, AnycubicVersion::V517);
    assert_eq!(pm5.supported_versions, &[AnycubicVersion::V517]);

    let pm5s = anycubic("pm5s");
    assert_eq!(pm5s.machine.machine_id, AnycubicMachineId::PhotonMonoM5s);
    assert_eq!(pm5s.default_version, AnycubicVersion::V518);
    assert_eq!(pm5s.supported_versions, &[AnycubicVersion::V518]);

    let pm4n = anycubic("pm4n");
    assert_eq!(pm4n.default_version, AnycubicVersion::V517);
    assert_eq!(pm4n.machine.resolution_x, 9024);
    assert_eq!(pm4n.machine.resolution_y, 5120);

    let m5sp = anycubic("m5sp");
    assert_eq!(m5sp.default_version, AnycubicVersion::V518);
    assert_eq!(m5sp.machine.resolution_x, 13312);
    assert_eq!(m5sp.machine.resolution_y, 5120);
}

#[test]
fn anycubic_family_exposes_all_existing_machine_profiles() {
    let registry = formats::global_registry();
    let expected = [
        ("pw0", keys::HEADER_ANYCUBIC_V1, keys::ANYCUBIC_PW0_RLE, 1),
        ("pws", keys::HEADER_ANYCUBIC_V1, keys::ANYCUBIC_PWS_RLE, 1),
        ("pwx", keys::HEADER_ANYCUBIC_V1, keys::ANYCUBIC_PW0_RLE, 1),
        (
            "dlp",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pwmo",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pwms",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pwma",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pwmx",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pmsq",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pm3",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "pm3m",
            keys::HEADER_ANYCUBIC_V516,
            keys::ANYCUBIC_PW0_RLE,
            516,
        ),
        (
            "dl2p",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pmx2",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pwmb",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pm3r",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pm3n",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pm4n",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "px6s",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pm5",
            keys::HEADER_ANYCUBIC_V517,
            keys::ANYCUBIC_PW0_RLE,
            517,
        ),
        (
            "pm5s",
            keys::HEADER_ANYCUBIC_V518,
            keys::ANYCUBIC_PW0_RLE,
            518,
        ),
        (
            "m5sp",
            keys::HEADER_ANYCUBIC_V518,
            keys::ANYCUBIC_PW0_RLE,
            518,
        ),
    ];

    for (name, header_key, codec_key, version) in expected {
        let spec = registry.spec_by_name(name).expect("anycubic spec");
        let specifics = registry
            .catalog_profile_for_spec(spec)
            .expect("anycubic profile")
            .as_anycubic()
            .expect("anycubic specifics");
        assert_eq!(spec.identity.name, name);
        assert_eq!(spec.family, FormatFamily::Anycubic);
        assert_eq!(
            specifics.default_version.as_u32(),
            version,
            "{name} version"
        );
        assert_eq!(specifics.extension, name);
        assert_eq!(spec.keys.header_key, header_key, "{name} header key");
        assert_eq!(spec.keys.layer_codec_key, codec_key, "{name} codec key");
    }
}

#[test]
fn anycubic_profiles_use_explicit_no_footer_policy() {
    let family = formats::global_registry()
        .families()
        .iter()
        .copied()
        .find(|family| family.identity.name == "anycubic")
        .expect("anycubic family");

    for spec in family.profiles {
        assert_eq!(
            spec.keys.footer_key,
            keys::NO_FOOTER,
            "{} should use the explicit no-footer policy",
            spec.identity.name
        );
    }
}

#[test]
fn anycubic_profiles_publish_concrete_strategy_keys() {
    let family = formats::global_registry()
        .families()
        .iter()
        .copied()
        .find(|family| family.identity.name == "anycubic")
        .expect("anycubic family");

    for spec in family.profiles {
        assert_eq!(spec.keys.family_key, keys::FAMILY_ANYCUBIC);
        assert_eq!(spec.keys.container_key, spec.container.as_key());
        assert!(
            spec.keys.header_key.0.starts_with("header/anycubic-"),
            "{} should use an Anycubic header strategy key",
            spec.identity.name
        );
        assert!(
            spec.keys.layer_codec_key.0.starts_with("layer/anycubic-"),
            "{} should use an Anycubic layer codec key",
            spec.identity.name
        );
    }
}

#[test]
fn anycubic_strategy_adapters_accept_direct_layout_and_codec_slots() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("pw0").expect("pw0 spec");
    let metadata = FileMetadata::builder()
        .resolution(2, 2)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");

    let header_adapter = AnycubicHeaderLayoutAdapter {
        specifics: anycubic("pw0"),
    };
    let direct_header = header_adapter
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("direct header");
    let spec_header = spec.build_header(&metadata).expect("spec header");
    assert_eq!(direct_header, spec_header);

    let codec_adapter = AnycubicLayerCodecAdapter {
        specifics: anycubic("pw0"),
    };
    let pixels = vec![0, 255, 0, 255];
    let encode_ctx = LayerEncodeContext {
        layer_index: 0,
        width: 2,
        height: 2,
        layer_metadata: None,
    };
    let mut encoded = Vec::new();
    codec_adapter
        .encode_layer_into(&pixels, &encode_ctx, &mut encoded)
        .expect("encode via direct codec");

    let mut decoded = Vec::new();
    codec_adapter
        .decode_layer_into(&encoded, &LayerDecodeContext::new(0, 2, 2), &mut decoded)
        .expect("decode via direct codec");
    assert_eq!(decoded, pixels);
}

#[test]
fn anycubic_spec_slots_match_direct_execution_behavior() {
    let registry = formats::global_registry();
    let metadata = FileMetadata::builder()
        .resolution(480, 854)
        .display_size_mm(68.04, 120.96)
        .machine_name("Anycubic Photon Zero")
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let pixels = vec![0, 255, 0, 255];

    let spec = registry.spec_by_name("pw0").expect("pw0 spec");
    let spec_header = spec
        .header
        .build_header(&metadata, spec, ExportTargetOptions::default())
        .expect("spec header");
    let executed_header = spec.build_header(&metadata).expect("execute header");

    assert_eq!(spec_header, executed_header);

    let encode_ctx = LayerEncodeContext {
        layer_index: 0,
        width: 2,
        height: 2,
        layer_metadata: None,
    };
    let mut spec_encoded = Vec::new();
    spec.layer_codec
        .encode_layer_into(&pixels, &encode_ctx, &mut spec_encoded)
        .expect("spec encode");
    let executed_encoded = spec
        .encode_layer(&pixels, &encode_ctx)
        .expect("execute encode");

    assert_eq!(spec_encoded, executed_encoded);
}

#[test]
fn anycubic_v518_file_mark_addresses_match_uvtools_table_order() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("m5sp").expect("m5sp spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Photon Mono M5s Pro")
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let header = spec.build_header(&metadata).expect("v5.18 header");

    let layer_image_address = le_u32(&header, 48) as usize;
    let model_address = le_u32(&header, 52) as usize;
    let sub_layer_definition_address = le_u32(&header, 56) as usize;
    let preview2_address = le_u32(&header, 60) as usize;

    assert_eq!(&header[model_address..model_address + 5], b"MODEL");
    assert_eq!(
        &header[sub_layer_definition_address..sub_layer_definition_address + 7],
        b"SUBIMGS"
    );
    assert_eq!(le_u32(&header, sub_layer_definition_address + 12), 68);
    assert_eq!(le_u32(&header, sub_layer_definition_address + 16), 1);
    assert_eq!(le_u32(&header, sub_layer_definition_address + 20), 1);
    assert_eq!(&header[preview2_address..preview2_address + 8], b"PREVIEW2");
    assert_eq!(preview2_address, sub_layer_definition_address + 68);
    assert!(layer_image_address > preview2_address);
}

#[test]
fn anycubic_header_parse_accepts_streaming_prefix_without_layer_payloads() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("pws").expect("pws spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .machine_name("Photon S")
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");
    let header = spec.build_header(&metadata).expect("pws header");
    let finalized = spec
        .finalize_header(&header, &[7, 11])
        .expect("finalize pws header");

    let parsed = spec
        .pipeline
        .parse_document(spec, &finalized)
        .expect("parse header prefix without layer payloads");

    assert_eq!(parsed.metadata.slicing.layer_count, 2);
    assert_eq!(parsed.layers.len(), 2);
    assert_eq!(parsed.layers[0].offset, finalized.len() as u64);
    assert_eq!(parsed.layers[0].size, 7);
    assert_eq!(parsed.layers[1].offset, finalized.len() as u64 + 7);
    assert_eq!(parsed.layers[1].size, 11);
}
