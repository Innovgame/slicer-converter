use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use slicer_converter_core::{
    common::ConvertError,
    formats::{self, common::FormatSpec, LayerDecodeContext, LayerEncodeContext, LayerOptions},
    transcode::{
        default_engine, registry::EmptyTranscodeRegistry, LayerTranscodeRequest,
        LayerTranscodeScratch, TranscodeEngine, TranscodeStats,
    },
};

static FALLBACK_REGISTRY: EmptyTranscodeRegistry = EmptyTranscodeRegistry;
static FALLBACK_ENGINE: TranscodeEngine = TranscodeEngine::new(&FALLBACK_REGISTRY);

struct RealBenchSpec {
    source_extension: &'static str,
    target_extension: &'static str,
    path_env_keys: &'static [&'static str],
    layer_limit_env_keys: &'static [&'static str],
    default_repo_path: Option<&'static [&'static str]>,
}

const REAL_BENCH_SPECS: &[RealBenchSpec] = &[
    RealBenchSpec {
        source_extension: "goo",
        target_extension: "ctb",
        path_env_keys: &["SC_REAL_GOO_PATH", "WITCH2_GOO_PATH"],
        layer_limit_env_keys: &["SC_REAL_GOO_LAYER_LIMIT", "SC_REAL_LAYER_LIMIT"],
        default_repo_path: None,
    },
    RealBenchSpec {
        source_extension: "prz",
        target_extension: "ctb",
        path_env_keys: &["SC_REAL_PRZ_PATH"],
        layer_limit_env_keys: &["SC_REAL_PRZ_LAYER_LIMIT", "SC_REAL_LAYER_LIMIT"],
        default_repo_path: None,
    },
    RealBenchSpec {
        source_extension: "ctb",
        target_extension: "goo",
        path_env_keys: &["SC_REAL_CTB_PATH"],
        layer_limit_env_keys: &["SC_REAL_CTB_LAYER_LIMIT", "SC_REAL_LAYER_LIMIT"],
        default_repo_path: None,
    },
    RealBenchSpec {
        source_extension: "cxdlpv4",
        target_extension: "goo",
        path_env_keys: &["SC_REAL_CXDLPV4_PATH"],
        layer_limit_env_keys: &["SC_REAL_CXDLPV4_LAYER_LIMIT", "SC_REAL_LAYER_LIMIT"],
        default_repo_path: Some(&[
            "dist",
            "tmp",
            "public-fixtures",
            "plain-ctb-v5-candidates",
            "extracted",
            "857-cube",
            "cube.cxdlpv4",
        ]),
    },
];

struct RealSourceLayer {
    layer_index: u32,
    start: usize,
    len: usize,
}

struct RealTranscodeBenchCase {
    spec: &'static RealBenchSpec,
    path: PathBuf,
    source_format: &'static FormatSpec,
    target_format: &'static FormatSpec,
    width: u32,
    height: u32,
    display_width_mm: f32,
    display_height_mm: f32,
    request_decode_options: LayerOptions,
    target_decode_options: LayerOptions,
    bytes: Vec<u8>,
    layers: Vec<RealSourceLayer>,
}

impl RealTranscodeBenchCase {
    fn label(&self) -> String {
        format!(
            "{} {}->{} [{} layers, {}x{}]",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("real-source"),
            self.source_format.identity.name,
            self.target_format.identity.name,
            self.layers.len(),
            self.width,
            self.height
        )
    }

    fn total_input_bytes(&self) -> u64 {
        self.layers.iter().map(|layer| layer.len as u64).sum()
    }

