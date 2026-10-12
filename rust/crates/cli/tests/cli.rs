use slicer_converter_core::formats::{self, anycubic::codecs::pw0_rle, ctb::CtbDecoder, sl1};
use std::{
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn fixture_path(name: &str) -> PathBuf {
    rust_workspace_root()
        .join("crates")
        .join("core")
        .join("tests")
        .join("fixtures")
        .join(name)
}

struct TempOutputs(Vec<PathBuf>);

impl Drop for TempOutputs {
    fn drop(&mut self) {
        for directory in &self.0 {
            let _ = std::fs::remove_dir_all(directory);
        }
    }
}

thread_local! {
    static TEMP_OUTPUTS: std::cell::RefCell<TempOutputs> = const { std::cell::RefCell::new(TempOutputs(Vec::new())) };
}

fn temp_output_path(name: &str) -> PathBuf {
    static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "slicer-converter-cli-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).expect("unique temp directory");
    TEMP_OUTPUTS.with(|outputs| outputs.borrow_mut().0.push(directory.clone()));
    directory.join(name)
}

#[test]
fn repeated_temp_basenames_are_isolated() {
    let first = temp_output_path("same.ctb");
    let second = temp_output_path("same.ctb");
    assert_ne!(first, second, "test outputs must not share paths");
    assert_eq!(
        first.file_name(),
        second.file_name(),
        "preserve profile suffix inference"
    );
}

#[test]
fn cli_rejects_a_truncated_ctb_without_creating_output() {
    let original = std::fs::read(fixture_path("sample.ctb")).unwrap();
    let table = u32::from_le_bytes(original[64..68].try_into().unwrap()) as usize;
    let input = temp_output_path("truncated.ctb");
    let output = temp_output_path("from-truncated.goo");
    std::fs::write(&input, &original[..table]).unwrap();
    let result = run_cli(&[
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(!output.exists());
    remove_if_exists(&input);
}

fn remove_if_exists(path: &Path) {
    if path.exists() {
        std::fs::remove_file(path).expect("existing temp output should be removable");
    }
}

fn repo_root() -> PathBuf {
    rust_workspace_root()
        .parent()
        .expect("rust workspace should live below repo root")
        .to_path_buf()
}

fn rust_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("CLI crate should live in rust/crates/cli")
        .to_path_buf()
}

fn cli_binary() -> String {
    std::env::var("CARGO_BIN_EXE_slicer-converter").expect("Cargo should build the CLI binary")
}

fn resolve_repo_relative_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() || path.exists() {
        path
    } else {
        repo_root().join(path)
    }
}

fn configured_fixture_path(keys: &[&str]) -> Option<PathBuf> {
    keys.iter()
        .find_map(std::env::var_os)
        .map(PathBuf::from)
        .map(resolve_repo_relative_path)
}

fn uvtools_cmd_path() -> Option<PathBuf> {
    configured_fixture_path(&["SC_UVTOOLS_CMD"]).or_else(|| {
        let path = repo_root()
            .join("dist")
            .join("tmp")
            .join("uvtools")
            .join("v6.0.3")
            .join("UVtoolsCmd.exe");
        path.exists().then_some(path)
    })
}

fn real_cxdlpv4_fixture_path() -> Option<PathBuf> {
    configured_fixture_path(&["SC_REAL_CXDLPV4_PATH"]).or_else(|| {
        let path = repo_root()
            .join("dist")
            .join("tmp")
            .join("public-fixtures")
            .join("plain-ctb-v5-candidates")
            .join("extracted")
            .join("857-cube")
            .join("cube.cxdlpv4");
        path.exists().then_some(path)
    })
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(cli_binary())
        .args(args)
        .output()
        .expect("CLI process should launch")
}

fn build_chitubox_zip_fixture() -> Vec<u8> {
    let pixels0 = vec![0, 32, 64, 96, 128, 160, 192, 255];
    let pixels1 = vec![255, 192, 160, 128, 96, 64, 32, 0];
    let png0 = sl1::encode_layer_data(&pixels0, 4, 2).expect("png0");
    let png1 = sl1::encode_layer_data(&pixels1, 4, 2).expect("png1");
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in [
        ("run.gcode", chitubox_zip_gcode().as_bytes().to_vec()),
        ("preview.png", png0.clone()),
        ("1.png", png0),
        ("2.png", png1),
    ] {
        zip.start_file(name, options).expect("start zip entry");
        zip.write_all(&bytes).expect("write zip entry");
    }

    zip.finish().expect("finish zip").into_inner()
}

fn chitubox_zip_gcode() -> &'static str {
    ";(****Build and Slicing Parameters****)\n\
     ;machineType: Mini 4K\n\
     ;estimatedPrintTime: 1234\n\
     ;volume: 3.5\n\
     ;layerHeight: 0.05\n\
     ;resolutionX: 4\n\
     ;resolutionY: 2\n\
     ;machineX: 80\n\
     ;machineY: 40\n\
     ;machineZ: 160\n\
     ;normalExposureTime: 2.5\n\
     ;bottomLayerExposureTime: 30\n\
     ;normalDropSpeed: 150\n\
     ;normalLayerLiftSpeed: 60\n\
     ;normalLayerLiftHeight: 6\n\
     ;bottomLayerCount: 1\n\
     ;totalLayer: 2\n\
     ;bottomLayerLiftHeight: 7\n\
     ;bottomLayerLiftSpeed: 45\n\
     ;bottomLightOffTime: 0.5\n\
     ;lightOffTime: 1.25\n\
     ;bottomPWMLight: 230\n\
     ;PWMLight: 180\n\
     ;antiAliasLevel: 4\n\
     ;mirror: 1\n"
}

