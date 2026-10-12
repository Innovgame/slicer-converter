use slicer_converter_core::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::{
        self,
        common::{
            ArchiveEncodeContext, ExportTargetOptions, FormatFamily, FormatSpec, LayerSource,
            VecOutputSink,
        },
        LayerEncodeContext,
    },
    transcode,
};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

struct CliArgs {
    input: PathBuf,
    output: PathBuf,
    timing: bool,
    threads: Option<usize>,
    profile: Option<String>,
    version: Option<u32>,
}

enum CliAction {
    Run(CliArgs),
    Help,
}

#[derive(Clone, Copy)]
struct DecodeConfig {
    width: u32,
    height: u32,
    display_width_mm: f32,
    display_height_mm: f32,
    anti_alias_level: u32,
    encryption_key: u32,
}

struct ThreadScratch {
    png_buf: Vec<u8>,
    pixel_buf: Vec<u8>,
    aux_buf: Vec<u8>,
    encode_buf: Vec<u8>,
}

#[derive(Default)]
struct TimingReport {
    read: Duration,
    layer_cpu: Duration,
    transcode: Duration,
    assemble: Duration,
    write: Duration,
    total: Duration,
}

#[derive(Clone, Copy, Default)]
struct TranscodeTiming {
    layer_cpu: Duration,
}

struct TranscodeReport {
    encoded_layers: Vec<Vec<u8>>,
    timing: TranscodeTiming,
    stats: transcode::TranscodeStats,
}

impl ThreadScratch {
    fn new(decode: DecodeConfig) -> Self {
        let pixel_capacity = (decode.width as usize).saturating_mul(decode.height as usize);
        Self {
            // ZIP layer payloads are often smaller than decoded pixels, but using the same
            // order of magnitude keeps the first few `read_to_end` growth steps cheap.
            png_buf: Vec::with_capacity(pixel_capacity),
            pixel_buf: Vec::with_capacity(pixel_capacity),
            aux_buf: Vec::new(),
            encode_buf: Vec::with_capacity(pixel_capacity),
        }
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    match parse_args(std::env::args().skip(1))? {
        CliAction::Help => {
            println!("{}", usage());
            Ok(())
        }
        CliAction::Run(args) => convert_file_with_options(&args.input, &args.output, &args)
            .map_err(|error| {
                format!(
                    "conversion failed for '{}' -> '{}': {error}",
                    args.input.display(),
                    args.output.display()
                )
            }),
    }
}

fn parse_args<I>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = String>,
{
    let mut input = None;
    let mut output = None;
    let mut timing = false;
    let mut threads = None;
    let mut profile = None;
    let mut version = None;
    let mut iter = args.into_iter().peekable();

    if iter.peek().is_none() {
        return Err(format!("missing required arguments\n{}", usage()));
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("missing value for --input\n{}", usage()))?;
                input = Some(PathBuf::from(value));
            }
            "--output" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("missing value for --output\n{}", usage()))?;
                output = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Ok(CliAction::Help);
            }
            "--timing" => {
                timing = true;
            }
            "--threads" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("missing value for --threads\n{}", usage()))?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid value for --threads: {value}\n{}", usage()))?;
                if parsed == 0 {
                    return Err(format!(
                        "threads must be a positive integer, got {value}\n{}",
                        usage()
                    ));
                }
                threads = Some(parsed);
            }
            "--profile" => {
                if profile.is_some() {
                    return Err(format!(
                        "--profile was provided more than once\n{}",
                        usage()
                    ));
                }
                let value = iter
                    .next()
                    .ok_or_else(|| format!("missing value for --profile\n{}", usage()))?;
                if value.trim().is_empty() || value.starts_with('-') {
                    return Err(format!("invalid value for --profile: {value}\n{}", usage()));
                }
                profile = Some(value);
            }
            "--version" => {
                if version.is_some() {
                    return Err(format!(
                        "--version was provided more than once\n{}",
                        usage()
                    ));
                }
                let value = iter
                    .next()
                    .ok_or_else(|| format!("missing value for --version\n{}", usage()))?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid value for --version: {value}\n{}", usage()))?;
                if parsed == 0 {
                    return Err(format!(
                        "version must be a positive integer, got {value}\n{}",
                        usage()
                    ));
                }
                version = Some(parsed);
            }
            _ => {
                return Err(format!("unknown argument: {arg}\n{}", usage()));
            }
        }
    }

    Ok(CliAction::Run(CliArgs {
        input: input.ok_or_else(|| format!("missing required --input\n{}", usage()))?,
        output: output.ok_or_else(|| format!("missing required --output\n{}", usage()))?,
        timing,
        threads,
        profile,
        version,
    }))
}

