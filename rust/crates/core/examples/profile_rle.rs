use slicer_converter_core::formats::goo;
use slicer_converter_core::formats::goo::decoder::GooDecoder;
use slicer_converter_core::formats::goo::LAYER_MAGIC;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GooChunkKind {
    Black,
    White,
    Gray,
    DiffSinglePositive,
    DiffRunPositive,
    DiffSingleNegative,
    DiffRunNegative,
}

#[derive(Debug, Clone, Default)]
struct GooChunkStats {
    black_chunks: u64,
    black_pixels: u64,
    white_chunks: u64,
    white_pixels: u64,
    gray_chunks: u64,
    gray_pixels: u64,
    diff_single_positive_chunks: u64,
    diff_single_positive_pixels: u64,
    diff_run_positive_chunks: u64,
    diff_run_positive_pixels: u64,
    diff_single_negative_chunks: u64,
    diff_single_negative_pixels: u64,
    diff_run_negative_chunks: u64,
    diff_run_negative_pixels: u64,
    total_chunks: u64,
    total_pixels: u64,
}

#[derive(Debug, Clone, Default)]
struct CtbRunStats {
    total_runs: u64,
    total_pixels: u64,
    single_runs: u64,
    bucket_2_3: u64,
    bucket_4_7: u64,
    bucket_8_15: u64,
    bucket_16_31: u64,
    bucket_32_63: u64,
    bucket_64_127: u64,
    bucket_128_255: u64,
    bucket_256_plus: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let input_path = std::env::args().nth(1).map(PathBuf::from).ok_or_else(|| {
        "usage: cargo run --example profile_rle --release -- <input.goo>".to_string()
    })?;
    let data = std::fs::read(&input_path)
        .map_err(|error| format!("failed to read input file: {error}"))?;

    let decoder = GooDecoder::parse_header(&data)
        .map_err(|error| format!("failed to parse goo header: {error}"))?;
    let parsed = decoder.to_parse_result();
    let width = parsed.metadata.printer.resolution_x;
    let height = parsed.metadata.printer.resolution_y;

    let mut goo_stats = GooChunkStats::default();
    let mut ctb_stats = CtbRunStats::default();

    for layer in &parsed.layers {
        let start = layer.offset as usize;
        let end = start + layer.size as usize;
        let layer_data = &data[start..end];
        scan_goo_layer(layer_data, &mut goo_stats).map_err(|error| {
            format!(
                "failed to scan goo layer at offset {} (size {}): {error}",
                layer.offset, layer.size
            )
        })?;

        let pixels = goo::decode_layer_data(layer_data, width, height)
            .map_err(|error| format!("failed to decode goo layer into pixels: {error}"))?;
        scan_ctb_runs(&pixels, &mut ctb_stats);
    }

    println!("file: {}", input_path.display());
    println!(
        "resolution: {}x{}, layers: {}",
        width,
        height,
        parsed.layers.len()
    );
    println!();
    print_goo_stats(&goo_stats);
    println!();
    print_ctb_stats(&ctb_stats);
    Ok(())
}

