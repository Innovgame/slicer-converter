use slicer_converter_core::{
    common::{ConvertError, FileMetadata, LayerOffset},
    formats::{
        self,
        common::{
            keys, ContainerKind, DefaultBinaryPipeline, ExportTargetOptions, FormatCapabilities,
            FormatCatalog, FormatFamily, FormatKey, FormatKeys, FormatSpec, HeaderLayout,
            LayerCodec, NoArchiveLayout, NoCrypto, NoFooter,
        },
        ctb, goo, prz, FormatIdentity, LayerDecodeContext, LayerEncodeContext, LayerOptions,
    },
    transcode::{
        default_engine, default_registry, FastPathOutcome, LayerTranscodeRequest,
        LayerTranscodeScratch, LayerTranscoder, TranscodeEngine, TranscodeRegistry,
        TranscodeRouteKey, TranscodeStats,
    },
};
use std::sync::atomic::{AtomicUsize, Ordering};

const GOO_LAYER_DEF_SIZE: usize = 70;

struct TestHeader;
struct DecodeIntoOnlyCodec;
struct EchoEncodeCodec;
struct PanicOnFallbackCodec;

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

impl LayerCodec for PanicOnFallbackCodec {
    fn decode_layer_into(
        &self,
        _data: &[u8],
        _ctx: &LayerDecodeContext,
        _output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        panic!("fallback decode_layer_into should not be called");
    }

    fn encode_layer_into(
        &self,
        _pixels: &[u8],
        _ctx: &LayerEncodeContext<'_>,
        _output: &mut Vec<u8>,
    ) -> Result<(), ConvertError> {
        Err(ConvertError::RleEncodeError(
            "panic fallback test codec should not encode".to_string(),
        ))
    }
}

static TEST_HEADER: TestHeader = TestHeader;
static DECODE_INTO_ONLY_CODEC: DecodeIntoOnlyCodec = DecodeIntoOnlyCodec;
static ECHO_ENCODE_CODEC: EchoEncodeCodec = EchoEncodeCodec;
static PANIC_ON_FALLBACK_CODEC: PanicOnFallbackCodec = PanicOnFallbackCodec;

macro_rules! test_spec {
    ($static_name:ident, $name:literal, $display:literal, $codec:ident, $profile:literal) => {
        static $static_name: FormatSpec = FormatSpec {
            identity: FormatIdentity {
                name: $name,
                aliases: &[$name],
                extensions: &[$name],
            },
            family: FormatFamily::Other("test"),
            catalog: FormatCatalog {
                profile_id: $name,
                display_name: $display,
                default_extension: $name,
                target_names: &[$name],
                default_export: false,
            },
            container: ContainerKind::Binary,
            capabilities: FormatCapabilities::binary_import_export(),
            keys: FormatKeys {
                family_key: keys::FAMILY_OTHER,
                profile_key: FormatKey($profile),
                container_key: keys::CONTAINER_BINARY,
                header_key: keys::GENERIC_HEADER_FALLBACK,
                layer_codec_key: keys::GENERIC_LAYER_FALLBACK,
                pixel_model_key: keys::PIXEL_GRAYSCALE_8,
                crypto_key: keys::NO_CRYPTO,
                footer_key: keys::NO_FOOTER,
                archive_key: keys::NO_ARCHIVE,
            },
            header: &TEST_HEADER,
            layer_codec: &$codec,
            crypto: &NoCrypto,
            footer: &NoFooter,
            archive: &NoArchiveLayout,
            pipeline: &DefaultBinaryPipeline,
        };
    };
}

test_spec!(
    DECODE_INTO_ONLY_SPEC,
    "decode-into-only",
    "Decode Into Only",
    DECODE_INTO_ONLY_CODEC,
    "profile/test-decode-into-only"
);
test_spec!(
    ECHO_ENCODE_SPEC,
    "echo-encode",
    "Echo Encode",
    ECHO_ENCODE_CODEC,
    "profile/test-echo-encode"
);
test_spec!(
    PANIC_ON_FALLBACK_SOURCE_SPEC,
    "panic-on-fallback-source",
    "Panic On Fallback Source",
    PANIC_ON_FALLBACK_CODEC,
    "profile/test-panic-on-fallback-source"
);

struct TestRegistry {
    route: TranscodeRouteKey,
    transcoders: &'static [&'static dyn LayerTranscoder],
}

