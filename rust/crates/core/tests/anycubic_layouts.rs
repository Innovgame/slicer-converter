use slicer_converter_core::common::FileMetadataBuilder;
use slicer_converter_core::formats::anycubic::codecs::Pw0Codec;
use slicer_converter_core::formats::anycubic::layout::AnycubicLayoutStrategy;
use slicer_converter_core::formats::anycubic::layouts::{
    V1Layout, V515Layout, V516Layout, V517Layout, V518Layout,
};
use slicer_converter_core::formats::anycubic::{AnycubicSpec, AnycubicVersion};
use slicer_converter_core::formats::common::{
    keys, ArchiveEncodeContext, ExportTargetOptions, LayerSource, VecOutputSink,
};

fn anycubic(name: &str) -> &'static AnycubicSpec {
    slicer_converter_core::formats::global_registry()
        .catalog_profile_by_name(name)
        .unwrap_or_else(|| panic!("{name} profile"))
        .as_anycubic()
        .unwrap_or_else(|| panic!("{name} specifics"))
}

fn build_registered_header(
    name: &str,
    metadata: &slicer_converter_core::common::FileMetadata,
) -> Vec<u8> {
    slicer_converter_core::formats::global_registry()
        .spec_by_name(name)
        .unwrap_or_else(|| panic!("{name} spec"))
        .build_header(metadata)
        .unwrap_or_else(|error| panic!("{name} header: {error}"))
}

#[test]
fn v1_layout_reports_v1_version() {
    assert_eq!(V1Layout.version(), AnycubicVersion::V1);
}

#[test]
fn v517_layout_builds_model_table_for_pm5() {
    let header = V517Layout
        .build_header(
            &FileMetadataBuilder::new()
                .layer_count(2)
                .build()
                .expect("valid fixture"),
            anycubic("pm5"),
            &Pw0Codec,
        )
        .expect("v517 header");
    assert!(header
        .windows("MODEL".len())
        .any(|window| window == b"MODEL"));
}

#[test]
fn v518_layout_builds_preview2_for_pm5s() {
    let header = V518Layout
        .build_header(
            &FileMetadataBuilder::new()
                .layer_count(2)
                .build()
                .expect("valid fixture"),
            anycubic("pm5s"),
            &Pw0Codec,
        )
        .expect("v518 header");
    assert!(header
        .windows("PREVIEW2".len())
        .any(|window| window == b"PREVIEW2"));
}

#[test]
fn anycubic_v515_header_contains_color_table_before_layers() {
    let metadata = FileMetadataBuilder::new()
        .resolution(3840, 2400)
        .display_size_mm(192.0, 120.0)
        .layer_count(2)
        .build()
        .expect("valid fixture");

    let header = V515Layout
        .build_header(&metadata, anycubic("pwmx"), &Pw0Codec)
        .unwrap();
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 515);
    assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 5);
    assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), 48);
    assert_eq!(u32::from_le_bytes(header[60..64].try_into().unwrap()), 80);
    let color_address = u32::from_le_bytes(header[32..36].try_into().unwrap()) as usize;
    let layerdef_address = u32::from_le_bytes(header[36..40].try_into().unwrap()) as usize;
    let layerdef = find_mark(&header, b"LAYERDEF").expect("layer definition table");

    assert!(
        color_address < layerdef_address,
        "color table should be written before layer definitions"
    );
    assert_eq!(
        layerdef_address, layerdef,
        "layer definition address should match LAYERDEF mark"
    );
    assert_eq!(
        u32::from_le_bytes(header[color_address..color_address + 4].try_into().unwrap()),
        0
    );
    assert_eq!(
        u32::from_le_bytes(
            header[color_address + 4..color_address + 8]
                .try_into()
                .unwrap()
        ),
        16
    );
}

