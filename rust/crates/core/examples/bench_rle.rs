/// Simple benchmark for RLE encode/decode performance using real GOO data.
///
/// Usage: cargo run --example bench_rle --release
use slicer_converter_core::formats::{ctb, goo};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let goo_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Untitled_20260413_1125_TI3U.goo");
    let data = std::fs::read(&goo_path).unwrap_or_else(|_| {
        eprintln!("File not found: {}", goo_path.display());
        std::process::exit(1);
    });

    let decoder = goo::decoder::GooDecoder::parse_header(&data).unwrap();
    let result = decoder.to_parse_result();
    let meta = &result.metadata;
    let w = meta.printer.resolution_x;
    let h = meta.printer.resolution_y;

    println!("Resolution: {w}x{h}, layers: {}", meta.slicing.layer_count);
    println!("Pixels per layer: {}", w as u64 * h as u64);

    // Collect first N layers for benchmark (override via BENCH_RLE_LAYERS).
    let requested_layers = std::env::var("BENCH_RLE_LAYERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20);
    let n = requested_layers.max(1).min(result.layers.len());
    let mut raw_layers: Vec<Vec<u8>> = Vec::new();
    let mut goo_encoded: Vec<&[u8]> = Vec::new();

    for layer_offset in &result.layers[..n] {
        let layer_data =
            &data[layer_offset.offset as usize..(layer_offset.offset + layer_offset.size) as usize];
        goo_encoded.push(layer_data);
    }

    // --- GOO Decode ---
    let start = Instant::now();
    for &layer_data in &goo_encoded {
        let pixels = goo::decode_layer_data(layer_data, w, h).unwrap();
        raw_layers.push(pixels);
    }
    let goo_decode_total = start.elapsed();
    println!(
        "\nGOO decode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        goo_decode_total.as_secs_f64() * 1000.0,
        goo_decode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- CTB Encode ---
    let mut ctb_encoded: Vec<Vec<u8>> = Vec::new();
    let start = Instant::now();
    for pixels in &raw_layers {
        let encoded = ctb::encode_layer_data(pixels, 0, w, h).unwrap();
        ctb_encoded.push(encoded);
    }
    let ctb_encode_total = start.elapsed();
    println!(
        "CTB encode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        ctb_encode_total.as_secs_f64() * 1000.0,
        ctb_encode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- CTB Decode ---
    let start = Instant::now();
    for encoded in &ctb_encoded {
        let _pixels = ctb::decode_layer_data(encoded, 0, w, h, 1, 0).unwrap();
    }
    let ctb_decode_total = start.elapsed();
    println!(
        "CTB decode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        ctb_decode_total.as_secs_f64() * 1000.0,
        ctb_decode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- GOO Encode ---
    let start = Instant::now();
    for pixels in &raw_layers {
        let _encoded = goo::encode_layer_data(pixels, w, h).unwrap();
    }
    let goo_encode_total = start.elapsed();
    println!(
        "GOO encode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        goo_encode_total.as_secs_f64() * 1000.0,
        goo_encode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- SL1 Encode (PNG) ---
    let start = Instant::now();
    for pixels in &raw_layers {
        let _encoded =
            slicer_converter_core::formats::sl1::encode_layer_data(pixels, w, h).unwrap();
    }
    let sl1_encode_total = start.elapsed();
    println!(
        "SL1 encode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        sl1_encode_total.as_secs_f64() * 1000.0,
        sl1_encode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- SL1 Decode (PNG) ---
    let sl1_encoded: Vec<Vec<u8>> = raw_layers
        .iter()
        .map(|p| slicer_converter_core::formats::sl1::encode_layer_data(p, w, h).unwrap())
        .collect();
    let start = Instant::now();
    for encoded in &sl1_encoded {
        let _pixels =
            slicer_converter_core::formats::sl1::decode_layer_data(encoded, w, h).unwrap();
    }
    let sl1_decode_total = start.elapsed();
    println!(
        "SL1 decode: {n} layers in {:.1}ms ({:.2}ms/layer)",
        sl1_decode_total.as_secs_f64() * 1000.0,
        sl1_decode_total.as_secs_f64() * 1000.0 / n as f64
    );

    // --- Total pipeline ---
    let total = goo_decode_total + ctb_encode_total;
    println!(
        "\nPipeline (GOO decode + CTB encode): {:.1}ms ({:.2}ms/layer)",
        total.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0 / n as f64
    );
}
