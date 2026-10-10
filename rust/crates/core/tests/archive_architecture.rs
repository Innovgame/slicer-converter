#[test]
fn hot_layer_codecs_write_into_caller_buffers_without_temporary_rle_vecs() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let checked_sources = [
        (
            "Anycubic codec trait",
            "src/formats/anycubic/codec.rs",
            "let bytes = self.encode_layer(pixels)?;",
        ),
        (
            "Anycubic ZIP layer codec",
            "src/formats/anycubic/archive/codec.rs",
            "extend_from_slice(&pw0_rle::encode_pw0_rle4(pixels))",
        ),
        (
            "PhotonS layer codec",
            "src/formats/chitubox/photons.rs",
            "let mut rle = Vec::new();",
        ),
        (
            "PhotonS layer codec",
            "src/formats/chitubox/photons.rs",
            "output.extend_from_slice(&rle);",
        ),
    ];

    for (label, relative_path, forbidden_fragment) in checked_sources {
        let source = std::fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("read {label} source: {error}"));
        assert!(
            !source.contains(forbidden_fragment),
            "{label} should encode directly into the caller-provided Vec on the layer hot path"
        );
    }

    for (label, relative_path, required_fragment) in [
        (
            "PW0 codec",
            "src/formats/anycubic/codecs/pw0.rs",
            "pw0_rle::encode_pw0_rle4_into(pixels, output)",
        ),
        (
            "PWS codec",
            "src/formats/anycubic/codecs/pws.rs",
            "pws_rle::encode_pws_rle1_into(pixels, output)",
        ),
        (
            "Anycubic ZIP layer codec",
            "src/formats/anycubic/archive/codec.rs",
            "pw0_rle::encode_pw0_rle4_into(pixels, output)",
        ),
        (
            "GOO-family wrapped layer encoder",
            "src/formats/goo/mod.rs",
            "encoder::begin_goo_layer_with_metadata_into(metadata, ctx.layer_index, output)",
        ),
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("read {label} source: {error}"));
        assert!(
            source.contains(required_fragment),
            "{label} should use an encode_into helper that preserves output buffer capacity"
        );
    }

    let goo_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/goo/mod.rs")).expect("read GOO");
    assert!(
        !goo_source.contains("RLE_SCRATCH"),
        "GOO-family wrapped encode should append RLE into the caller output instead of staging it in a thread-local scratch Vec"
    );
}

#[test]
fn anycubic_full_size_preview_decodes_rle_directly_to_rgba() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (label, relative_path, direct_rgba_fragment, full_decode_fragment) in [
        (
            "PW0 codec",
            "src/formats/anycubic/codecs/pw0.rs",
            "pw0_rle::decode_pw0_rle4_rgba(data)?",
            "let pixels = pw0_rle::decode_pw0_rle4(data)?;",
        ),
        (
            "PWS codec",
            "src/formats/anycubic/codecs/pws.rs",
            "pws_rle::decode_pws_rle1_rgba(data)?",
            "let pixels = pws_rle::decode_pws_rle1(data)?;",
        ),
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("read {label} source: {error}"));
        let preview_body = source
            .split("fn decode_layer_preview_rgba")
            .nth(1)
            .and_then(|tail| tail.split("fn encode_layer").next())
            .unwrap_or_else(|| panic!("{label} preview body"));

        assert!(
            preview_body.contains(direct_rgba_fragment),
            "{label} full-size preview should decode RLE directly to RGBA"
        );
        assert!(
            !preview_body.contains(full_decode_fragment),
            "{label} full-size preview should not allocate a full grayscale buffer first"
        );
    }
}

#[test]
fn goo_full_size_preview_decodes_rle_directly_to_rgba() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(manifest_dir.join("src/formats/goo/mod.rs")).expect("read GOO");
    let preview_body = source
        .split("pub fn decode_layer_preview_rgba")
        .nth(1)
        .and_then(|tail| tail.split("/// Encode grayscale pixels").next())
        .expect("GOO decode_layer_preview_rgba body");

    assert!(
        preview_body.contains("rle::decode_goo_rle_rgba(data, width, height)?"),
        "GOO full-size preview should decode RLE directly to RGBA"
    );
    assert!(
        !preview_body.contains("let pixels = decode_layer_data(data, width, height)?;"),
        "GOO full-size preview should not allocate a full grayscale buffer first"
    );
}

