use std::path::PathBuf;

use slicer_converter_core::{
    common::{downsample_to_rgba_nearest, FileMetadata, LayerOffset},
    formats::{
        self,
        common::{
            ArchiveEncodeContext, EncodedLayerStream, FormatSpec, LayerSource, VecOutputSink,
        },
        LayerDecodeContext, LayerEncodeContext,
    },
};

fn real_cxdlpv4_fixture_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SC_REAL_CXDLPV4_PATH") {
        return Some(PathBuf::from(path));
    }

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../dist/tmp/public-fixtures/plain-ctb-v5-candidates/extracted/857-cube/cube.cxdlpv4",
    );
    default_path.exists().then_some(default_path)
}

fn uvtools_generated_cxdlp_v3_fixture_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SC_UVTOOLS_CXDLP_V3_PATH") {
        return Some(PathBuf::from(path));
    }

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../dist/tmp/creality-generated/cube.v3.cxdlp");
    default_path.exists().then_some(default_path)
}

fn uvtools_generated_cxdlp_v2_fixture_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SC_UVTOOLS_CXDLP_V2_PATH") {
        return Some(PathBuf::from(path));
    }

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../dist/tmp/creality-generated/cube.v2.cxdlp");
    default_path.exists().then_some(default_path)
}

fn parse_document(
    spec: &'static FormatSpec,
    data: &[u8],
    context: &str,
) -> (FileMetadata, Vec<LayerOffset>) {
    let parsed = spec.pipeline.parse_document(spec, data).expect(context);
    (parsed.metadata, parsed.layers)
}

fn push_be_utf16_string(output: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    output.extend_from_slice(&((units.len() * 2) as u32).to_be_bytes());
    for unit in units {
        output.extend_from_slice(&unit.to_be_bytes());
    }
}

fn push_cxdlp_line(output: &mut Vec<u8>, start_y: u16, end_y: u16, start_x: u16, gray: u8) {
    output.push(((start_y >> 5) & 0xff) as u8);
    output.push((((start_y << 3) + (end_y >> 10)) & 0xff) as u8);
    output.push(((end_y >> 2) & 0xff) as u8);
    output.push((((end_y << 6) + (start_x >> 8)) & 0xff) as u8);
    output.push((start_x & 0xff) as u8);
    output.push(gray);
}

fn build_synthetic_cxdlp_v1_document() -> Vec<u8> {
    let mut data = Vec::new();

    data.extend_from_slice(&9u32.to_be_bytes());
    data.extend_from_slice(b"CXSW3DV2\0");
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&4u16.to_be_bytes());
    data.extend_from_slice(&4u16.to_be_bytes());

    for (width, height) in [(116usize, 116usize), (290, 290), (290, 290)] {
        data.extend(std::iter::repeat_n(0u8, width * height * 2));
        data.extend_from_slice(b"\r\n");
    }

    push_be_utf16_string(&mut data, "4.0");
    push_be_utf16_string(&mut data, "4.0");
    push_be_utf16_string(&mut data, "0.05");
    data.extend_from_slice(&21u16.to_be_bytes());
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&20u16.to_be_bytes());
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&5u16.to_be_bytes());
    data.extend_from_slice(&60u16.to_be_bytes());
    data.extend_from_slice(&6u16.to_be_bytes());
    data.extend_from_slice(&50u16.to_be_bytes());
    data.extend_from_slice(&150u16.to_be_bytes());
    data.extend_from_slice(&240u16.to_be_bytes());
    data.extend_from_slice(&120u16.to_be_bytes());

    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(b"\r\n");

    data.extend_from_slice(&3u32.to_be_bytes());
    data.extend_from_slice(&2u32.to_be_bytes());
    push_cxdlp_line(&mut data, 1, 3, 2, 200);
    push_cxdlp_line(&mut data, 0, 0, 0, 80);
    data.extend_from_slice(b"\r\n");

    data.extend_from_slice(&9u32.to_be_bytes());
    data.extend_from_slice(b"CXSW3DV2\0");
    data.extend_from_slice(&7u32.to_be_bytes());
    data
}