#[test]
fn explicit_anycubic_version_selects_header_layout() {
    let metadata = FileMetadataBuilder::new()
        .machine_name("Anycubic Photon Mono")
        .resolution(2560, 1620)
        .display_size_mm(120.0, 80.0)
        .machine_z_mm(165.0)
        .mirror(false, true)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .transition_layer_count(0)
        .anti_alias_level(1)
        .exposure(2.0, 30.0)
        .light_pwm(255, 255)
        .build()
        .expect("metadata");
    let options = ExportTargetOptions::default().with_format_version(515);
    let spec = slicer_converter_core::formats::global_registry()
        .resolve_export_spec("pwms", Some(&metadata), options)
        .expect("pwms spec");

    let header = spec
        .build_header_with_options(&metadata, options)
        .expect("header");
    let finalized = spec
        .finalize_header_with_options(header, &[4], options)
        .expect("finalized header");

    assert_eq!(
        u32::from_le_bytes(finalized[12..16].try_into().unwrap()),
        515
    );
}

#[test]
fn explicit_anycubic_version_flows_through_document_pipeline() {
    let metadata = FileMetadataBuilder::new()
        .machine_name("Anycubic Photon Mono")
        .resolution(2560, 1620)
        .display_size_mm(120.0, 80.0)
        .machine_z_mm(165.0)
        .mirror(false, true)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .transition_layer_count(0)
        .anti_alias_level(1)
        .exposure(2.0, 30.0)
        .light_pwm(255, 255)
        .build()
        .expect("metadata");
    let options = ExportTargetOptions::default().with_format_version(515);
    let spec = slicer_converter_core::formats::global_registry()
        .resolve_export_spec("pwms", Some(&metadata), options)
        .expect("pwms spec");
    let layers = vec![vec![1, 2, 3, 4]];
    let mut sink = VecOutputSink::new();
    let ctx = ArchiveEncodeContext::new("ignored").with_export_options(options);

    spec.pipeline
        .encode_document_into(
            spec,
            &metadata,
            LayerSource::Encoded(&layers),
            &mut sink,
            &ctx,
        )
        .expect("encode document");

    let output = sink.into_inner();

    assert_eq!(u32::from_le_bytes(output[12..16].try_into().unwrap()), 515);
}

#[test]
fn anycubic_v516_header_contains_machine_and_extra_tables() {
    let metadata = FileMetadataBuilder::new()
        .resolution(1620, 2560)
        .display_size_mm(82.62, 130.56)
        .machine_name("Anycubic Photon Mono")
        .layer_count(2)
        .build()
        .expect("valid fixture");

    let header = build_registered_header("pwmo", &metadata);
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 516);
    assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 8);
    assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), 52);
    assert_eq!(u32::from_le_bytes(header[64..68].try_into().unwrap()), 84);
    let extra = find_mark(&header, b"EXTRA").expect("extra table");
    let machine = find_mark(&header, b"MACHINE").expect("machine table");
    let layer_image = u32::from_le_bytes(header[48..52].try_into().unwrap()) as usize;

    assert!(
        extra < machine,
        "extra table should be written before machine"
    );
    assert!(
        machine < layer_image,
        "machine table should live before layer image payloads"
    );
}

#[test]
fn anycubic_v517_header_contains_software_and_model_tables() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .machine_name("Anycubic Photon Mono M5")
        .layer_count(2)
        .build()
        .expect("valid fixture");

    let header = build_registered_header("pm5", &metadata);
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 517);
    assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 9);
    assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), 60);
    assert_eq!(u32::from_le_bytes(header[72..76].try_into().unwrap()), 92);

    let machine = find_mark(&header, b"MACHINE").expect("machine table");
    let software = find_mark(&header, b"SOFTWARE").expect("software table");
    let model = find_mark(&header, b"MODEL").expect("model table");

    assert_eq!(
        u32::from_le_bytes(header[machine + 12..machine + 16].try_into().unwrap()),
        156
    );
    assert_eq!(
        u32::from_le_bytes(header[machine + 132..machine + 136].try_into().unwrap()),
        7
    );

    let software_address = u32::from_le_bytes(header[24..28].try_into().unwrap()) as usize;
    let model_address = u32::from_le_bytes(header[48..52].try_into().unwrap()) as usize;

    assert_eq!(software_address, software);
    assert_eq!(model_address, model);
    assert!(machine < software);
    assert!(software < model);
}