fn build_anycubic_zip_fixture() -> Vec<u8> {
    let pixels0 = vec![0, 17, 34, 51, 68, 85, 102, 119];
    let pixels1 = vec![136, 153, 170, 187, 204, 221, 238, 255];
    let rle0 = pw0_rle::encode_pw0_rle4(&pixels0);
    let rle1 = pw0_rle::encode_pw0_rle4(&pixels1);
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in [
        (
            "anycubic_photon_resins.pwsp",
            anycubic_zip_settings_json().as_bytes().to_vec(),
        ),
        (
            "layers_controller.conf",
            anycubic_zip_layers_json().as_bytes().to_vec(),
        ),
        (
            "print_info.json",
            anycubic_zip_print_info_json().as_bytes().to_vec(),
        ),
        ("layer_images/layer_0.pw0Img", rle0),
        ("layer_images/layer_1.pw0Img", rle1),
    ] {
        zip.start_file(name, options).expect("start zip entry");
        zip.write_all(&bytes).expect("write zip entry");
    }

    zip.finish().expect("finish zip").into_inner()
}

fn build_anycubic_zip_pwsz_fixture() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in [
        (
            "anycubic_photon_resins.pwsp",
            anycubic_zip_pwsz_settings_json().as_bytes().to_vec(),
        ),
        (
            "layers_controller.conf",
            anycubic_zip_one_layer_json().as_bytes().to_vec(),
        ),
        (
            "print_info.json",
            anycubic_zip_print_info_json().as_bytes().to_vec(),
        ),
        ("layer_images/layer_0.pwszImg", pwsz_rectangle_layer()),
    ] {
        zip.start_file(name, options).expect("start zip entry");
        zip.write_all(&bytes).expect("write zip entry");
    }

    zip.finish().expect("finish zip").into_inner()
}

fn anycubic_zip_settings_json() -> &'static str {
    r#"{
  "machine_type": {
    "name": "Photon Mono M7",
    "key_suffix": "pm7",
    "key_image_format": "pw0Img",
    "res_x": 4,
    "res_y": 2,
    "print_xsize": 80.0,
    "print_ysize": 40.0,
    "print_zsize": 160.0,
    "raster_antialiasing": 8
  }
}"#
}

fn anycubic_zip_pwsz_settings_json() -> &'static str {
    r#"{
  "machine_type": {
    "name": "Photon Mono M7 Pro",
    "key_suffix": "pwsz",
    "key_image_format": "pwszImg",
    "res_x": 4,
    "res_y": 2,
    "print_xsize": 80.0,
    "print_ysize": 40.0,
    "print_zsize": 160.0,
    "raster_antialiasing": 8
  }
}"#
}

fn anycubic_zip_layers_json() -> &'static str {
    r#"{
  "paras": [
    {
      "exposure_time": 2.2,
      "layer_index": 0,
      "layer_thickness": 0.05,
      "zup_height": 5.0,
      "zup_speed": 6.0
    },
    {
      "exposure_time": 2.4,
      "layer_index": 1,
      "layer_thickness": 0.05,
      "zup_height": 5.5,
      "zup_speed": 6.5
    }
  ]
}"#
}

fn anycubic_zip_one_layer_json() -> &'static str {
    r#"{
  "paras": [
    {
      "exposure_time": 2.2,
      "layer_index": 0,
      "layer_thickness": 0.05,
      "zup_height": 5.0,
      "zup_speed": 6.0
    }
  ]
}"#
}

fn anycubic_zip_print_info_json() -> &'static str {
    r#"{
  "model_layers_count": 2,
  "cost": 6.5,
  "print_time": 321,
  "volume": 4.5,
  "weight": 5.5
}"#
}

fn pwsz_rectangle_layer() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"{==\0");
    for value in [1600.0f32, -20.0, -20.0, 20.0, 20.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(b"[--\0");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    for (start_x, start_y, end_x, end_y) in [
        (-20.0f32, -20.0, 20.0, -20.0),
        (20.0, -20.0, 20.0, 20.0),
        (20.0, 20.0, -20.0, 20.0),
        (-20.0, 20.0, -20.0, -20.0),
    ] {
        for value in [start_x, start_y, end_x, end_y] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(1);
    }
    bytes.extend_from_slice(b"--]\0");
    bytes.extend_from_slice(b"==}\0");
    bytes
}

#[test]
fn cli_converts_goo_to_ctb_with_input_and_output_flags() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.ctb");
    remove_if_exists(&output);
    let status = Command::new(cli_binary())
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .status()
        .expect("CLI process should launch");

    assert!(status.success(), "CLI should exit successfully");
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("ctb")).expect("output should be CTB");
    assert_eq!(detected.identity.name, "ctb");

    let mut decoder = CtbDecoder::new();
    let (metadata, layers) = decoder
        .parse_header(&data)
        .expect("generated CTB should parse");
    assert_eq!(metadata.slicing.layer_count as usize, layers.len());
    assert!(!layers.is_empty(), "generated CTB should contain layers");

    remove_if_exists(&output);
}