impl TranscodeRegistry for TestRegistry {
    fn lookup_route(&self, route: TranscodeRouteKey) -> &[&'static dyn LayerTranscoder] {
        if route == self.route {
            self.transcoders
        } else {
            &[]
        }
    }
}

struct TestTranscoder {
    outcome: FastPathOutcome,
    output: &'static [u8],
    calls: &'static AtomicUsize,
}

impl LayerTranscoder for TestTranscoder {
    fn try_transcode_layer_into(
        &self,
        _src: &[u8],
        _req: &LayerTranscodeRequest<'_>,
        _scratch: &mut LayerTranscodeScratch,
        out: &mut Vec<u8>,
    ) -> Result<FastPathOutcome, ConvertError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.outcome == FastPathOutcome::Applied {
            out.clear();
            out.extend_from_slice(self.output);
        }
        Ok(self.outcome)
    }
}

fn request<'a>(
    layer_index: u32,
    width: u32,
    height: u32,
    options: &'a LayerOptions,
) -> LayerTranscodeRequest<'a> {
    LayerTranscodeRequest {
        layer_index,
        width,
        height,
        display_width_mm: 0.0,
        display_height_mm: 0.0,
        decode_options: options,
        encode_ctx: LayerEncodeContext {
            layer_index,
            width,
            height,
            layer_metadata: None,
        },
    }
}

fn test_engine(
    route: TranscodeRouteKey,
    transcoders: Vec<&'static dyn LayerTranscoder>,
) -> &'static TranscodeEngine {
    let transcoder_slice: &'static [&'static dyn LayerTranscoder] =
        Box::leak(transcoders.into_boxed_slice());
    let registry: &'static dyn TranscodeRegistry = Box::leak(Box::new(TestRegistry {
        route,
        transcoders: transcoder_slice,
    }));
    Box::leak(Box::new(TranscodeEngine::new(registry)))
}

fn format_spec(name: &str) -> &'static FormatSpec {
    formats::global_registry()
        .spec_by_name(name)
        .unwrap_or_else(|| panic!("{name} spec should be registered"))
}

fn ctb_spec() -> &'static FormatSpec {
    format_spec("ctb")
}

fn route_key(source: &str, target: &str) -> TranscodeRouteKey {
    TranscodeRouteKey::from_specs(format_spec(source), format_spec(target))
}

fn assert_fast_path_route(stats: &TranscodeStats, source: &str, target: &str, layers_total: u64) {
    let route_stats = stats
        .route_stats(route_key(source, target))
        .unwrap_or_else(|| panic!("{source} -> {target} route should be recorded"));

    assert_eq!(
        route_stats.layers_total, layers_total,
        "{source} -> {target}"
    );
    assert_eq!(
        route_stats.fast_path_hits, layers_total,
        "{source} -> {target}"
    );
    assert_eq!(route_stats.fallback_hits, 0, "{source} -> {target}");
}

#[test]
fn default_engine_falls_back_when_no_registered_route_matches() {
    let options = LayerOptions::default();
    let req = request(0, 3, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    default_engine()
        .transcode_layer_into(
            &DECODE_INTO_ONLY_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0xFF],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("default engine should fall back to decode + encode");

    assert_eq!(out, vec![0x11, 0x22, 0x33]);
}

#[test]
fn default_registry_registers_goo_to_ctb_fast_path() {
    let transcoders = default_registry().lookup_route(route_key("goo", "ctb"));
    assert!(
        !transcoders.is_empty(),
        "default registry should expose goo -> ctb fast path"
    );
}

#[test]
fn default_registry_registers_prz_to_ctb_fast_path() {
    let transcoders = default_registry().lookup_route(route_key("prz", "ctb"));
    assert!(
        !transcoders.is_empty(),
        "default registry should expose prz -> ctb fast path"
    );
}

#[test]
fn default_registry_registers_ctb_to_goo_fast_path() {
    let transcoders = default_registry().lookup_route(route_key("ctb", "goo"));
    assert!(
        !transcoders.is_empty(),
        "default registry should expose ctb -> goo fast path"
    );
}

#[test]
fn engine_uses_applied_fast_path_without_invoking_fallback() {
    let route = TranscodeRouteKey::from_specs(&PANIC_ON_FALLBACK_SOURCE_SPEC, &ECHO_ENCODE_SPEC);
    let calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let transcoder: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0xAA, 0xBB],
        calls,
    }));
    let engine = test_engine(route, vec![transcoder]);
    let options = LayerOptions::default();
    let req = request(0, 2, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = vec![0xFF];

    engine
        .transcode_layer_into(
            &PANIC_ON_FALLBACK_SOURCE_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0x00],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("applied fast path should succeed");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(out, vec![0xAA, 0xBB]);
}

