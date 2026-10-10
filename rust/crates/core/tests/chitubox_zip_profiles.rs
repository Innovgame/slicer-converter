use std::io::{Cursor, Read, Write};

use slicer_converter_core::{
    common::{FileMetadataBuilder, Thumbnail},
    formats::{
        self,
        common::{ArchiveEncodeContext, LayerSource, VecOutputSink},
        sl1, LayerDecodeContext,
    },
};

#[test]
fn chitubox_zip_pipeline_parses_gcode_metadata_and_png_layers() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    assert!(spec.capabilities.can_import);
    assert!(spec.capabilities.can_export);
    assert!(spec.capabilities.supports_preview);

    let pixels0 = vec![0, 32, 64, 96, 128, 160, 192, 255];
    let pixels1 = vec![255, 192, 160, 128, 96, 64, 32, 0];
    let png0 = sl1::encode_layer_data(&pixels0, 4, 2).expect("png0");
    let png1 = sl1::encode_layer_data(&pixels1, 4, 2).expect("png1");
    let data = zip_fixture(vec![
        ("run.gcode", chitubox_zip_gcode().as_bytes().to_vec()),
        ("preview.png", png0.clone()),
        ("preview_cropping.png", png1.clone()),
        ("1.png", png0.clone()),
        ("2.png", png1),
    ]);

    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("chitubox zip document");

    assert_eq!(parsed.metadata.printer.machine_name, "Mini 4K");
    assert_eq!(parsed.metadata.printer.resolution_x, 4);
    assert_eq!(parsed.metadata.printer.resolution_y, 2);
    assert_eq!(parsed.metadata.printer.display_width_mm, 80.0);
    assert_eq!(parsed.metadata.printer.display_height_mm, 40.0);
    assert_eq!(parsed.metadata.printer.machine_z_mm, 160.0);
    assert!(parsed.metadata.printer.mirror_x);
    assert_eq!(parsed.metadata.thumbnails.len(), 2);
    assert_eq!(parsed.metadata.thumbnails[0].width, 4);
    assert_eq!(parsed.metadata.thumbnails[0].height, 2);
    assert_eq!(
        &parsed.metadata.thumbnails[0].pixels_rgba[..8],
        &[0, 0, 0, 255, 32, 32, 32, 255]
    );
    assert_eq!(parsed.metadata.slicing.layer_height_mm, 0.05);
    assert_eq!(parsed.metadata.slicing.layer_count, 2);
    assert_eq!(parsed.metadata.slicing.bottom_layer_count, 1);
    assert_eq!(parsed.metadata.slicing.anti_alias_level, 4);
    assert_eq!(parsed.metadata.exposure.exposure_time, 2.5);
    assert_eq!(parsed.metadata.exposure.bottom_exposure_time, 30.0);
    assert_eq!(parsed.metadata.exposure.light_off_delay, 1.25);
    assert_eq!(parsed.metadata.exposure.bottom_light_off_delay, 0.5);
    assert_eq!(parsed.metadata.exposure.light_pwm, 180);
    assert_eq!(parsed.metadata.exposure.bottom_light_pwm, 230);
    assert_eq!(parsed.metadata.motion.lift_height, 6.0);
    assert_eq!(parsed.metadata.motion.lift_speed, 60.0);
    assert_eq!(parsed.metadata.motion.bottom_lift_height, 7.0);
    assert_eq!(parsed.metadata.motion.bottom_lift_speed, 45.0);
    assert_eq!(parsed.metadata.motion.retract_speed, 150.0);
    assert_eq!(parsed.metadata.stats.print_time_sec, 1234);
    assert_eq!(parsed.metadata.stats.volume_ml, 3.5);
    assert_eq!(parsed.metadata.stats.weight_g, 4.2);
    assert_eq!(parsed.metadata.stats.cost, 1.2);

    assert_eq!(parsed.layers.len(), 2);
    assert_eq!(parsed.layers[0].offset, 3);
    assert_eq!(parsed.layers[1].offset, 4);
    assert!(parsed.layers[0].size > 0);

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    let mut encoded = Vec::new();
    archive
        .by_index(parsed.layers[0].offset as usize)
        .expect("first layer")
        .read_to_end(&mut encoded)
        .expect("layer bytes");

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(&encoded, &LayerDecodeContext::new(0, 4, 2), &mut decoded)
        .expect("decode png layer");
    assert_eq!(decoded, pixels0);
}

#[test]
fn chitubox_zip_profile_is_export_capable() {
    let spec = formats::global_registry()
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    assert!(spec.capabilities.can_import);
    assert!(spec.capabilities.can_export);
}

#[test]
fn chitubox_zip_build_header_returns_run_gcode_metadata() {
    let spec = formats::global_registry()
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    let metadata = FileMetadataBuilder::new()
        .resolution(4, 2)
        .display_size_mm(80.0, 40.0)
        .machine_z_mm(160.0)
        .machine_name("Mini 4K")
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .build()
        .expect("valid metadata");

    let header = spec
        .build_header(&metadata)
        .expect("Chitubox ZIP header metadata");
    let header = String::from_utf8(header).expect("gcode metadata is utf8");

    assert!(header.contains(";machineType: Mini 4K"));
    assert!(header.contains(";totalLayer: 2"));
}