#[test]
fn creality_cxdlpv4_layer_encoder_writes_parameters_and_roundtrips_pixels() {
    let spec = formats::get_format_spec_by_name("cxdlpv4").expect("cxdlpv4 spec");
    let metadata = FileMetadata::builder()
        .resolution(8, 1)
        .display_size_mm(8.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .exposure(2.0, 20.0)
        .light_off_delay(0.2, 1.5)
        .light_pwm(120, 240)
        .lift(5.0, 60.0)
        .bottom_lift(6.0, 50.0)
        .lift2(1.25, 20.0)
        .retract(150.0)
        .retract2(0.5, 90.0)
        .wait_times(0.4, 0.6, 0.8)
        .build()
        .expect("metadata");
    let layer_metadata = metadata.as_layer_encode_metadata();
    let ctx = LayerEncodeContext {
        layer_index: 0,
        width: 8,
        height: 1,
        layer_metadata: Some(layer_metadata),
    };
    let pixels = [0, 0, 255, 255, 255, 2, 2, 0];

    let encoded = spec
        .encode_layer(&pixels, &ctx)
        .expect("encode CXDLPv4 layer");

    assert_eq!(encoded.len(), 51);
    assert_eq!(read_le_f32(&encoded, 0), 7.25);
    assert_eq!(read_le_f32(&encoded, 4), 50.0);
    assert_eq!(read_le_f32(&encoded, 8), 1.25);
    assert_eq!(read_le_f32(&encoded, 12), 20.0);
    assert_eq!(read_le_f32(&encoded, 16), 150.0);
    assert_eq!(read_le_f32(&encoded, 20), 0.5);
    assert_eq!(read_le_f32(&encoded, 24), 90.0);
    assert_eq!(read_le_f32(&encoded, 28), 0.6);
    assert_eq!(read_le_f32(&encoded, 32), 0.8);
    assert_eq!(read_le_f32(&encoded, 36), 0.4);
    assert_eq!(read_le_f32(&encoded, 40), 240.0);
    assert_eq!(&encoded[44..], &[0x80, 2, 0xff, 3, 0x81, 2, 0]);

    let decoded = spec
        .decode_layer(&encoded, &LayerDecodeContext::new(0, 8, 1))
        .expect("decode encoded CXDLPv4 layer");
    assert_eq!(decoded, vec![0, 0, 255, 255, 255, 3, 3, 0]);
}

#[test]
fn creality_cxdlpv4_document_encoder_writes_parseable_file_with_checksum() {
    let spec = formats::get_format_spec_by_name("cxdlpv4").expect("cxdlpv4 spec");
    let metadata = FileMetadata::builder()
        .machine_name("CL-TEST")
        .resolution(8, 1)
        .display_size_mm(8.0, 1.0)
        .machine_z_mm(100.0)
        .mirror(true, false)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .transition_layer_count(1)
        .anti_alias_level(2)
        .exposure(2.0, 20.0)
        .light_off_delay(0.2, 1.5)
        .light_pwm(120, 240)
        .lift(5.0, 60.0)
        .bottom_lift(6.0, 50.0)
        .lift2(1.25, 20.0)
        .retract(150.0)
        .retract2(0.5, 90.0)
        .wait_times(0.4, 0.6, 0.8)
        .statistics(123, 1.5, 1.8, 0.2)
        .build()
        .expect("metadata");
    let layer_metadata = metadata.as_layer_encode_metadata();
    let layer_pixels = [
        vec![0, 0, 255, 255, 255, 2, 2, 0],
        vec![255, 255, 0, 0, 0, 0, 4, 4],
    ];
    let encoded_layers = layer_pixels
        .iter()
        .enumerate()
        .map(|(layer_index, pixels)| {
            spec.encode_layer(
                pixels,
                &LayerEncodeContext {
                    layer_index: layer_index as u32,
                    width: 8,
                    height: 1,
                    layer_metadata: Some(layer_metadata),
                },
            )
            .expect("encode layer")
        })
        .collect::<Vec<_>>();
    let mut sink = VecOutputSink::new();

    let stats = spec
        .pipeline
        .encode_document_into(
            spec,
            &metadata,
            LayerSource::Encoded(&encoded_layers),
            &mut sink,
            &ArchiveEncodeContext::new("cxdlpv4-roundtrip"),
        )
        .expect("encode CXDLPv4 document");
    let data = sink.into_inner();

    assert_eq!(stats.bytes_written, data.len());
    assert_eq!(stats.layers_written, 2);
    assert_eq!(
        formats::detect_format_spec(&data, Some("cxdlpv4"))
            .expect("detect encoded cxdlpv4")
            .identity
            .name,
        "cxdlpv4"
    );

    let (parsed, layers) =
        parse_document(spec, &data, "parse generated CXDLPv4 header and checksum");
    assert_eq!(parsed.printer.machine_name, "CL-TEST");
    assert_eq!(parsed.printer.resolution_x, 8);
    assert_eq!(parsed.printer.resolution_y, 1);
    assert_eq!(parsed.printer.display_width_mm, 8.0);
    assert_eq!(parsed.printer.display_height_mm, 1.0);
    assert_eq!(parsed.printer.machine_z_mm, 100.0);
    assert!(parsed.printer.mirror_x);
    assert_eq!(parsed.slicing.layer_count, 2);
    assert_eq!(parsed.slicing.bottom_layer_count, 1);
    assert_eq!(parsed.slicing.transition_layer_count, 1);
    assert_eq!(parsed.slicing.anti_alias_level, 2);
    assert_eq!(parsed.slicing.encryption_key, 0);
    assert_eq!(parsed.exposure.exposure_time, 2.0);
    assert_eq!(parsed.exposure.bottom_exposure_time, 20.0);
    assert_eq!(parsed.exposure.light_pwm, 120);
    assert_eq!(parsed.exposure.bottom_light_pwm, 240);
    assert_eq!(parsed.stats.print_time_sec, 123);
    assert_eq!(parsed.stats.volume_ml, 1.5);
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, encoded_layers[0].len() as u64);
    assert_eq!(layers[1].size, encoded_layers[1].len() as u64);
    assert_eq!(layers[1].offset, layers[0].offset + layers[0].size);
    assert_eq!(layers[1].offset + layers[1].size, data.len() as u64 - 4);

    let first_layer =
        &data[layers[0].offset as usize..(layers[0].offset + layers[0].size) as usize];
    let decoded = spec
        .decode_layer(first_layer, &LayerDecodeContext::new(0, 8, 1))
        .expect("decode generated CXDLPv4 layer");
    assert_eq!(decoded, vec![0, 0, 255, 255, 255, 3, 3, 0]);
}

