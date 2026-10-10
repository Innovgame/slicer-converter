#[test]
fn wasm_merge_bitplanes_borrows_plane_slices_without_per_plane_vec_copies() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let merge_body = wasm_api_source
        .split("pub(super) fn merge_bitplanes_bytes(")
        .nth(1)
        .and_then(|tail| {
            tail.split("\npub(super) fn merge_bitplanes_uint8array")
                .next()
        })
        .expect("merge_bitplanes_bytes body");

    assert!(
        merge_body.contains("Vec<&[u8]>"),
        "merge_bitplanes should borrow views into planes_flat instead of owning per-plane buffers"
    );
    assert!(
        !merge_body.contains(".to_vec()"),
        "merge_bitplanes should not clone each bitplane before merging"
    );
}

#[test]
fn wasm_decode_encode_export_stays_thin_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let decode_encode_export = export_body(&lib_source, "decode_encode_layer");

    assert!(
        decode_encode_export.contains("wasm_api::decode_encode_layer_uint8array("),
        "decode_encode_layer should delegate typed-array output construction to wasm_api"
    );
    assert!(
        !decode_encode_export.contains("thread_local!"),
        "decode_encode_layer should delegate buffer ownership to wasm_api"
    );
    assert!(
        !decode_encode_export.contains("transcode_layer_with_formats"),
        "decode_encode_layer should delegate transcode execution to wasm_api"
    );
    assert!(
        !decode_encode_export.contains("output.stats"),
        "decode_encode_layer should not touch internal transcode stats"
    );
}

#[test]
fn wasm_query_exports_stay_thin_facades() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    for (export_name, delegated_call) in [
        (
            "list_supported_formats",
            "wasm_api::supported_formats_json()",
        ),
        ("parse_header", "wasm_api::parse_header_json("),
        (
            "build_archive_metadata_entries",
            "wasm_api::build_archive_metadata_entries_json(",
        ),
        (
            "describe_archive_layer_entries",
            "wasm_api::describe_archive_layer_entries_json(",
        ),
    ] {
        let body = export_body(&lib_source, export_name);
        assert!(
            body.contains(delegated_call),
            "{export_name} should delegate query serialization to wasm_api"
        );
        for local_fragment in [
            "serde_json::to_string",
            "detect_format_spec(",
            "common::ParseResult",
            "formats::catalog::supported_format_infos(",
            "lookup_archive_format(",
            "ArchiveEncodeContext::new",
            "zip_compression_method(",
            "ArchiveMetadataEntryInfo",
            "ArchiveLayerEntryInfo",
        ] {
            assert!(
                !body.contains(local_fragment),
                "{export_name} should not keep query implementation fragment {local_fragment} in lib.rs"
            );
        }
    }
}

#[test]
fn wasm_parse_header_adapter_uses_document_pipeline_not_tuple_wrapper() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let production_source = wasm_api_source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before test module");
    let parse_header_body = production_source
        .split("pub(super) fn parse_header_json(")
        .nth(1)
        .and_then(|tail| {
            tail.split("\npub(super) fn build_archive_metadata_entries_json")
                .next()
        })
        .expect("parse_header_json body");
    let compact_parse_header_body = parse_header_body.split_whitespace().collect::<String>();

    assert!(
        compact_parse_header_body.contains("fmt.pipeline.parse_document(fmt,data)"),
        "WASM parse header adapter should read the document pipeline result directly"
    );
    assert!(
        !compact_parse_header_body.contains("fmt.parse_header(data)"),
        "WASM parse header adapter should not depend on the tuple-shaped compatibility wrapper"
    );
}