#[test]
fn engine_tries_later_strategy_after_fast_path_declines() {
    let route = TranscodeRouteKey::from_specs(&PANIC_ON_FALLBACK_SOURCE_SPEC, &ECHO_ENCODE_SPEC);
    let first_calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let second_calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let first: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Fallback,
        output: &[],
        calls: first_calls,
    }));
    let second: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0x10, 0x20, 0x30],
        calls: second_calls,
    }));
    let engine = test_engine(route, vec![first, second]);
    let options = LayerOptions::default();
    let req = request(0, 3, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    engine
        .transcode_layer_into(
            &PANIC_ON_FALLBACK_SOURCE_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0x00],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("later fast path should succeed");

    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
    assert_eq!(out, vec![0x10, 0x20, 0x30]);
}

#[test]
fn engine_stops_after_first_applied_strategy() {
    let route = TranscodeRouteKey::from_specs(&PANIC_ON_FALLBACK_SOURCE_SPEC, &ECHO_ENCODE_SPEC);
    let first_calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let second_calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let first: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0x77],
        calls: first_calls,
    }));
    let second: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0x99],
        calls: second_calls,
    }));
    let engine = test_engine(route, vec![first, second]);
    let options = LayerOptions::default();
    let req = request(0, 1, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    engine
        .transcode_layer_into(
            &PANIC_ON_FALLBACK_SOURCE_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0x00],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("first fast path should short-circuit");

    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    assert_eq!(out, vec![0x77]);
}

