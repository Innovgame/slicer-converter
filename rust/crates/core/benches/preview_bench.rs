use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use slicer_converter_core::common::downsample_grayscale_nearest;
use slicer_converter_core::formats::{self, common::FormatSpec, LayerDecodeContext};
use std::path::PathBuf;

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join(name)
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

struct TestFile {
    label: &'static str,
    data: Vec<u8>,
    format: &'static str,
    width: u32,
    height: u32,
    anti_alias_level: u32,
}

fn parse_document(
    fmt: &'static FormatSpec,
    data: &[u8],
) -> slicer_converter_core::formats::common::ParsedDocument {
    fmt.pipeline.parse_document(fmt, data).unwrap()
}

fn load_test_files() -> Vec<TestFile> {
    let mut files = Vec::new();

    // Real large CTB file (~14MB, high resolution)
    if let Ok(data) = std::fs::read(data_path("converted_real.ctb")) {
        let fmt = formats::detect_format_spec(&data, Some("ctb")).unwrap();
        let parsed = parse_document(fmt, &data);
        let meta = parsed.metadata;
        files.push(TestFile {
            label: "real_ctb",
            data,
            format: "ctb",
            width: meta.printer.resolution_x,
            height: meta.printer.resolution_y,
            anti_alias_level: meta.slicing.anti_alias_level,
        });
    }

    // Real large GOO file (~12MB)
    if let Ok(data) = std::fs::read(data_path("Untitled_20260413_1125_TI3U.goo")) {
        let fmt = formats::detect_format_spec(&data, Some("goo")).unwrap();
        let parsed = parse_document(fmt, &data);
        let meta = parsed.metadata;
        files.push(TestFile {
            label: "real_goo",
            data,
            format: "goo",
            width: meta.printer.resolution_x,
            height: meta.printer.resolution_y,
            anti_alias_level: meta.slicing.anti_alias_level,
        });
    }

    // Small fixture files (64x64)
    for (name, format) in [
        ("sample.ctb", "ctb"),
        ("sample.goo", "goo"),
        ("sample.pw0", "pw0"),
        ("sample.pws", "pws"),
    ] {
        if let Ok(data) = std::fs::read(fixture_path(name)) {
            files.push(TestFile {
                label: Box::leak(format!("fixture_{format}").into_boxed_str()),
                data,
                format,
                width: 64,
                height: 64,
                anti_alias_level: 1,
            });
        }
    }

    files
}

fn bench_preview_old_path(c: &mut Criterion) {
    let files = load_test_files();
    let mut group = c.benchmark_group("preview_old_path");

    for file in &files {
        let fmt = formats::get_format_spec_by_name(file.format).unwrap();
        let layers = parse_document(fmt, &file.data).layers;
        if layers.is_empty() {
            continue;
        }

        let ctx = LayerDecodeContext::new(0, file.width, file.height).with_options(
            slicer_converter_core::formats::LayerOptions::new()
                .with_anti_alias_level(file.anti_alias_level),
        );

        group.bench_with_input(
            BenchmarkId::new("decode_layer+downsample", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                b.iter(|| {
                    // Old path: allocate new Vec each time + downsample
                    let pixels = fmt.decode_layer(black_box(data), ctx).unwrap();
                    let _preview =
                        downsample_grayscale_nearest(&pixels, file.width, file.height, 2048);
                });
            },
        );
    }

    group.finish();
}

fn bench_preview_new_path(c: &mut Criterion) {
    let files = load_test_files();
    let mut group = c.benchmark_group("preview_new_path");

    for file in &files {
        let fmt = formats::get_format_spec_by_name(file.format).unwrap();
        let layers = parse_document(fmt, &file.data).layers;
        if layers.is_empty() {
            continue;
        }

        let ctx = LayerDecodeContext::new(0, file.width, file.height).with_options(
            slicer_converter_core::formats::LayerOptions::new()
                .with_anti_alias_level(file.anti_alias_level),
        );

        group.bench_with_input(
            BenchmarkId::new("decode_into+downsample", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                // Pre-allocate reusable buffer (simulates thread_local)
                let mut buf = Vec::new();
                b.iter(|| {
                    buf.clear();
                    fmt.decode_layer_into(black_box(data), ctx, &mut buf)
                        .unwrap();
                    let _preview =
                        downsample_grayscale_nearest(&buf, file.width, file.height, 2048);
                });
            },
        );
    }

    group.finish();
}

fn bench_preview_comparison(c: &mut Criterion) {
    let files = load_test_files();
    let mut group = c.benchmark_group("preview_comparison");

    for file in &files {
        let fmt = formats::get_format_spec_by_name(file.format).unwrap();
        let layers = parse_document(fmt, &file.data).layers;
        if layers.is_empty() {
            continue;
        }

        let ctx = LayerDecodeContext::new(0, file.width, file.height).with_options(
            slicer_converter_core::formats::LayerOptions::new()
                .with_anti_alias_level(file.anti_alias_level),
        );

        // Old: new alloc each time
        // Verify decode works before benchmarking
        let test_result = fmt.decode_layer(black_box(&file.data), &ctx);
        if test_result.is_err() {
            eprintln!(
                "Skipping {} (decode error: {:?})",
                file.label,
                test_result.err()
            );
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("old_alloc", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                b.iter(|| {
                    let pixels = fmt.decode_layer(black_box(data), ctx).unwrap();
                    downsample_grayscale_nearest(&pixels, file.width, file.height, 2048)
                });
            },
        );

        // New: reuse buffer
        group.bench_with_input(
            BenchmarkId::new("new_reuse", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                let mut buf = Vec::new();
                b.iter(|| {
                    buf.clear();
                    fmt.decode_layer_into(black_box(data), ctx, &mut buf)
                        .unwrap();
                    downsample_grayscale_nearest(&buf, file.width, file.height, 2048)
                });
            },
        );

        // Decode-only (no downsample) - isolate decode cost
        group.bench_with_input(
            BenchmarkId::new("decode_only_alloc", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                b.iter(|| fmt.decode_layer(black_box(data), ctx).unwrap());
            },
        );

        // Decode-only reuse
        group.bench_with_input(
            BenchmarkId::new("decode_only_reuse", file.label),
            &(&file.data, &ctx),
            |b, (data, ctx)| {
                let mut buf = Vec::new();
                b.iter(|| {
                    buf.clear();
                    fmt.decode_layer_into(black_box(data), ctx, &mut buf)
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    preview_benches,
    bench_preview_old_path,
    bench_preview_new_path,
    bench_preview_comparison,
);
criterion_main!(preview_benches);