#[test]
fn cli_converts_goo_to_sl1_with_zero_based_layer_entries() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.sl1");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "GOO -> SL1 CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("sl1")).expect("output should be SL1");
    assert_eq!(detected.identity.name, "sl1");

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    let job_name = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("temp output stem should be utf8");
    archive
        .by_name(&format!("{job_name}_00000.png"))
        .expect("SL1 layers should start at zero with the job name prefix");
    archive
        .by_name(&format!("{job_name}_00009.png"))
        .expect("sample GOO has ten zero-based SL1 layers");
    assert!(
        archive.by_name(&format!("{job_name}_00010.png")).is_err(),
        "sample GOO should not produce a one-based extra terminal SL1 layer"
    );

    remove_if_exists(&output);
}

#[test]
fn cli_converts_chitubox_zip_to_goo_with_generated_uvtools_fixture() {
    let input = temp_output_path("input-chitubox.zip");
    let output = temp_output_path("output-from-chitubox.goo");
    remove_if_exists(&input);
    remove_if_exists(&output);
    std::fs::write(&input, build_chitubox_zip_fixture())
        .expect("generated chitubox zip should be writable");

    let result = run_cli(&[
        "--input",
        input.to_str().expect("temp input path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&input);
    remove_if_exists(&output);
}

#[test]
fn cli_converts_anycubic_zip_to_goo_with_generated_uvtools_fixture() {
    let input = temp_output_path("input-anycubic.pm7");
    let output = temp_output_path("output-from-anycubic-zip.goo");
    remove_if_exists(&input);
    remove_if_exists(&output);
    std::fs::write(&input, build_anycubic_zip_fixture())
        .expect("generated anycubic zip should be writable");

    let result = run_cli(&[
        "--input",
        input.to_str().expect("temp input path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&input);
    remove_if_exists(&output);
}

#[test]
fn cli_converts_anycubic_zip_content_with_generic_zip_extension() {
    let input = temp_output_path("input-anycubic-generic.zip");
    let output = temp_output_path("output-from-anycubic-generic-zip.goo");
    remove_if_exists(&input);
    remove_if_exists(&output);
    std::fs::write(&input, build_anycubic_zip_fixture())
        .expect("generated anycubic zip should be writable");

    let result = run_cli(&[
        "--input",
        input.to_str().expect("temp input path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&input);
    remove_if_exists(&output);
}

#[test]
fn cli_converts_anycubic_zip_pwsz_to_goo_with_generated_uvtools_fixture() {
    let input = temp_output_path("input-anycubic-pwsz.pwsz");
    let output = temp_output_path("output-from-anycubic-pwsz.goo");
    remove_if_exists(&input);
    remove_if_exists(&output);
    std::fs::write(&input, build_anycubic_zip_pwsz_fixture())
        .expect("generated anycubic pwsz zip should be writable");

    let result = run_cli(&[
        "--input",
        input.to_str().expect("temp input path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&input);
    remove_if_exists(&output);
}

#[test]
fn cli_converts_real_anycubic_pwszimg_fixture_when_configured() {
    let Some(input) =
        configured_fixture_path(&["SC_REAL_ANYCUBIC_PWSZIMG_PATH", "SC_REAL_PWSZIMG_PATH"])
    else {
        eprintln!("skipping real Anycubic pwszImg CLI smoke: set SC_REAL_ANYCUBIC_PWSZIMG_PATH");
        return;
    };
    assert!(
        input.exists(),
        "configured real Anycubic pwszImg fixture should exist: {}",
        input.display()
    );

    let output = temp_output_path("output-from-real-anycubic-pwszimg.goo");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--input",
        input.to_str().expect("real fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("routes=[layer/anycubic-zip-rle->layer/goo-rle layers="),
        "timing output should record Anycubic Zip -> GOO route stats: {stderr}"
    );

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&output);
}

#[test]
fn cli_converts_real_creality_cxdlpv4_fixture_when_available() {
    let Some(input) = real_cxdlpv4_fixture_path() else {
        eprintln!("skipping real Creality CXDLPv4 CLI smoke: set SC_REAL_CXDLPV4_PATH");
        return;
    };
    assert!(
        input.exists(),
        "real Creality CXDLPv4 fixture should exist: {}",
        input.display()
    );

    let output = temp_output_path("output-from-real-creality-cxdlpv4.goo");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--threads",
        "1",
        "--input",
        input.to_str().expect("real fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp output path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("routes=[layer/creality-cxdlp-rle->layer/goo-rle layers="),
        "timing output should record Creality CXDLPv4 -> GOO route stats: {stderr}"
    );
    assert!(
        stderr.contains("fallback_hits=200"),
        "CXDLPv4 -> GOO should currently use fallback for all layers: {stderr}"
    );

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&output);
}

#[test]
fn cli_converts_special_chitubox_external_fixtures_when_configured() {
    struct Case {
        label: &'static str,
        env_keys: &'static [&'static str],
        route_stats: &'static str,
    }

    let cases = [
        Case {
            label: "encrypted-ctb",
            env_keys: &[
                "SC_UVTOOLS_ENCRYPTED_CTB_PATH",
                "SC_REAL_ENCRYPTED_CTB_PATH",
            ],
            route_stats: "routes=[layer/chitu-ctb-encrypted-rle->layer/goo-rle layers=",
        },
        Case {
            label: "photons",
            env_keys: &["SC_UVTOOLS_PHOTONS_PATH", "SC_REAL_PHOTONS_PATH"],
            route_stats: "routes=[layer/photons-rle->layer/goo-rle layers=",
        },
        Case {
            label: "phz",
            env_keys: &["SC_UVTOOLS_PHZ_PATH", "SC_REAL_PHZ_PATH"],
            route_stats: "routes=[layer/phz-rle->layer/goo-rle layers=",
        },
        Case {
            label: "fdg",
            env_keys: &["SC_UVTOOLS_FDG_PATH", "SC_REAL_FDG_PATH"],
            route_stats: "routes=[layer/fdg-rle->layer/goo-rle layers=",
        },
    ];

    let mut configured = 0usize;
    for case in cases {
        let Some(input) = configured_fixture_path(case.env_keys) else {
            eprintln!(
                "skipping {} special Chitubox CLI smoke: set one of {:?}",
                case.label, case.env_keys
            );
            continue;
        };
        configured += 1;
        assert!(
            input.exists(),
            "configured {} fixture should exist: {}",
            case.label,
            input.display()
        );

        let output = temp_output_path(&format!("output-from-special-{}.goo", case.label));
        remove_if_exists(&output);

        let result = run_cli(&[
            "--timing",
            "--input",
            input.to_str().expect("special fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp output path should be utf8"),
        ]);

        assert!(
            result.status.success(),
            "{} CLI smoke should exit successfully: {}",
            case.label,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists(), "CLI should write the target file");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(case.route_stats),
            "timing output should record {} route stats: {stderr}",
            case.label
        );

        let data = std::fs::read(&output).expect("CLI output should be readable");
        let detected =
            formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
        assert_eq!(detected.identity.name, "goo");

        remove_if_exists(&output);
    }

    if configured == 0 {
        eprintln!(
            "skipping special Chitubox CLI smoke: set SC_UVTOOLS_* or SC_REAL_* fixture paths"
        );
    }
}

#[test]
fn cli_converts_ctb_v4_v5_external_fixtures_when_configured() {
    struct Case {
        label: &'static str,
        env_keys: &'static [&'static str],
        route_stats: &'static str,
    }

    let cases = [
        Case {
            label: "ctb-v4",
            env_keys: &["SC_UVTOOLS_CTB_V4_PATH", "SC_REAL_CTB_V4_PATH"],
            route_stats: "routes=[layer/ctb-rle->layer/goo-rle layers=",
        },
        Case {
            label: "ctb-v5",
            env_keys: &["SC_UVTOOLS_CTB_V5_PATH", "SC_REAL_CTB_V5_PATH"],
            route_stats: "routes=[layer/ctb-rle->layer/goo-rle layers=",
        },
    ];

    let mut configured = 0usize;
    for case in cases {
        let Some(input) = configured_fixture_path(case.env_keys) else {
            eprintln!(
                "skipping {} CTB CLI smoke: set one of {:?}",
                case.label, case.env_keys
            );
            continue;
        };
        configured += 1;
        assert!(
            input.exists(),
            "configured {} fixture should exist: {}",
            case.label,
            input.display()
        );

        let output = temp_output_path(&format!("output-from-{}.goo", case.label));
        remove_if_exists(&output);

        let result = run_cli(&[
            "--timing",
            "--input",
            input.to_str().expect("ctb fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp output path should be utf8"),
        ]);

        assert!(
            result.status.success(),
            "{} CLI smoke should exit successfully: {}",
            case.label,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists(), "CLI should write the target file");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(case.route_stats),
            "timing output should record {} route stats: {stderr}",
            case.label
        );

        let data = std::fs::read(&output).expect("CLI output should be readable");
        let detected =
            formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
        assert_eq!(detected.identity.name, "goo");

        remove_if_exists(&output);
    }

    if configured == 0 {
        eprintln!(
            "skipping CTB v4/v5 CLI smoke: set SC_UVTOOLS_CTB_V*_PATH or SC_REAL_CTB_V*_PATH"
        );
    }
}

#[test]
fn cli_help_prints_usage_and_supported_formats() {
    let output = run_cli(&["--help"]);
    assert!(output.status.success(), "help should exit successfully");

    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf8");
    assert!(
        stdout.contains(
            "usage: slicer-converter [--timing] [--threads <n>] [--profile <id>] [--version <n>] --input <input> --output <output>"
        ),
        "help should print usage"
    );
    assert!(
        stdout.contains("supported formats"),
        "help should list supported formats"
    );
    for spec in formats::global_registry()
        .all_specs()
        .iter()
        .copied()
        .filter(|spec| spec.capabilities.can_import || spec.capabilities.can_export)
    {
        for extension in spec.identity.extensions {
            let dotted = format!(".{extension}");
            assert!(
                stdout.contains(&dotted),
                "help should list registered extension {dotted}"
            );
        }
    }
    assert!(
        stdout.contains("--timing"),
        "help should document the timing flag"
    );
    assert!(
        stdout.contains("--threads"),
        "help should document the threads flag"
    );
    assert!(
        stdout.contains("--profile"),
        "help should document the profile flag"
    );
    assert!(
        stdout.contains("--version"),
        "help should document the version flag"
    );
}

#[test]
fn cli_profile_overrides_plain_ctb_extension() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.ctb");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
        "--profile",
        "ctb-v4",
    ]);

    assert!(
        result.status.success(),
        "CLI should accept --profile: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let data = std::fs::read(&output).expect("CLI output should be readable");
    assert_eq!(
        u32::from_le_bytes(data[0..4].try_into().expect("magic")),
        slicer_converter_core::formats::ctb::MAGIC_CTB_V4
    );
    assert_eq!(
        u32::from_le_bytes(data[4..8].try_into().expect("version")),
        4
    );

    remove_if_exists(&output);
}

#[test]
fn cli_version_selects_anycubic_layout_and_rejects_other_families() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.pwmx");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
        "--version",
        "1",
    ]);

    assert!(
        result.status.success(),
        "CLI should accept Anycubic --version: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let data = std::fs::read(&output).expect("CLI output should be readable");
    assert_eq!(
        u32::from_le_bytes(data[12..16].try_into().expect("anycubic version")),
        1
    );
    remove_if_exists(&output);

    let ctb_output = temp_output_path("versioned.ctb");
    remove_if_exists(&ctb_output);
    let rejected = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        ctb_output.to_str().expect("temp path should be utf8"),
        "--version",
        "4",
    ]);
    assert!(
        !rejected.status.success(),
        "CTB should reject --version and ask for --profile"
    );
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        stderr.contains("--version applies only to Anycubic targets"),
        "rejection should explain the version flag: {stderr}"
    );
    remove_if_exists(&ctb_output);
}

#[test]
fn cli_warns_when_photons_export_resamples_layers() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("warning.photons");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
        "--profile",
        "photons",
    ]);

    assert!(
        result.status.success(),
        "PhotonS export should still succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("resamples layers from 64x64 to 1440x2560"),
        "CLI should warn before PhotonS resampling: {stderr}"
    );
    remove_if_exists(&output);
}