#[test]
fn anycubic_binary_encoder_writes_structs_directly_into_output_buffer() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/encoder.rs"))
        .expect("read Anycubic binary encoder source");
    let write_struct_body = source
        .split("fn write_struct")
        .nth(1)
        .and_then(|tail| tail.split("fn find_table_mark").next())
        .expect("write_struct body");

    assert!(
        write_struct_body.contains("Cursor::new(buf)"),
        "Anycubic binary encoder should write BinWrite structs directly into the target Vec"
    );
    assert!(
        !write_struct_body.contains("let mut tmp = Vec::new();"),
        "Anycubic binary encoder should not allocate a temporary Vec for each struct write"
    );
    assert!(
        !write_struct_body.contains("buf.extend_from_slice(&tmp);"),
        "Anycubic binary encoder should not copy temporary struct bytes into the target Vec"
    );
}

#[test]
fn anycubic_binary_header_finalize_patches_owned_header_in_place() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mod_source = std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/mod.rs"))
        .expect("read Anycubic mod source");
    assert!(
        !mod_source.contains("finalize_anycubic_header_bytes"),
        "Anycubic should not re-export a borrowed header finalize compatibility wrapper"
    );

    let decoder_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/decoder.rs"))
            .expect("read Anycubic decoder source");
    assert!(
        !decoder_source.contains("pub fn finalize_anycubic_header_bytes"),
        "Anycubic decoder should not keep a borrowed header finalize compatibility wrapper"
    );

    let layout_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/layout.rs"))
            .expect("read Anycubic layout strategy source");
    assert!(
        !layout_source.contains("fn finalize_header(&self, header: &[u8]"),
        "Anycubic layout strategy should finalize owned headers directly"
    );

    let encoder_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/encoder.rs"))
            .expect("read Anycubic binary encoder source");
    assert!(
        !encoder_source.contains("pub fn finalize_anycubic_header_bytes"),
        "Anycubic encoder should not expose finalize helper outside the crate"
    );
    let finalize_body = encoder_source
        .split("pub(crate) fn finalize_anycubic_header_bytes")
        .nth(1)
        .and_then(|tail| tail.split("fn build_v1_header").next())
        .expect("Anycubic finalize body");

    assert!(
        finalize_body.contains("mut header: Vec<u8>"),
        "Anycubic header finalize should take ownership of the freshly-built header"
    );
    assert!(
        !finalize_body.contains("header.to_vec()"),
        "Anycubic header finalize should not clone the whole header before patching layer offsets"
    );

    let pipeline_source =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/pipeline.rs"))
            .expect("read common pipeline source");
    assert!(
        pipeline_source.contains("finalize_header_owned(")
            && pipeline_source.contains("header,")
            && pipeline_source.contains("ctx.export_options"),
        "binary pipeline should pass the freshly-built header and export options into owned finalize"
    );
}

#[test]
fn cxd_lp_decode_borrows_plain_payload_slice_when_no_layer_crypto_is_needed() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("src/formats/creality/cxdlp.rs"))
        .expect("read CXDLP source");

    assert!(
        source.contains("Cow::Borrowed(&data[payload_start..])"),
        "CXDLP decode should borrow the payload slice for the common unencrypted path"
    );
    assert!(
        !source.contains("let mut payload = data[payload_start..].to_vec();\r\n        if ctx.options.encryption_key != 0")
            && !source.contains("let mut payload = data[payload_start..].to_vec();\n        if ctx.options.encryption_key != 0"),
        "CXDLP decode should not eagerly clone the layer payload before checking encryption"
    );
}

