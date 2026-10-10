#![cfg(target_arch = "wasm32")]

use js_sys::Reflect;
use slicer_converter_core::common::ParseResult;
use slicer_converter_wasm::{
    build_header, crc32_bytes, decode_encode_layer, decode_encode_layer_stats, decode_layer,
    decode_layer_preview, encode_layer, finalize_header, list_supported_formats, parse_header,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

const FIXTURE_GOO: &[u8] = include_bytes!("../../core/tests/fixtures/sample.goo");
const FIXTURE_CTB: &[u8] = include_bytes!("../../core/tests/fixtures/sample.ctb");
const FIXTURE_PW0: &[u8] = include_bytes!("../../core/tests/fixtures/sample.pw0");
const FIXTURE_PWS: &[u8] = include_bytes!("../../core/tests/fixtures/sample.pws");
const FIXTURE_VDT: &[u8] = include_bytes!("../../core/tests/fixtures/sample.vdt");

fn parse_result(json: String) -> ParseResult {
    serde_json::from_str(&json).expect("parse_header should return ParseResult")
}

fn bytes(array: &js_sys::Uint8Array) -> Vec<u8> {
    array.to_vec()
}

fn make_test_layer(width: u32, height: u32) -> Vec<u8> {
    let count = (width * height) as usize;
    let mut pixels = vec![0u8; count];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let x = (index % width as usize) as u8;
        let y = (index / width as usize) as u8;
        *pixel = x.wrapping_add(y.wrapping_mul(3));
    }
    pixels
}

fn ctb_quantize(pixels: &[u8]) -> Vec<u8> {
    pixels
        .iter()
        .map(|&pixel| {
            let gray7 = pixel >> 1;
            (gray7 << 1) | (gray7 >> 6)
        })
        .collect()
}

#[wasm_bindgen_test]
fn parse_header_detects_goo_fixture() {
    let result = parse_result(parse_header(FIXTURE_GOO, Some("goo".to_string())).unwrap());

    assert_eq!(result.format, "goo");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}

#[wasm_bindgen_test]
fn parse_header_detects_ctb_fixture() {
    let result = parse_result(parse_header(FIXTURE_CTB, Some("ctb".to_string())).unwrap());

    assert_eq!(result.format, "ctb");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}