#[test]
fn anycubic_v518_header_contains_subimgs_and_preview2_tables() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .machine_name("Anycubic Photon Mono M5s")
        .layer_count(2)
        .thumbnails(vec![
            slicer_converter_core::common::Thumbnail {
                width: 1,
                height: 1,
                pixels_rgba: vec![255, 255, 255, 255],
            },
            slicer_converter_core::common::Thumbnail {
                width: 1,
                height: 1,
                pixels_rgba: vec![0, 0, 0, 255],
            },
        ])
        .build()
        .expect("valid fixture");

    let header = build_registered_header("pm5s", &metadata);
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 518);
    assert_eq!(u32::from_le_bytes(header[16..20].try_into().unwrap()), 11);
    assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), 68);
    assert_eq!(u32::from_le_bytes(header[80..84].try_into().unwrap()), 96);

    let machine = find_mark(&header, b"MACHINE").expect("machine table");
    let subimgs = find_mark(&header, b"SUBIMGS").expect("subimgs table");
    let preview2 = find_mark(&header, b"PREVIEW2").expect("preview2 table");

    assert_eq!(
        u32::from_le_bytes(header[machine + 12..machine + 16].try_into().unwrap()),
        224
    );
    assert_eq!(
        u32::from_le_bytes(header[machine + 132..machine + 136].try_into().unwrap()),
        15
    );

    let software_address = u32::from_le_bytes(header[24..28].try_into().unwrap()) as usize;
    let model_address = u32::from_le_bytes(header[52..56].try_into().unwrap()) as usize;
    let subimgs_address = u32::from_le_bytes(header[56..60].try_into().unwrap()) as usize;
    let preview2_address = u32::from_le_bytes(header[60..64].try_into().unwrap()) as usize;

    assert_eq!(
        software_address,
        find_mark(&header, b"SOFTWARE").expect("software table")
    );
    assert_eq!(
        &header[model_address..model_address + "MODEL".len()],
        b"MODEL"
    );
    assert_eq!(subimgs_address, subimgs);
    assert_eq!(preview2_address, preview2);
    assert!(subimgs < preview2);
}

#[test]
fn pwmo_spec_uses_v516_layout_version() {
    assert_eq!(V516Layout.version(), AnycubicVersion::V516);
}

#[test]
fn pm5s_spec_uses_v518_layout_version() {
    assert_eq!(V518Layout.version(), AnycubicVersion::V518);
}

#[test]
fn anycubic_registered_specs_keep_expected_codec_layout_pairs() {
    assert_pairing("pw0", "pw0", AnycubicVersion::V1);
    assert_pairing("pws", "pws", AnycubicVersion::V1);
    assert_pairing("pwx", "pw0", AnycubicVersion::V1);
    assert_pairing("dlp", "pw0", AnycubicVersion::V516);
    assert_pairing("pwmo", "pw0", AnycubicVersion::V516);
    assert_pairing("pwms", "pw0", AnycubicVersion::V516);
    assert_pairing("pwma", "pw0", AnycubicVersion::V516);
    assert_pairing("pwmx", "pw0", AnycubicVersion::V516);
    assert_pairing("pmsq", "pw0", AnycubicVersion::V516);
    assert_pairing("pm3", "pw0", AnycubicVersion::V516);
    assert_pairing("pm3m", "pw0", AnycubicVersion::V516);
    assert_pairing("dl2p", "pw0", AnycubicVersion::V517);
    assert_pairing("pmx2", "pw0", AnycubicVersion::V517);
    assert_pairing("pwmb", "pw0", AnycubicVersion::V517);
    assert_pairing("pm3r", "pw0", AnycubicVersion::V517);
    assert_pairing("pm3n", "pw0", AnycubicVersion::V517);
    assert_pairing("pm4n", "pw0", AnycubicVersion::V517);
    assert_pairing("px6s", "pw0", AnycubicVersion::V517);
    assert_pairing("pm5", "pw0", AnycubicVersion::V517);
    assert_pairing("pm5s", "pw0", AnycubicVersion::V518);
    assert_pairing("m5sp", "pw0", AnycubicVersion::V518);
}

#[test]
fn anycubic_v1_parser_rejects_project_generated_big_endian_headers() {
    let fixture = project_generated_big_endian_anycubic_v1_fixture();

    assert!(
        V1Layout.parse_header(&fixture).is_err(),
        "Anycubic v1 parser should only accept the little-endian external layout"
    );
}

