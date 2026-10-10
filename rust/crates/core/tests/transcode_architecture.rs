#[test]
fn transcode_registry_lookup_is_route_first() {
    use slicer_converter_core::transcode::{
        default_registry, LayerTranscodeStats, TranscodeRouteDescriptor, TranscodeRouteKey,
        TranscodeStats,
    };

    let source = slicer_converter_core::formats::get_format_spec_by_name("goo").expect("goo spec");
    let target = slicer_converter_core::formats::get_format_spec_by_name("ctb").expect("ctb spec");
    let descriptor = TranscodeRouteDescriptor::from_specs(source, target);
    let route = TranscodeRouteKey::from_specs(source, target);
    let registry = default_registry();

    assert_eq!(descriptor.route_key, route);
    assert_eq!(descriptor.source_profile_key, source.keys.profile_key);
    assert_eq!(descriptor.target_profile_key, target.keys.profile_key);
    assert!(!registry.lookup_route(route).is_empty());

    let mut stats = TranscodeStats::default();
    stats.record_layer_for_route(
        route,
        LayerTranscodeStats {
            strategy_attempts: 1,
            strategy_declines: 0,
            fast_path_applied: true,
            fallback_used: false,
        },
    );
    assert_eq!(
        stats
            .route_stats(route)
            .expect("route stats")
            .fast_path_hits,
        1
    );
}

#[test]
fn layer_transcode_request_does_not_duplicate_route_key_source() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let request_path = manifest_dir.join("src/transcode/request.rs");
    let source = std::fs::read_to_string(&request_path).expect("read transcode request source");
    let request_struct = source
        .split("pub struct LayerTranscodeRequest")
        .nth(1)
        .expect("LayerTranscodeRequest struct should exist")
        .split("impl LayerTranscodeRequest")
        .next()
        .expect("request struct before impl block");

    assert!(
        !source.contains("TranscodeRouteContext"),
        "LayerTranscodeRequest should not carry diagnostic route context on the hot path"
    );
    assert!(
        !source.contains("TranscodeRouteDescriptor"),
        "LayerTranscodeRequest should not carry diagnostic route descriptors on the hot path"
    );
    assert!(
        !source.contains("TranscodeRouteKey"),
        "request.rs should not define route-keyed diagnostics or request fields"
    );
    assert!(
        !request_struct.contains("route_key"),
        "LayerTranscodeRequest should contain only layer execution facts"
    );
    assert!(
        !source.contains("pub fn route_key"),
        "route key accessors belong to route descriptors, not layer requests"
    );
}

#[test]
fn transcode_route_constructors_are_spec_only() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let route_path = manifest_dir.join("src/transcode/route.rs");
    let source = std::fs::read_to_string(&route_path).expect("read transcode route source");

    assert!(
        !source.contains("from_format_names"),
        "route construction should not perform name-based registry lookups"
    );
    assert!(
        !source.contains("goo_family_to_ctb"),
        "route construction should not keep family-specific shortcut constructors"
    );
    assert!(
        !source.contains("ctb_to_goo"),
        "route construction should not keep format-pair shortcut constructors"
    );
    assert!(
        source.contains("TranscodeRouteDescriptor"),
        "diagnostic route facts should be named TranscodeRouteDescriptor"
    );
    assert!(
        !source.contains("from_formats"),
        "from_specs should be the single FormatSpec-based route constructor"
    );
}

#[test]
fn transcode_registry_entries_declare_specs_not_precomputed_route_keys() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_path = manifest_dir.join("src/transcode/registry.rs");
    let paths_path = manifest_dir.join("src/transcode/paths/mod.rs");
    let registry_source =
        std::fs::read_to_string(&registry_path).expect("read transcode registry source");
    let paths_source = std::fs::read_to_string(&paths_path).expect("read transcode paths source");

    assert!(
        registry_source.contains("pub source: &'static FormatSpec"),
        "route registry entries should declare source specs"
    );
    assert!(
        registry_source.contains("pub target: &'static FormatSpec"),
        "route registry entries should declare target specs"
    );
    assert!(
        !registry_source.contains("pub route: TranscodeRouteKey"),
        "route registry entries should derive dispatch keys from specs"
    );
    assert!(
        !paths_source.contains("_ROUTE"),
        "route path registration should not precompute named route constants"
    );
}