#[test]
fn cli_exports_special_profiles_and_reimports_layer_count() {
    struct Case {
        profile: &'static str,
        output_name: &'static str,
        detect_hint: &'static str,
        expected_name: &'static str,
    }

    let cases = [
        Case {
            profile: "phz",
            output_name: "special.phz",
            detect_hint: "phz",
            expected_name: "phz",
        },
        Case {
            profile: "fdg",
            output_name: "special.fdg",
            detect_hint: "fdg",
            expected_name: "fdg",
        },
        Case {
            profile: "photons",
            output_name: "special.photons",
            detect_hint: "photons",
            expected_name: "photons",
        },
        Case {
            profile: "encrypted-ctb",
            output_name: "special.encrypted.ctb",
            detect_hint: "encrypted.ctb",
            expected_name: "encrypted-ctb",
        },
        Case {
            profile: "cxdlpv4",
            output_name: "special.cxdlpv4",
            detect_hint: "cxdlpv4",
            expected_name: "cxdlpv4",
        },
    ];

    let input = fixture_path("sample.goo");
    let input_bytes = std::fs::read(&input).expect("sample.goo should be readable");
    let source = formats::detect_format_spec(&input_bytes, Some("goo")).expect("source GOO");
    let source_document = source
        .pipeline
        .parse_document(source, &input_bytes)
        .expect("sample.goo should parse");
    let expected_layers = source_document.layers.len();

    for case in cases {
        let output = temp_output_path(case.output_name);
        let roundtrip = temp_output_path(&format!("roundtrip-{}.goo", case.profile));
        remove_if_exists(&output);
        remove_if_exists(&roundtrip);

        let exported = run_cli(&[
            "--input",
            input.to_str().expect("fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp path should be utf8"),
            "--profile",
            case.profile,
        ]);
        assert!(
            exported.status.success(),
            "{} export should succeed: {}",
            case.profile,
            String::from_utf8_lossy(&exported.stderr)
        );

        let data = std::fs::read(&output).expect("exported file should be readable");
        let detected = formats::detect_format_spec(&data, Some(case.detect_hint))
            .unwrap_or_else(|_| panic!("{} export should be detectable", case.profile));
        assert_eq!(detected.identity.name, case.expected_name);
        let parsed = detected
            .pipeline
            .parse_document(detected, &data)
            .unwrap_or_else(|error| panic!("{} export should reparse: {error}", case.profile));
        assert_eq!(
            parsed.layers.len(),
            expected_layers,
            "{} export should keep the source layer count",
            case.profile
        );

        let imported = run_cli(&[
            "--input",
            output.to_str().expect("temp path should be utf8"),
            "--output",
            roundtrip.to_str().expect("temp path should be utf8"),
        ]);
        assert!(
            imported.status.success(),
            "{} reimport should succeed: {}",
            case.profile,
            String::from_utf8_lossy(&imported.stderr)
        );
        let roundtrip_bytes = std::fs::read(&roundtrip).expect("roundtrip GOO should be readable");
        let roundtrip_spec =
            formats::detect_format_spec(&roundtrip_bytes, Some("goo")).expect("roundtrip GOO");
        let roundtrip_document = roundtrip_spec
            .pipeline
            .parse_document(roundtrip_spec, &roundtrip_bytes)
            .expect("roundtrip GOO should parse");
        assert_eq!(
            roundtrip_document.layers.len(),
            expected_layers,
            "{} roundtrip should keep the source layer count",
            case.profile
        );

        remove_if_exists(&output);
        remove_if_exists(&roundtrip);
    }
}