#[test]
fn cxd_lp_preview_decodes_rle_directly_to_rgba_without_full_grayscale_buffer() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("src/formats/creality/cxdlp.rs"))
        .expect("read CXDLP source");
    let preview_body = source
        .split("fn decode_cxdlp_rle_preview_rgba")
        .nth(1)
        .and_then(|tail| tail.split("fn encode_cxdlp_v4_layer_into").next())
        .expect("decode_cxdlp_rle_preview_rgba body");

    assert!(
        preview_body.contains("decode_cxdlp_rle_rgba_into("),
        "CXDLP preview should stream RLE directly into an RGBA output buffer"
    );
    assert!(
        !preview_body.contains("let mut pixels = Vec::new();"),
        "CXDLP preview should not allocate a full grayscale layer before RGBA conversion"
    );
    assert!(
        !preview_body.contains("decode_cxdlp_rle_into(data, width, height, &mut pixels)"),
        "CXDLP preview should avoid the full grayscale decode fallback"
    );
}

#[test]
fn chitubox_encrypted_profile_decoders_borrow_plain_layers_when_no_crypto_is_needed() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (label, relative_path, borrow_fragment, eager_clone_fragment) in [
        (
            "PHZ layer codec",
            "src/formats/chitubox/phz.rs",
            "Cow::Borrowed(data)",
            "let mut payload = data.to_vec();",
        ),
        (
            "FDG layer codec",
            "src/formats/chitubox/fdg.rs",
            "Cow::Borrowed(data)",
            "let mut payload = data.to_vec();",
        ),
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("read {label} source: {error}"));
        assert!(
            source.contains("use std::borrow::Cow;"),
            "{label} should use Cow so the unencrypted decode path can borrow layer bytes"
        );
        assert!(
            source.contains(borrow_fragment),
            "{label} should borrow layer bytes when encryption_key is zero"
        );
        assert!(
            !source.contains(eager_clone_fragment),
            "{label} should not eagerly clone every layer before checking encryption"
        );
    }
}

#[test]
fn anycubic_zip_archive_has_focused_modules() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let archive_rs_path = manifest_dir.join("src/formats/anycubic/archive.rs");
    let archive_root = manifest_dir.join("src/formats/anycubic/archive");
    let archive_rs =
        std::fs::read_to_string(&archive_rs_path).expect("read anycubic archive.rs source");

    for module_name in [
        "calc_volumes",
        "codec",
        "layout",
        "manifest",
        "scene",
        "zip_io",
    ] {
        let declaration = format!("mod {module_name};");
        assert!(
            archive_rs.contains(&declaration),
            "archive.rs should declare focused module {module_name}"
        );
        assert!(
            archive_root.join(format!("{module_name}.rs")).is_file(),
            "focused module file should exist for {module_name}"
        );
    }

    assert!(
        !archive_rs.contains("struct AnycubicZipSettingsManifest"),
        "manifest JSON structs belong in archive/manifest.rs"
    );
    assert!(
        !archive_rs.contains("impl LayerCodec for AnycubicZipLayerCodec"),
        "layer codec behavior belongs in archive/codec.rs"
    );
    assert!(
        !archive_rs.contains("impl ArchiveLayout for AnycubicZipArchiveLayout"),
        "archive entry layout belongs in archive/layout.rs"
    );
}

#[test]
fn chitubox_zip_archive_has_focused_modules() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let archive_rs_path = manifest_dir.join("src/formats/chitubox/archive.rs");
    let archive_root = manifest_dir.join("src/formats/chitubox/archive");
    let archive_rs =
        std::fs::read_to_string(&archive_rs_path).expect("read chitubox archive.rs source");

    for module_name in ["gcode", "layout", "zip_io"] {
        let declaration = format!("mod {module_name};");
        assert!(
            archive_rs.contains(&declaration),
            "archive.rs should declare focused module {module_name}"
        );
        assert!(
            archive_root.join(format!("{module_name}.rs")).is_file(),
            "focused module file should exist for {module_name}"
        );
    }

    assert!(
        !archive_rs.contains("fn parse_gcode_metadata"),
        "run.gcode metadata parsing belongs in archive/gcode.rs"
    );
    assert!(
        !archive_rs.contains("impl ArchiveLayout for ChituboxZipArchiveLayout"),
        "archive entry layout belongs in archive/layout.rs"
    );
    assert!(
        !archive_rs.contains("fn collect_zip_entries"),
        "ZIP entry collection belongs in archive/zip_io.rs"
    );
}

