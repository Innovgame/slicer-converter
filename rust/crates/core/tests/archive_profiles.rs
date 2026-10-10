use std::io::{Cursor, Read};

use slicer_converter_core::common::{ConvertError, FileMetadata};
use slicer_converter_core::formats::{
    self,
    common::{
        keys, ArchiveEncodeContext, ContainerKind, EncodedLayerStream, LayerSource, VecOutputSink,
    },
};

#[test]
fn archive_formats_expose_archive_container_keys() {
    let registry = formats::global_registry();
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");

    let archive_ctx = ArchiveEncodeContext::new("demo");

    for (name, expected_metadata_entries, expected_first_layer) in [
        (
            "sl1",
            &["config.ini", "prusaslicer.ini"][..],
            "demo_00000.png",
        ),
        ("vdt", &["manifest.json"][..], "0.png"),
    ] {
        let spec = registry.spec_by_name(name).expect("archive spec");

        assert_eq!(spec.container, ContainerKind::Archive, "{name} container");
        assert!(spec.capabilities.is_archive, "{name} archive capability");
        assert_eq!(
            spec.keys.container_key,
            keys::CONTAINER_ARCHIVE,
            "{name} container key"
        );
        assert_eq!(
            spec.keys.archive_key,
            keys::ZIP_ARCHIVE,
            "{name} archive key"
        );
        assert_eq!(
            spec.keys.layer_codec_key,
            keys::PNG_GRAYSCALE,
            "{name} layer codec"
        );
        assert_eq!(
            spec.keys.pixel_model_key,
            keys::PIXEL_PNG_GRAYSCALE,
            "{name} pixel model"
        );

        let metadata_entries = spec
            .archive
            .build_metadata_entries(&archive_ctx, &metadata, 2, spec)
            .expect("metadata entries");
        let metadata_entry_names = metadata_entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            metadata_entry_names, expected_metadata_entries,
            "{name} metadata entries"
        );

        let first_layer = spec.archive.describe_layer_entry(&archive_ctx, 0, spec);
        assert_eq!(first_layer.name, expected_first_layer, "{name} layer name");
        assert!(!first_layer.store, "{name} layer should be deflated");
    }
}

#[test]
fn vdt_metadata_entry_includes_uvtools_layer_records() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("vdt").expect("vdt archive spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .exposure(2.5, 25.0)
        .lift(5.0, 60.0)
        .bottom_lift(6.0, 45.0)
        .retract(150.0)
        .light_off_delay(0.2, 0.8)
        .build()
        .expect("metadata");
    let archive_ctx = ArchiveEncodeContext::new("demo");

    let metadata_entries = spec
        .archive
        .build_metadata_entries(&archive_ctx, &metadata, 2, spec)
        .expect("vdt metadata entries");
    let manifest_entry = metadata_entries
        .iter()
        .find(|entry| entry.name == "manifest.json")
        .expect("manifest entry");
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_entry.bytes).expect("manifest json");
    let layers = manifest["layers"].as_array().expect("manifest layers");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0]["height"], 0.05);
    assert_eq!(layers[1]["height"], 0.1);
    assert_eq!(layers[0]["exposure_time"], 25.0);
    assert_eq!(layers[1]["exposure_time"], 2.5);
    assert_eq!(layers[0]["lift_distance"], 6.0);
    assert_eq!(layers[1]["lift_distance"], 5.0);
}