#[test]
fn cli_special_profile_exports_open_in_uvtools_when_available() {
    let Some(uvtools) = uvtools_cmd_path() else {
        eprintln!("skipping UVTools special-profile acceptance: set SC_UVTOOLS_CMD");
        return;
    };

    let cases = [
        ("phz", "uvtools.phz"),
        ("fdg", "uvtools.fdg"),
        ("photons", "uvtools.photons"),
        ("encrypted-ctb", "uvtools.encrypted.ctb"),
        ("cxdlpv4", "uvtools.cxdlpv4"),
    ];
    let input = fixture_path("sample.goo");
    let input_bytes = std::fs::read(&input).expect("sample.goo should be readable");
    let source = formats::detect_format_spec(&input_bytes, Some("goo")).expect("source GOO");
    let source_document = source
        .pipeline
        .parse_document(source, &input_bytes)
        .expect("sample.goo should parse");
    let expected_layer_count = format!("LayerCount: {}", source_document.layers.len());

    for (profile, output_name) in cases {
        let output = temp_output_path(output_name);
        remove_if_exists(&output);
        let exported = run_cli(&[
            "--input",
            input.to_str().expect("fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp path should be utf8"),
            "--profile",
            profile,
        ]);
        assert!(
            exported.status.success(),
            "{profile} export should succeed before UVTools: {}",
            String::from_utf8_lossy(&exported.stderr)
        );

        let properties = Command::new(&uvtools)
            .args([
                "--no-progress",
                "print-properties",
                output.to_str().expect("temp path should be utf8"),
            ])
            .output()
            .unwrap_or_else(|error| panic!("failed to run UVTools for {profile}: {error}"));
        let stdout = String::from_utf8_lossy(&properties.stdout);
        let stderr = String::from_utf8_lossy(&properties.stderr);
        let report = format!("{stdout}{stderr}");
        assert!(
            report.contains("Opening file") && report.contains("CanDecode: True"),
            "UVTools should decode {profile}: {report}"
        );
        assert!(
            report.contains(&expected_layer_count),
            "UVTools should report {expected_layer_count} for {profile}: {report}"
        );
        assert!(
            !report.contains("Exception") && !report.contains("Unable to"),
            "UVTools should not reject {profile}: {report}"
        );
        remove_if_exists(&output);
    }
}

