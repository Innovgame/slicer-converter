use slicer_converter_core::{
    common::ConvertError,
    formats::{self, ctb::CtbDecoder},
};

const SAMPLE: &[u8] = include_bytes!("fixtures/sample.ctb");

#[test]
fn rejects_truncated_layer_table_before_allocating() {
    let table = u32::from_le_bytes(SAMPLE[64..68].try_into().unwrap()) as usize;
    let error = CtbDecoder::new()
        .parse_header(&SAMPLE[..table])
        .expect_err("truncated table must fail");
    assert!(matches!(error, ConvertError::BufferTooSmall { .. }));
}

#[test]
fn rejects_excessive_layer_count_before_allocating() {
    let mut data = SAMPLE.to_vec();
    data[68..72].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(CtbDecoder::new().parse_header(&data).is_err());
}

#[test]
fn header_prefix_does_not_require_payload_but_complete_document_does() {
    let (_, layers) = CtbDecoder::new().parse_header(SAMPLE).unwrap();
    let first_payload = layers[0].offset as usize;
    let prefix = &SAMPLE[..first_payload];
    assert_eq!(CtbDecoder::new().parse_header(prefix).unwrap().1.len(), 10);
    let spec = formats::detect_format_spec(SAMPLE, Some("ctb")).unwrap();
    assert!(
        spec.pipeline
            .parse_document(spec, prefix)
            .unwrap()
            .validate_complete(prefix.len())
            .is_err(),
        "complete documents require all payload bytes"
    );
    assert_eq!(
        spec.pipeline
            .parse_document(spec, SAMPLE)
            .unwrap()
            .layers
            .len(),
        10
    );
}