#[wasm_bindgen_test]
fn goo_roundtrip_via_exported_api() {
    let width = 32;
    let height = 24;
    let original = make_test_layer(width, height);

    let encoded = encode_layer(&original, 0, "goo", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let decoded = decode_layer(&encoded, 0, "goo", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn ctb_roundtrip_via_exported_api() {
    let width = 40;
    let height = 16;
    let original = ctb_quantize(&make_test_layer(width, height));

    let encoded = encode_layer(&original, 0, "ctb", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let decoded = decode_layer(&encoded, 0, "ctb", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn decode_encode_layer_transcodes_ctb_to_goo() {
    let width = 40;
    let height = 16;
    let original = ctb_quantize(&make_test_layer(width, height));

    let ctb_encoded = encode_layer(&original, 0, "ctb", width, height, None, None).unwrap();
    let ctb_encoded = bytes(&ctb_encoded);
    let goo_encoded = decode_encode_layer(
        &ctb_encoded,
        0,
        "ctb",
        "goo",
        width,
        height,
        1,
        0,
        None,
        None,
    )
    .unwrap();
    let goo_encoded = bytes(&goo_encoded);
    let decoded = decode_layer(&goo_encoded, 0, "goo", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn decode_encode_layer_stats_reports_fast_path_for_ctb_to_goo() {
    let width = 40;
    let height = 16;
    let original = ctb_quantize(&make_test_layer(width, height));

    let ctb_encoded = encode_layer(&original, 0, "ctb", width, height, None, None).unwrap();
    let ctb_encoded = bytes(&ctb_encoded);
    let json = decode_encode_layer_stats(
        &ctb_encoded,
        0,
        "ctb",
        "goo",
        width,
        height,
        1,
        0,
        None,
        None,
    )
    .unwrap();
    let stats: serde_json::Value = serde_json::from_str(&json).expect("stats json");

    assert_eq!(stats["strategy_attempts"], 1);
    assert_eq!(stats["strategy_declines"], 0);
    assert_eq!(stats["fast_path_applied"], true);
    assert_eq!(stats["fallback_used"], false);
}

#[wasm_bindgen_test]
fn decode_encode_layer_accepts_export_target_aliases() {
    let width = 40;
    let height = 16;
    let original = ctb_quantize(&make_test_layer(width, height));

    let goo_encoded = encode_layer(&original, 0, "goo", width, height, None, None).unwrap();
    let goo_encoded = bytes(&goo_encoded);
    let ctb_v4_encoded = decode_encode_layer(
        &goo_encoded,
        0,
        "goo",
        "v4.ctb",
        width,
        height,
        1,
        0,
        None,
        None,
    )
    .unwrap();
    let ctb_v4_encoded = bytes(&ctb_v4_encoded);
    let decoded = decode_layer(&ctb_v4_encoded, 0, "ctb-v4", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn finalize_header_patches_ctb_header() {
    let parsed = parse_result(parse_header(FIXTURE_CTB, Some("ctb".to_string())).unwrap());
    let metadata_json = serde_json::to_string(&parsed.metadata).unwrap();
    let header = build_header(&metadata_json, "ctb", None).unwrap();
    let header = bytes(&header);
    let layer_sizes = vec![32u32; parsed.layers.len()];
    let patched = finalize_header(&header, "ctb", &layer_sizes, None).unwrap();
    let patched = bytes(&patched);

    assert_eq!(patched.len(), header.len());
    assert_ne!(patched, header);
}

#[wasm_bindgen_test]
fn anycubic_header_api_uses_explicit_numeric_version() {
    let metadata = slicer_converter_core::common::FileMetadata::builder()
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
    let metadata_json = serde_json::to_string(&metadata).expect("metadata json");

    let header = build_header(&metadata_json, "pwms", Some(515)).expect("v515 header");
    let finalized =
        finalize_header(&header.to_vec(), "pwms", &[4], Some(515)).expect("v515 finalize");
    let finalized = bytes(&finalized);

    assert_eq!(
        u32::from_le_bytes(finalized[12..16].try_into().unwrap()),
        515
    );
}

#[wasm_bindgen_test]
fn crc32_bytes_matches_known_vectors() {
    assert_eq!(crc32_bytes(&[]), 0x00000000);
    assert_eq!(crc32_bytes(&[0x7b, 0x7d]), 0xa3a6bf43);
    assert_eq!(crc32_bytes(&[0x89, 0x50, 0x4e, 0x47]), 0x5bebbea5);
}

#[wasm_bindgen_test]
fn decode_layer_preview_downsamples_in_wasm() {
    let width = 8;
    let height = 4;
    let original = make_test_layer(width, height);

    let encoded = encode_layer(&original, 0, "goo", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let packed = decode_layer_preview(&encoded, 0, "goo", width, height, 1, 0, 4).unwrap();
    let packed = bytes(&packed);

    // Packed format: [u32 LE width][u32 LE height][rgba_pixels...]
    let preview_width = u32::from_le_bytes(packed[0..4].try_into().unwrap());
    let preview_height = u32::from_le_bytes(packed[4..8].try_into().unwrap());
    let pixels = &packed[8..];

    assert_eq!(preview_width, 4);
    assert_eq!(preview_height, 2);
    assert_eq!(
        pixels,
        &[
            0, 0, 0, 255, 2, 2, 2, 255, 4, 4, 4, 255, 6, 6, 6, 255, 6, 6, 6, 255, 8, 8, 8, 255, 10,
            10, 10, 255, 12, 12, 12, 255,
        ]
    );
}

#[wasm_bindgen_test]
fn list_supported_formats_exposes_zip_metadata() {
    let json_str = list_supported_formats().unwrap();
    let formats: serde_json::Value = serde_json::from_str(&json_str).expect("format metadata");
    let formats = formats.as_array().expect("format metadata array");

    assert!(formats.iter().any(|format| {
        format.get("name") == Some(&serde_json::Value::String("sl1".to_string()))
            && format.get("display_name")
                == Some(&serde_json::Value::String("Prusa SL1 archive".to_string()))
            && format.get("is_zip") == Some(&serde_json::Value::Bool(true))
    }));
    assert!(formats.iter().any(|format| {
        format.get("name") == Some(&serde_json::Value::String("ctb".to_string()))
            && format.get("is_zip") == Some(&serde_json::Value::Bool(false))
            && format
                .get("capabilities")
                .and_then(|value| value.get("supports_anti_alias"))
                == Some(&serde_json::Value::Bool(true))
    }));
    assert!(formats.iter().any(|format| {
        format.get("name") == Some(&serde_json::Value::String("pw0".to_string()))
            && format.get("family") == Some(&serde_json::Value::String("anycubic".to_string()))
            && format
                .get("capabilities")
                .and_then(|value| value.get("supports_preview"))
                == Some(&serde_json::Value::Bool(true))
            && format
                .get("specifics")
                .and_then(|value| value.get("family"))
                == Some(&serde_json::Value::String("anycubic".to_string()))
    }));
}

#[wasm_bindgen_test]
fn list_supported_formats_exposes_anycubic_manual_target_facts() {
    let json_str = list_supported_formats().unwrap();
    let formats: serde_json::Value = serde_json::from_str(&json_str).expect("format metadata");
    let formats = formats.as_array().expect("format metadata array");

    assert!(formats.iter().any(|format| {
        format.get("name") == Some(&serde_json::Value::String("pwmx".to_string()))
            && format
                .get("specifics")
                .and_then(|value| value.get("extension"))
                == Some(&serde_json::Value::String("pwmx".to_string()))
            && format
                .get("specifics")
                .and_then(|value| value.get("display_name"))
                == Some(&serde_json::Value::String("Photon Mono X".to_string()))
    }));
    assert!(formats.iter().any(|format| {
        format.get("name") == Some(&serde_json::Value::String("m5sp".to_string()))
            && format
                .get("specifics")
                .and_then(|value| value.get("default_version"))
                == Some(&serde_json::Value::String("v518".to_string()))
    }));
}

#[wasm_bindgen_test]
fn list_supported_formats_exposes_anycubic_archive_specifics() {
    let json_str = list_supported_formats().unwrap();
    let formats: serde_json::Value = serde_json::from_str(&json_str).expect("format metadata");
    let formats = formats.as_array().expect("format metadata array");
    let anycubic_zip = formats
        .iter()
        .find(|format| {
            format.get("name") == Some(&serde_json::Value::String("anycubic-zip".to_string()))
        })
        .expect("anycubic zip facts");
    let specifics = anycubic_zip
        .get("specifics")
        .expect("anycubic zip specifics");

    assert_eq!(
        specifics.get("family"),
        Some(&serde_json::Value::String("anycubic-archive".to_string()))
    );
    assert_eq!(
        specifics.get("layer_entry_prefix"),
        Some(&serde_json::Value::String(
            "layer_images/layer_".to_string()
        ))
    );
    assert_eq!(
        specifics.get("settings_manifest_entry"),
        Some(&serde_json::Value::String(
            "anycubic_photon_resins.pwsp".to_string()
        ))
    );
    assert_eq!(
        specifics.get("layers_manifest_entry"),
        Some(&serde_json::Value::String(
            "layers_controller.conf".to_string()
        ))
    );
    assert_eq!(
        specifics.get("default_layer_image_extension"),
        Some(&serde_json::Value::String("pw0Img".to_string()))
    );
    assert_eq!(
        specifics
            .get("layer_image_extensions")
            .and_then(|value| value.as_array())
            .expect("layer image extensions")
            .iter()
            .map(|value| value.as_str().expect("extension"))
            .collect::<Vec<_>>(),
        vec!["pw0Img", "pwszImg"]
    );
}

#[wasm_bindgen_test]
fn decode_layer_unknown_format_exposes_structured_error_fields() {
    let error = decode_layer(&[], 0, "not-a-format", 1, 1, 1, 0).unwrap_err();
    let error_value = JsValue::from(error);

    let message = Reflect::get(&error_value, &JsValue::from_str("message"))
        .unwrap()
        .as_string()
        .unwrap();
    let code = Reflect::get(&error_value, &JsValue::from_str("code"))
        .unwrap()
        .as_string()
        .unwrap();
    let retryable = Reflect::get(&error_value, &JsValue::from_str("retryable"))
        .unwrap()
        .as_bool()
        .unwrap();

    assert_eq!(message, "Unknown file format");
    assert_eq!(code, "E_UNKNOWN_FORMAT");
    assert!(!retryable);
}

// ── PW0 WASM API tests ─────────────────────────────────────────────────

fn pw0_quantize(pixels: &[u8]) -> Vec<u8> {
    pixels
        .iter()
        .map(|&p| {
            let code = p >> 4;
            (code << 4) | code
        })
        .collect()
}

fn pws_binarize(pixels: &[u8]) -> Vec<u8> {
    pixels
        .iter()
        .map(|&p| if p >= 128 { 0xFF } else { 0x00 })
        .collect()
}

#[wasm_bindgen_test]
fn parse_header_detects_pw0_fixture() {
    let result = parse_result(parse_header(FIXTURE_PW0, Some("pw0".to_string())).unwrap());

    assert_eq!(result.format, "pw0");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}

#[wasm_bindgen_test]
fn parse_header_detects_pws_fixture() {
    let result = parse_result(parse_header(FIXTURE_PWS, Some("pws".to_string())).unwrap());

    assert_eq!(result.format, "pws");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}

#[wasm_bindgen_test]
fn parse_header_detects_vdt_fixture() {
    let result = parse_result(parse_header(FIXTURE_VDT, Some("vdt".to_string())).unwrap());

    assert_eq!(result.format, "vdt");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}

#[wasm_bindgen_test]
fn pw0_roundtrip_via_exported_api() {
    let width = 32;
    let height = 24;
    let original = pw0_quantize(&make_test_layer(width, height));

    let encoded = encode_layer(&original, 0, "pw0", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let decoded = decode_layer(&encoded, 0, "pw0", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn pws_roundtrip_via_exported_api() {
    let width = 32;
    let height = 24;
    let original = pws_binarize(&make_test_layer(width, height));

    let encoded = encode_layer(&original, 0, "pws", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let decoded = decode_layer(&encoded, 0, "pws", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn pw0_and_pws_roundtrip_via_exported_api_after_family_refactor() {
    let width = 32;
    let height = 24;
    let source: Vec<u8> = (0..(width * height) as usize)
        .map(|index| if index % 2 == 0 { 0xFF } else { 0x00 })
        .collect();

    let pw0_encoded = encode_layer(&source, 0, "pw0", width, height, None, None).unwrap();
    let pw0_encoded = bytes(&pw0_encoded);
    let pw0_decoded = decode_layer(&pw0_encoded, 0, "pw0", width, height, 1, 0).unwrap();
    assert_eq!(bytes(&pw0_decoded).len(), source.len());

    let pws_encoded = encode_layer(&source, 0, "pws", width, height, None, None).unwrap();
    let pws_encoded = bytes(&pws_encoded);
    let pws_decoded = decode_layer(&pws_encoded, 0, "pws", width, height, 1, 0).unwrap();
    assert_eq!(bytes(&pws_decoded).len(), source.len());
}

#[wasm_bindgen_test]
fn vdt_roundtrip_via_exported_api() {
    let width = 32;
    let height = 24;
    let original = make_test_layer(width, height);

    let encoded = encode_layer(&original, 0, "vdt", width, height, None, None).unwrap();
    let encoded = bytes(&encoded);
    let decoded = decode_layer(&encoded, 0, "vdt", width, height, 1, 0).unwrap();

    assert_eq!(bytes(&decoded), original);
}

#[wasm_bindgen_test]
fn pw0_cross_format_via_exported_api() {
    let width = 32;
    let height = 24;
    let original = pw0_quantize(&make_test_layer(width, height));

    // PW0 → GOO → verify
    let pw0_encoded = encode_layer(&original, 0, "pw0", width, height, None, None).unwrap();
    let pw0_encoded = bytes(&pw0_encoded);
    let decoded = decode_layer(&pw0_encoded, 0, "pw0", width, height, 1, 0).unwrap();
    let decoded = bytes(&decoded);
    let goo_encoded = encode_layer(&decoded, 0, "goo", width, height, None, None).unwrap();
    let goo_encoded = bytes(&goo_encoded);
    let from_goo = decode_layer(&goo_encoded, 0, "goo", width, height, 1, 0).unwrap();

    assert_eq!(
        bytes(&from_goo),
        original,
        "PW0→GOO cross-format via WASM API failed"
    );
}

#[wasm_bindgen_test]
fn finalize_header_patches_pw0_header() {
    let parsed = parse_result(parse_header(FIXTURE_PW0, Some("pw0".to_string())).unwrap());
    let metadata_json = serde_json::to_string(&parsed.metadata).unwrap();
    let header = build_header(&metadata_json, "pw0", None).unwrap();
    let header = bytes(&header);
    let layer_sizes = vec![32u32; parsed.layers.len()];
    let patched = finalize_header(&header, "pw0", &layer_sizes, None).unwrap();
    let patched = bytes(&patched);

    assert_eq!(patched.len(), header.len());
    assert_ne!(patched, header);
}