#[test]
fn wasm_api_does_not_export_parallel_anycubic_fact_apis() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs");
    let wasm_api_rs =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");

    for forbidden in [
        "list_anycubic_machine_profiles",
        "resolve_anycubic_export_plan",
        "anycubic_machine_profile_infos",
        "resolve_anycubic_export_plan_json",
        "ANYCUBIC_PROFILES",
    ] {
        assert!(
            !lib_rs.contains(forbidden),
            "lib.rs should not contain {forbidden}"
        );
        assert!(
            !wasm_api_rs.contains(forbidden),
            "wasm_api.rs should not contain {forbidden}"
        );
    }
}

#[test]
fn wasm_supported_formats_read_catalog_projection_not_runtime_or_facts_bus() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let catalog_source =
        std::fs::read_to_string(core_src_path("formats/catalog.rs")).expect("read catalog.rs");
    let production_source = wasm_api_source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before test module");
    let supported_formats_body = production_source
        .split("pub(super) fn supported_formats_json()")
        .nth(1)
        .and_then(|tail| tail.split("\npub(super) fn parse_header_json").next())
        .expect("supported_formats_json body");
    let catalog_infos_body = catalog_source
        .split("pub fn supported_format_infos(")
        .nth(1)
        .and_then(|tail| tail.split("\nfn format_family_name").next())
        .expect("catalog supported_format_infos body");
    let specifics_body = catalog_source
        .split("fn supported_format_specifics(")
        .nth(1)
        .and_then(|tail| tail.split("\nfn chitubox_kind_name").next())
        .expect("supported_format_specifics body");

    assert!(
        supported_formats_body.contains(".catalog_profiles()"),
        "WASM adapter should provide catalog profiles to the catalog serializer"
    );
    assert!(
        supported_formats_body.contains("formats::catalog::supported_format_infos("),
        "WASM adapter should delegate supported format description construction to catalog"
    );
    assert!(
        catalog_infos_body.contains("profile.spec()"),
        "catalog format descriptions should read specs through catalog projection"
    );
    assert!(
        catalog_infos_body.contains("supported_format_specifics(*profile)"),
        "catalog format descriptions should serialize specifics from catalog projection"
    );
    assert!(
        specifics_body.contains("profile: &CatalogProfile"),
        "catalog supported_format_specifics should receive catalog projection, not a runtime FormatSpec"
    );
    assert!(
        !specifics_body.contains(concat!("Format", "Facts")),
        "catalog supported_format_specifics should not use the old facts bus"
    );
    let old_facts_variant = format!("{}::", concat!("Format", "Facts"));
    for forbidden in [
        "struct SupportedFormatInfo",
        "enum SupportedFormatSpecifics",
        "fn supported_format_specifics(",
        "fn chitubox_kind_name(",
    ] {
        assert!(
            !production_source.contains(forbidden),
            "WASM adapter should not own catalog description fragment {forbidden}"
        );
    }
    assert!(
        !production_source.contains(&old_facts_variant),
        "WASM adapter should not own catalog description fragment {old_facts_variant}"
    );
    assert!(
        !specifics_body.contains(".specifics"),
        "catalog format descriptions should not read FormatSpec.specifics directly"
    );
}

#[test]
fn catalog_supported_format_descriptions_stay_js_adapter_free() {
    let catalog_source =
        std::fs::read_to_string(core_src_path("formats/catalog.rs")).expect("read catalog.rs");

    assert!(
        catalog_source.contains("use serde::Serialize;"),
        "catalog DTOs should remain serializable without owning JSON or JS adapter concerns"
    );
    assert!(
        catalog_source.contains("pub fn supported_format_infos("),
        "catalog should own supported format description construction"
    );
    for forbidden in [
        "wasm_bindgen",
        "JsValue",
        "js_sys",
        "js_error_from",
        "serde_json::",
        "wasm_api",
    ] {
        assert!(
            !catalog_source.contains(forbidden),
            "catalog should not depend on WASM adapter concern {forbidden}"
        );
    }
}

fn core_src_path(relative_path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("wasm crate should live below rust/crates")
        .join("core")
        .join("src")
        .join(relative_path)
}