#[test]
fn creality_cxdlpv4_document_encoder_accepts_streaming_layers() {
    struct RecordingStream {
        layers: Vec<Vec<u8>>,
        calls: Vec<usize>,
    }

    impl EncodedLayerStream for RecordingStream {
        fn read_layer_into(
            &mut self,
            layer_index: usize,
            output: &mut Vec<u8>,
        ) -> Result<(), slicer_converter_core::common::ConvertError> {
            self.calls.push(layer_index);
            output.extend_from_slice(&self.layers[layer_index]);
            Ok(())
        }
    }

    let spec = formats::get_format_spec_by_name("cxdlpv4").expect("cxdlpv4 spec");
    let metadata = FileMetadata::builder()
        .machine_name("CL-STREAM")
        .resolution(8, 1)
        .display_size_mm(8.0, 1.0)
        .machine_z_mm(100.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .bottom_layer_count(1)
        .exposure(2.0, 20.0)
        .light_pwm(120, 240)
        .build()
        .expect("metadata");
    let layer_metadata = metadata.as_layer_encode_metadata();
    let encoded_layers = [
        vec![0, 0, 255, 255, 255, 2, 2, 0],
        vec![255, 255, 0, 0, 0, 0, 4, 4],
    ]
    .iter()
    .enumerate()
    .map(|(layer_index, pixels)| {
        spec.encode_layer(
            pixels,
            &LayerEncodeContext {
                layer_index: layer_index as u32,
                width: 8,
                height: 1,
                layer_metadata: Some(layer_metadata),
            },
        )
        .expect("encode layer")
    })
    .collect::<Vec<_>>();
    let expected_layer_sizes = encoded_layers
        .iter()
        .map(|layer| layer.len() as u64)
        .collect::<Vec<_>>();
    let mut stream = RecordingStream {
        layers: encoded_layers,
        calls: Vec::new(),
    };
    let mut sink = VecOutputSink::new();

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
            &ArchiveEncodeContext::new("cxdlpv4-stream"),
        )
        .expect("encode streaming CXDLPv4 document");
    let data = sink.into_inner();

    assert_eq!(stream.calls, vec![0, 1, 0, 1]);
    assert_eq!(stats.bytes_written, data.len());
    assert_eq!(stats.layers_written, 2);

    let (parsed, layers) = parse_document(spec, &data, "parse streaming output");
    assert_eq!(parsed.printer.machine_name, "CL-STREAM");
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].size, expected_layer_sizes[0]);
    assert_eq!(layers[1].size, expected_layer_sizes[1]);
    assert_eq!(layers[1].offset + layers[1].size, data.len() as u64 - 4);
}