#[test]
fn cli_converts_ctb_to_goo_with_input_and_output_flags() {
    let input = fixture_path("sample.ctb");
    let output = temp_output_path("output.goo");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
    assert_eq!(detected.identity.name, "goo");

    remove_if_exists(&output);
}

#[test]
fn cli_converts_goo_to_versioned_ctb_with_uvtools_suffix() {
    struct Case {
        output_name: &'static str,
        expected_route: &'static str,
        expected_magic: u32,
        expected_version: u32,
    }

    let cases = [
        Case {
            output_name: "output.v2.ctb",
            expected_route: "routes=[layer/goo-rle->layer/ctb-rle layers=",
            expected_magic: slicer_converter_core::formats::ctb::MAGIC_CTB,
            expected_version: 2,
        },
        Case {
            output_name: "output.v3.ctb",
            expected_route: "routes=[layer/goo-rle->layer/ctb-rle layers=",
            expected_magic: slicer_converter_core::formats::ctb::MAGIC_CTB,
            expected_version: 3,
        },
        Case {
            output_name: "output.v4.ctb",
            expected_route: "routes=[layer/goo-rle->layer/ctb-rle layers=",
            expected_magic: slicer_converter_core::formats::ctb::MAGIC_CTB_V4,
            expected_version: 4,
        },
        Case {
            output_name: "output.v5.ctb",
            expected_route: "routes=[layer/goo-rle->layer/ctb-rle layers=",
            expected_magic: slicer_converter_core::formats::ctb::MAGIC_CTB_V4,
            expected_version: 5,
        },
        Case {
            output_name: "output.gktwo.ctb",
            expected_route: "routes=[layer/goo-rle->layer/ctb-rle layers=",
            expected_magic: slicer_converter_core::formats::ctb::MAGIC_CTB_V4_GKTWO,
            expected_version: 4,
        },
    ];

    let input = fixture_path("sample.goo");
    for case in cases {
        let output = temp_output_path(case.output_name);
        remove_if_exists(&output);

        let result = run_cli(&[
            "--timing",
            "--input",
            input.to_str().expect("fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp path should be utf8"),
        ]);

        assert!(
            result.status.success(),
            "{} CLI should exit successfully: {}",
            case.output_name,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists(), "CLI should write the target file");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(case.expected_route),
            "timing output should record {} route stats: {stderr}",
            case.output_name
        );

        let data = std::fs::read(&output).expect("CLI output should be readable");
        assert_eq!(
            u32::from_le_bytes(data[0..4].try_into().expect("magic")),
            case.expected_magic
        );
        assert_eq!(
            u32::from_le_bytes(data[4..8].try_into().expect("version")),
            case.expected_version
        );

        remove_if_exists(&output);
    }
}