#[test]
fn zip_archive_io_helpers_are_shared_across_archive_families() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_archive =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/archive.rs"))
            .expect("read common archive source");
    let anycubic_zip_io =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/archive/zip_io.rs"))
            .expect("read anycubic zip_io source");
    let chitubox_zip_io =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/archive/zip_io.rs"))
            .expect("read chitubox zip_io source");

    for helper in [
        "pub(crate) fn encode_png_thumbnail",
        "pub(crate) fn collect_zip_entries",
        "pub(crate) fn read_required_zip_text",
        "pub(crate) fn read_optional_zip_text",
        "pub(crate) fn read_optional_zip_bytes",
    ] {
        assert!(
            common_archive.contains(helper),
            "common archive module should own shared helper {helper}"
        );
    }

    for family_source in [anycubic_zip_io, chitubox_zip_io] {
        for duplicate in [
            "fn encode_thumbnail_png",
            "fn collect_zip_entries",
            "fn read_zip_text",
            "fn read_required_zip_text",
            "fn read_optional_zip_text",
            "fn read_optional_zip_bytes",
        ] {
            assert!(
                !family_source.contains(duplicate),
                "archive family zip_io modules should not duplicate shared helper {duplicate}"
            );
        }
    }
}

#[test]
fn archive_layer_entry_sorting_is_shared() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_archive =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/archive.rs"))
            .expect("read common archive source");
    let anycubic_layout =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/archive/layout.rs"))
            .expect("read anycubic archive layout source");
    let chitubox_layout =
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/archive/layout.rs"))
            .expect("read chitubox archive layout source");
    let sl1_adapters = std::fs::read_to_string(manifest_dir.join("src/formats/sl1/adapters.rs"))
        .expect("read sl1 adapters source");
    let vdt_adapters = std::fs::read_to_string(manifest_dir.join("src/formats/vdt/adapters.rs"))
        .expect("read vdt adapters source");

    assert!(
        common_archive.contains("pub(crate) fn sorted_layer_offsets"),
        "archive layer entry sorting should live in common archive module"
    );
    for layout_source in [anycubic_layout, chitubox_layout, sl1_adapters, vdt_adapters] {
        assert!(
            layout_source.contains("sorted_layer_offsets("),
            "archive layouts should delegate layer offset sorting to common archive helper"
        );
        assert!(
            !layout_source.contains("sort_by_key"),
            "archive layouts should not duplicate layer sorting"
        );
    }
}

#[test]
fn legacy_archive_decoders_reuse_common_zip_helpers() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sl1_decoder = std::fs::read_to_string(manifest_dir.join("src/formats/sl1/decoder.rs"))
        .expect("read sl1 decoder source");
    let vdt_decoder = std::fs::read_to_string(manifest_dir.join("src/formats/vdt/decoder.rs"))
        .expect("read vdt decoder source");

    assert!(
        sl1_decoder.contains("collect_zip_entries("),
        "SL1 legacy decoder should enumerate ZIP entries through common archive helper"
    );
    assert!(
        sl1_decoder.contains("read_required_zip_text("),
        "SL1 legacy decoder should read required ZIP text through common archive helper"
    );
    assert!(
        sl1_decoder.contains("read_optional_zip_text("),
        "SL1 legacy decoder should read optional ZIP text through common archive helper"
    );
    assert!(
        sl1_decoder.contains("read_zip_entry_bytes("),
        "SL1 legacy decoder should read ZIP entry bytes through common archive helper"
    );
    assert!(
        sl1_decoder.contains("sorted_layer_offsets("),
        "SL1 legacy decoder should reuse common layer ordering"
    );

    assert!(
        vdt_decoder.contains("collect_zip_entries("),
        "VDT legacy decoder should enumerate ZIP entries through common archive helper"
    );
    assert!(
        vdt_decoder.contains("read_required_zip_text("),
        "VDT legacy decoder should read manifest text through common archive helper"
    );
    assert!(
        vdt_decoder.contains("sorted_layer_offsets("),
        "VDT legacy decoder should reuse common layer ordering"
    );

    for decoder_source in [sl1_decoder, vdt_decoder] {
        assert!(
            !decoder_source.contains("read_to_string"),
            "legacy archive decoders should not duplicate ZIP text reading"
        );
        assert!(
            !decoder_source.contains("read_to_end"),
            "legacy archive decoders should not duplicate ZIP byte reading"
        );
    }
}