    fn pixel_capacity(&self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TranscodeRunSummary {
    total_output_bytes: usize,
    raw_fingerprint: u32,
    semantic_fingerprint: u32,
    stats: TranscodeStats,
}

fn bench_real_fast_paths(criterion: &mut Criterion) {
    let cases = load_real_cases();
    if cases.is_empty() {
        eprintln!(
            "skipping transcode_fast_path_bench: set one or more of SC_REAL_GOO_PATH / SC_REAL_PRZ_PATH / SC_REAL_CTB_PATH / SC_REAL_CXDLPV4_PATH"
        );
        return;
    }

    let mut group = criterion.benchmark_group("transcode_fast_path_real_file");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(20));
    group.sampling_mode(SamplingMode::Flat);

    for case in cases {
        let baseline = match verify_fast_path_matches_fallback(&case) {
            Ok(run) => run,
            Err(error) => {
                panic!(
                    "real-file fast path verification failed for '{}': {error}",
                    case.path.display()
                );
            }
        };

        eprintln!(
            "benchmarking {} on '{}' using {} layers ({} input bytes) {}",
            case.spec.source_target_label(),
            case.path.display(),
            case.layers.len(),
            case.total_input_bytes(),
            format_stats_summary(&baseline.stats)
        );

        let label = case.label();
        group.throughput(Throughput::Bytes(case.total_input_bytes()));

        group.bench_function(
            BenchmarkId::new(
                format!("fallback {}", case.spec.source_target_label()),
                &label,
            ),
            |bencher| {
                bencher.iter(|| {
                    black_box(
                        transcode_all_layers(&FALLBACK_ENGINE, &case)
                            .expect("fallback-only engine should transcode the real source"),
                    )
                })
            },
        );

        group.bench_function(
            BenchmarkId::new(
                format!("fast_path {}", case.spec.source_target_label()),
                &label,
            ),
            |bencher| {
                bencher.iter(|| {
                    black_box(
                        transcode_all_layers(default_engine(), &case)
                            .expect("default engine should transcode the real source"),
                    )
                })
            },
        );
    }

    group.finish();
}

impl RealBenchSpec {
    fn source_target_label(&self) -> String {
        format!("{}->{}", self.source_extension, self.target_extension)
    }
}

fn load_real_cases() -> Vec<RealTranscodeBenchCase> {
    let mut cases = Vec::new();

    for spec in REAL_BENCH_SPECS {
        let Some(path) = path_for_spec(spec) else {
            continue;
        };

        match load_real_case(spec, &path) {
            Ok(case) => cases.push(case),
            Err(error) => eprintln!(
                "skipping transcode_fast_path_bench for {}: failed to load '{}': {error}",
                spec.source_target_label(),
                path.display()
            ),
        }
    }

    cases
}

fn path_from_env(keys: &[&str]) -> Option<PathBuf> {
    keys.iter()
        .find_map(|key| env::var_os(key).map(PathBuf::from))
}

fn path_for_spec(spec: &RealBenchSpec) -> Option<PathBuf> {
    path_from_env(spec.path_env_keys).or_else(|| path_from_default_repo_fixture(spec))
}

fn path_from_default_repo_fixture(spec: &RealBenchSpec) -> Option<PathBuf> {
    let segments = spec.default_repo_path?;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust crate should live below repo root")
        .to_path_buf();
    let path = segments
        .iter()
        .fold(repo_root, |path, segment| path.join(segment));
    path.exists().then_some(path)
}

fn load_real_case(
    spec: &'static RealBenchSpec,
    path: &Path,
) -> Result<RealTranscodeBenchCase, String> {
    let bytes = fs::read(path).map_err(|error| format!("read failed: {error}"))?;
    let detected = formats::detect_format_spec(&bytes, Some(spec.source_extension))
        .map_err(|error| format!("format detect failed: {error}"))?;
    if !detected
        .identity
        .extensions
        .contains(&spec.source_extension)
    {
        return Err(format!(
            "expected source extension '{}', detected '{}'",
            spec.source_extension, detected.identity.name
        ));
    }

    let source_format = formats::global_registry()
        .spec_by_extension(spec.source_extension)
        .ok_or_else(|| format!("missing source format for .{}", spec.source_extension))?;
    let target_format = formats::global_registry()
        .spec_by_extension(spec.target_extension)
        .ok_or_else(|| format!("missing target format for .{}", spec.target_extension))?;

    let parsed = source_format
        .pipeline
        .parse_document(source_format, &bytes)
        .map_err(|error| format!("header parse failed: {error}"))?;
    let metadata = parsed.metadata;
    let layer_offsets = parsed.layers;

    let width = metadata.printer.resolution_x;
    let height = metadata.printer.resolution_y;
    let display_width_mm = metadata.printer.display_width_mm;
    let display_height_mm = metadata.printer.display_height_mm;
    let selected_indices = select_layer_indices(
        layer_offsets.len(),
        layer_limit_from_env(spec.layer_limit_env_keys)?,
    );
    if selected_indices.is_empty() {
        return Err("no layers selected for benchmark".into());
    }

    let mut layers = Vec::with_capacity(selected_indices.len());
    for layer_index in selected_indices {
        let layer = &layer_offsets[layer_index];
        let start = usize::try_from(layer.offset)
            .map_err(|_| format!("layer {layer_index} offset does not fit into usize"))?;
        let len = usize::try_from(layer.size)
            .map_err(|_| format!("layer {layer_index} size does not fit into usize"))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| format!("layer {layer_index} offset overflow"))?;
        if end > bytes.len() {
            return Err(format!(
                "layer {layer_index} range [{}..{}) exceeds file size {}",
                start,
                end,
                bytes.len()
            ));
        }
        layers.push(RealSourceLayer {
            layer_index: layer_index as u32,
            start,
            len,
        });
    }

    Ok(RealTranscodeBenchCase {
        spec,
        path: path.to_path_buf(),
        source_format,
        target_format,
        width,
        height,
        display_width_mm,
        display_height_mm,
        request_decode_options: LayerOptions::new()
            .with_anti_alias_level(metadata.slicing.anti_alias_level)
            .with_encryption_key(metadata.slicing.encryption_key),
        target_decode_options: LayerOptions::default(),
        bytes,
        layers,
    })
}

