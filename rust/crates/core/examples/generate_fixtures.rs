use slicer_converter_core::common::{FileMetadata, FileMetadataBuilder};
use slicer_converter_core::formats::common::{
    ArchiveEncodeContext, FormatSpec, LayerEncodeContext, LayerSource, VecOutputSink,
};
use slicer_converter_core::formats::sl1;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Generate slicer fixture files (.goo, .ctb, .sl1, .pw0, .pws, .vdt) for regression testing.
fn encode_document(
    target_format: &str,
    metadata: &FileMetadata,
    encoded_layers: &[Vec<u8>],
    job_name: &str,
) -> Vec<u8> {
    let spec = slicer_converter_core::formats::get_format_spec_by_name(target_format)
        .expect("format spec");
    let mut sink = VecOutputSink::new();
    let ctx = ArchiveEncodeContext::new(job_name);
    spec.pipeline
        .encode_document_into(
            spec,
            metadata,
            LayerSource::Encoded(encoded_layers),
            &mut sink,
            &ctx,
        )
        .expect("document encode");
    sink.into_inner()
}

fn encode_layers(spec: &FormatSpec, metadata: &FileMetadata, layers: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let layer_metadata = metadata.as_layer_encode_metadata();
    layers
        .iter()
        .enumerate()
        .map(|(index, pixels)| {
            let ctx = LayerEncodeContext {
                layer_index: index as u32,
                width: metadata.printer.resolution_x,
                height: metadata.printer.resolution_y,
                layer_metadata: Some(layer_metadata),
            };
            spec.encode_layer(pixels, &ctx).expect("encode layer")
        })
        .collect()
}

fn encode_binary_document(
    target_format: &str,
    metadata: &FileMetadata,
    layers: &[Vec<u8>],
) -> Vec<u8> {
    let spec = slicer_converter_core::formats::get_format_spec_by_name(target_format)
        .expect("format spec");
    let encoded_layers = encode_layers(spec, metadata, layers);
    encode_document(target_format, metadata, &encoded_layers, "fixture")
}

fn main() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    fs::create_dir_all(&fixtures_dir).unwrap();

    let (w, h) = (64u32, 64u32);
    let layer_count = 10u32;

    let metadata = FileMetadataBuilder::new()
        .resolution(w, h)
        .display_size_mm(68.04, 120.96)
        .machine_z_mm(165.0)
        .machine_name("TestPrinter")
        .layer_height_mm(0.05)
        .layer_count(layer_count)
        .bottom_layer_count(3)
        .exposure(2.5, 30.0)
        .light_pwm(255, 255)
        .lift(5.0, 65.0)
        .retract(150.0)
        .build()
        .expect("valid metadata");

    // Generate layers: gradient patterns that vary per layer
    let layers: Vec<Vec<u8>> = (0..layer_count)
        .map(|l| {
            let mut pixels = vec![0u8; (w * h) as usize];
            for (i, p) in pixels.iter_mut().enumerate() {
                let x = (i % w as usize) as u8;
                let y = (i / w as usize) as u8;
                // Circle pattern that grows with layer
                let cx = (w / 2) as i32;
                let cy = (h / 2) as i32;
                let dx = x as i32 - cx;
                let dy = y as i32 - cy;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let radius = 10.0 + (l as f32) * 2.0;
                *p = if dist < radius { 255 } else { 0 };
            }
            pixels
        })
        .collect();

    // ── Generate .goo file ─────────────────────────────────────
    {
        let file = encode_binary_document("goo", &metadata, &layers);
        let output_path = fixtures_dir.join("sample.goo");
        fs::write(&output_path, &file).unwrap();
        println!(
            "Generated {} ({} bytes, {} layers)",
            output_path.display(),
            file.len(),
            layer_count
        );
    }

    // ── Generate .ctb file ─────────────────────────────────────
    {
        let file = encode_binary_document("ctb", &metadata, &layers);
        let output_path = fixtures_dir.join("sample.ctb");
        fs::write(&output_path, &file).unwrap();
        println!(
            "Generated {} ({} bytes, {} layers)",
            output_path.display(),
            file.len(),
            layer_count
        );
    }

    // ── Generate .sl1 file ─────────────────────────────────────
    {
        let mut png_layers = Vec::new();
        for pixels in &layers {
            let png = sl1::encode_layer_data(pixels, w, h).unwrap();
            png_layers.push(png);
        }

        let sl1_data = encode_document("sl1", &metadata, &png_layers, "fixture");
        let output_path = fixtures_dir.join("sample.sl1");
        fs::write(&output_path, &sl1_data).unwrap();
        println!(
            "Generated {} ({} bytes, {} layers)",
            output_path.display(),
            sl1_data.len(),
            layer_count
        );
    }

    // ── Generate .pw0 file ─────────────────────────────────────
    generate_pw0(&fixtures_dir, &metadata, &layers);

    // ── Generate .pws file ─────────────────────────────────────
    generate_pws(&fixtures_dir, &metadata, &layers);

    // ── Generate Anycubic V516/V517/V518 fixtures ──────────────
    // These exercise the modern AnycubicLayoutStrategy decode path
    // (parse_anycubic_v51{6,7,8}_header_data) end-to-end with realistic
    // multi-layer payloads, complementing the synthetic round-trip unit
    // tests in `anycubic_layouts.rs`.
    generate_anycubic_modern(&fixtures_dir, &metadata, &layers, "sample.pwma", "pwma");
    generate_anycubic_modern(&fixtures_dir, &metadata, &layers, "sample.pm5", "pm5");
    generate_anycubic_modern(&fixtures_dir, &metadata, &layers, "sample.pm5s", "pm5s");

    // ── Generate .vdt file ─────────────────────────────────────
    generate_vdt(&fixtures_dir, &metadata, &layers, w, h);

    println!("Done! Refreshed regression fixtures under rust/crates/core/tests/fixtures.");
}