#[test]
fn archive_metadata_entry_construction_is_shared() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_archive =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/archive.rs"))
            .expect("read common archive source");
    let anycubic_zip_io =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/archive/zip_io.rs"))
            .expect("read anycubic zip_io source");
    let layout_sources = [
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/archive/layout.rs"))
            .expect("read anycubic layout source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/archive/layout.rs"))
            .expect("read chitubox layout source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/sl1/adapters.rs"))
            .expect("read sl1 adapters source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/vdt/adapters.rs"))
            .expect("read vdt adapters source"),
    ];

    for helper in [
        "pub(crate) fn archive_entry_descriptor",
        "pub(crate) fn bytes_archive_entry",
        "pub(crate) fn text_archive_entry",
        "pub(crate) fn json_archive_entry",
    ] {
        assert!(
            common_archive.contains(helper),
            "common archive module should own shared entry constructor {helper}"
        );
    }

    assert!(
        !anycubic_zip_io.contains("fn json_archive_entry")
            && !anycubic_zip_io.contains("fn bytes_archive_entry"),
        "Anycubic zip_io should not keep local archive entry constructors"
    );

    for layout_source in layout_sources {
        assert!(
            !layout_source.contains("ArchiveEntry {"),
            "archive layouts should use common archive entry constructors"
        );
        assert!(
            !layout_source.contains("ArchiveEntryDescriptor {\n            name:")
                && !layout_source.contains("ArchiveEntryDescriptor {\r\n            name:"),
            "archive layouts should use common archive entry descriptor constructors"
        );
    }
}

#[test]
fn zip_archive_detection_helpers_are_shared() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_archive =
        std::fs::read_to_string(manifest_dir.join("src/formats/common/archive.rs"))
            .expect("read common archive source");
    let archive_header_sources = [
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/archive.rs"))
            .expect("read anycubic archive source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/chitubox/archive.rs"))
            .expect("read chitubox archive source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/sl1/adapters.rs"))
            .expect("read sl1 adapters source"),
        std::fs::read_to_string(manifest_dir.join("src/formats/vdt/adapters.rs"))
            .expect("read vdt adapters source"),
    ];

    for helper in [
        "pub(crate) fn has_zip_magic",
        "pub(crate) fn contains_archive_marker",
        "pub(crate) fn contains_archive_markers",
    ] {
        assert!(
            common_archive.contains(helper),
            "common archive module should own ZIP detection helper {helper}"
        );
    }

    for source in archive_header_sources {
        assert!(
            !source.contains("fn has_zip_magic"),
            "archive header adapters should not duplicate ZIP magic detection"
        );
        assert!(
            !source.contains("fn contains_marker"),
            "archive header adapters should not duplicate archive marker scanning"
        );
        assert!(
            !source.contains("data.windows("),
            "archive header adapters should delegate marker scanning to common archive helpers"
        );
    }
}