fn usage() -> String {
    [
        "usage: slicer-converter [--timing] [--threads <n>] [--profile <id>] [--version <n>] --input <input> --output <output>"
            .to_string(),
        String::new(),
        format!("supported formats: {}", supported_format_extensions()),
        "source format is auto-detected from bytes with extension as hint".to_string(),
        "target format is chosen from the output file name, unless --profile is set".to_string(),
        "--profile selects an explicit export profile id or alias, such as ctb-v4, pwmx, or cxdlpv4"
            .to_string(),
        "--version selects an Anycubic layout version, such as 1, 515, 516, 517, or 518".to_string(),
        "other families select versions through --profile or a versioned output name such as model.v4.ctb"
            .to_string(),
        "--timing prints stage timings to stderr after a successful conversion".to_string(),
        "--threads overrides the worker thread count used for layer transcoding".to_string(),
    ]
    .join("\n")
}

fn supported_format_extensions() -> String {
    let mut seen = BTreeSet::new();
    let mut extensions = Vec::new();

    for spec in formats::global_registry()
        .all_specs()
        .iter()
        .copied()
        .filter(|spec| spec.capabilities.can_import || spec.capabilities.can_export)
    {
        for extension in spec.identity.extensions {
            if seen.insert(*extension) {
                extensions.push(format!(".{extension}"));
            }
        }
    }

    extensions.join(", ")
}

fn convert_file_with_options(input: &Path, output: &Path, options: &CliArgs) -> Result<(), String> {
    let total_started_at = Instant::now();
    let read_started_at = Instant::now();
    let source_bytes = fs::read(input).map_err(|error| format!("failed to read input: {error}"))?;
    let read_elapsed = read_started_at.elapsed();
    let input_extension = extension_hint(input)
        .ok_or_else(|| format!("input path '{}' has no file extension", input.display()))?;
    let output_extension = extension_hint(output)
        .ok_or_else(|| format!("output path '{}' has no file extension", output.display()))?;

    let source_format = formats::detect_format_spec(&source_bytes, Some(input_extension))
        .map_err(|error| format!("failed to detect source format: {error}"))?;
    ensure_format_can_import(source_format)?;
    let parsed = source_format
        .pipeline
        .parse_document(source_format, &source_bytes)
        .map_err(|error| format!("failed to parse source header: {error}"))?;
    if !source_format.capabilities.is_archive {
        parsed
            .validate_complete(source_bytes.len())
            .map_err(|error| format!("invalid source document: {error}"))?;
    }
    let export_options = export_target_options(options);
    let target_format = target_format_for_output(
        output,
        output_extension,
        Some(&parsed.metadata),
        export_options,
    )?;
    ensure_format_can_export(target_format, output_extension)?;
    ensure_version_applies(target_format, options.version)?;
    warn_export_diagnostics(&parsed.metadata, target_format);
    let route_descriptor =
        transcode::TranscodeRouteDescriptor::from_specs(source_format, target_format);

    let metadata = parsed.metadata;
    let layer_offsets = parsed.layers;
    let decode = DecodeConfig {
        width: metadata.printer.resolution_x,
        height: metadata.printer.resolution_y,
        display_width_mm: metadata.printer.display_width_mm,
        display_height_mm: metadata.printer.display_height_mm,
        anti_alias_level: metadata.slicing.anti_alias_level,
        encryption_key: metadata.slicing.encryption_key,
    };

    let transcode_started_at = Instant::now();
    let transcode = transcode_layers(
        &source_bytes,
        source_format.identity.name,
        target_format.identity.name,
        &metadata,
        &layer_offsets,
        decode,
        options.threads,
    )
    .map_err(|error| format!("failed to transcode layers: {error}"))?;
    let transcode_elapsed = transcode_started_at.elapsed();

    let assemble_started_at = Instant::now();
    let job_name = output_job_name(output);
    let output_bytes = assemble_target_file(
        target_format.identity.name,
        &metadata,
        &transcode.encoded_layers,
        &job_name,
        export_options,
    )
    .map_err(|error| format!("failed to encode target file: {error}"))?;
    let assemble_elapsed = assemble_started_at.elapsed();

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create output directory: {error}"))?;
        }
    }

    let write_started_at = Instant::now();
    fs::write(output, output_bytes).map_err(|error| format!("failed to write output: {error}"))?;
    let write_elapsed = write_started_at.elapsed();

    if options.timing {
        let report = TimingReport {
            read: read_elapsed,
            layer_cpu: transcode.timing.layer_cpu,
            transcode: transcode_elapsed,
            assemble: assemble_elapsed,
            write: write_elapsed,
            total: total_started_at.elapsed(),
        };
        eprintln!(
            "{}",
            format_timing_line(&report, &transcode.stats, route_descriptor)
        );
    }

    Ok(())
}