#[test]
fn archive_specs_encode_documents_through_zip_pipeline() {
    let registry = formats::global_registry();
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");
    let layers = vec![b"png-layer-0".to_vec(), b"png-layer-1".to_vec()];
    let archive_ctx = ArchiveEncodeContext::new("demo");

    for (name, expected_entries) in [
        (
            "sl1",
            &[
                "config.ini",
                "prusaslicer.ini",
                "demo_00000.png",
                "demo_00001.png",
            ][..],
        ),
        ("vdt", &["manifest.json", "0.png", "1.png"][..]),
    ] {
        let spec = registry.spec_by_name(name).expect("archive spec");
        let mut sink = VecOutputSink::new();
        let stats = spec
            .pipeline
            .encode_document_into(
                spec,
                &metadata,
                LayerSource::Encoded(&layers),
                &mut sink,
                &archive_ctx,
            )
            .expect("encode archive document");
        let archive_bytes = sink.into_inner();

        assert!(archive_bytes.starts_with(b"PK"), "{name} should write ZIP");
        assert_eq!(stats.layers_written, layers.len(), "{name} layer count");
        assert_eq!(
            stats.bytes_written,
            archive_bytes.len(),
            "{name} byte count"
        );

        let mut archive = zip::ZipArchive::new(Cursor::new(&archive_bytes)).expect("zip archive");
        let entry_names = (0..archive.len())
            .map(|index| {
                archive
                    .by_index(index)
                    .expect("zip entry")
                    .name()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(entry_names, expected_entries, "{name} entry order");

        for (index, expected) in layers.iter().enumerate() {
            let layer_name = expected_entries[expected_entries.len() - layers.len() + index];
            let mut entry = archive.by_name(layer_name).expect("layer entry");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("layer bytes");
            assert_eq!(&bytes, expected, "{name} {layer_name} bytes");
        }
    }
}

struct TestEncodedLayerStream {
    layers: Vec<Vec<u8>>,
    visited: Vec<usize>,
}

impl EncodedLayerStream for TestEncodedLayerStream {
    fn read_layer_into(
        &mut self,
        layer_index: usize,
        output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        self.visited.push(layer_index);
        output.extend_from_slice(&self.layers[layer_index]);
        Ok(())
    }
}

#[test]
fn archive_specs_can_encode_documents_from_streaming_layer_source() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("vdt").expect("vdt archive spec");
    let metadata = FileMetadata::builder()
        .resolution(4, 4)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .build()
        .expect("metadata");
    let mut stream = TestEncodedLayerStream {
        layers: vec![b"stream-layer-0".to_vec(), b"stream-layer-1".to_vec()],
        visited: Vec::new(),
    };
    let mut sink = VecOutputSink::new();
    let archive_ctx = ArchiveEncodeContext::new("streamed");

    let stats = spec
        .pipeline
        .encode_document_into(
            spec,
            &metadata,
            LayerSource::Streaming {
                layer_count: 2,
                stream: &mut stream,
            },
            &mut sink,
            &archive_ctx,
        )
        .expect("streaming archive encode");

    assert_eq!(stream.visited, vec![0, 1]);
    assert_eq!(stats.layers_written, 2);

    let archive_bytes = sink.into_inner();
    let mut archive = zip::ZipArchive::new(Cursor::new(&archive_bytes)).expect("zip archive");
    let mut first = Vec::new();
    archive
        .by_name("0.png")
        .expect("first VDT layer")
        .read_to_end(&mut first)
        .expect("first layer bytes");
    assert_eq!(first, b"stream-layer-0");

    let mut second = Vec::new();
    archive
        .by_name("1.png")
        .expect("second VDT layer")
        .read_to_end(&mut second)
        .expect("second layer bytes");
    assert_eq!(second, b"stream-layer-1");
}

#[test]
fn png_archive_specs_publish_archive_strategy_keys() {
    let registry = formats::global_registry();

    for name in ["sl1", "vdt"] {
        let spec = registry.spec_by_name(name).expect("PNG archive spec");
        assert!(spec.keys.header_key.0.ends_with("-archive"));
        assert_eq!(
            spec.keys.layer_codec_key,
            keys::PNG_GRAYSCALE,
            "{name} should keep the shared PNG layer codec key"
        );
        assert_eq!(
            spec.keys.archive_key,
            keys::ZIP_ARCHIVE,
            "{name} should keep the shared ZIP archive key"
        );
    }
}