#[test]
fn cli_converts_goo_to_cbddlp_with_input_and_output_flags() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.cbddlp");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected =
        formats::detect_format_spec(&data, Some("cbddlp")).expect("output should be CBDDLP");
    assert_eq!(detected.identity.name, "cbddlp");
    assert_eq!(
        u32::from_le_bytes(data[0..4].try_into().expect("magic")),
        slicer_converter_core::formats::ctb::MAGIC_CBDDLP
    );

    remove_if_exists(&output);
}

#[test]
fn cli_converts_goo_to_photon_with_input_and_output_flags() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.photon");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("routes=[layer/goo-rle->layer/cbddlp-bitplane layers="),
        "timing output should record GOO -> Photon bitplane route stats: {stderr}"
    );

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected =
        formats::detect_format_spec(&data, Some("photon")).expect("output should be Photon");
    assert_eq!(detected.identity.name, "photon");
    assert_eq!(
        u32::from_le_bytes(data[0..4].try_into().expect("magic")),
        slicer_converter_core::formats::ctb::MAGIC_CBDDLP
    );

    remove_if_exists(&output);
}

#[test]
fn cli_converts_goo_to_anycubic_binary_profiles_with_input_and_output_flags() {
    struct Case {
        output_name: &'static str,
        extension_hint: &'static str,
        expected_format: &'static str,
        expected_route: &'static str,
    }

    let cases = [
        Case {
            output_name: "output.pw0",
            extension_hint: "pw0",
            expected_format: "pw0",
            expected_route: "routes=[layer/goo-rle->layer/anycubic-pw0-rle layers=",
        },
        Case {
            output_name: "output.pws",
            extension_hint: "pws",
            expected_format: "pws",
            expected_route: "routes=[layer/goo-rle->layer/anycubic-pws-rle layers=",
        },
    ];

    let input = fixture_path("sample.goo");
    for case in cases {
        let output = temp_output_path(case.output_name);
        remove_if_exists(&output);

        let result = run_cli(&[
            "--timing",
            "--input",
            input.to_str().expect("fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp path should be utf8"),
        ]);

        assert!(
            result.status.success(),
            "{} CLI should exit successfully: {}",
            case.output_name,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists(), "CLI should write the target file");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(case.expected_route),
            "timing output should record {} route stats: {stderr}",
            case.output_name
        );

        let data = std::fs::read(&output).expect("CLI output should be readable");
        let detected = formats::detect_format_spec(&data, Some(case.extension_hint))
            .expect("output should be an Anycubic binary profile");
        assert_eq!(detected.identity.name, case.expected_format);

        remove_if_exists(&output);
    }
}

#[test]
fn cli_converts_goo_to_anycubic_zip_with_input_and_output_flags() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output.pm7");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "GOO -> Anycubic Zip CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("routes=[layer/goo-rle->layer/anycubic-zip-rle layers="),
        "timing output should record GOO -> Anycubic Zip route stats: {stderr}"
    );

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected =
        formats::detect_format_spec(&data, Some("pm7")).expect("output should be Anycubic Zip");
    assert_eq!(detected.identity.name, "anycubic-zip");

    let spec = formats::global_registry()
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("exported Anycubic Zip should parse");
    assert_eq!(
        parsed.layers.len() as u32,
        parsed.metadata.slicing.layer_count
    );
    assert!(!parsed.layers.is_empty());

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    for required in [
        "anycubic_photon_resins.pwsp",
        "layers_controller.conf",
        "lcd_function.json",
        "print_info.json",
        "software_info.conf",
        "calc_layer_volumes.data",
        "scene.slice",
        "layer_images/layer_0.pw0Img",
    ] {
        archive
            .by_name(required)
            .unwrap_or_else(|_| panic!("missing required entry {required}"));
    }

    remove_if_exists(&output);
}

#[test]
fn cli_converts_anycubic_binary_samples_to_goo_with_input_and_output_flags() {
    struct Case {
        input_name: &'static str,
        output_name: &'static str,
        expected_route: &'static str,
    }

    let cases = [
        Case {
            input_name: "sample.pw0",
            output_name: "output-from-pw0.goo",
            expected_route: "routes=[layer/anycubic-pw0-rle->layer/goo-rle layers=",
        },
        Case {
            input_name: "sample.pws",
            output_name: "output-from-pws.goo",
            expected_route: "routes=[layer/anycubic-pws-rle->layer/goo-rle layers=",
        },
    ];

    for case in cases {
        let input = fixture_path(case.input_name);
        let output = temp_output_path(case.output_name);
        remove_if_exists(&output);

        let result = run_cli(&[
            "--timing",
            "--input",
            input.to_str().expect("fixture path should be utf8"),
            "--output",
            output.to_str().expect("temp path should be utf8"),
        ]);

        assert!(
            result.status.success(),
            "{} CLI should exit successfully: {}",
            case.input_name,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists(), "CLI should write the target file");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(case.expected_route),
            "timing output should record {} route stats: {stderr}",
            case.input_name
        );

        let data = std::fs::read(&output).expect("CLI output should be readable");
        let detected =
            formats::detect_format_spec(&data, Some("goo")).expect("output should be GOO");
        assert_eq!(detected.identity.name, "goo");

        remove_if_exists(&output);
    }
}