#[test]
fn creality_cxdlp_v1_synthetic_fixture_parses_and_decodes() {
    let data = build_synthetic_cxdlp_v1_document();
    let spec = formats::detect_format_spec(&data, Some("v1.cxdlp")).expect("detect cxdlp v1");
    let spec_by_magic = formats::detect_format_spec(&data, None).expect("detect cxdlp v1 magic");

    assert_eq!(spec.identity.name, "cxdlp-v1");
    assert_eq!(spec_by_magic.identity.name, "cxdlp-v1");

    let (metadata, layers) = parse_document(spec, &data, "parse cxdlp v1 header");
    assert_eq!(metadata.printer.machine_name, "Creality CXDLP v1");
    assert_eq!(metadata.printer.resolution_x, 4);
    assert_eq!(metadata.printer.resolution_y, 4);
    assert_eq!(metadata.printer.display_width_mm, 4.0);
    assert_eq!(metadata.printer.display_height_mm, 4.0);
    assert_eq!(metadata.slicing.layer_height_mm, 0.05);
    assert_eq!(metadata.slicing.layer_count, 1);
    assert_eq!(metadata.slicing.bottom_layer_count, 1);
    assert_eq!(metadata.exposure.exposure_time, 2.1);
    assert_eq!(metadata.exposure.bottom_exposure_time, 20.0);
    assert_eq!(metadata.exposure.light_pwm, 120);
    assert_eq!(metadata.exposure.bottom_light_pwm, 240);
    assert_eq!(layers.len(), 1);

    let first_layer = &layers[0];
    let start = first_layer.offset as usize;
    let end = start + first_layer.size as usize;
    let decoded = spec
        .decode_layer(&data[start..end], &LayerDecodeContext::new(0, 4, 4))
        .expect("decode synthetic CXDLP v1 layer");

    assert_eq!(
        decoded,
        vec![80, 0, 0, 0, 0, 0, 200, 0, 0, 0, 200, 0, 0, 0, 200, 0]
    );
}

#[test]
fn creality_cxdlp_v3_uvtools_fixture_parses_and_decodes_when_available() {
    let Some(input) = uvtools_generated_cxdlp_v3_fixture_path() else {
        eprintln!("skipping UVTools-generated CXDLP v3 smoke: set SC_UVTOOLS_CXDLP_V3_PATH");
        return;
    };

    let data = std::fs::read(&input).unwrap_or_else(|error| {
        panic!(
            "UVTools-generated CXDLP v3 fixture should be readable at {}: {error}",
            input.display()
        )
    });
    let spec = formats::detect_format_spec(&data, Some("cxdlp")).expect("detect cxdlp v3");
    let spec_by_magic = formats::detect_format_spec(&data, None).expect("detect cxdlp v3 magic");

    assert_eq!(spec.identity.name, "cxdlp");
    assert_eq!(spec_by_magic.identity.name, "cxdlp");

    let (metadata, layers) = parse_document(spec, &data, "parse cxdlp v3 header");
    assert_eq!(metadata.printer.machine_name, "CL-103L");
    assert_eq!(metadata.printer.resolution_x, 7680);
    assert_eq!(metadata.printer.resolution_y, 4320);
    assert_eq!(metadata.printer.display_width_mm, 228.1);
    assert_eq!(metadata.printer.display_height_mm, 128.3);
    assert_eq!(metadata.slicing.layer_height_mm, 0.05);
    assert_eq!(metadata.slicing.layer_count, 200);
    assert_eq!(metadata.slicing.bottom_layer_count, 6);
    assert_eq!(metadata.exposure.exposure_time, 2.1);
    assert_eq!(metadata.exposure.bottom_exposure_time, 20.0);
    assert_eq!(metadata.exposure.light_pwm, 255);
    assert_eq!(metadata.exposure.bottom_light_pwm, 255);
    assert_eq!(layers.len(), 200);

    let first_layer = &layers[0];
    let start = first_layer.offset as usize;
    let end = start + first_layer.size as usize;
    let decoded = spec
        .decode_layer(
            &data[start..end],
            &LayerDecodeContext::new(
                0,
                metadata.printer.resolution_x,
                metadata.printer.resolution_y,
            ),
        )
        .expect("decode first CXDLP v3 layer");

    assert_eq!(decoded.len(), 7680 * 4320);
    assert_eq!(decoded.iter().filter(|pixel| **pixel != 0).count(), 114_244);
    assert_eq!(
        decoded.iter().map(|pixel| u64::from(*pixel)).sum::<u64>(),
        29_132_220
    );
}