fn format_duration(duration: Duration) -> String {
    format!("{:.2}ms", duration.as_secs_f64() * 1000.0)
}

fn format_route_breakdown(stats: &transcode::TranscodeStats) -> Option<String> {
    let rendered = stats
        .routes()
        .map(|(route, route_stats)| {
            format!(
                "{}->{} layers={} fast_path_hits={} fallback_hits={} strategy_attempts={} strategy_declines={}",
                route.source_layer_codec.0,
                route.target_layer_codec.0,
                route_stats.layers_total,
                route_stats.fast_path_hits,
                route_stats.fallback_hits,
                route_stats.strategy_attempts,
                route_stats.strategy_declines,
            )
        })
        .collect::<Vec<_>>();

    (!rendered.is_empty()).then(|| format!("routes=[{}]", rendered.join("; ")))
}

fn format_timing_line(
    report: &TimingReport,
    stats: &transcode::TranscodeStats,
    route: transcode::TranscodeRouteDescriptor,
) -> String {
    let mut line = format!(
        "timing: read={} layer_cpu={} transcode={} assemble={} write={} total={} fast_path_hits={} fallback_hits={} strategy_attempts={} strategy_declines={} conversion={}->{} families={}->{} headers={}->{} archives={}->{}",
        format_duration(report.read),
        format_duration(report.layer_cpu),
        format_duration(report.transcode),
        format_duration(report.assemble),
        format_duration(report.write),
        format_duration(report.total),
        stats.fast_path_hits,
        stats.fallback_hits,
        stats.strategy_attempts,
        stats.strategy_declines,
        route.source_profile_key.0,
        route.target_profile_key.0,
        route.source_family_key.0,
        route.target_family_key.0,
        route.source_header_key.0,
        route.target_header_key.0,
        route.source_archive_key.0,
        route.target_archive_key.0,
    );

    if let Some(routes) = format_route_breakdown(stats) {
        line.push(' ');
        line.push_str(&routes);
    }

    line
}

fn extension_hint(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

fn export_target_options(options: &CliArgs) -> ExportTargetOptions<'_> {
    let mut export_options = ExportTargetOptions::new();
    if let Some(profile) = options.profile.as_deref() {
        export_options = export_options.with_explicit_profile(profile);
    }
    if let Some(version) = options.version {
        export_options = export_options.with_format_version(version);
    }
    export_options
}

fn target_format_for_output(
    output: &Path,
    extension: &str,
    metadata: Option<&FileMetadata>,
    export_options: ExportTargetOptions<'_>,
) -> Result<&'static FormatSpec, String> {
    let target = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(extension);
    let registry = formats::global_registry();
    match registry.resolve_export_spec(target, metadata, export_options) {
        Ok(format) => return Ok(format),
        Err(error)
            if export_options.explicit_profile.is_some()
                || export_options.format_version.is_some() =>
        {
            return Err(format!("failed to resolve export target: {error}"));
        }
        Err(_) => {}
    }

    if let Some(format) = registry
        .spec_by_name(target)
        .or_else(|| registry.spec_by_extension(extension))
    {
        ensure_format_can_export(format, extension)?;
    }

    Err(format!("unsupported output format: .{extension}"))
}

fn warn_export_diagnostics(metadata: &FileMetadata, target: &'static FormatSpec) {
    for warning in formats::diagnose_export(metadata, target).warnings {
        eprintln!("{}", warning.message);
    }
}

fn ensure_version_applies(format: &FormatSpec, version: Option<u32>) -> Result<(), String> {
    if version.is_some() && format.family != FormatFamily::Anycubic {
        return Err(
            "--version applies only to Anycubic targets; use --profile or a versioned output name such as model.v4.ctb"
                .to_string(),
        );
    }
    Ok(())
}

fn ensure_format_can_import(format: &FormatSpec) -> Result<(), String> {
    if !format.capabilities.can_import {
        return Err(format!(
            "source format '{}' cannot import yet",
            format.identity.name
        ));
    }
    Ok(())
}

