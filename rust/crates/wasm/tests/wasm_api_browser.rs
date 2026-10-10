#![cfg(target_arch = "wasm32")]

use slicer_converter_core::common::ParseResult;
use slicer_converter_wasm::parse_header;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const FIXTURE_GOO: &[u8] = include_bytes!("../../core/tests/fixtures/sample.goo");

fn parse_result(json: String) -> ParseResult {
    serde_json::from_str(&json).expect("parse_header should return ParseResult")
}

#[wasm_bindgen_test]
fn parse_header_detects_goo_fixture_in_browser() {
    let result = parse_result(parse_header(FIXTURE_GOO, Some("goo".to_string())).unwrap());

    assert_eq!(result.format, "goo");
    assert_eq!(result.metadata.printer.resolution_x, 64);
    assert_eq!(result.metadata.printer.resolution_y, 64);
    assert_eq!(result.layers.len(), 10);
}