#[test]
fn preview_thumbnail_resize_logic_is_shared() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let common_layer = std::fs::read_to_string(manifest_dir.join("src/common/layer.rs"))
        .expect("read common layer source");
    let ctb_encoder = std::fs::read_to_string(manifest_dir.join("src/formats/ctb/encoder.rs"))
        .expect("read ctb encoder source");
    let goo_encoder = std::fs::read_to_string(manifest_dir.join("src/formats/goo/encoder.rs"))
        .expect("read goo encoder source");

    assert!(
        common_layer.contains("pub fn thumbnail_rgba_for_size"),
        "thumbnail selection and resize should live in common layer helpers"
    );

    for encoder_source in [ctb_encoder, goo_encoder] {
        assert!(
            encoder_source.contains("thumbnail_rgba_for_size("),
            "preview encoders should share thumbnail selection and resize"
        );
        assert!(
            !encoder_source.contains("fn resize_rgba_nearest"),
            "preview encoders should not duplicate RGBA resize"
        );
        assert!(
            !encoder_source.contains("min_by_key(|t|"),
            "preview encoders should not duplicate closest-thumbnail selection"
        );
    }
}

#[test]
fn archive_output_has_no_legacy_whole_zip_builders() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !manifest_dir.join("src/formats/vdt/encoder.rs").exists(),
        "VDT should not keep a dedicated whole-archive encoder module"
    );

    let source_files = [
        ("sl1 encoder", "src/formats/sl1/encoder.rs"),
        ("vdt module", "src/formats/vdt/mod.rs"),
    ];

    for (label, relative_path) in source_files {
        let source = std::fs::read_to_string(manifest_dir.join(relative_path))
            .unwrap_or_else(|error| panic!("read {label} source: {error}"));
        assert!(
            !source.contains("build_sl1_zip") && !source.contains("build_vdt_zip"),
            "{label} should not expose whole-archive ZIP builders; use ArchiveLayout + FormatPipeline"
        );
    }
}

#[test]
fn transcode_fast_paths_write_raw_output_directly_without_aux_copy() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ctb_to_goo =
        std::fs::read_to_string(manifest_dir.join("src/transcode/paths/ctb_to_goo.rs"))
            .expect("read ctb_to_goo fast path");
    let goo_to_ctb =
        std::fs::read_to_string(manifest_dir.join("src/transcode/paths/goo_family_to_ctb.rs"))
            .expect("read goo_family_to_ctb fast path");

    assert!(
        !ctb_to_goo.contains("scratch.aux_buf"),
        "CTB->GOO fast path should write raw and wrapped output directly into out instead of staging RLE in aux_buf"
    );
    assert!(
        !goo_to_ctb.contains("scratch.aux_buf"),
        "GOO-family->CTB fast path should write directly into out instead of staging in aux_buf"
    );
    assert!(
        ctb_to_goo.contains("begin_goo_layer_with_metadata_into"),
        "CTB->GOO should still support metadata wrapping when the route needs a GOO layer definition"
    );
}

#[test]
fn prz_reuses_goo_family_adapters_without_reexport_module() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prz_adapters_path = manifest_dir.join("src/formats/prz/adapters.rs");
    let prz_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/prz/mod.rs")).expect("read PRZ mod");
    let prz_specs = std::fs::read_to_string(manifest_dir.join("src/formats/prz/specs.rs"))
        .expect("read PRZ specs");

    assert!(
        !prz_adapters_path.exists(),
        "PRZ should not keep a pure re-export adapters module for GOO-family strategy types"
    );
    assert!(
        !prz_mod.contains("pub mod adapters;"),
        "PRZ module should not expose a compatibility adapters module"
    );
    assert!(
        prz_specs.contains("goo::adapters::")
            && prz_specs.contains("GooFamilyFooterPolicy")
            && prz_specs.contains("GooFamilyHeaderLayout")
            && prz_specs.contains("GooFamilyLayerCodec"),
        "PRZ specs should import the shared GOO-family adapters directly"
    );
}

#[test]
fn fixture_generator_uses_format_pipeline_for_file_assembly() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("examples/generate_fixtures.rs"))
        .expect("read generate_fixtures example");

    assert!(
        source.contains("encode_document("),
        "fixture generation should route full file assembly through a shared encode_document helper"
    );
    for legacy_fragment in [
        "extend_from_slice(&header)",
        "extend_from_slice(&footer)",
        "format.build_header(metadata)",
        "format.finalize_header(",
    ] {
        assert!(
            !source.contains(legacy_fragment),
            "fixture generator should not keep manual header/layer/footer assembly fragment {legacy_fragment}"
        );
    }
}