fn scan_goo_layer(data: &[u8], stats: &mut GooChunkStats) -> Result<(), String> {
    if data.is_empty() {
        return Err("empty goo layer".into());
    }
    if data[0] != LAYER_MAGIC {
        return Err(format!(
            "expected goo layer magic 0x55, got {:#04x}",
            data[0]
        ));
    }

    let mut index = 1usize;
    let end = data.len().saturating_sub(1);
    while index < end {
        let byte0 = data[index];
        index += 1;
        match (byte0 >> 6) & 0x03 {
            0b00 => {
                let stride = read_goo_stride(data, &mut index, byte0, end)?;
                record_goo_chunk(stats, GooChunkKind::Black, stride);
            }
            0b11 => {
                let stride = read_goo_stride(data, &mut index, byte0, end)?;
                record_goo_chunk(stats, GooChunkKind::White, stride);
            }
            0b01 => {
                if index >= end {
                    return Err("gray chunk missing value byte".into());
                }
                index += 1;
                let stride = read_goo_stride(data, &mut index, byte0, end)?;
                record_goo_chunk(stats, GooChunkKind::Gray, stride);
            }
            0b10 => {
                let diff_mode = (byte0 >> 4) & 0x03;
                match diff_mode {
                    0b00 => record_goo_chunk(stats, GooChunkKind::DiffSinglePositive, 1),
                    0b10 => record_goo_chunk(stats, GooChunkKind::DiffSingleNegative, 1),
                    0b01 => {
                        if index >= end {
                            return Err("positive diff run missing count byte".into());
                        }
                        let run = data[index] as usize;
                        index += 1;
                        record_goo_chunk(stats, GooChunkKind::DiffRunPositive, run);
                    }
                    0b11 => {
                        if index >= end {
                            return Err("negative diff run missing count byte".into());
                        }
                        let run = data[index] as usize;
                        index += 1;
                        record_goo_chunk(stats, GooChunkKind::DiffRunNegative, run);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

fn read_goo_stride(data: &[u8], index: &mut usize, byte0: u8, end: usize) -> Result<usize, String> {
    let length_type = (byte0 >> 4) & 0x03;
    let low4 = (byte0 & 0x0F) as usize;
    match length_type {
        0b00 => Ok(low4),
        0b01 => {
            if *index >= end {
                return Err("unexpected end while reading 12-bit stride".into());
            }
            let b1 = data[*index] as usize;
            *index += 1;
            Ok((b1 << 4) | low4)
        }
        0b10 => {
            if *index + 1 >= end {
                return Err("unexpected end while reading 20-bit stride".into());
            }
            let b1 = data[*index] as usize;
            let b2 = data[*index + 1] as usize;
            *index += 2;
            Ok((b1 << 12) | (b2 << 4) | low4)
        }
        0b11 => {
            if *index + 2 >= end {
                return Err("unexpected end while reading 28-bit stride".into());
            }
            let b1 = data[*index] as usize;
            let b2 = data[*index + 1] as usize;
            let b3 = data[*index + 2] as usize;
            *index += 3;
            Ok((b1 << 20) | (b2 << 12) | (b3 << 4) | low4)
        }
        _ => unreachable!(),
    }
}

fn record_goo_chunk(stats: &mut GooChunkStats, kind: GooChunkKind, pixels: usize) {
    let pixels = pixels as u64;
    stats.total_chunks += 1;
    stats.total_pixels += pixels;
    match kind {
        GooChunkKind::Black => {
            stats.black_chunks += 1;
            stats.black_pixels += pixels;
        }
        GooChunkKind::White => {
            stats.white_chunks += 1;
            stats.white_pixels += pixels;
        }
        GooChunkKind::Gray => {
            stats.gray_chunks += 1;
            stats.gray_pixels += pixels;
        }
        GooChunkKind::DiffSinglePositive => {
            stats.diff_single_positive_chunks += 1;
            stats.diff_single_positive_pixels += pixels;
        }
        GooChunkKind::DiffRunPositive => {
            stats.diff_run_positive_chunks += 1;
            stats.diff_run_positive_pixels += pixels;
        }
        GooChunkKind::DiffSingleNegative => {
            stats.diff_single_negative_chunks += 1;
            stats.diff_single_negative_pixels += pixels;
        }
        GooChunkKind::DiffRunNegative => {
            stats.diff_run_negative_chunks += 1;
            stats.diff_run_negative_pixels += pixels;
        }
    }
}

fn scan_ctb_runs(pixels: &[u8], stats: &mut CtbRunStats) {
    if pixels.is_empty() {
        return;
    }

    let mut index = 0usize;
    while index < pixels.len() {
        let cur_gray7 = pixels[index] >> 1;
        let mut end = index + 1;
        while end < pixels.len() && (pixels[end] >> 1) == cur_gray7 {
            end += 1;
        }
        record_ctb_run(stats, end - index);
        index = end;
    }
}

fn record_ctb_run(stats: &mut CtbRunStats, len: usize) {
    let len_u64 = len as u64;
    stats.total_runs += 1;
    stats.total_pixels += len_u64;
    match len {
        1 => stats.single_runs += 1,
        2..=3 => stats.bucket_2_3 += 1,
        4..=7 => stats.bucket_4_7 += 1,
        8..=15 => stats.bucket_8_15 += 1,
        16..=31 => stats.bucket_16_31 += 1,
        32..=63 => stats.bucket_32_63 += 1,
        64..=127 => stats.bucket_64_127 += 1,
        128..=255 => stats.bucket_128_255 += 1,
        _ => stats.bucket_256_plus += 1,
    }
}

fn print_goo_stats(stats: &GooChunkStats) {
    println!("GOO decode chunk mix:");
    print_goo_line("black", stats.black_chunks, stats.black_pixels, stats);
    print_goo_line("white", stats.white_chunks, stats.white_pixels, stats);
    print_goo_line("gray", stats.gray_chunks, stats.gray_pixels, stats);
    print_goo_line(
        "diff single +",
        stats.diff_single_positive_chunks,
        stats.diff_single_positive_pixels,
        stats,
    );
    print_goo_line(
        "diff run +",
        stats.diff_run_positive_chunks,
        stats.diff_run_positive_pixels,
        stats,
    );
    print_goo_line(
        "diff single -",
        stats.diff_single_negative_chunks,
        stats.diff_single_negative_pixels,
        stats,
    );
    print_goo_line(
        "diff run -",
        stats.diff_run_negative_chunks,
        stats.diff_run_negative_pixels,
        stats,
    );
    println!(
        "  total: chunks={}, pixels={}",
        stats.total_chunks, stats.total_pixels
    );
}

fn print_goo_line(label: &str, chunks: u64, pixels: u64, total: &GooChunkStats) {
    let chunk_pct = percent(chunks, total.total_chunks);
    let pixel_pct = percent(pixels, total.total_pixels);
    println!(
        "  {:<13} chunks {:>10} ({:>6.2}%), pixels {:>14} ({:>6.2}%)",
        label, chunks, chunk_pct, pixels, pixel_pct
    );
}

fn print_ctb_stats(stats: &CtbRunStats) {
    println!("CTB gray7 run distribution:");
    print_ctb_line("1", stats.single_runs, stats);
    print_ctb_line("2-3", stats.bucket_2_3, stats);
    print_ctb_line("4-7", stats.bucket_4_7, stats);
    print_ctb_line("8-15", stats.bucket_8_15, stats);
    print_ctb_line("16-31", stats.bucket_16_31, stats);
    print_ctb_line("32-63", stats.bucket_32_63, stats);
    print_ctb_line("64-127", stats.bucket_64_127, stats);
    print_ctb_line("128-255", stats.bucket_128_255, stats);
    print_ctb_line("256+", stats.bucket_256_plus, stats);
    let avg = if stats.total_runs == 0 {
        0.0
    } else {
        stats.total_pixels as f64 / stats.total_runs as f64
    };
    println!(
        "  total: runs={}, pixels={}, avg_run_len={:.2}",
        stats.total_runs, stats.total_pixels, avg
    );
}

fn print_ctb_line(label: &str, runs: u64, total: &CtbRunStats) {
    let run_pct = percent(runs, total.total_runs);
    println!("  {:<8} runs {:>10} ({:>6.2}%)", label, runs, run_pct);
}

fn percent(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_goo_layer_counts_each_chunk_type_and_stride() {
        let layer = [
            LAYER_MAGIC,
            0b0000_0011,
            0b0100_0010,
            0x7F,
            0b1000_0101,
            0b1001_0010,
            4,
            0b1010_0011,
            0b1011_0100,
            6,
            0b1100_0111,
            0,
        ];
        let mut stats = GooChunkStats::default();

        scan_goo_layer(&layer, &mut stats).expect("goo scan should succeed");

        assert_eq!(stats.black_chunks, 1);
        assert_eq!(stats.black_pixels, 3);
        assert_eq!(stats.gray_chunks, 1);
        assert_eq!(stats.gray_pixels, 2);
        assert_eq!(stats.diff_single_positive_chunks, 1);
        assert_eq!(stats.diff_single_positive_pixels, 1);
        assert_eq!(stats.diff_run_positive_chunks, 1);
        assert_eq!(stats.diff_run_positive_pixels, 4);
        assert_eq!(stats.diff_single_negative_chunks, 1);
        assert_eq!(stats.diff_single_negative_pixels, 1);
        assert_eq!(stats.diff_run_negative_chunks, 1);
        assert_eq!(stats.diff_run_negative_pixels, 6);
        assert_eq!(stats.white_chunks, 1);
        assert_eq!(stats.white_pixels, 7);
        assert_eq!(stats.total_chunks, 7);
        assert_eq!(stats.total_pixels, 24);
    }

    #[test]
    fn scan_ctb_runs_groups_adjacent_pixels_by_gray7() {
        let pixels = [4u8, 5, 5, 10, 11, 11, 11, 64];
        let mut stats = CtbRunStats::default();

        scan_ctb_runs(&pixels, &mut stats);

        assert_eq!(stats.total_runs, 3);
        assert_eq!(stats.total_pixels, 8);
        assert_eq!(stats.bucket_2_3, 1);
        assert_eq!(stats.bucket_4_7, 1);
        assert_eq!(stats.single_runs, 1);
    }
}
