//! Quick benchmark comparing PNG compression levels for SL1/VDT layer encoding.
//!
//! Usage: cargo run --release --example bench_png_levels
//!
//! Tests a realistic sparse grayscale layer (mostly black, small white region)
//! at the actual printer resolution used in our bench sample (8520×4320).

use std::io::Cursor;
use std::time::Instant;

use flate2::write::DeflateEncoder;
use flate2::Compression as FlateCompression;
use std::io::Write;

fn make_sparse_layer(width: usize, height: usize) -> Vec<u8> {
    // Simulate a typical 3D print layer: mostly black with a small object region.
    // ~5% of pixels are non-zero (circle in center).
    let mut pixels = vec![0u8; width * height];
    let cx = width / 2;
    let cy = height / 2;
    let radius = ((width.min(height) as f64) * 0.12) as usize; // ~12% of smaller dim
    for y in 0..height {
        for x in 0..width {
            let dx = x as isize - cx as isize;
            let dy = y as isize - cy as isize;
            if (dx * dx + dy * dy) < (radius * radius) as isize {
                pixels[y * width + x] = 200;
            }
        }
    }
    pixels
}

fn encode_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    compression: png::Compression,
    filter: png::Filter,
) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut buf), width, height);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(compression);
        encoder.set_filter(filter);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
    }
    buf
}

fn zip_deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), FlateCompression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn main() {
    let width: u32 = 8520;
    let height: u32 = 4320;
    let pixels = make_sparse_layer(width as usize, height as usize);
    let raw_size = pixels.len();
    println!(
        "Raw pixel buffer: {} bytes ({:.1} MB)",
        raw_size,
        raw_size as f64 / 1048576.0
    );
    println!("Resolution: {}×{}", width, height);
    println!();

    let configs: Vec<(&str, png::Compression, png::Filter)> = vec![
        (
            "Fast+NoFilter (current)",
            png::Compression::Fast,
            png::Filter::NoFilter,
        ),
        ("Fast+Sub", png::Compression::Fast, png::Filter::Sub),
        (
            "Balanced+NoFilter",
            png::Compression::Balanced,
            png::Filter::NoFilter,
        ),
        ("Balanced+Sub", png::Compression::Balanced, png::Filter::Sub),
        (
            "High+NoFilter",
            png::Compression::High,
            png::Filter::NoFilter,
        ),
        ("High+Sub", png::Compression::High, png::Filter::Sub),
        (
            "NoCompression+NoFilter",
            png::Compression::NoCompression,
            png::Filter::NoFilter,
        ),
    ];

    println!(
        "{:<28} {:>10} {:>10} {:>12} {:>10} {:>12}",
        "Config", "PNG KB", "PNG ms", "PNG+ZIP KB", "ZIP ms", "Total ms"
    );
    println!("{}", "-".repeat(88));

    for (name, compression, filter) in &configs {
        // Warmup
        let _ = encode_png(&pixels, width, height, *compression, *filter);

        // PNG encode (average of 3 runs)
        let mut png_times = Vec::new();
        let mut png_bytes = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let png_data = encode_png(&pixels, width, height, *compression, *filter);
            png_times.push(start.elapsed().as_secs_f64() * 1000.0);
            png_bytes = png_data;
        }
        let avg_png_ms = png_times.iter().sum::<f64>() / png_times.len() as f64;
        let png_kb = png_bytes.len() as f64 / 1024.0;

        // ZIP deflate of PNG output (average of 3 runs)
        let mut zip_times = Vec::new();
        let mut zip_bytes = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let zipped = zip_deflate(&png_bytes);
            zip_times.push(start.elapsed().as_secs_f64() * 1000.0);
            zip_bytes = zipped;
        }
        let avg_zip_ms = zip_times.iter().sum::<f64>() / zip_times.len() as f64;
        let zip_kb = zip_bytes.len() as f64 / 1024.0;

        println!(
            "{:<28} {:>10.1} {:>10.1} {:>12.1} {:>10.1} {:>12.1}",
            name,
            png_kb,
            avg_png_ms,
            zip_kb,
            avg_zip_ms,
            avg_png_ms + avg_zip_ms
        );
    }

    println!();
    println!("Note: 'PNG+ZIP KB' = size after ZIP deflate of the PNG bytes.");
    println!("If PNG alone is small enough, ZIP stored (method 0) becomes viable.");
}