#[test]
fn layer_transcoders_do_not_own_route_selection() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let request_path = manifest_dir.join("src/transcode/request.rs");
    let paths_dir = manifest_dir.join("src/transcode/paths");
    let request_source =
        std::fs::read_to_string(&request_path).expect("read transcode request source");
    let trait_body = request_source
        .split("pub trait LayerTranscoder")
        .nth(1)
        .expect("LayerTranscoder trait should exist")
        .split("#[cfg(test)]")
        .next()
        .expect("trait body before test module");

    assert!(
        !trait_body.contains("fn route_key"),
        "LayerTranscoder strategies should not expose route selection metadata"
    );

    for path_name in [
        "goo_family_to_ctb.rs",
        "goo_to_ctb.rs",
        "prz_to_ctb.rs",
        "ctb_to_goo.rs",
    ] {
        let source = std::fs::read_to_string(paths_dir.join(path_name))
            .unwrap_or_else(|err| panic!("read {path_name}: {err}"));
        assert!(
            !source.contains("self.route_key()"),
            "{path_name} should trust registry route selection"
        );
        assert!(
            !source.contains("req.route_key() !="),
            "{path_name} should not duplicate route-selection checks"
        );
    }
}

#[test]
fn transcode_engine_derives_route_descriptor_from_specs() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine_path = manifest_dir.join("src/transcode/engine.rs");
    let source = std::fs::read_to_string(&engine_path).expect("read transcode engine source");
    let public_api_source = source
        .split("impl TranscodeEngine")
        .nth(1)
        .expect("TranscodeEngine impl exists")
        .split("static DEFAULT_ENGINE")
        .next()
        .expect("TranscodeEngine impl is bounded");

    assert!(
        public_api_source.contains("TranscodeRouteDescriptor::from_specs(src_fmt, tgt_fmt)"),
        "TranscodeEngine should derive conversion route descriptors from source and target specs"
    );
    assert!(
        !public_api_source.contains("route: TranscodeRouteDescriptor"),
        "TranscodeEngine methods should not require callers to pass route descriptors"
    );
    assert!(
        source.contains("lookup_route(route.route_key)"),
        "registry lookup should use the descriptor-derived dispatch key"
    );
    assert!(
        !source.contains("lookup_route(req.route_key())"),
        "engine dispatch should not read route facts from LayerTranscodeRequest"
    );
}

#[test]
fn public_transcode_entrypoints_do_not_pass_route_descriptors_to_engine() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = rust_workspace_root();
    let lib_path = manifest_dir.join("src/lib.rs");
    let main_path = workspace_dir.join("crates/cli/src/main.rs");
    let wasm_api_path = workspace_dir.join("crates/wasm/src/wasm_api.rs");
    let lib_source = std::fs::read_to_string(&lib_path).expect("read lib.rs");
    let main_source = std::fs::read_to_string(&main_path).expect("read main.rs");
    let wasm_api_source = std::fs::read_to_string(&wasm_api_path).expect("read wasm_api.rs");

    for (label, source) in [
        ("main.rs", main_source.as_str()),
        ("wasm_api.rs", wasm_api_source.as_str()),
    ] {
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source before test module");
        let helper_source = if label == "main.rs" {
            production_source
                .split("fn transcode_layer_bytes_with_engine")
                .nth(1)
                .expect("main.rs layer byte helper exists")
        } else {
            production_source
                .split("fn decode_encode_layer_with_buffers")
                .nth(1)
                .expect("wasm layer buffer helper exists")
        };
        assert!(
            !helper_source.contains("route: transcode::TranscodeRouteDescriptor"),
            "{label} layer transcode helpers should not accept externally-built route descriptors"
        );
    }

    let lib_production_source = lib_source
        .split("#[cfg(test)]")
        .next()
        .expect("lib production source before test module");
    assert!(
        !lib_production_source.contains("TranscodeRouteKey::from_specs"),
        "lib.rs public transcode facade should not bypass route descriptors"
    );
    assert!(
        !lib_production_source.contains("TranscodeRouteDescriptor::from_specs"),
        "lib.rs public transcode facade should let TranscodeEngine derive route descriptors"
    );
}