#[test]
fn cli_converts_sl1_to_vdt_with_input_and_output_flags() {
    let input = fixture_path("sample.sl1");
    let output = temp_output_path("output.vdt");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("vdt")).expect("output should be VDT");
    assert_eq!(detected.identity.name, "vdt");

    remove_if_exists(&output);
}

#[test]
fn cli_converts_goo_to_vdt_with_uvtools_layer_manifest() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("goo-output.vdt");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected = formats::detect_format_spec(&data, Some("vdt")).expect("output should be VDT");
    assert_eq!(detected.identity.name, "vdt");

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("VDT zip archive");
    let png_layer_count = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .filter(|name| {
            name.strip_suffix(".png")
                .is_some_and(|stem| stem.chars().all(|ch| ch.is_ascii_digit()))
        })
        .count();
    let manifest: serde_json::Value = serde_json::from_reader(
        archive
            .by_name("manifest.json")
            .expect("VDT manifest should exist"),
    )
    .expect("VDT manifest should parse");
    let manifest_layers = manifest["layers"].as_array().expect("manifest layers");

    assert_eq!(
        manifest_layers.len(),
        png_layer_count,
        "UVTools derives VDT layer count from manifest.layers"
    );
    assert_eq!(manifest_layers[0]["height"], 0.05);

    remove_if_exists(&output);
}

#[test]
fn cli_prints_timing_to_stderr_when_enabled() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("timed-output.ctb");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stderr = String::from_utf8(result.stderr).expect("timing stderr should be utf8");
    assert!(
        stderr.contains("timing:"),
        "timing output should be present"
    );
    assert!(
        stderr.contains("read="),
        "timing output should include read stage"
    );
    assert!(
        stderr.contains("transcode="),
        "timing output should include transcode stage"
    );
    assert!(
        stderr.contains("layer_cpu="),
        "timing output should include accumulated layer CPU time"
    );
    assert!(!stderr.contains(" decode="));
    assert!(!stderr.contains(" encode="));
    assert!(
        stderr.contains("assemble="),
        "timing output should include assemble stage"
    );
    assert!(
        stderr.contains("write="),
        "timing output should include write stage"
    );
    assert!(
        stderr.contains("total="),
        "timing output should include total stage"
    );
    assert!(
        stderr.contains("fast_path_hits="),
        "timing output should include fast path hit count"
    );
    assert!(
        stderr.contains("fallback_hits="),
        "timing output should include fallback hit count"
    );
    assert!(!stderr.contains("pairs=["));
    assert!(
        stderr.contains("routes=[layer/goo-rle->layer/ctb-rle"),
        "timing output should include the route-key breakdown"
    );

    remove_if_exists(&output);
}

#[test]
fn cli_accepts_threads_flag_for_conversion() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("threaded-output.ctb");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--threads",
        "1",
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "CLI should accept --threads: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    remove_if_exists(&output);
}

#[test]
fn cli_rejects_non_positive_threads() {
    let result = run_cli(&[
        "--threads",
        "0",
        "--input",
        fixture_path("sample.goo")
            .to_str()
            .expect("fixture path should be utf8"),
        "--output",
        temp_output_path("invalid-threads.ctb")
            .to_str()
            .expect("temp path should be utf8"),
    ]);

    assert!(
        !result.status.success(),
        "CLI should reject non-positive threads"
    );

    let stderr = String::from_utf8(result.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("threads"),
        "error should explain the threads constraint"
    );
}

#[test]
fn cli_converts_goo_to_chitubox_zip_with_input_and_output_flags() {
    let input = fixture_path("sample.goo");
    let output = temp_output_path("output-chitubox.zip");
    remove_if_exists(&output);

    let result = run_cli(&[
        "--timing",
        "--input",
        input.to_str().expect("fixture path should be utf8"),
        "--output",
        output.to_str().expect("temp path should be utf8"),
    ]);

    assert!(
        result.status.success(),
        "GOO -> Chitubox Zip CLI should exit successfully: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "CLI should write the target file");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("routes=[layer/goo-rle->layer/png-grayscale layers="),
        "timing output should record GOO -> Chitubox Zip route stats: {stderr}"
    );

    let data = std::fs::read(&output).expect("CLI output should be readable");
    let detected =
        formats::detect_format_spec(&data, Some("zip")).expect("output should be Chitubox Zip");
    assert_eq!(detected.identity.name, "chitubox-zip");

    let spec = formats::global_registry()
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("exported Chitubox Zip should parse");
    assert_eq!(
        parsed.layers.len() as u32,
        parsed.metadata.slicing.layer_count
    );
    assert!(!parsed.layers.is_empty());

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    archive.by_name("run.gcode").expect("gcode metadata");
    archive.by_name("1.png").expect("first layer png");

    remove_if_exists(&output);
}
