/// End-to-end test: parse the generated fixture files and verify their content.
use slicer_converter_core::formats::anycubic::codecs::{pws_rle, Pw0Codec};
use slicer_converter_core::formats::anycubic::AnycubicLayerCodec;
use slicer_converter_core::formats::common::{ArchiveEncodeContext, LayerSource, VecOutputSink};
use slicer_converter_core::formats::{self, ctb, goo, sl1, vdt};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn encode_document(
    target_format: &str,
    metadata: &slicer_converter_core::common::FileMetadata,
    encoded_layers: &[Vec<u8>],
    job_name: &str,
) -> Vec<u8> {
    let spec = formats::get_format_spec_by_name(target_format).expect("format spec");
    let mut sink = VecOutputSink::new();
    let ctx = ArchiveEncodeContext::new(job_name);
    spec.pipeline
        .encode_document_into(
            spec,
            metadata,
            LayerSource::Encoded(encoded_layers),
            &mut sink,
            &ctx,
        )
        .expect("document encode");
    sink.into_inner()
}

fn parse_pw0_family(
    data: &[u8],
) -> (
    slicer_converter_core::common::FileMetadata,
    Vec<slicer_converter_core::common::LayerOffset>,
) {
    let spec = formats::get_format_spec_by_name("pw0").expect("pw0 spec");
    let parsed = spec.pipeline.parse_document(spec, data).unwrap();
    (parsed.metadata, parsed.layers)
}