#[test]
fn public_transcode_entrypoints_reuse_core_layer_execution_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = rust_workspace_root();
    let transcode_mod_path = manifest_dir.join("src/transcode/mod.rs");
    let execute_path = manifest_dir.join("src/transcode/execute.rs");
    let main_path = workspace_dir.join("crates/cli/src/main.rs");
    let wasm_api_path = workspace_dir.join("crates/wasm/src/wasm_api.rs");
    let transcode_mod_source =
        std::fs::read_to_string(&transcode_mod_path).expect("read transcode/mod.rs");
    let execute_source = std::fs::read_to_string(&execute_path).unwrap_or_default();
    let main_source = std::fs::read_to_string(&main_path).expect("read main.rs");
    let wasm_api_source = std::fs::read_to_string(&wasm_api_path).expect("read wasm_api.rs");

    assert!(
        transcode_mod_source.contains("pub mod execute"),
        "transcode module should expose a shared layer execution facade module"
    );
    assert!(
        transcode_mod_source.contains("execute_layer_into_with_stats"),
        "transcode module should re-export the layer execution facade"
    );
    assert!(
        execute_source.contains("pub struct LayerExecutionBuffers"),
        "core facade should own the stable buffer parameter object"
    );
    assert!(
        execute_source.contains("pub fn execute_layer_into_with_stats"),
        "core facade should expose stats-returning layer execution"
    );

    for (label, source) in [
        ("main.rs", main_source.as_str()),
        ("wasm_api.rs", wasm_api_source.as_str()),
    ] {
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source before test module");
        let helper_source = if label == "main.rs" {
            production_source
                .split("fn transcode_layer_bytes_with_engine")
                .nth(1)
                .expect("main.rs layer byte helper exists")
        } else {
            production_source
                .split("fn decode_encode_layer_with_buffers")
                .nth(1)
                .expect("wasm layer buffer helper exists")
        };
        assert!(
            helper_source.contains("execute_layer_into"),
            "{label} layer transcode helpers should route through the shared core facade"
        );
        assert!(
            !helper_source.contains("LayerTranscodeScratch {"),
            "{label} should not hand-roll core transcode scratch transfer"
        );
        assert!(
            !helper_source.contains("default_engine().transcode_layer_into"),
            "{label} should not bypass the shared core facade for public layer execution"
        );
    }
}

fn rust_workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("core crate should live in rust/crates/core")
        .to_path_buf()
}

#[test]
fn transcode_fast_path_bench_tracks_creality_cxdlpv4_real_source() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bench_path = manifest_dir.join("benches/transcode_fast_path_bench.rs");
    let bench_source = std::fs::read_to_string(&bench_path).expect("read transcode bench source");

    assert!(
        bench_source.contains("SC_REAL_CXDLPV4_PATH"),
        "transcode fast-path bench should accept the real Creality CXDLPv4 fixture path"
    );
    assert!(
        bench_source.contains("SC_REAL_CXDLPV4_LAYER_LIMIT"),
        "transcode fast-path bench should allow limiting CXDLPv4 layer count"
    );
    assert!(
        bench_source.contains("source_extension: \"cxdlpv4\""),
        "transcode fast-path bench should include the CXDLPv4 -> GOO route"
    );
}