fn ensure_format_can_export(format: &FormatSpec, extension: &str) -> Result<(), String> {
    if !format.capabilities.can_export {
        return Err(format!(
            "unsupported output format: .{extension} ({} cannot export)",
            format.identity.name
        ));
    }
    Ok(())
}

fn transcode_layers(
    source_bytes: &[u8],
    source_format_name: &'static str,
    target_format_name: &'static str,
    metadata: &FileMetadata,
    layer_offsets: &[LayerOffset],
    decode: DecodeConfig,
    requested_worker_count: Option<usize>,
) -> Result<TranscodeReport, ConvertError> {
    if layer_offsets.is_empty() {
        return Ok(TranscodeReport {
            encoded_layers: Vec::new(),
            timing: TranscodeTiming::default(),
            stats: transcode::TranscodeStats::default(),
        });
    }

    let worker_count = suggested_worker_count(layer_offsets.len(), requested_worker_count);
    if worker_count <= 1 {
        return transcode_range(
            source_bytes,
            source_format_name,
            target_format_name,
            metadata,
            layer_offsets,
            decode,
        )
        .map(|report| TranscodeReport {
            encoded_layers: report.entries.into_iter().map(|(_, bytes)| bytes).collect(),
            timing: report.timing,
            stats: report.stats,
        });
    }

    let mut ordered = vec![None; layer_offsets.len()];
    let next_layer = AtomicUsize::new(0);

    let scoped_results = thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..worker_count {
            let next_layer = &next_layer;
            handles.push(scope.spawn(move || {
                transcode_dynamic(
                    source_bytes,
                    source_format_name,
                    target_format_name,
                    metadata,
                    layer_offsets,
                    next_layer,
                    decode,
                )
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| ConvertError::RleEncodeError("worker thread panicked".into()))??;
            results.push(chunk);
        }
        Ok::<_, ConvertError>(results)
    })?;

    let mut timing = TranscodeTiming::default();
    let mut stats = transcode::TranscodeStats::default();
    for chunk in scoped_results {
        timing.layer_cpu += chunk.timing.layer_cpu;
        stats.merge(chunk.stats);
        for (index, bytes) in chunk.entries {
            ordered[index] = Some(bytes);
        }
    }

    let encoded_layers = ordered
        .into_iter()
        .enumerate()
        .map(|(index, bytes)| {
            bytes.ok_or_else(|| {
                ConvertError::RleEncodeError(format!(
                    "missing transcoded layer result at index {index}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TranscodeReport {
        encoded_layers,
        timing,
        stats,
    })
}

fn suggested_worker_count(layer_count: usize, requested: Option<usize>) -> usize {
    let available = requested.unwrap_or_else(|| {
        thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1)
    });
    resolve_worker_count(layer_count, requested, available)
}

fn resolve_worker_count(layer_count: usize, requested: Option<usize>, available: usize) -> usize {
    let configured = match requested {
        Some(explicit) => explicit,
        None => available.min(16),
    };
    configured.min(layer_count).max(1)
}

fn next_layer_index(next: &AtomicUsize, total: usize) -> Option<usize> {
    let index = next.fetch_add(1, Ordering::Relaxed);
    (index < total).then_some(index)
}

#[allow(clippy::too_many_arguments)]
fn transcode_layer_bytes_with_engine(
    source_format: &FormatSpec,
    target_format: &FormatSpec,
    data: &[u8],
    layer_index: u32,
    decode: DecodeConfig,
    layer_metadata: Option<slicer_converter_core::common::LayerEncodeMetadata<'_>>,
    scratch: &mut ThreadScratch,
) -> Result<transcode::LayerTranscodeStats, ConvertError> {
    let decode_options = formats::LayerOptions::new()
        .with_anti_alias_level(decode.anti_alias_level)
        .with_encryption_key(decode.encryption_key);
    let decode_ctx = formats::LayerDecodeContext::new(layer_index, decode.width, decode.height)
        .with_display_size_mm(decode.display_width_mm, decode.display_height_mm)
        .with_options(decode_options);
    let encode_ctx = LayerEncodeContext {
        layer_index,
        width: decode.width,
        height: decode.height,
        layer_metadata,
    };
    transcode::execute_layer_into_with_stats(
        source_format,
        target_format,
        data,
        &decode_ctx,
        &encode_ctx,
        transcode::LayerExecutionBuffers {
            pixel_buf: &mut scratch.pixel_buf,
            aux_buf: &mut scratch.aux_buf,
            out: &mut scratch.encode_buf,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn transcode_source_layer_into(
    source_bytes: &[u8],
    source_format: &FormatSpec,
    target_format: &FormatSpec,
    archive: Option<&mut zip::ZipArchive<std::io::Cursor<&[u8]>>>,
    layer: &LayerOffset,
    layer_index: u32,
    decode: DecodeConfig,
    layer_metadata: slicer_converter_core::common::LayerEncodeMetadata<'_>,
    scratch: &mut ThreadScratch,
) -> Result<transcode::LayerTranscodeStats, ConvertError> {
    if source_format.capabilities.is_archive {
        let archive = archive.expect("zip source requires zip archive");
        read_zip_entry_into(archive, layer.offset as usize, &mut scratch.png_buf)?;
        let layer_bytes = std::mem::take(&mut scratch.png_buf);
        let result = transcode_layer_bytes_with_engine(
            source_format,
            target_format,
            &layer_bytes,
            layer_index,
            decode,
            Some(layer_metadata),
            scratch,
        );
        scratch.png_buf = layer_bytes;
        result
    } else {
        let start = layer.offset as usize;
        let end = start + layer.size as usize;
        transcode_layer_bytes_with_engine(
            source_format,
            target_format,
            &source_bytes[start..end],
            layer_index,
            decode,
            Some(layer_metadata),
            scratch,
        )
    }
}

fn transcode_range(
    source_bytes: &[u8],
    source_format_name: &'static str,
    target_format_name: &'static str,
    metadata: &FileMetadata,
    layer_offsets: &[LayerOffset],
    decode: DecodeConfig,
) -> Result<WorkerTranscodeReport, ConvertError> {
    let source_format = formats::get_format_spec_by_name(source_format_name)?;
    let target_format = formats::get_format_spec_by_name(target_format_name)?;

    let mut archive = if source_format.capabilities.is_archive {
        Some(
            zip::ZipArchive::new(std::io::Cursor::new(source_bytes))
                .map_err(|error| ConvertError::ZipError(error.to_string()))?,
        )
    } else {
        None
    };

    let layer_metadata = metadata.as_layer_encode_metadata();
    let route = transcode::TranscodeRouteDescriptor::from_specs(source_format, target_format);
    let mut scratch = ThreadScratch::new(decode);
    let mut out = Vec::with_capacity(layer_offsets.len());
    let mut timing = TranscodeTiming::default();
    let mut stats = transcode::TranscodeStats::default();

    for (offset_in_chunk, layer) in layer_offsets.iter().enumerate() {
        let index = offset_in_chunk;
        let transcode_started_at = Instant::now();
        let layer_stats = transcode_source_layer_into(
            source_bytes,
            source_format,
            target_format,
            archive.as_mut(),
            layer,
            index as u32,
            decode,
            layer_metadata,
            &mut scratch,
        )?;
        stats.record_layer_for_route(route.route_key, layer_stats);
        timing.layer_cpu += transcode_started_at.elapsed();
        out.push((index, take_encoded_layer(&mut scratch.encode_buf)));
    }

    Ok(WorkerTranscodeReport {
        entries: out,
        timing,
        stats,
    })
}

fn transcode_dynamic(
    source_bytes: &[u8],
    source_format_name: &'static str,
    target_format_name: &'static str,
    metadata: &FileMetadata,
    layer_offsets: &[LayerOffset],
    next_layer: &AtomicUsize,
    decode: DecodeConfig,
) -> Result<WorkerTranscodeReport, ConvertError> {
    let source_format = formats::get_format_spec_by_name(source_format_name)?;
    let target_format = formats::get_format_spec_by_name(target_format_name)?;

    let mut archive = if source_format.capabilities.is_archive {
        Some(
            zip::ZipArchive::new(std::io::Cursor::new(source_bytes))
                .map_err(|error| ConvertError::ZipError(error.to_string()))?,
        )
    } else {
        None
    };

    let layer_metadata = metadata.as_layer_encode_metadata();
    let route = transcode::TranscodeRouteDescriptor::from_specs(source_format, target_format);
    let mut scratch = ThreadScratch::new(decode);
    let mut out = Vec::new();
    let mut timing = TranscodeTiming::default();
    let mut stats = transcode::TranscodeStats::default();

    while let Some(index) = next_layer_index(next_layer, layer_offsets.len()) {
        let layer = &layer_offsets[index];
        let transcode_started_at = Instant::now();
        let layer_stats = transcode_source_layer_into(
            source_bytes,
            source_format,
            target_format,
            archive.as_mut(),
            layer,
            index as u32,
            decode,
            layer_metadata,
            &mut scratch,
        )?;
        stats.record_layer_for_route(route.route_key, layer_stats);
        timing.layer_cpu += transcode_started_at.elapsed();
        out.push((index, take_encoded_layer(&mut scratch.encode_buf)));
    }

    Ok(WorkerTranscodeReport {
        entries: out,
        timing,
        stats,
    })
}

struct WorkerTranscodeReport {
    entries: Vec<(usize, Vec<u8>)>,
    timing: TranscodeTiming,
    stats: transcode::TranscodeStats,
}

fn take_encoded_layer(scratch: &mut Vec<u8>) -> Vec<u8> {
    let next_capacity = scratch.capacity().max(scratch.len());
    let encoded = std::mem::take(scratch);
    *scratch = Vec::with_capacity(next_capacity);
    encoded
}

fn read_zip_entry_into(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    entry_index: usize,
    output: &mut Vec<u8>,
) -> Result<(), ConvertError> {
    let mut file = archive
        .by_index(entry_index)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    output.clear();
    let expected_size = usize::try_from(file.size()).unwrap_or(usize::MAX);
    if output.capacity() < expected_size {
        output.reserve(expected_size.saturating_sub(output.capacity()));
    }
    file.read_to_end(output)
        .map_err(|error| ConvertError::ZipError(error.to_string()))?;
    Ok(())
}

fn assemble_target_file(
    target_format_name: &str,
    metadata: &FileMetadata,
    encoded_layers: &[Vec<u8>],
    job_name: &str,
    export_options: ExportTargetOptions<'_>,
) -> Result<Vec<u8>, ConvertError> {
    let target_format = formats::get_format_spec_by_name(target_format_name)?;
    let archive_ctx = ArchiveEncodeContext::new(job_name).with_export_options(export_options);
    let mut sink = VecOutputSink::new();
    target_format.pipeline.encode_document_into(
        target_format,
        metadata,
        LayerSource::Encoded(encoded_layers),
        &mut sink,
        &archive_ctx,
    )?;
    Ok(sink.into_inner())
}

fn output_job_name(output: &Path) -> String {
    output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("output")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{DecodeConfig, ThreadScratch, TimingReport};
    use slicer_converter_core::{
        common::{ConvertError, FileMetadata, LayerOffset},
        formats::{
            common::{
                keys, ContainerKind, DefaultBinaryPipeline, ExportTargetOptions,
                FormatCapabilities, FormatCatalog, FormatFamily, FormatKey, FormatKeys, FormatSpec,
                HeaderLayout, LayerCodec, NoArchiveLayout, NoCrypto, NoFooter,
            },
            FormatIdentity, LayerDecodeContext, LayerEncodeContext,
        },
        transcode::{self, LayerTranscodeStats},
    };
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
        time::Duration,
    };

    struct TestHeader;
    struct DecodeIntoOnlyCodec;
    struct EchoEncodeCodec;

    impl HeaderLayout for TestHeader {
        fn try_detect(&self, _data: &[u8], _hint: Option<&str>) -> bool {
            false
        }

        fn parse_header(
            &self,
            _data: &[u8],
            _spec: &FormatSpec,
        ) -> Result<(FileMetadata, Vec<LayerOffset>), ConvertError> {
            Err(ConvertError::UnknownFormat)
        }

        fn build_header(
            &self,
            _metadata: &FileMetadata,
            _spec: &FormatSpec,
            _options: ExportTargetOptions<'_>,
        ) -> Result<Vec<u8>, ConvertError> {
            Err(ConvertError::RleEncodeError(
                "test spec does not build headers".to_string(),
            ))
        }

        fn finalize_header(
            &self,
            header: &[u8],
            _layer_sizes: &[u32],
            _spec: &FormatSpec,
            _options: ExportTargetOptions<'_>,
        ) -> Result<Vec<u8>, ConvertError> {
            Ok(header.to_vec())
        }
    }

    impl LayerCodec for DecodeIntoOnlyCodec {
        fn decode_layer_into(
            &self,
            _data: &[u8],
            _ctx: &LayerDecodeContext,
            output: &mut Vec<u8>,
        ) -> Result<(), ConvertError> {
            output.extend_from_slice(&[0x11, 0x22, 0x33]);
            Ok(())
        }

        fn encode_layer_into(
            &self,
            _pixels: &[u8],
            _ctx: &LayerEncodeContext<'_>,
            _output: &mut Vec<u8>,
        ) -> Result<(), ConvertError> {
            Err(ConvertError::RleEncodeError(
                "decode-only test codec should not encode".to_string(),
            ))
        }
    }

    impl LayerCodec for EchoEncodeCodec {
        fn decode_layer_into(
            &self,
            _data: &[u8],
            _ctx: &LayerDecodeContext,
            _output: &mut Vec<u8>,
        ) -> Result<(), ConvertError> {
            Err(ConvertError::RleDecodeError {
                offset: 0,
                detail: "echo-only test codec should not decode".to_string(),
            })
        }

        fn encode_layer_into(
            &self,
            pixels: &[u8],
            _ctx: &LayerEncodeContext<'_>,
            output: &mut Vec<u8>,
        ) -> Result<(), ConvertError> {
            output.clear();
            output.extend_from_slice(pixels);
            Ok(())
        }
    }

    static TEST_HEADER: TestHeader = TestHeader;
    static DECODE_INTO_ONLY_CODEC: DecodeIntoOnlyCodec = DecodeIntoOnlyCodec;
    static ECHO_ENCODE_CODEC: EchoEncodeCodec = EchoEncodeCodec;

    static DECODE_INTO_ONLY_SPEC: FormatSpec = FormatSpec {
        identity: FormatIdentity {
            name: "decode-into-only",
            aliases: &["decode-into-only"],
            extensions: &["decode-into-only"],
        },
        family: FormatFamily::Other("test"),
        catalog: FormatCatalog {
            profile_id: "decode-into-only",
            display_name: "Decode Into Only",
            default_extension: "decode-into-only",
            target_names: &["decode-into-only"],
            default_export: false,
        },
        container: ContainerKind::Binary,
        capabilities: FormatCapabilities::binary_import_export(),
        keys: FormatKeys {
            family_key: keys::FAMILY_OTHER,
            profile_key: FormatKey("profile/cli-test-decode-into-only"),
            container_key: keys::CONTAINER_BINARY,
            header_key: keys::GENERIC_HEADER_FALLBACK,
            layer_codec_key: keys::GENERIC_LAYER_FALLBACK,
            pixel_model_key: keys::PIXEL_GRAYSCALE_8,
            crypto_key: keys::NO_CRYPTO,
            footer_key: keys::NO_FOOTER,
            archive_key: keys::NO_ARCHIVE,
        },
        header: &TEST_HEADER,
        layer_codec: &DECODE_INTO_ONLY_CODEC,
        crypto: &NoCrypto,
        footer: &NoFooter,
        archive: &NoArchiveLayout,
        pipeline: &DefaultBinaryPipeline,
    };

    static ECHO_ENCODE_SPEC: FormatSpec = FormatSpec {
        identity: FormatIdentity {
            name: "echo-encode",
            aliases: &["echo-encode"],
            extensions: &["echo-encode"],
        },
        family: FormatFamily::Other("test"),
        catalog: FormatCatalog {
            profile_id: "echo-encode",
            display_name: "Echo Encode",
            default_extension: "echo-encode",
            target_names: &["echo-encode"],
            default_export: false,
        },
        container: ContainerKind::Binary,
        capabilities: FormatCapabilities::binary_import_export(),
        keys: FormatKeys {
            family_key: keys::FAMILY_OTHER,
            profile_key: FormatKey("profile/cli-test-echo-encode"),
            container_key: keys::CONTAINER_BINARY,
            header_key: keys::GENERIC_HEADER_FALLBACK,
            layer_codec_key: keys::GENERIC_LAYER_FALLBACK,
            pixel_model_key: keys::PIXEL_GRAYSCALE_8,
            crypto_key: keys::NO_CRYPTO,
            footer_key: keys::NO_FOOTER,
            archive_key: keys::NO_ARCHIVE,
        },
        header: &TEST_HEADER,
        layer_codec: &ECHO_ENCODE_CODEC,
        crypto: &NoCrypto,
        footer: &NoFooter,
        archive: &NoArchiveLayout,
        pipeline: &DefaultBinaryPipeline,
    };

    #[test]
    fn resolve_worker_count_caps_default_parallelism_at_sixteen() {
        assert_eq!(super::resolve_worker_count(200, None, 32), 16);
        assert_eq!(super::resolve_worker_count(200, None, 16), 16);
        assert_eq!(super::resolve_worker_count(200, None, 12), 12);
    }

    #[test]
    fn resolve_worker_count_respects_explicit_threads_override() {
        assert_eq!(super::resolve_worker_count(200, Some(24), 32), 24);
        assert_eq!(super::resolve_worker_count(8, Some(24), 32), 8);
        assert_eq!(super::resolve_worker_count(8, Some(1), 32), 1);
    }

    #[test]
    fn next_layer_index_is_exhaustive_and_unique_in_parallel() {
        let next = AtomicUsize::new(0);
        let total = 97usize;
        let seen = Mutex::new(Vec::new());

        std::thread::scope(|scope| {
            for _ in 0..4 {
                let seen = &seen;
                let next = &next;
                scope.spawn(move || {
                    while let Some(index) = super::next_layer_index(next, total) {
                        seen.lock().expect("seen mutex poisoned").push(index);
                    }
                });
            }
        });

        let mut seen = seen.into_inner().expect("seen mutex poisoned");
        seen.sort_unstable();

        assert_eq!(
            seen.len(),
            total,
            "all indices should be scheduled exactly once"
        );
        assert_eq!(seen, (0..total).collect::<Vec<_>>());
        assert_eq!(super::next_layer_index(&next, total), None);
        assert!(
            next.load(Ordering::Relaxed) >= total,
            "counter may advance past total while probing for exhaustion"
        );
    }

    #[test]
    fn take_encoded_layer_moves_bytes_and_leaves_empty_scratch() {
        let mut scratch = vec![1u8, 2, 3, 4];
        let encoded = super::take_encoded_layer(&mut scratch);

        assert_eq!(encoded, vec![1u8, 2, 3, 4]);
        assert!(
            scratch.is_empty(),
            "scratch buffer should be empty after take"
        );

        scratch.extend_from_slice(&[9u8, 8, 7]);
        assert_eq!(
            scratch,
            vec![9u8, 8, 7],
            "scratch buffer should remain reusable"
        );
    }

    #[test]
    fn transcode_layer_bytes_with_engine_falls_back_to_decode_then_encode() {
        let decode = DecodeConfig {
            width: 3,
            height: 1,
            display_width_mm: 3.0,
            display_height_mm: 1.0,
            anti_alias_level: 1,
            encryption_key: 0,
        };
        let mut scratch = ThreadScratch::new(decode);

        super::transcode_layer_bytes_with_engine(
            &DECODE_INTO_ONLY_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0xFF],
            0,
            decode,
            None,
            &mut scratch,
        )
        .expect("CLI transcode helper should use engine fallback");

        assert_eq!(scratch.encode_buf, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn format_timing_line_includes_route_breakdown() {
        let mut stats = transcode::TranscodeStats::default();
        let descriptor = transcode::TranscodeRouteDescriptor::from_specs(
            &DECODE_INTO_ONLY_SPEC,
            &ECHO_ENCODE_SPEC,
        );
        stats.record_layer_for_route(
            transcode::TranscodeRouteKey::from_specs(&DECODE_INTO_ONLY_SPEC, &ECHO_ENCODE_SPEC),
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 0,
                fast_path_applied: true,
                fallback_used: false,
            },
        );
        stats.record_layer_for_route(
            transcode::TranscodeRouteKey::from_specs(&DECODE_INTO_ONLY_SPEC, &ECHO_ENCODE_SPEC),
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 1,
                fast_path_applied: false,
                fallback_used: true,
            },
        );
        stats.record_layer_for_route(
            transcode::TranscodeRouteKey::from_specs(&ECHO_ENCODE_SPEC, &DECODE_INTO_ONLY_SPEC),
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 0,
                fast_path_applied: true,
                fallback_used: false,
            },
        );

        let line = super::format_timing_line(
            &TimingReport {
                read: Duration::from_millis(1),
                layer_cpu: Duration::from_millis(2),
                transcode: Duration::from_millis(4),
                assemble: Duration::from_millis(5),
                write: Duration::from_millis(6),
                total: Duration::from_millis(7),
            },
            &stats,
            descriptor,
        );

        assert!(line.contains("timing:"));
        assert!(line.contains("layer_cpu=2.00ms"));
        assert!(!line.contains(" decode="));
        assert!(!line.contains(" encode="));
        assert!(line.contains("fast_path_hits=2"));
        assert!(line.contains("fallback_hits=1"));
        assert!(!line.contains("pairs=["));
        assert!(line.contains(
            "conversion=profile/cli-test-decode-into-only->profile/cli-test-echo-encode"
        ));
        assert!(
            line.contains(
                "routes=[layer/generic-fallback->layer/generic-fallback layers=3 fast_path_hits=2 fallback_hits=1 strategy_attempts=3 strategy_declines=1]"
            ),
            "route breakdown should be appended in stable order"
        );
    }
}