#[test]
fn creality_cxdlp_v2_uvtools_fixture_parses_and_decodes_when_available() {
    let Some(input) = uvtools_generated_cxdlp_v2_fixture_path() else {
        eprintln!("skipping UVTools-generated CXDLP v2 smoke: set SC_UVTOOLS_CXDLP_V2_PATH");
        return;
    };

    let data = std::fs::read(&input).unwrap_or_else(|error| {
        panic!(
            "UVTools-generated CXDLP v2 fixture should be readable at {}: {error}",
            input.display()
        )
    });
    let spec = formats::detect_format_spec(&data, Some("cxdlp")).expect("detect cxdlp v2");
    let spec_by_magic = formats::detect_format_spec(&data, None).expect("detect cxdlp v2 magic");

    assert_eq!(spec.identity.name, "cxdlp-v2");
    assert_eq!(spec_by_magic.identity.name, "cxdlp-v2");

    let (metadata, layers) = parse_document(spec, &data, "parse cxdlp v2 header");
    assert_eq!(metadata.printer.machine_name, "CL-103L");
    assert_eq!(metadata.printer.resolution_x, 7680);
    assert_eq!(metadata.printer.resolution_y, 4320);
    assert_eq!(metadata.slicing.layer_count, 200);
    assert_eq!(layers.len(), 200);

    let first_layer = &layers[0];
    let start = first_layer.offset as usize;
    let end = start + first_layer.size as usize;
    let decoded = spec
        .decode_layer(
            &data[start..end],
            &LayerDecodeContext::new(
                0,
                metadata.printer.resolution_x,
                metadata.printer.resolution_y,
            ),
        )
        .expect("decode first CXDLP v2 layer");

    assert_eq!(decoded.len(), 7680 * 4320);
    assert_eq!(decoded.iter().filter(|pixel| **pixel != 0).count(), 114_244);
}

fn read_le_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(data[offset..offset + 4].try_into().expect("f32 bytes"))
}

#[test]
fn creality_cxdlpv4_real_fixture_parses_header_and_layer_offsets_when_available() {
    let Some(input) = real_cxdlpv4_fixture_path() else {
        eprintln!("skipping real CXDLPv4 parse smoke: set SC_REAL_CXDLPV4_PATH");
        return;
    };

    let data = std::fs::read(&input).unwrap_or_else(|error| {
        panic!(
            "real CXDLPv4 fixture should be readable at {}: {error}",
            input.display()
        )
    });
    let spec = formats::detect_format_spec(&data, Some("cxdlpv4")).expect("detect cxdlpv4");
    let spec_by_magic = formats::detect_format_spec(&data, None).expect("detect cxdlpv4 magic");

    assert_eq!(spec.identity.name, "cxdlpv4");
    assert_eq!(spec_by_magic.identity.name, "cxdlpv4");

    let (metadata, layers) = parse_document(spec, &data, "parse cxdlpv4 header");
    assert_eq!(metadata.printer.machine_name, "CL-103L");
    assert_eq!(metadata.printer.resolution_x, 7680);
    assert_eq!(metadata.printer.resolution_y, 4320);
    assert_eq!(metadata.printer.display_width_mm, 228.096);
    assert_eq!(metadata.printer.display_height_mm, 128.304);
    assert_eq!(metadata.printer.machine_z_mm, 230.0);
    assert_eq!(metadata.slicing.layer_count, 200);
    assert_eq!(metadata.slicing.bottom_layer_count, 6);
    assert_eq!(metadata.slicing.transition_layer_count, 6);
    assert_eq!(metadata.slicing.anti_alias_level, 1);
    assert_eq!(metadata.slicing.encryption_key, 0);
    assert_eq!(metadata.exposure.exposure_time, 2.1);
    assert_eq!(metadata.exposure.bottom_exposure_time, 20.0);
    assert_eq!(metadata.exposure.light_pwm, 255);
    assert_eq!(metadata.exposure.bottom_light_pwm, 255);
    assert_eq!(metadata.stats.print_time_sec, 1416);
    assert_eq!(metadata.stats.volume_ml, 0.0);

    assert_eq!(layers.len(), 200);
    assert_eq!(layers[0].offset, 22363);
    assert_eq!(layers[0].size, 2079);
    assert_eq!(layers[199].offset + layers[199].size, data.len() as u64 - 4);
}

