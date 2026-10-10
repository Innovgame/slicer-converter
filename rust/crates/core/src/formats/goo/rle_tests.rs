use super::rle::{
    decode_goo_rle, encode_goo_rle, encode_goo_rle_with_options, encode_goo_rle_with_options_append,
};

fn read_stride(data: &[u8], i: &mut usize, byte0: u8) -> usize {
    let length_type = (byte0 >> 4) & 0x03;
    let low4 = (byte0 & 0x0F) as usize;

    match length_type {
        0b00 => low4,
        0b01 => {
            let b1 = data[*i] as usize;
            *i += 1;
            (b1 << 4) | low4
        }
        0b10 => {
            let b1 = data[*i] as usize;
            let b2 = data[*i + 1] as usize;
            *i += 2;
            (b1 << 12) | (b2 << 4) | low4
        }
        0b11 => {
            let b1 = data[*i] as usize;
            let b2 = data[*i + 1] as usize;
            let b3 = data[*i + 2] as usize;
            *i += 3;
            (b1 << 20) | (b2 << 12) | (b3 << 4) | low4
        }
        _ => unreachable!(),
    }
}

fn contains_diff_chunks(data: &[u8]) -> bool {
    let mut i = 1;
    let end = data.len().saturating_sub(1);

    while i < end {
        let byte0 = data[i];
        i += 1;

        match (byte0 >> 6) & 0x03 {
            0b00 | 0b11 => {
                let _ = read_stride(data, &mut i, byte0);
            }
            0b01 => {
                assert!(i < end, "expected gray value byte before checksum");
                i += 1;
                let _ = read_stride(data, &mut i, byte0);
            }
            0b10 => return true,
            _ => unreachable!(),
        }
    }

    false
}

#[test]
fn roundtrip_all_black() {
    let pixels = vec![0u8; 100 * 100];
    let encoded = encode_goo_rle(&pixels).unwrap();
    assert_eq!(encoded[0], 0x55);
    let decoded = decode_goo_rle(&encoded, 100, 100).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_all_white() {
    let pixels = vec![0xFFu8; 50 * 50];
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 50, 50).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_gray_pattern() {
    let mut pixels = vec![0u8; 200];
    for (i, p) in pixels.iter_mut().enumerate() {
        *p = (i % 256) as u8;
    }
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 10, 20).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_large_solid() {
    let pixels = vec![128u8; 1_000_000];
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 1000, 1000).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_repeated_pairs() {
    let pixels: Vec<u8> = (0..50).flat_map(|v| [v * 2, v * 2]).collect();
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 10, 10).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_all_unique() {
    let pixels: Vec<u8> = (0..=255).collect();
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 16, 16).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_alternating_bw() {
    let pixels: Vec<u8> = (0..1000)
        .map(|i| if i % 2 == 0 { 0x00 } else { 0xFF })
        .collect();
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 50, 20).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_long_gray_run() {
    let pixels = vec![42u8; 100_000];
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 1000, 100).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_descending() {
    let pixels: Vec<u8> = (0..=255).rev().collect();
    let encoded = encode_goo_rle(&pixels).unwrap();
    let decoded = decode_goo_rle(&encoded, 16, 16).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn roundtrip_without_diff_compression_for_prz() {
    let pixels: Vec<u8> = (0..=255).cycle().take(16 * 16).collect();
    let encoded = encode_goo_rle_with_options(&pixels, false).unwrap();
    assert!(!contains_diff_chunks(&encoded));

    let decoded = decode_goo_rle(&encoded, 16, 16).unwrap();
    assert_eq!(decoded, pixels);
}

#[test]
fn append_encoder_preserves_prefix_and_checksums_only_appended_rle() {
    let prefix = [0xAA, 0xBB, 0xCC, 0xDD];
    let pixels = vec![0, 32, 32, 64, 255, 255, 255, 0];
    let standalone = encode_goo_rle_with_options(&pixels, true).unwrap();
    let mut appended = prefix.to_vec();

    encode_goo_rle_with_options_append(&pixels, true, &mut appended).unwrap();

    assert_eq!(&appended[..prefix.len()], prefix);
    assert_eq!(&appended[prefix.len()..], standalone.as_slice());
}