#[test]
fn wasm_binary_header_exports_stay_thin_facades() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    for (export_name, delegated_call) in [
        ("build_header", "wasm_api::build_header_uint8array("),
        (
            "build_document_from_layers",
            "wasm_api::build_document_from_layers_uint8array(",
        ),
        ("build_footer", "wasm_api::build_footer_uint8array("),
        ("finalize_header", "wasm_api::finalize_header_uint8array("),
    ] {
        let body = export_body(&lib_source, export_name);
        assert!(
            body.contains(delegated_call),
            "{export_name} should delegate binary header/footer work to wasm_api"
        );
        for local_fragment in [
            "serde_json::from_str",
            "common::FileMetadata",
            "lookup_export_format(",
            "fmt.build_header(",
            "fmt.footer",
            "fmt.pipeline",
            "fmt.finalize_header(",
        ] {
            assert!(
                !body.contains(local_fragment),
                "{export_name} should not keep binary header/footer implementation fragment {local_fragment} in lib.rs"
            );
        }
    }
}

#[test]
fn wasm_merge_bitplanes_export_stays_thin_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let body = export_body(&lib_source, "merge_bitplanes");

    assert!(
        body.contains("wasm_api::merge_bitplanes_uint8array("),
        "merge_bitplanes should delegate typed-array output construction to wasm_api"
    );
    for local_fragment in [
        "plane_count == 0",
        "planes_flat.len()",
        "expected",
        "formats::ctb::merge_bitplanes(",
        "js_error_from_message(",
        "js_error_from_convert",
    ] {
        assert!(
            !body.contains(local_fragment),
            "merge_bitplanes should not keep bitplane implementation fragment {local_fragment} in lib.rs"
        );
    }
}

#[test]
fn wasm_decode_layer_export_stays_thin_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let body = export_body(&lib_source, "decode_layer");

    assert!(
        body.contains("wasm_api::decode_layer_uint8array("),
        "decode_layer should delegate typed-array output construction to wasm_api"
    );
    for local_fragment in [
        "lookup_format(",
        "decode_context_from(",
        "fmt.decode_layer(",
        "js_error_from_convert",
    ] {
        assert!(
            !body.contains(local_fragment),
            "decode_layer should not keep layer decode implementation fragment {local_fragment} in lib.rs"
        );
    }
}

#[test]
fn wasm_encode_layer_export_stays_thin_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let body = export_body(&lib_source, "encode_layer");

    assert!(
        body.contains("wasm_api::encode_layer_uint8array("),
        "encode_layer should delegate typed-array output construction to wasm_api"
    );
    for local_fragment in [
        "thread_local!",
        "lookup_export_format(",
        "parse_layer_metadata(",
        "encode_context_from(",
        "fmt.encode_layer_into(",
        "js_error_from_convert",
        "uint8array_from_slice",
    ] {
        assert!(
            !body.contains(local_fragment),
            "encode_layer should not keep layer encode implementation fragment {local_fragment} in lib.rs"
        );
    }
}

#[test]
fn wasm_decode_layer_preview_export_stays_thin_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let body = export_body(&lib_source, "decode_layer_preview");

    assert!(
        body.contains("wasm_api::decode_layer_preview_uint8array("),
        "decode_layer_preview should delegate typed-array output construction to wasm_api"
    );
    for local_fragment in [
        "thread_local!",
        "PREVIEW_BUF",
        "max_preview_size == 0",
        "lookup_format(",
        "decode_context_from(",
        "decode_layer_preview_rgba(",
        "decode_layer_into(",
        "downsample_to_rgba_nearest",
        "extend_from_slice(&preview.width",
        "extend_from_slice(&preview.height",
        "js_error_from_convert",
        "js_error_from_message",
    ] {
        assert!(
            !body.contains(local_fragment),
            "decode_layer_preview should not keep preview implementation fragment {local_fragment} in lib.rs"
        );
    }
}