#[test]
fn creality_cxdlpv4_real_fixture_decodes_sparse_layer_rle_when_available() {
    let Some(input) = real_cxdlpv4_fixture_path() else {
        eprintln!("skipping real CXDLPv4 layer decode smoke: set SC_REAL_CXDLPV4_PATH");
        return;
    };

    let data = std::fs::read(&input).unwrap_or_else(|error| {
        panic!(
            "real CXDLPv4 fixture should be readable at {}: {error}",
            input.display()
        )
    });
    let spec = formats::detect_format_spec(&data, Some("cxdlpv4")).expect("detect cxdlpv4");
    let (metadata, layers) = parse_document(spec, &data, "parse cxdlpv4 header");
    let first_layer = &layers[0];
    let start = first_layer.offset as usize;
    let end = start + first_layer.size as usize;

    let decoded = spec
        .decode_layer(
            &data[start..end],
            &LayerDecodeContext::new(
                0,
                metadata.printer.resolution_x,
                metadata.printer.resolution_y,
            ),
        )
        .expect("decode first CXDLPv4 layer");

    assert_eq!(decoded.len(), 7680 * 4320);
    assert_eq!(decoded.iter().filter(|pixel| **pixel != 0).count(), 114_244);
    assert_eq!(
        decoded.iter().map(|pixel| u64::from(*pixel)).sum::<u64>(),
        29_132_220
    );
    assert_eq!(decoded.last(), Some(&0));
}

#[test]
fn creality_cxdlpv4_real_fixture_decodes_layer_preview_when_available() {
    let Some(input) = real_cxdlpv4_fixture_path() else {
        eprintln!("skipping real CXDLPv4 layer preview smoke: set SC_REAL_CXDLPV4_PATH");
        return;
    };

    let data = std::fs::read(&input).unwrap_or_else(|error| {
        panic!(
            "real CXDLPv4 fixture should be readable at {}: {error}",
            input.display()
        )
    });
    let spec = formats::detect_format_spec(&data, Some("cxdlpv4")).expect("detect cxdlpv4");
    let (metadata, layers) = parse_document(spec, &data, "parse cxdlpv4 header");
    let first_layer = &layers[0];
    let start = first_layer.offset as usize;
    let end = start + first_layer.size as usize;
    let ctx = LayerDecodeContext::new(
        0,
        metadata.printer.resolution_x,
        metadata.printer.resolution_y,
    );
    let full_pixels = spec
        .decode_layer(&data[start..end], &ctx)
        .expect("decode first CXDLPv4 layer");
    let expected = downsample_to_rgba_nearest(
        &full_pixels,
        metadata.printer.resolution_x,
        metadata.printer.resolution_y,
        512,
    );

    let preview = spec
        .decode_layer_preview_rgba(&data[start..end], &ctx, 512)
        .expect("decode CXDLPv4 layer preview")
        .expect("CXDLPv4 should provide layer preview pixels");

    assert_eq!(preview.width, 512);
    assert_eq!(preview.height, 288);
    assert_eq!(preview.pixels, expected.pixels);
}

#[test]
fn creality_cxdlpv4_preview_does_not_require_full_sparse_layer_expansion() {
    let spec = formats::get_format_spec_by_name("cxdlpv4").expect("cxdlpv4 spec");
    let mut layer_data = vec![0; 44];
    layer_data.push(0x7f);

    let preview = spec
        .decode_layer_preview_rgba(&layer_data, &LayerDecodeContext::new(0, 4096, 4096), 4)
        .expect("decode sparse CXDLPv4 preview")
        .expect("preview pixels");

    assert_eq!(preview.width, 4);
    assert_eq!(preview.height, 4);
    assert_eq!(&preview.pixels[..8], &[255, 255, 255, 255, 0, 0, 0, 255]);
    assert!(preview.pixels[8..]
        .chunks_exact(4)
        .all(|rgba| rgba == [0, 0, 0, 255]));
}
