use slicer_converter_core::common::downsample_to_rgba_nearest;
use slicer_converter_core::formats::anycubic::codecs::{Pw0Codec, PwsCodec};
use slicer_converter_core::formats::anycubic::{AnycubicLayerCodec, AnycubicLayoutStrategy};
use slicer_converter_core::formats::LayerDecodeContext;

fn make_pw0_pixels() -> Vec<u8> {
    (0..192usize)
        .map(|index| {
            let value = (index % 16) as u8;
            (value << 4) | value
        })
        .collect()
}

fn make_pws_pixels() -> Vec<u8> {
    (0..192usize)
        .map(|index| if index % 3 == 0 { 0xFF } else { 0x00 })
        .collect()
}

#[test]
fn pw0_codec_roundtrips_layer_pixels() {
    let codec = Pw0Codec;
    let pixels = make_pw0_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let decoded = codec.decode_layer(&encoded).expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn pws_codec_roundtrips_layer_pixels() {
    let codec = PwsCodec;
    let pixels = make_pws_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let decoded = codec.decode_layer(&encoded).expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn pw0_codec_preview_matches_generic_downsample() {
    let codec = Pw0Codec;
    let width = 16;
    let height = 12;
    let max_preview_size = 6;
    let pixels = make_pw0_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let preview = codec
        .decode_layer_preview_rgba(&encoded, width, height, max_preview_size)
        .expect("preview");
    let expected = downsample_to_rgba_nearest(&pixels, width, height, max_preview_size);
    assert_eq!(preview.width, expected.width);
    assert_eq!(preview.height, expected.height);
    assert_eq!(preview.pixels, expected.pixels);
}

#[test]
fn pws_codec_preview_matches_generic_downsample() {
    let codec = PwsCodec;
    let width = 16;
    let height = 12;
    let max_preview_size = 6;
    let pixels = make_pws_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let preview = codec
        .decode_layer_preview_rgba(&encoded, width, height, max_preview_size)
        .expect("preview");
    let expected = downsample_to_rgba_nearest(&pixels, width, height, max_preview_size);
    assert_eq!(preview.width, expected.width);
    assert_eq!(preview.height, expected.height);
    assert_eq!(preview.pixels, expected.pixels);
}

#[test]
fn registry_pw0_spec_reports_pw0_name() {
    let pw0 = slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0");
    assert_eq!(pw0.identity.name, "pw0");
}

#[test]
fn registry_pws_spec_reports_pws_extension() {
    let pws = slicer_converter_core::formats::get_format_spec_by_name("pws").expect("pws");
    assert_eq!(pws.identity.extensions, &["pws"]);
}

#[test]
fn registry_returns_specs_for_pw0_and_pws() {
    let pw0 = slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0");
    let pws = slicer_converter_core::formats::get_format_spec_by_name("pws").expect("pws");

    assert_eq!(pw0.identity.name, "pw0");
    assert_eq!(
        slicer_converter_core::formats::global_registry()
            .catalog_profile_for_spec(pw0)
            .expect("pw0 profile")
            .as_anycubic()
            .expect("pw0 specifics")
            .extension,
        "pw0"
    );
    assert_eq!(pws.identity.name, "pws");
    assert_eq!(
        slicer_converter_core::formats::global_registry()
            .catalog_profile_for_spec(pws)
            .expect("pws profile")
            .as_anycubic()
            .expect("pws specifics")
            .extension,
        "pws"
    );
}

#[test]
fn registry_pw0_spec_decodes_layer_via_codec() {
    let pixels: Vec<u8> = (0..64).map(|index| ((index % 16) as u8) * 17).collect();
    let encoded = Pw0Codec.encode_layer(&pixels).expect("encode");
    let pw0 = slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0");
    let decoded = pw0
        .decode_layer(&encoded, &LayerDecodeContext::new(0, 8, 8))
        .expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn registry_pw0_builder_matches_direct_layout_builder() {
    let metadata = slicer_converter_core::common::FileMetadataBuilder::new()
        .resolution(480, 854)
        .display_size_mm(55.4, 98.63)
        .layer_count(3)
        .build()
        .expect("valid fixture");
    let pw0 = slicer_converter_core::formats::get_format_spec_by_name("pw0").expect("pw0");
    let registry = pw0.build_header(&metadata).expect("registry");
    let specifics = slicer_converter_core::formats::global_registry()
        .catalog_profile_for_spec(pw0)
        .expect("pw0 profile")
        .as_anycubic()
        .expect("pw0 specifics");
    let family = slicer_converter_core::formats::anycubic::layouts::V1Layout
        .build_header(&metadata, specifics, &Pw0Codec)
        .expect("direct layout");
    assert_eq!(registry, family);
}

#[test]
fn registry_pws_builder_matches_direct_layout_builder() {
    let metadata = slicer_converter_core::common::FileMetadataBuilder::new()
        .resolution(1440, 2560)
        .display_size_mm(68.04, 120.96)
        .layer_count(3)
        .build()
        .expect("valid fixture");
    let pws = slicer_converter_core::formats::get_format_spec_by_name("pws").expect("pws");
    let registry = pws.build_header(&metadata).expect("registry");
    let specifics = slicer_converter_core::formats::global_registry()
        .catalog_profile_for_spec(pws)
        .expect("pws profile")
        .as_anycubic()
        .expect("pws specifics");
    let family = slicer_converter_core::formats::anycubic::layouts::V1Layout
        .build_header(&metadata, specifics, &PwsCodec)
        .expect("direct layout");
    assert_eq!(registry, family);
}

#[test]
fn family_pw0_codec_is_publicly_reexported() {
    let codec = slicer_converter_core::formats::anycubic::codecs::Pw0Codec;
    let pixels = make_pw0_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let decoded = codec.decode_layer(&encoded).expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn family_pws_codec_is_publicly_reexported() {
    let codec = slicer_converter_core::formats::anycubic::codecs::PwsCodec;
    let pixels = make_pws_pixels();
    let encoded = codec.encode_layer(&pixels).expect("encode");
    let decoded = codec.decode_layer(&encoded).expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn anycubic_codecs_expose_pw0_raw_rle_helpers() {
    let pixels = make_pw0_pixels();
    let encoded =
        slicer_converter_core::formats::anycubic::codecs::pw0_rle::encode_pw0_rle4(&pixels);
    let decoded =
        slicer_converter_core::formats::anycubic::codecs::pw0_rle::decode_pw0_rle4(&encoded)
            .expect("decode");
    assert_eq!(decoded, pixels);
}

#[test]
fn anycubic_codecs_expose_pws_raw_rle_helpers() {
    let pixels = make_pws_pixels();
    let encoded =
        slicer_converter_core::formats::anycubic::codecs::pws_rle::encode_pws_rle1(&pixels);
    let decoded =
        slicer_converter_core::formats::anycubic::codecs::pws_rle::decode_pws_rle1(&encoded)
            .expect("decode");
    assert_eq!(decoded, pixels);
}
