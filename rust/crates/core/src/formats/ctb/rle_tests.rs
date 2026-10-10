use super::rle::{decode_cbddlp_rle, decode_ctb_rle, decode_ctb_rle_preview_rgba, encode_ctb_rle};
use crate::common::downsample_to_rgba_nearest;

#[test]
fn roundtrip_uniform_black() {
    let pixels = vec![0u8; 100 * 100];
    let encoded = encode_ctb_rle(&pixels).unwrap();
    let decoded = decode_ctb_rle(&encoded, 100, 100).unwrap();
    assert_eq!(pixels, decoded);
}

#[test]
fn roundtrip_uniform_white() {
    let pixels = vec![254u8; 50 * 50];
    let encoded = encode_ctb_rle(&pixels).unwrap();
    let decoded = decode_ctb_rle(&encoded, 50, 50).unwrap();
    assert!(decoded.iter().all(|&p| p == 255));
}

#[test]
fn roundtrip_alternating() {
    let mut pixels = vec![0u8; 200];
    for i in (0..200).step_by(2) {
        pixels[i] = 128;
    }
    let encoded = encode_ctb_rle(&pixels).unwrap();
    let decoded = decode_ctb_rle(&encoded, 10, 20).unwrap();
    for i in 0..200 {
        let expected_gray7 = pixels[i] >> 1;
        let actual_gray7 = decoded[i] >> 1;
        assert_eq!(expected_gray7, actual_gray7, "mismatch at pixel {i}");
    }
}

#[test]
fn roundtrip_large_stride() {
    let pixels = vec![100u8; 1_000_000];
    let encoded = encode_ctb_rle(&pixels).unwrap();
    let decoded = decode_ctb_rle(&encoded, 1000, 1000).unwrap();
    let expected_gray7 = 100u8 >> 1;
    for &p in &decoded {
        assert_eq!(p >> 1, expected_gray7);
    }
}

#[test]
fn cbddlp_simple() {
    let data = vec![0x0A, 0x85];
    let decoded = decode_cbddlp_rle(&data, 15, 1, 1).unwrap();
    assert_eq!(decoded.len(), 15);
    assert!(decoded[..10].iter().all(|&p| p == 0));
    assert!(decoded[10..].iter().all(|&p| p == 255));
}

#[test]
fn preview_fast_path_matches_generic_downsample() {
    let width = 128;
    let height = 96;
    let max_preview_size = 48;
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            pixels[y * width as usize + x] = ((x + y) % 256) as u8;
        }
    }

    let encoded = encode_ctb_rle(&pixels).unwrap();
    let (rgba_fast, w_fast, h_fast) =
        decode_ctb_rle_preview_rgba(&encoded, width, height, max_preview_size).unwrap();

    let decoded = decode_ctb_rle(&encoded, width, height).unwrap();
    let expected = downsample_to_rgba_nearest(&decoded, width, height, max_preview_size);
    assert_eq!(w_fast, expected.width);
    assert_eq!(h_fast, expected.height);
    assert_eq!(rgba_fast, expected.pixels);
}