#[test]
fn fixture_conversion_tests_use_format_pipeline_for_file_assembly() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("tests/fixture_files.rs"))
        .expect("read fixture_files tests");
    let conversion_body = source
        .split("fn cross_format_convert_fixture_goo_to_ctb()")
        .nth(1)
        .expect("GOO to CTB fixture conversion test exists")
        .split("\n#[test]")
        .next()
        .expect("conversion test body should be bounded by next test");

    assert!(
        conversion_body.contains("encode_document("),
        "fixture conversion tests should assemble output files through the shared FormatPipeline helper"
    );
    for legacy_fragment in [
        "build_header_bytes(",
        "finalize_header_bytes(",
        "extend_from_slice(layer)",
    ] {
        assert!(
            !conversion_body.contains(legacy_fragment),
            "fixture conversion test should not keep manual file assembly fragment {legacy_fragment}"
        );
    }
}

#[test]
fn cross_format_file_assembly_roundtrips_use_format_pipeline() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest_dir.join("tests/cross_format.rs"))
        .expect("read cross_format tests");
    let roundtrip_body = source
        .split("fn goo_family_file_assembly_roundtrip()")
        .nth(1)
        .expect("GOO family file assembly test exists")
        .split("\n    #[test]")
        .next()
        .expect("roundtrip test body should be bounded by next test");

    assert!(
        roundtrip_body.contains("encode_document("),
        "cross-format full-file roundtrip should assemble through the shared FormatPipeline helper"
    );
    for legacy_fragment in [
        "build_header_bytes(",
        "build_footer_bytes(",
        "extend_from_slice(&footer)",
    ] {
        assert!(
            !roundtrip_body.contains(legacy_fragment),
            "cross-format full-file roundtrip should not keep manual assembly fragment {legacy_fragment}"
        );
    }
}

#[test]
fn vdt_archive_entry_policy_is_factored_out_of_decoder_and_adapter_glue() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vdt_mod =
        std::fs::read_to_string(manifest_dir.join("src/formats/vdt/mod.rs")).expect("read vdt mod");
    let decoder_source = std::fs::read_to_string(manifest_dir.join("src/formats/vdt/decoder.rs"))
        .expect("read vdt decoder");
    let adapters_source = std::fs::read_to_string(manifest_dir.join("src/formats/vdt/adapters.rs"))
        .expect("read vdt adapters");

    assert!(
        vdt_mod.contains("mod entries;"),
        "VDT archive entry naming and parsing should live in a dedicated entries module"
    );
    assert!(
        !decoder_source.contains("fn vdt_layer_index"),
        "VDT decoder should consume shared entry policy instead of owning layer-name parsing"
    );
    assert!(
        !adapters_source.contains("format!(\"{layer_index}.png\")"),
        "VDT ArchiveLayout should use shared entry naming instead of hardcoding names"
    );
}

#[test]
fn png_row_preview_uses_lightweight_row_column_plan() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sl1_decoder = std::fs::read_to_string(manifest_dir.join("src/formats/sl1/decoder.rs"))
        .expect("read sl1 decoder");
    let row_preview_body = sl1_decoder
        .split("fn decode_png_layer_preview_rgba_rows")
        .nth(1)
        .expect("PNG row preview function exists")
        .split("fn sample_png_row_to_rgba")
        .next()
        .expect("PNG row preview body is bounded by sample helper");

    assert!(
        sl1_decoder.contains("struct PngPreviewDecodePlan"),
        "PNG preview should use a row/column plan instead of the generic per-pixel index plan"
    );
    assert!(
        row_preview_body.contains("PngPreviewDecodePlan"),
        "PNG row preview should receive the lightweight PNG preview plan"
    );
    assert!(
        !row_preview_body.contains("target_indices")
            && !row_preview_body.contains("source_row_for_dst_y")
            && !row_preview_body.contains("source_col_for_dst_x"),
        "PNG row preview should not allocate a per-pixel PreviewDecodePlan target index table"
    );
}