#[test]
fn wasm_decode_layer_preview_reuses_packed_output_buffer() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source = std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs"))
        .expect("read wasm_api.rs source");
    let preview_uint8array_body = wasm_api_source
        .split("pub(super) fn decode_layer_preview_uint8array")
        .nth(1)
        .and_then(|tail| tail.split("fn encode_layer_with_output").next())
        .expect("decode_layer_preview_uint8array body");

    assert!(
        preview_uint8array_body.contains("static PREVIEW_PACKED_BUF: RefCell<Vec<u8>>"),
        "decode_layer_preview_uint8array should keep a reusable packed preview output buffer"
    );
    assert!(
        preview_uint8array_body.contains("uint8array_from_slice(&packed)"),
        "decode_layer_preview_uint8array should copy JS output directly from the reusable packed buffer"
    );
    assert!(
        !preview_uint8array_body.contains("decode_layer_preview_bytes("),
        "decode_layer_preview_uint8array should not allocate an intermediate owned Vec"
    );
    assert!(
        !preview_uint8array_body.contains(".map(|bytes| uint8array_from_slice(&bytes))"),
        "decode_layer_preview_uint8array should avoid the bytes-then-Uint8Array copy pattern"
    );
}

#[test]
fn wasm_export_facade_does_not_own_typed_array_output_copying() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    assert!(
        !lib_source.contains("fn uint8array_from_slice"),
        "lib.rs should leave byte-to-Uint8Array copying to wasm_api"
    );
    assert!(
        !lib_source.contains(".map(|bytes| uint8array_from_slice(&bytes))"),
        "lib.rs should not allocate JS typed-array outputs after receiving intermediate Vec bytes"
    );
}

#[test]
fn wasm_binary_exports_return_js_uint8array() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    for export_name in [
        "decode_layer",
        "decode_layer_preview",
        "encode_layer",
        "build_header",
        "build_footer",
        "merge_bitplanes",
        "finalize_header",
        "decode_encode_layer",
    ] {
        let export_source = lib_source
            .split(&format!("pub fn {export_name}"))
            .nth(1)
            .and_then(|tail| tail.split(" {\n").next())
            .unwrap_or_else(|| panic!("missing wasm export {export_name}"));
        assert!(
            !export_source.contains("Result<Vec<u8>, JsValue>"),
            "{export_name} should return js_sys::Uint8Array so JS output copying goes through TypedArray.set"
        );
    }
}

#[test]
fn wasm_decode_encode_stats_export_reuses_buffer_facade() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let stats_export = export_body(&lib_source, "decode_encode_layer_stats");
    let stats_helper = wasm_api_source
        .split("fn decode_encode_layer_stats_json(")
        .nth(1)
        .and_then(|tail| tail.split("\nfn js_error_from_payload").next())
        .expect("decode_encode_layer_stats_json body");

    assert!(
        stats_export.contains("wasm_api::decode_encode_layer_stats_json("),
        "stats export should delegate stats serialization to wasm_api"
    );
    assert!(
        !stats_export.contains("uint8array_from_slice"),
        "stats export should not copy encoded layer bytes back to JS"
    );
    assert!(
        !stats_export.contains("serde_json::to_string"),
        "stats export should not keep stats JSON serialization in lib.rs"
    );
    assert!(
        stats_helper.contains("with_layer_buffer_pool(|buffers|"),
        "stats helper should borrow the WASM layer buffer pool at the API boundary"
    );
    assert!(
        stats_helper.contains("decode_encode_layer_with_buffers(data, params, buffers)"),
        "stats helper should reuse the same WASM buffer-backed transcode facade"
    );
    assert!(
        stats_helper.contains("serde_json::to_string(&stats)"),
        "stats helper should serialize only layer transcode stats"
    );
    assert!(
        !stats_helper.contains("encode_buf.clone()"),
        "stats helper should not clone encoded layer bytes"
    );
    assert!(
        !stats_export.contains("execute_layer_into"),
        "stats export should not duplicate core transcode execution"
    );
}