#[test]
fn chitubox_zip_pipeline_encodes_gcode_metadata_and_png_layers() {
    let spec = formats::global_registry()
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    let thumbnails = vec![
        Thumbnail {
            width: 2,
            height: 1,
            pixels_rgba: vec![0, 0, 0, 255, 255, 255, 255, 255],
        },
        Thumbnail {
            width: 1,
            height: 2,
            pixels_rgba: vec![64, 64, 64, 255, 192, 192, 192, 255],
        },
    ];
    let metadata = FileMetadataBuilder::new()
        .resolution(4, 2)
        .display_size_mm(80.0, 40.0)
        .machine_z_mm(160.0)
        .machine_name("Mini 4K")
        .mirror(true, false)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .anti_alias_level(4)
        .exposure(2.5, 30.0)
        .light_off_delay(1.25, 0.5)
        .light_pwm(180, 230)
        .lift(6.0, 60.0)
        .bottom_lift(7.0, 45.0)
        .retract(150.0)
        .statistics(1234, 3.5, 4.2, 1.2)
        .thumbnails(thumbnails)
        .build()
        .expect("valid metadata");
    let pixels0 = vec![0, 32, 64, 96, 128, 160, 192, 255];
    let pixels1 = vec![255, 192, 160, 128, 96, 64, 32, 0];
    let png0 = sl1::encode_layer_data(&pixels0, 4, 2).expect("png0");
    let png1 = sl1::encode_layer_data(&pixels1, 4, 2).expect("png1");

    let mut sink = VecOutputSink::new();
    spec.pipeline
        .encode_document_into(
            spec,
            &metadata,
            LayerSource::Encoded(&[png0.clone(), png1]),
            &mut sink,
            &ArchiveEncodeContext::new("mini-4k"),
        )
        .expect("encode Chitubox ZIP");
    let data = sink.into_inner();

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    assert!(archive.by_name("run.gcode").is_ok());
    assert!(archive.by_name("preview.png").is_ok());
    assert!(archive.by_name("preview_cropping.png").is_ok());
    assert!(archive.by_name("1.png").is_ok());
    assert!(archive.by_name("2.png").is_ok());

    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("parse generated Chitubox ZIP");
    assert_eq!(parsed.metadata.printer.machine_name, "Mini 4K");
    assert_eq!(parsed.metadata.slicing.layer_count, 2);
    assert_eq!(parsed.metadata.thumbnails.len(), 2);
    assert_eq!(parsed.metadata.thumbnails[0].width, 2);
    assert_eq!(parsed.metadata.thumbnails[0].height, 1);
    assert_eq!(parsed.metadata.thumbnails[1].width, 1);
    assert_eq!(parsed.metadata.thumbnails[1].height, 2);
    assert_eq!(parsed.layers.len(), 2);

    let mut encoded = Vec::new();
    archive
        .by_name("1.png")
        .expect("first generated layer")
        .read_to_end(&mut encoded)
        .expect("read first layer");
    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(&encoded, &LayerDecodeContext::new(0, 4, 2), &mut decoded)
        .expect("decode first generated layer");
    assert_eq!(decoded, pixels0);
}

fn chitubox_zip_gcode() -> &'static str {
    ";(****Build and Slicing Parameters****)\n\
     ;fileName: generated.zip\n\
     ;machineType: Mini 4K\n\
     ;estimatedPrintTime: 1234\n\
     ;volume: 3.5\n\
     ;weight: 4.2\n\
     ;price: 1.2\n\
     ;layerHeight: 0.05\n\
     ;resolutionX: 4\n\
     ;resolutionY: 2\n\
     ;machineX: 80\n\
     ;machineY: 40\n\
     ;machineZ: 160\n\
     ;normalExposureTime: 2.5\n\
     ;bottomLayerExposureTime: 30\n\
     ;normalDropSpeed: 150\n\
     ;normalLayerLiftSpeed: 60\n\
     ;normalLayerLiftHeight: 6\n\
     ;bottomLayerCount: 1\n\
     ;totalLayer: 2\n\
     ;bottomLayerLiftHeight: 7\n\
     ;bottomLayerLiftSpeed: 45\n\
     ;bottomLightOffTime: 0.5\n\
     ;lightOffTime: 1.25\n\
     ;bottomPWMLight: 230\n\
     ;PWMLight: 180\n\
     ;antiAliasLevel: 4\n\
     ;mirror: 1\n"
}

fn zip_fixture(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in entries {
        zip.start_file(name, options).expect("start zip entry");
        zip.write_all(&bytes).expect("write zip entry");
    }

    zip.finish().expect("finish zip").into_inner()
}