fn find_mark(data: &[u8], mark: &[u8]) -> Option<usize> {
    data.windows(mark.len()).position(|window| window == mark)
}

fn assert_pairing(
    expected_extension: &str,
    expected_codec: &str,
    expected_version: AnycubicVersion,
) {
    let spec = slicer_converter_core::formats::get_format_spec_by_name(expected_extension)
        .expect("anycubic spec");
    let expected_codec_key = match expected_codec {
        "pw0" => keys::ANYCUBIC_PW0_RLE,
        "pws" => keys::ANYCUBIC_PWS_RLE,
        other => panic!("unexpected codec name {other}"),
    };
    let expected_header_key = match expected_version {
        AnycubicVersion::V1 => keys::HEADER_ANYCUBIC_V1,
        AnycubicVersion::V516 => keys::HEADER_ANYCUBIC_V516,
        AnycubicVersion::V517 => keys::HEADER_ANYCUBIC_V517,
        AnycubicVersion::V518 => keys::HEADER_ANYCUBIC_V518,
        AnycubicVersion::V515 => panic!("v515 is not exposed as a registry profile"),
    };

    assert_eq!(spec.identity.name, expected_extension);
    assert_eq!(spec.keys.layer_codec_key, expected_codec_key);
    assert_eq!(spec.keys.header_key, expected_header_key);
    let specifics = slicer_converter_core::formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("anycubic profile")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(specifics.default_version, expected_version);

    assert!(
        specifics.supports_version(expected_version),
        "{} should advertise version {:?} in its typed specifics",
        expected_extension,
        expected_version
    );
}

fn project_generated_big_endian_anycubic_v1_fixture() -> Vec<u8> {
    let header_addr = 40u32;
    let header_len = 88u32;
    let layerdef_addr = header_addr + header_len;
    let layer_data_addr = layerdef_addr + 20 + 32;
    let layer_data_len = 4u32;
    let mut data = Vec::new();

    data.extend_from_slice(b"ANYCUBIC\0\0\0\0");
    push_be_u32(&mut data, 1);
    push_be_u32(&mut data, 3);
    push_be_u32(&mut data, header_addr);
    push_be_u32(&mut data, 0);
    push_be_u32(&mut data, 0);
    push_be_u32(&mut data, 0);
    push_be_u32(&mut data, layerdef_addr);

    data.extend_from_slice(b"HEADER\0\0\0\0\0\0");
    push_be_u32(&mut data, 72);
    push_be_f32(&mut data, 47.25);
    push_be_f32(&mut data, 0.05);
    push_be_f32(&mut data, 2.5);
    push_be_f32(&mut data, 0.0);
    push_be_f32(&mut data, 30.0);
    push_be_u32(&mut data, 1);
    push_be_f32(&mut data, 6.0);
    push_be_f32(&mut data, 180.0);
    push_be_f32(&mut data, 180.0);
    push_be_f32(&mut data, 1.0);
    push_be_u32(&mut data, 1);
    push_be_u32(&mut data, 480);
    push_be_u32(&mut data, 854);
    push_be_f32(&mut data, 0.0);
    push_be_f32(&mut data, 0.0);
    push_be_f32(&mut data, 0.0);
    push_be_f32(&mut data, 6.0);
    push_be_f32(&mut data, 180.0);

    data.extend_from_slice(b"LAYERDEF\0\0\0\0");
    push_be_u32(&mut data, 36);
    push_be_u32(&mut data, 1);
    push_be_u32(&mut data, layer_data_addr);
    push_be_u32(&mut data, layer_data_len);
    push_be_f32(&mut data, 6.0);
    push_be_f32(&mut data, 180.0);
    push_be_f32(&mut data, 2.5);
    push_be_f32(&mut data, 0.05);
    push_be_u32(&mut data, 4);
    push_be_u32(&mut data, 0);

    data.extend_from_slice(&[0, 1, 2, 3]);
    data
}

fn push_be_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_be_bytes());
}

fn push_be_f32(data: &mut Vec<u8>, value: f32) {
    data.extend_from_slice(&value.to_be_bytes());
}