#[test]
fn wasm_decode_encode_stats_path_does_not_clone_encoded_layer_bytes() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let stats_body = wasm_api_source
        .split("fn decode_encode_layer_stats_json(")
        .nth(1)
        .and_then(|tail| tail.split("\nfn js_error_from_payload").next())
        .expect("decode_encode_layer_stats_json body");

    assert!(
        stats_body.contains("decode_encode_layer_with_buffers(data, params, buffers)"),
        "stats-only WASM API should execute layer transcode using an already-borrowed buffer pool"
    );
    assert!(
        !stats_body.contains("encode_buf.clone()"),
        "stats-only WASM API should not clone encoded layer bytes"
    );
    assert!(
        !stats_body.contains(".bytes"),
        "stats-only WASM API should not materialize encoded bytes"
    );
}

#[test]
fn wasm_export_facade_keeps_behavior_tests_out_of_lib_rs() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    assert!(
        !lib_source.contains("#[cfg(test)]\nmod tests"),
        "WASM facade behavior tests should live in integration test files, not src/lib.rs"
    );

    for test_name in [
        "supported_format_infos_include_unified_capabilities_when_available",
        "decode_encode_layer_stats_serializes_fast_path_stats",
        "archive_entry_plan_api_uses_rust_archive_layout_rules",
    ] {
        assert!(
            !lib_source.contains(&format!("fn {test_name}")),
            "{test_name} should live outside src/lib.rs"
        );
    }
}

#[test]
fn wasm_export_facade_does_not_construct_wasm_api_request_objects() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_source =
        std::fs::read_to_string(manifest_dir.join("src/lib.rs")).expect("read lib.rs source");

    assert!(
        !lib_source.contains("use wasm_api::{"),
        "lib.rs should not import wasm_api adapter request objects"
    );

    for request_type in [
        "LayerApiParams",
        "EncodeLayerApiParams",
        "TranscodeApiParams",
    ] {
        assert!(
            !lib_source.contains(request_type),
            "lib.rs should forward public WASM arguments and let wasm_api own {request_type}"
        );
    }
}

#[test]
fn wasm_api_adapter_keeps_visibility_within_lib_module_boundary() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let production_source = wasm_api_source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before test module");

    assert!(
        !production_source.contains("pub(crate)"),
        "wasm_api.rs is a private child module of lib.rs; adapter entrypoints should be pub(super), and implementation details should be private"
    );
}

#[test]
fn wasm_architecture_source_guards_do_not_depend_on_lib_rs_test_module_boundaries() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let guard_source = std::fs::read_to_string(manifest_dir.join("tests/wasm_api_architecture.rs"))
        .expect("read wasm_api_architecture.rs source");
    let production_source = guard_source
        .split("fn export_body")
        .next()
        .expect("source guard helpers should follow source guard tests");

    assert!(
        !production_source.contains("tail.split(\"\\n}\\n\\n#[cfg(test)]\")"),
        "WASM architecture source guards should not parse lib.rs using removed inline-test module boundaries"
    );
}

fn export_body<'a>(source: &'a str, export_name: &str) -> &'a str {
    source
        .split(&format!("pub fn {export_name}"))
        .nth(1)
        .and_then(|tail| tail.split("\n}\n\n").next())
        .unwrap_or_else(|| panic!("missing wasm export {export_name} body"))
}

#[test]
fn wasm_target_format_lookups_use_export_resolver() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let transcode_body = wasm_api_source
        .split("fn decode_encode_layer_with_buffers(")
        .nth(1)
        .and_then(|tail| {
            tail.split("transcode::execute_layer_into_with_stats(")
                .next()
        })
        .expect("decode_encode_layer_with_buffers setup body");

    assert!(
        wasm_api_source.contains("fn lookup_export_format("),
        "WASM helpers should expose a target-format lookup through registry export resolution"
    );
    assert!(
        transcode_body.contains("lookup_export_format(params.target_format"),
        "WASM transcode target selection should use export resolution"
    );
    assert!(
        !transcode_body.contains("let tgt_fmt = lookup_format(params.target_format"),
        "WASM transcode target selection should not bypass export resolution"
    );
}