#[test]
fn parse_fixture_goo_file() {
    let data = std::fs::read(fixture_path("sample.goo"))
        .expect("sample.goo not found - run `pnpm fixtures` first");

    // Detect format
    let fmt = formats::detect_format_spec(&data, Some("goo")).unwrap();
    assert_eq!(fmt.identity.name, "goo");

    // Parse header
    let decoder = goo::decoder::GooDecoder::parse_header(&data).unwrap();
    let result = decoder.to_parse_result();
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.metadata.slicing.layer_count, 10);
    assert_eq!(result.layers.len(), 10);

    // Decode each layer
    for (i, layer_offset) in result.layers.iter().enumerate() {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = goo::decode_layer_data(layer_data, 64, 64).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn parse_fixture_ctb_file() {
    let data = std::fs::read(fixture_path("sample.ctb")).expect("sample.ctb not found");

    let fmt = formats::detect_format_spec(&data, Some("ctb")).unwrap();
    assert_eq!(fmt.identity.name, "ctb");

    let mut decoder = ctb::CtbDecoder::new();
    let (metadata, layers) = decoder.parse_header(&data).unwrap();
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(metadata.slicing.layer_count, 10);
    assert_eq!(layers.len(), 10);

    // Decode each layer
    for (i, layer_offset) in layers.iter().enumerate() {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = ctb::decode_layer_data(layer_data, i as u32, 64, 64, 1, 0).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn parse_fixture_sl1_file() {
    let data = std::fs::read(fixture_path("sample.sl1")).expect("sample.sl1 not found");

    let fmt = formats::detect_format_spec(&data, Some("sl1")).unwrap();
    assert_eq!(fmt.identity.name, "sl1");

    let (metadata, layers) = sl1::Sl1Decoder::parse_header(&data).unwrap();
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(layers.len(), 10);

    // Decode each layer from the ZIP archive
    for (i, layer_offset) in layers.iter().enumerate() {
        let pixels =
            sl1::Sl1Decoder::decode_layer_from_archive(&data, layer_offset.offset as usize, 64, 64)
                .unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn cross_format_convert_fixture_goo_to_sl1() {
    let goo_data = std::fs::read(fixture_path("sample.goo")).expect("sample.goo not found");

    // Parse GOO
    let decoder = goo::decoder::GooDecoder::parse_header(&goo_data).unwrap();
    let result = decoder.to_parse_result();

    // Decode all layers
    let mut pixel_layers = Vec::new();
    for layer_offset in &result.layers {
        let layer_data = &goo_data
            [layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = goo::decode_layer_data(layer_data, 64, 64).unwrap();
        pixel_layers.push(pixels);
    }

    // Re-encode as SL1 PNG layers
    let mut png_layers = Vec::new();
    for pixels in &pixel_layers {
        let png = sl1::encode_layer_data(pixels, 64, 64).unwrap();
        png_layers.push(png);
    }

    // Build SL1 ZIP through the shared format pipeline.
    let sl1_data = encode_document("sl1", &result.metadata, &png_layers, "converted");
    assert!(!sl1_data.is_empty());

    // Parse it back and verify
    let (_meta2, layers2) = sl1::Sl1Decoder::parse_header(&sl1_data).unwrap();
    assert_eq!(layers2.len(), 10);

    // Decode and compare
    for (i, original) in pixel_layers.iter().enumerate() {
        let decoded = sl1::Sl1Decoder::decode_layer_from_archive(
            &sl1_data,
            layers2[i].offset as usize,
            64,
            64,
        )
        .unwrap();
        assert_eq!(&decoded, original, "GOO->SL1 layer {i} mismatch");
    }
}

#[test]
fn cross_format_convert_fixture_goo_to_ctb() {
    let goo_data = std::fs::read(fixture_path("sample.goo")).expect("sample.goo not found");

    // Parse GOO (same as browser JS does)
    let decoder = goo::decoder::GooDecoder::parse_header(&goo_data).unwrap();
    let result = decoder.to_parse_result();
    let metadata = &result.metadata;

    // Decode all GOO layers 鈫?pixels 鈫?encode as CTB RLE
    let mut ctb_layers = Vec::new();
    for layer_offset in &result.layers {
        let layer_data = &goo_data
            [layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = goo::decode_layer_data(layer_data, 64, 64).unwrap();
        let encoded = ctb::encode_layer_data(&pixels, ctb_layers.len() as u32, 64, 64).unwrap();
        ctb_layers.push(encoded);
    }

    let ctb_file = encode_document("ctb", metadata, &ctb_layers, "converted");

    // Verify: parse the generated CTB and check layer table
    let mut ctb_decoder = ctb::CtbDecoder::new();
    let (parsed_meta, parsed_layers) = ctb_decoder.parse_header(&ctb_file).unwrap();
    assert_eq!(parsed_meta.slicing.layer_count, 10);
    assert_eq!(parsed_layers.len(), 10);

    // Verify each layer's data is accessible and decodable
    for (i, layer_info) in parsed_layers.iter().enumerate() {
        assert!(
            layer_info.offset > 0,
            "Layer {i} data_address should be non-zero"
        );
        assert!(
            layer_info.size > 0,
            "Layer {i} data_size should be non-zero"
        );
        let layer_data =
            &ctb_file[layer_info.offset as usize..(layer_info.offset + layer_info.size) as usize];
        let pixels = ctb::decode_layer_data(layer_data, i as u32, 64, 64, 1, 0).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn parse_fixture_pw0_file() {
    let data = std::fs::read(fixture_path("sample.pw0"))
        .expect("sample.pw0 not found - run `pnpm fixtures` first");

    let fmt = formats::detect_format_spec(&data, Some("pw0")).unwrap();
    assert_eq!(fmt.identity.name, "pw0");

    let (metadata, layers) = parse_pw0_family(&data);
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(metadata.slicing.layer_count, 10);
    assert_eq!(layers.len(), 10);

    for (i, layer_offset) in layers.iter().enumerate() {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = Pw0Codec.decode_layer(layer_data).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn parse_fixture_pws_file() {
    let data = std::fs::read(fixture_path("sample.pws"))
        .expect("sample.pws not found - run `pnpm fixtures` first");

    // PWS shares ANYCUBIC magic with PW0 — format selection is by extension/name
    let fmt = formats::get_format_spec_by_name("pws").unwrap();
    assert_eq!(fmt.identity.name, "pws");

    let (metadata, layers) = parse_pw0_family(&data);
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(metadata.slicing.layer_count, 10);
    assert_eq!(layers.len(), 10);

    for (i, layer_offset) in layers.iter().enumerate() {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = pws_rle::decode_pws_rle1(layer_data).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn parse_fixture_vdt_file() {
    let data = std::fs::read(fixture_path("sample.vdt"))
        .expect("sample.vdt not found - run `pnpm fixtures` first");

    let fmt = formats::detect_format_spec(&data, Some("vdt")).unwrap();
    assert_eq!(fmt.identity.name, "vdt");

    let (metadata, layers) = vdt::parse_header_data(&data).unwrap();
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(layers.len(), 10);

    // VDT layers are ZIP entries; offset = ZIP entry index
    // Extract each layer PNG from the archive and decode
    let cursor = std::io::Cursor::new(&data);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    for (i, layer_offset) in layers.iter().enumerate() {
        let entry_index = layer_offset.offset as usize;
        let mut file = archive.by_index(entry_index).unwrap();
        let mut png_data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut png_data).unwrap();
        let pixels = vdt::decode_layer_data(&png_data, 64, 64).unwrap();
        assert_eq!(pixels.len(), 64 * 64, "Layer {i} wrong pixel count");
    }
}

#[test]
fn cross_format_convert_pw0_to_pws_roundtrip() {
    let pw0_data = std::fs::read(fixture_path("sample.pw0")).expect("sample.pw0 not found");

    let (_metadata, layers) = parse_pw0_family(&pw0_data);

    let mut pws_encoded = Vec::new();
    let mut original_pixels = Vec::new();
    for layer_offset in &layers {
        let layer_data = &pw0_data
            [layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = Pw0Codec.decode_layer(layer_data).unwrap();
        let encoded = pws_rle::encode_pws_rle1(&pixels);
        original_pixels.push(pixels);
        pws_encoded.push(encoded);
    }

    // Decode PWS back and compare (PWS is 1-bit, our test data is only 0/255 so lossless)
    for (i, (original, encoded)) in original_pixels.iter().zip(pws_encoded.iter()).enumerate() {
        let decoded = pws_rle::decode_pws_rle1(encoded).unwrap();
        assert_eq!(decoded.len(), original.len(), "Layer {i} size mismatch");
        for (j, (o, d)) in original.iter().zip(decoded.iter()).enumerate() {
            let binarized = if *o >= 128 { 255u8 } else { 0u8 };
            assert_eq!(
                *d, binarized,
                "Layer {i} pixel {j}: expected {binarized}, got {d}"
            );
        }
    }
}

// ─── Anycubic V516 / V517 / V518 generated fixture parse tests ──────────────
// These exercise the AnycubicLayoutStrategy::parse_header path
// (`parse_anycubic_v51{6,7,8}_header_data`) against fully-formed multi-layer
// payloads produced by `pnpm fixtures`. Complements
// the synthetic round-trip unit tests in `anycubic_layouts.rs`.

fn parse_anycubic_modern_fixture(
    filename: &str,
    ext: &str,
    layer_count: u32,
    expected_layer_size: usize,
) {
    let data = std::fs::read(fixture_path(filename))
        .unwrap_or_else(|_| panic!("{filename} not found - run `pnpm fixtures` first"));

    let fmt = formats::detect_format_spec(&data, Some(ext)).unwrap();
    assert_eq!(fmt.identity.name, ext);

    let parsed = fmt.pipeline.parse_document(fmt, &data).unwrap();
    let metadata = parsed.metadata;
    let layers = parsed.layers;
    assert_eq!(metadata.printer.resolution_x, 64);
    assert_eq!(metadata.printer.resolution_y, 64);
    assert_eq!(metadata.slicing.layer_count, layer_count);
    assert_eq!(layers.len() as u32, layer_count);

    // Decode each layer payload via the shared Pw0 codec (all V516+ formats use Pw0).
    for (i, layer_offset) in layers.iter().enumerate() {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        let pixels = Pw0Codec.decode_layer(layer_data).unwrap();
        assert_eq!(
            pixels.len(),
            expected_layer_size,
            "Layer {i} wrong pixel count"
        );
    }
}

#[test]
fn parse_fixture_pwma_file_v516() {
    parse_anycubic_modern_fixture("sample.pwma", "pwma", 10, 64 * 64);
}

#[test]
fn parse_fixture_pm5_file_v517() {
    parse_anycubic_modern_fixture("sample.pm5", "pm5", 10, 64 * 64);
}

#[test]
fn parse_fixture_pm5s_file_v518() {
    parse_anycubic_modern_fixture("sample.pm5s", "pm5s", 10, 64 * 64);
}
