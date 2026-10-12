use slicer_converter_core::formats::{
    self,
    common::{ArchiveEncodeContext, LayerSource, VecOutputSink},
};

#[test]
fn anycubic_zip_preserves_bottom_exposure_and_layer_count() {
    let registry = formats::global_registry();
    let source = registry.spec_by_name("goo").unwrap();
    let mut metadata = source
        .pipeline
        .parse_document(source, include_bytes!("fixtures/sample.goo"))
        .unwrap()
        .metadata;
    metadata.slicing.layer_count = 2;
    metadata.slicing.bottom_layer_count = 1;
    metadata.motion.lift_height2 = 2.0;
    let target = registry.spec_by_name("anycubic-zip").unwrap();
    let mut sink = VecOutputSink::new();
    target
        .pipeline
        .encode_document_into(
            target,
            &metadata,
            LayerSource::Encoded(&[vec![15, 255, 0, 1], vec![15, 255, 0, 1]]),
            &mut sink,
            &ArchiveEncodeContext::new("test"),
        )
        .unwrap();
    let data = sink.into_inner();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
    let settings: serde_json::Value =
        serde_json::from_reader(archive.by_name("anycubic_photon_resins.pwsp").unwrap()).unwrap();
    let resin = &settings["machine_extern"]["user_resins"][0];
    assert_eq!(
        settings["machine_extern"]["active_resins"][0],
        resin["property"]["name"]
    );
    assert_eq!(resin["slicepara"]["bott_layers"], 1);
    assert_eq!(resin["slicepara"]["bott_time"], 30.0);
    assert_eq!(resin["slicepara"]["zup_height"], 7.0);
    assert!((resin["slicepara"]["zup_speed"].as_f64().unwrap() - 65.0 / 60.0).abs() < 1e-5);
    assert_eq!(
        resin["slice_extpara"]["transition_layercount"],
        metadata.slicing.transition_layer_count
    );
    let layers: serde_json::Value =
        serde_json::from_reader(archive.by_name("layers_controller.conf").unwrap()).unwrap();
    assert_eq!(layers["paras"][0]["exposure_time"], 30.0);
    assert_eq!(layers["paras"][1]["exposure_time"], 2.5);
    let parsed = target.pipeline.parse_document(target, &data).unwrap();
    assert_eq!(
        parsed.metadata.exposure.bottom_exposure_time,
        metadata.exposure.bottom_exposure_time
    );
    assert_eq!(
        parsed.metadata.slicing.bottom_layer_count,
        metadata.slicing.bottom_layer_count
    );
    assert_eq!(
        parsed.metadata.exposure.exposure_time,
        metadata.exposure.exposure_time
    );
}

#[test]
fn checksum_creality_profiles_require_full_document_reads() {
    for name in ["cxdlpv4", "cxdlp", "cxdlp-v2"] {
        let spec = formats::global_registry().spec_by_name(name).unwrap();
        assert!(
            spec.capabilities.requires_full_document_parse,
            "{name} needs the complete file for checksum validation"
        );
    }
}