#[test]
fn wasm_layer_transcode_buffers_preserve_core_execution_stats() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let production_source = wasm_api_source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before test module");
    let transcode_body = production_source
        .split("fn decode_encode_layer_with_buffers(")
        .nth(1)
        .and_then(|tail| {
            tail.split("\npub(super) fn decode_encode_layer_uint8array")
                .next()
        })
        .expect("decode_encode_layer_with_buffers body");

    assert!(
        !production_source.contains("fn transcode_layer_with_formats("),
        "WASM adapter should not keep a local layer execution helper once core exposes the facade"
    );
    assert!(
        !production_source.contains("struct TranscodedLayerOutput"),
        "stats-only WASM API should not require a bytes-plus-stats output wrapper"
    );
    assert!(
        transcode_body.contains("Result<transcode::LayerTranscodeStats, JsValue>"),
        "WASM layer transcode helper should return stats directly"
    );
    assert!(
        transcode_body.contains("transcode::execute_layer_into_with_stats("),
        "WASM adapter should preserve core layer execution stats internally"
    );
    assert!(
        !transcode_body.contains("bytes:"),
        "WASM layer transcode helper should not package encoded bytes into a local output object"
    );
    assert!(
        !transcode_body.contains("stats:"),
        "WASM layer transcode helper should not package stats into a local output object"
    );
    assert!(
        !transcode_body.contains("transcode::execute_layer_into("),
        "WASM adapter should not use the no-stats facade for layer transcode execution"
    );
    assert!(
        transcode_body.contains("transcode::LayerExecutionBuffers"),
        "WASM adapter should pass reusable buffers through the core facade parameter object"
    );
    assert!(
        !transcode_body.contains("transcode_layer_with_formats("),
        "decode_encode_layer_with_buffers should not bounce through a WASM-local execution helper"
    );
}

#[test]
fn wasm_layer_buffers_are_grouped_behind_adapter_pool() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_api_source =
        std::fs::read_to_string(manifest_dir.join("src/wasm_api.rs")).expect("read wasm_api.rs");
    let production_source = wasm_api_source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before test module");
    let transcode_body = production_source
        .split("fn decode_encode_layer_with_buffers(")
        .nth(1)
        .and_then(|tail| {
            tail.split("\npub(super) fn decode_encode_layer_uint8array")
                .next()
        })
        .expect("decode_encode_layer_with_buffers body");

    assert!(
        production_source.contains("struct LayerBufferPool"),
        "WASM adapter should name reusable layer buffers as a single pool"
    );
    assert!(
        production_source.contains("fn with_layer_buffer_pool<"),
        "WASM adapter should centralize thread-local buffer borrowing behind one helper"
    );
    assert!(
        !transcode_body.contains("with_layer_buffer_pool(|buffers|"),
        "decode_encode_layer_with_buffers should receive an already-borrowed buffer pool"
    );
    assert!(
        production_source.contains("decode_encode_layer_uint8array")
            && production_source.contains("decode_encode_layer_stats_json")
            && production_source.contains("with_layer_buffer_pool(|buffers|"),
        "public WASM layer transcode adapters should borrow the buffer pool at their API boundary"
    );
    assert!(
        !transcode_body.contains("PIXEL_BUF.with"),
        "decode_encode_layer_with_buffers should not own nested thread-local borrowing"
    );
    assert!(
        !transcode_body.contains("AUX_BUF.with"),
        "decode_encode_layer_with_buffers should not own nested thread-local borrowing"
    );
    assert!(
        !transcode_body.contains("ENCODE_BUF.with"),
        "decode_encode_layer_with_buffers should not own nested thread-local borrowing"
    );
}