#[test]
fn engine_reports_fast_path_hit_stats_when_strategy_applies() {
    let route = TranscodeRouteKey::from_specs(&PANIC_ON_FALLBACK_SOURCE_SPEC, &ECHO_ENCODE_SPEC);
    let calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let transcoder: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0xAB],
        calls,
    }));
    let engine = test_engine(route, vec![transcoder]);
    let options = LayerOptions::default();
    let req = request(0, 1, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = engine
        .transcode_layer_into_with_stats(
            &PANIC_ON_FALLBACK_SOURCE_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0x00],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("applied fast path should produce stats");

    assert_eq!(stats.strategy_attempts, 1);
    assert_eq!(stats.strategy_declines, 0);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn engine_reports_fallback_stats_when_strategy_declines() {
    let route = TranscodeRouteKey::from_specs(&DECODE_INTO_ONLY_SPEC, &ECHO_ENCODE_SPEC);
    let calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let transcoder: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Fallback,
        output: &[],
        calls,
    }));
    let engine = test_engine(route, vec![transcoder]);
    let options = LayerOptions::default();
    let req = request(0, 3, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = engine
        .transcode_layer_into_with_stats(
            &DECODE_INTO_ONLY_SPEC,
            &ECHO_ENCODE_SPEC,
            &[0xFF],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("fallback path should still return stats");

    assert_eq!(stats.strategy_attempts, 1);
    assert_eq!(stats.strategy_declines, 1);
    assert!(!stats.fast_path_applied);
    assert!(stats.fallback_used);
    assert_eq!(out, vec![0x11, 0x22, 0x33]);
}

#[test]
fn engine_uses_format_specs_for_route_lookup() {
    let route = route_key("goo", "ctb");
    let calls = Box::leak(Box::new(AtomicUsize::new(0)));
    let transcoder: &'static dyn LayerTranscoder = Box::leak(Box::new(TestTranscoder {
        outcome: FastPathOutcome::Applied,
        output: &[0xCA, 0xFE],
        calls,
    }));
    let engine = test_engine(route, vec![transcoder]);
    let options = LayerOptions::default();
    let req = request(0, 2, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = engine
        .transcode_layer_into_with_stats(
            format_spec("goo"),
            ctb_spec(),
            &[0x00],
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("spec-derived route lookup should find the route-only transcoder");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(out, vec![0xCA, 0xFE]);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_uses_registry_route_for_goo_to_ctb_fast_path() {
    let pixels = vec![0, 0, 32, 32, 128, 128, 255, 255];
    let src = goo::encode_layer_data(&pixels, 8, 1).expect("encode GOO");
    let expected = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
    let options = LayerOptions::default();
    let req = request(0, 8, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            format_spec("goo"),
            ctb_spec(),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("registry-routed fast path should succeed");

    assert_eq!(out, expected);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_uses_registry_route_for_ctb_to_goo_fast_path() {
    let pixels = vec![0, 16, 32, 64, 128, 192, 224, 255];
    let src = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
    let baseline = ctb::decode_layer_data(&src, 0, 8, 1, 1, 0).expect("baseline CTB decode");
    let options = LayerOptions::default();
    let req = request(0, 8, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            ctb_spec(),
            format_spec("goo"),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("registry-routed fast path should succeed");

    let decoded = goo::decode_layer_data(&out, 8, 1).expect("decode GOO");
    assert_eq!(decoded, baseline);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_reports_fast_path_hit_for_prz_to_ctb() {
    let pixels = vec![
        0, 0, 32, 32, 32, 128, 128, 255, 255, 255, 64, 64, 64, 64, 16, 16,
    ];
    let src = prz::encode_layer_data(&pixels, 8, 2).expect("encode PRZ");
    let expected = ctb::encode_layer_data(&pixels, 0, 8, 2).expect("encode CTB");
    let options = LayerOptions::default();
    let req = request(0, 8, 2, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            format_spec("prz"),
            ctb_spec(),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("default engine should transcode PRZ to CTB");

    assert_eq!(out, expected);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_preserves_fast_path_hits_for_common_route_matrix() {
    let options = LayerOptions::default();
    let mut route_stats = TranscodeStats::default();

    {
        let pixels = vec![0, 0, 32, 32, 128, 128, 255, 255];
        let src = goo::encode_layer_data(&pixels, 8, 1).expect("encode GOO");
        let expected = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
        let req = request(0, 8, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let stats = default_engine()
            .transcode_layer_into_with_stats(
                format_spec("goo"),
                ctb_spec(),
                &src,
                &req,
                &mut scratch,
                &mut out,
            )
            .expect("GOO -> CTB should use the raw fast path");

        assert_eq!(out, expected);
        route_stats.record_layer_for_route(route_key("goo", "ctb"), stats);
    }

    {
        let pixels = vec![0, 16, 32, 64, 128, 192, 224, 255];
        let src = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
        let baseline = ctb::decode_layer_data(&src, 0, 8, 1, 1, 0).expect("baseline CTB decode");
        let req = request(0, 8, 1, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let stats = default_engine()
            .transcode_layer_into_with_stats(
                ctb_spec(),
                format_spec("goo"),
                &src,
                &req,
                &mut scratch,
                &mut out,
            )
            .expect("CTB -> GOO should use the raw fast path");

        let decoded = goo::decode_layer_data(&out, 8, 1).expect("decode raw GOO");
        assert_eq!(decoded, baseline);
        route_stats.record_layer_for_route(route_key("ctb", "goo"), stats);
    }

    {
        let pixels = vec![
            0, 0, 32, 32, 32, 128, 128, 255, 255, 255, 64, 64, 64, 64, 16, 16,
        ];
        let src = prz::encode_layer_data(&pixels, 8, 2).expect("encode PRZ");
        let expected = ctb::encode_layer_data(&pixels, 0, 8, 2).expect("encode CTB");
        let req = request(0, 8, 2, &options);
        let mut scratch = LayerTranscodeScratch::default();
        let mut out = Vec::new();

        let stats = default_engine()
            .transcode_layer_into_with_stats(
                format_spec("prz"),
                ctb_spec(),
                &src,
                &req,
                &mut scratch,
                &mut out,
            )
            .expect("PRZ -> CTB should use the GOO-family fast path");

        assert_eq!(out, expected);
        route_stats.record_layer_for_route(route_key("prz", "ctb"), stats);
    }

    assert_eq!(route_stats.layers_total, 3);
    assert_eq!(route_stats.fast_path_hits, 3);
    assert_eq!(route_stats.fallback_hits, 0);
    assert_eq!(route_stats.routes().count(), 2);
    assert_fast_path_route(&route_stats, "goo", "ctb", 2);
    assert_fast_path_route(&route_stats, "ctb", "goo", 1);
    assert_eq!(route_key("goo", "ctb"), route_key("prz", "ctb"));
}

#[test]
fn default_engine_keeps_goo_family_diff_compatibility_for_prz_to_ctb_fast_path() {
    let src = vec![
        slicer_converter_core::formats::goo::LAYER_MAGIC,
        0x41,
        10,
        0x92,
        4,
        0,
    ];
    let expected_pixels = vec![10, 12, 14, 16, 18];
    let expected = ctb::encode_layer_data(&expected_pixels, 0, 5, 1).expect("encode CTB");
    let options = LayerOptions::default();
    let req = request(0, 5, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            format_spec("prz"),
            ctb_spec(),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("default engine should accept compatible GOO-family diff bytes");

    assert_eq!(out, expected);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_reports_fast_path_hit_for_ctb_to_goo_raw_layer() {
    let pixels = vec![0, 0, 32, 32, 128, 128, 255, 255];
    let src = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
    let baseline = ctb::decode_layer_data(&src, 0, 8, 1, 1, 0).expect("baseline CTB decode");
    let options = LayerOptions::default();
    let req = request(0, 8, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            ctb_spec(),
            format_spec("goo"),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("default engine should transcode CTB to raw GOO");

    let decoded = goo::decode_layer_data(&out, 8, 1).expect("decode raw GOO");
    assert_eq!(decoded, baseline);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_reports_fast_path_hit_for_ctb_to_goo_wrapped_layer() {
    let pixels = vec![0, 16, 32, 64, 128, 192, 224, 255];
    let src = ctb::encode_layer_data(&pixels, 0, 8, 1).expect("encode CTB");
    let baseline = ctb::decode_layer_data(&src, 0, 8, 1, 1, 0).expect("baseline CTB decode");
    let metadata = FileMetadata::builder()
        .resolution(8, 1)
        .display_size_mm(1.0, 1.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .build()
        .expect("metadata");
    let layer_metadata = metadata.as_layer_encode_metadata();
    let options = LayerOptions::default();
    let req = LayerTranscodeRequest {
        layer_index: 0,
        width: 8,
        height: 1,
        display_width_mm: 1.0,
        display_height_mm: 1.0,
        decode_options: &options,
        encode_ctx: LayerEncodeContext {
            layer_index: 0,
            width: 8,
            height: 1,
            layer_metadata: Some(layer_metadata),
        },
    };
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            ctb_spec(),
            format_spec("goo"),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("default engine should transcode CTB to wrapped GOO");

    let wrapped_rle = &out[GOO_LAYER_DEF_SIZE..out.len() - 2];
    let decoded = goo::decode_layer_data(wrapped_rle, 8, 1).expect("decode wrapped GOO RLE");
    assert_eq!(decoded, baseline);
    assert!(stats.fast_path_applied);
    assert!(!stats.fallback_used);
}

#[test]
fn default_engine_falls_back_for_ctb_to_goo_when_anti_alias_level_is_multi_plane() {
    let pixels = vec![0, 255, 0, 255];
    let src = ctb::encode_layer_data(&pixels, 0, 4, 1).expect("encode CTB");
    let options = LayerOptions::default().with_anti_alias_level(2);
    let req = request(0, 4, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            ctb_spec(),
            format_spec("goo"),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("fallback should still succeed");

    assert!(!stats.fast_path_applied);
    assert!(stats.fallback_used);
}

#[test]
fn default_engine_falls_back_for_ctb_to_goo_when_encrypted_source_uses_nonzero_key() {
    let pixels = vec![0, 255, 0, 255];
    let mut src = ctb::encode_layer_data(&pixels, 0, 4, 1).expect("encode CTB");
    ctb::crypto::crypt_layer_data(&mut src, 123, 0);
    let baseline = ctb::decode_layer_data(&src, 0, 4, 1, 1, 123).expect("baseline CTB decode");
    let options = LayerOptions::default().with_encryption_key(123);
    let req = request(0, 4, 1, &options);
    let mut scratch = LayerTranscodeScratch::default();
    let mut out = Vec::new();

    let stats = default_engine()
        .transcode_layer_into_with_stats(
            ctb_spec(),
            format_spec("goo"),
            &src,
            &req,
            &mut scratch,
            &mut out,
        )
        .expect("fallback should still succeed");

    let decoded = goo::decode_layer_data(&out, 4, 1).expect("decode fallback GOO");
    assert_eq!(decoded, baseline);
    assert!(!stats.fast_path_applied);
    assert!(stats.fallback_used);
}

#[test]
fn default_registry_route_lookup_preserves_ctb_to_goo_fast_path() {
    let transcoders = default_registry().lookup_route(route_key("ctb", "goo"));

    assert!(
        !transcoders.is_empty(),
        "route lookup should preserve existing ctb -> goo fast path"
    );
}