fn generate_pw0(fixtures_dir: &Path, metadata: &FileMetadata, layers: &[Vec<u8>]) {
    let file = encode_binary_document("pw0", metadata, layers);
    let output_path = fixtures_dir.join("sample.pw0");
    fs::write(&output_path, &file).unwrap();
    println!(
        "Generated {} ({} bytes, {} layers)",
        output_path.display(),
        file.len(),
        layers.len()
    );
}

fn generate_pws(fixtures_dir: &Path, metadata: &FileMetadata, layers: &[Vec<u8>]) {
    let file = encode_binary_document("pws", metadata, layers);
    let output_path = fixtures_dir.join("sample.pws");
    fs::write(&output_path, &file).unwrap();
    println!(
        "Generated {} ({} bytes, {} layers)",
        output_path.display(),
        file.len(),
        layers.len()
    );
}

fn generate_vdt(fixtures_dir: &Path, metadata: &FileMetadata, layers: &[Vec<u8>], w: u32, h: u32) {
    let mut layer_pngs = Vec::new();
    for pixels in layers {
        let png = sl1::encode_layer_data(pixels, w, h).unwrap();
        layer_pngs.push(png);
    }

    let vdt_data = encode_document("vdt", metadata, &layer_pngs, "fixture");
    let output_path = fixtures_dir.join("sample.vdt");
    fs::write(&output_path, &vdt_data).unwrap();
    println!(
        "Generated {} ({} bytes, {} layers)",
        output_path.display(),
        vdt_data.len(),
        layers.len()
    );
}

/// Generate a fixture file using a modern Anycubic format (V516 / V517 / V518).
///
/// Uses the spec's own `FormatPipeline`; all V516+ formats use the spec's Pw0 codec slot.
fn generate_anycubic_modern(
    fixtures_dir: &Path,
    metadata: &FileMetadata,
    layers: &[Vec<u8>],
    filename: &str,
    format_name: &str,
) {
    let file = encode_binary_document(format_name, metadata, layers);
    let output_path = fixtures_dir.join(filename);
    fs::write(&output_path, &file).unwrap();
    println!(
        "Generated {} ({} bytes, {} layers)",
        output_path.display(),
        file.len(),
        layers.len()
    );
}
