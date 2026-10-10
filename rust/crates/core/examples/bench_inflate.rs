/// Benchmark for deflate decompression (flate2/miniz_oxide).
/// Usage: cargo run --example bench_inflate --release
use std::io::{Cursor, Read, Write};
use std::time::Instant;

fn main() {
    // Generate ~2MB of realistic grayscale layer data
    // Mix of solid regions + gradients + noise (typical 3D print layer)
    let size = 2560 * 1440; // ~3.5MB (typical 2K layer)
    let mut data = Vec::with_capacity(size);
    let mut rng: u32 = 42;
    for i in 0..size {
        let x = (i % 2560) as u32;
        let y = (i / 2560) as u32;
        // Simulate a circular cross-section with some noise
        let cx = 1280i32;
        let cy = 720i32;
        let dx = x as i32 - cx;
        let dy = y as i32 - cy;
        let dist_sq = (dx * dx + dy * dy) as u32;
        let radius_sq = 500u32 * 500;
        if dist_sq < radius_sq {
            // Inside: mostly white with some gradient at edges
            let edge = (radius_sq - dist_sq).min(10000) as f32 / 10000.0;
            // LCG pseudo-random for noise
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let noise = ((rng >> 16) % 8) as u8;
            data.push((255.0 * edge) as u8 + noise);
        } else {
            data.push(0u8); // Black outside
        }
    }

    // Compress with default level
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&data).unwrap();
    let compressed = encoder.finish().unwrap();

    println!(
        "Original: {} bytes, Compressed: {} bytes, Ratio: {:.1}x",
        data.len(),
        compressed.len(),
        data.len() as f64 / compressed.len() as f64
    );

    // Benchmark decompression
    let iterations = 200;

    // Warmup
    for _ in 0..10 {
        let mut decoder = flate2::read::DeflateDecoder::new(Cursor::new(&compressed));
        let mut out = Vec::with_capacity(size);
        decoder.read_to_end(&mut out).unwrap();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let mut decoder = flate2::read::DeflateDecoder::new(Cursor::new(&compressed));
        let mut out = Vec::with_capacity(size);
        decoder.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), size);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    let throughput = (size as f64 * iterations as f64) / elapsed.as_secs_f64() / 1_000_000.0;

    println!(
        "flate2 (native Rust, release): {} iterations in {:.1}ms",
        iterations,
        elapsed.as_secs_f64() * 1000.0
    );
    println!("  avg: {:.2}ms/iter", avg_ms);
    println!("  throughput: {:.0} MB/s (decompressed)", throughput);
}