fn layer_limit_from_env(keys: &[&str]) -> Result<Option<usize>, String> {
    let Some(raw) = keys.iter().find_map(env::var_os) else {
        return Ok(None);
    };

    let raw = raw
        .into_string()
        .map_err(|_| "layer limit env var must be valid UTF-8".to_string())?;
    let limit = raw
        .parse::<usize>()
        .map_err(|error| format!("invalid layer limit '{raw}': {error}"))?;

    Ok((limit > 0).then_some(limit))
}

fn select_layer_indices(total_layers: usize, limit: Option<usize>) -> Vec<usize> {
    if total_layers == 0 {
        return Vec::new();
    }

    match limit {
        None => (0..total_layers).collect(),
        Some(limit) if limit >= total_layers => (0..total_layers).collect(),
        Some(1) => vec![total_layers / 2],
        Some(limit) => (0..limit)
            .map(|i| i * (total_layers - 1) / (limit - 1))
            .collect(),
    }
}

fn verify_fast_path_matches_fallback(
    case: &RealTranscodeBenchCase,
) -> Result<TranscodeRunSummary, String> {
    let fallback = transcode_all_layers(&FALLBACK_ENGINE, case)
        .map_err(|error| format!("fallback engine failed: {error}"))?;
    let fast_path = transcode_all_layers(default_engine(), case)
        .map_err(|error| format!("default engine failed: {error}"))?;

    if fallback.semantic_fingerprint != fast_path.semantic_fingerprint {
        return Err(format!(
            "semantic fingerprint mismatch: fallback={} fast_path={}",
            fallback.semantic_fingerprint, fast_path.semantic_fingerprint
        ));
    }

    Ok(fast_path)
}

fn transcode_all_layers(
    engine: &TranscodeEngine,
    case: &RealTranscodeBenchCase,
) -> Result<TranscodeRunSummary, ConvertError> {
    let mut scratch = LayerTranscodeScratch::new(case.pixel_capacity());
    let mut out = Vec::new();
    let mut normalized = Vec::with_capacity(case.pixel_capacity());
    let mut total_output_bytes = 0usize;
    let mut raw_hasher = crc32fast::Hasher::new();
    let mut semantic_hasher = crc32fast::Hasher::new();
    let mut stats = TranscodeStats::default();
    let route = slicer_converter_core::transcode::TranscodeRouteDescriptor::from_specs(
        case.source_format,
        case.target_format,
    );

    for layer in &case.layers {
        let req = LayerTranscodeRequest {
            layer_index: layer.layer_index,
            width: case.width,
            height: case.height,
            display_width_mm: case.display_width_mm,
            display_height_mm: case.display_height_mm,
            decode_options: &case.request_decode_options,
            encode_ctx: LayerEncodeContext {
                layer_index: layer.layer_index,
                width: case.width,
                height: case.height,
                layer_metadata: None,
            },
        };
        let src = &case.bytes[layer.start..layer.start + layer.len];
        out.clear();
        let layer_stats = engine.transcode_layer_into_with_stats(
            case.source_format,
            case.target_format,
            src,
            &req,
            &mut scratch,
            &mut out,
        )?;
        stats.record_layer_for_route(route.route_key, layer_stats);
        total_output_bytes += out.len();
        raw_hasher.update(&(out.len() as u32).to_le_bytes());
        raw_hasher.update(&out);

        normalized.clear();
        let decode_ctx = LayerDecodeContext::new(layer.layer_index, case.width, case.height)
            .with_options(case.target_decode_options.clone());
        case.target_format
            .decode_layer_into(&out, &decode_ctx, &mut normalized)?;
        semantic_hasher.update(&(normalized.len() as u32).to_le_bytes());
        semantic_hasher.update(&normalized);
    }

    Ok(TranscodeRunSummary {
        total_output_bytes,
        raw_fingerprint: raw_hasher.finalize(),
        semantic_fingerprint: semantic_hasher.finalize(),
        stats,
    })
}

fn format_stats_summary(stats: &TranscodeStats) -> String {
    let routes = stats
        .routes()
        .map(|(route, route_stats)| {
            format!(
                "{}->{} fast_path_hits={}/{} fallback_hits={} strategy_attempts={} strategy_declines={}",
                route.source_layer_codec.0,
                route.target_layer_codec.0,
                route_stats.fast_path_hits,
                route_stats.layers_total,
                route_stats.fallback_hits,
                route_stats.strategy_attempts,
                route_stats.strategy_declines,
            )
        })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        "hits=unavailable".to_string()
    } else {
        format!("hits=[{}]", routes.join("; "))
    }
}

criterion_group!(transcode_fast_path_benches, bench_real_fast_paths);
criterion_main!(transcode_fast_path_benches);
