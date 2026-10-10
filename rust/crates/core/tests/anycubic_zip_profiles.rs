use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use slicer_converter_core::common::{FileMetadata, Thumbnail};
use slicer_converter_core::formats::{
    self,
    anycubic::{
        archive::{AnycubicZipArchiveLayout, AnycubicZipHeaderLayout, AnycubicZipLayerCodec},
        codecs::pw0_rle,
        specifics::AnycubicArchiveSpec,
    },
    common::{
        keys, ArchiveEncodeContext, ContainerKind, DefaultArchivePipeline, FormatCapabilities,
        FormatCatalog, FormatFamily, FormatKey, FormatKeys, FormatSpec, HeaderLayout, LayerSource,
        NoCrypto, NoFooter, VecOutputSink, ZipEntryInfo,
    },
    sl1, FormatIdentity, LayerDecodeContext, LayerEncodeContext,
};

fn read_le_u32_at(data: &[u8], index: &mut usize, label: &str) -> u32 {
    let end = index.saturating_add(4);
    assert!(end <= data.len(), "truncated {label} at {index}");
    let value = u32::from_le_bytes(data[*index..end].try_into().expect("u32"));
    *index = end;
    value
}

fn read_le_f32_at(data: &[u8], index: &mut usize, label: &str) -> f32 {
    let end = index.saturating_add(4);
    assert!(end <= data.len(), "truncated {label} at {index}");
    let value = f32::from_le_bytes(data[*index..end].try_into().expect("f32"));
    *index = end;
    value
}

fn zip_entry(index: usize, name: &str, uncompressed_size: u64) -> ZipEntryInfo {
    ZipEntryInfo {
        index,
        name: name.to_string(),
        compressed_size: uncompressed_size,
        uncompressed_size,
    }
}

#[test]
fn anycubic_zip_pipeline_parses_manifests_and_pw0_layers() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    assert!(spec.capabilities.can_import);
    assert!(spec.capabilities.can_export);
    assert!(spec.capabilities.supports_preview);

    let pixels0 = vec![0, 17, 34, 51, 68, 85, 102, 119];
    let pixels1 = vec![136, 153, 170, 187, 204, 221, 238, 255];
    let rle0 = pw0_rle::encode_pw0_rle4(&pixels0);
    let rle1 = pw0_rle::encode_pw0_rle4(&pixels1);
    let data = zip_fixture(vec![
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
        ("preview_images/preview_0.png", png_preview(&pixels0)),
        ("preview_images/preview_1.png", png_preview(&pixels1)),
        ("layer_images/layer_0.pw0Img", rle0.clone()),
        ("layer_images/layer_1.pw0Img", rle1),
    ]);

    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("anycubic zip document");

    assert_eq!(parsed.metadata.printer.machine_name, "Photon Mono M7");
    assert_eq!(parsed.metadata.printer.resolution_x, 4);
    assert_eq!(parsed.metadata.printer.resolution_y, 2);
    assert_eq!(parsed.metadata.printer.display_width_mm, 80.0);
    assert_eq!(parsed.metadata.printer.display_height_mm, 40.0);
    assert_eq!(parsed.metadata.printer.machine_z_mm, 160.0);
    assert_eq!(parsed.metadata.thumbnails.len(), 2);
    assert_eq!(parsed.metadata.thumbnails[0].width, 4);
    assert_eq!(parsed.metadata.thumbnails[0].height, 2);
    assert_eq!(
        &parsed.metadata.thumbnails[0].pixels_rgba[..8],
        &[0, 0, 0, 255, 17, 17, 17, 255]
    );
    assert_eq!(parsed.metadata.slicing.layer_count, 2);
    assert_eq!(parsed.metadata.slicing.layer_height_mm, 0.05);
    assert_eq!(parsed.metadata.slicing.anti_alias_level, 8);
    assert_eq!(parsed.metadata.exposure.exposure_time, 2.2);
    assert_eq!(parsed.metadata.motion.lift_height, 5.0);
    assert_eq!(parsed.metadata.motion.lift_speed, 6.0);
    assert_eq!(parsed.metadata.stats.print_time_sec, 321);
    assert_eq!(parsed.metadata.stats.volume_ml, 4.5);
    assert_eq!(parsed.metadata.stats.weight_g, 5.5);
    assert_eq!(parsed.metadata.stats.cost, 6.5);
    assert_eq!(parsed.metadata.per_layer_settings.len(), 2);
    assert_eq!(parsed.metadata.per_layer_settings[1].position_z, 0.1);
    assert_eq!(
        parsed.metadata.per_layer_settings[1].exposure_time,
        Some(2.4)
    );

    assert_eq!(parsed.layers.len(), 2);
    assert_eq!(parsed.layers[0].offset, 5);
    assert_eq!(parsed.layers[1].offset, 6);
    assert!(parsed.layers[0].size > 0);

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    let mut encoded = Vec::new();
    archive
        .by_index(parsed.layers[0].offset as usize)
        .expect("first layer")
        .read_to_end(&mut encoded)
        .expect("layer bytes");

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(&encoded, &LayerDecodeContext::new(0, 4, 2), &mut decoded)
        .expect("decode pw0Img layer");
    assert_eq!(decoded, pixels0);
}

#[test]
fn anycubic_zip_archive_export_builds_printer_native_entries() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    assert!(spec.capabilities.can_export);

    let pixels0 = vec![0, 17, 34, 51, 68, 85, 102, 119];
    let pixels1 = vec![136, 153, 170, 187, 204, 221, 238, 255];
    let thumbnail = Thumbnail {
        width: 2,
        height: 2,
        pixels_rgba: vec![
            0, 0, 0, 255, 64, 64, 64, 255, 128, 128, 128, 255, 255, 255, 255, 255,
        ],
    };
    let metadata = FileMetadata::builder()
        .machine_name("Photon Mono M7")
        .resolution(4, 2)
        .display_size_mm(80.0, 40.0)
        .machine_z_mm(160.0)
        .layer_height_mm(0.05)
        .layer_count(2)
        .anti_alias_level(8)
        .exposure(2.2, 30.0)
        .lift(5.0, 6.0)
        .statistics(321, 4.5, 5.5, 6.5)
        .thumbnails(vec![thumbnail])
        .build()
        .expect("metadata");

    let mut layers = Vec::new();
    for (layer_index, pixels) in [&pixels0, &pixels1].iter().enumerate() {
        let mut encoded = Vec::new();
        spec.layer_codec
            .encode_layer_into(
                pixels,
                &LayerEncodeContext {
                    layer_index: layer_index as u32,
                    width: 4,
                    height: 2,
                    layer_metadata: Some(metadata.as_layer_encode_metadata()),
                },
                &mut encoded,
            )
            .expect("encode pw0Img layer");
        layers.push(encoded);
    }

    let mut sink = VecOutputSink::new();
    let archive_ctx = ArchiveEncodeContext::new("anycubic");
    let stats = spec
        .pipeline
        .encode_document_into(
            spec,
            &metadata,
            LayerSource::Encoded(&layers),
            &mut sink,
            &archive_ctx,
        )
        .expect("encode anycubic zip");
    let archive_bytes = sink.into_inner();

    assert!(archive_bytes.starts_with(b"PK"));
    assert_eq!(stats.layers_written, 2);
    assert_eq!(stats.bytes_written, archive_bytes.len());

    let mut archive = zip::ZipArchive::new(Cursor::new(&archive_bytes)).expect("zip archive");
    let entry_names = (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .expect("zip entry")
                .name()
                .to_string()
        })
        .collect::<Vec<_>>();
    for required in [
        "preview_images/preview_0.png",
        "anycubic_photon_resins.pwsp",
        "layers_controller.conf",
        "lcd_function.json",
        "print_info.json",
        "software_info.conf",
        "calc_layer_volumes.data",
        "scene.slice",
        "layer_images/layer_0.pw0Img",
        "layer_images/layer_1.pw0Img",
    ] {
        assert!(
            entry_names.iter().any(|name| name == required),
            "missing {required}; entries={entry_names:?}"
        );
    }

    let parsed = spec
        .pipeline
        .parse_document(spec, &archive_bytes)
        .expect("parse exported anycubic zip");
    assert_eq!(parsed.metadata.printer.machine_name, "Photon Mono M7");
    assert_eq!(parsed.metadata.printer.resolution_x, 4);
    assert_eq!(parsed.metadata.printer.resolution_y, 2);
    assert_eq!(parsed.metadata.slicing.layer_count, 2);
    assert_eq!(parsed.metadata.slicing.anti_alias_level, 8);
    assert_eq!(parsed.metadata.stats.print_time_sec, 321);
    assert_eq!(parsed.metadata.stats.volume_ml, 4.5);
    assert_eq!(parsed.metadata.stats.weight_g, 5.5);
    assert_eq!(parsed.metadata.thumbnails.len(), 1);
    assert_eq!(parsed.layers.len(), 2);

    let mut encoded = Vec::new();
    archive
        .by_name("layer_images/layer_0.pw0Img")
        .expect("exported first layer")
        .read_to_end(&mut encoded)
        .expect("layer bytes");
    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(&encoded, &LayerDecodeContext::new(0, 4, 2), &mut decoded)
        .expect("decode exported layer");
    assert_eq!(decoded, pixels0);
}

#[test]
fn anycubic_zip_archive_layout_reads_entry_contract_from_typed_specifics() {
    let spec = custom_anycubic_zip_spec();
    let entries = vec![
        zip_entry(0, "alt_layers/slice_1.customImg", 5),
        zip_entry(1, "layer_images/layer_0.pw0Img", 7),
        zip_entry(2, "alt_layers/slice_0.customImg", 11),
    ];

    let layers = spec
        .archive
        .list_layer_entries(&entries, spec)
        .expect("custom archive layers");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].offset, 2);
    assert_eq!(layers[1].offset, 0);
    assert_eq!(
        spec.archive
            .describe_layer_entry(&ArchiveEncodeContext::new("ignored"), 3, spec)
            .name,
        "alt_layers/slice_3.customImg"
    );
}

#[test]
fn anycubic_zip_header_layout_reads_required_manifest_markers_from_typed_specifics() {
    assert!(CUSTOM_DETECT_ANYCUBIC_ZIP_HEADER.try_detect(
        b"PK\x03\x04 custom_settings.json custom_layers.json",
        Some("customzip")
    ));
    assert!(!CUSTOM_DETECT_ANYCUBIC_ZIP_HEADER.try_detect(
        b"PK\x03\x04 anycubic_photon_resins.pwsp layers_controller.conf",
        Some("customzip")
    ));
}

#[test]
fn anycubic_zip_manifest_entry_names_are_driven_by_typed_specifics() {
    let spec = custom_anycubic_zip_spec();
    let metadata = FileMetadata::builder()
        .machine_name("Photon Mono M7")
        .resolution(4, 2)
        .display_size_mm(80.0, 40.0)
        .machine_z_mm(160.0)
        .layer_height_mm(0.05)
        .layer_count(1)
        .anti_alias_level(8)
        .exposure(2.2, 30.0)
        .lift(5.0, 6.0)
        .build()
        .expect("metadata");

    let metadata_entries = spec
        .archive
        .build_metadata_entries(&ArchiveEncodeContext::new("ignored"), &metadata, 1, spec)
        .expect("custom metadata entries");
    let metadata_entry_names = metadata_entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();

    assert!(metadata_entry_names.contains(&"custom_settings.json"));
    assert!(metadata_entry_names.contains(&"custom_layers.json"));
    assert!(!metadata_entry_names.contains(&"anycubic_photon_resins.pwsp"));
    assert!(!metadata_entry_names.contains(&"layers_controller.conf"));

    let data = zip_fixture(vec![
        (
            "custom_settings.json",
            anycubic_zip_settings_json().as_bytes().to_vec(),
        ),
        (
            "custom_layers.json",
            anycubic_zip_layers_json().as_bytes().to_vec(),
        ),
        ("alt_layers/slice_0.customImg", vec![1, 2, 3]),
    ]);
    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("custom manifest names parse");

    assert_eq!(parsed.metadata.printer.machine_name, "Photon Mono M7");
    assert_eq!(parsed.layers.len(), 1);
    assert_eq!(parsed.layers[0].offset, 2);
}

#[test]
fn anycubic_zip_archive_layout_uses_adapter_specifics_not_catalog_facts() {
    let spec = custom_anycubic_zip_spec_without_catalog_facts();
    let entries = vec![
        zip_entry(0, "alt_layers/slice_1.customImg", 5),
        zip_entry(1, "layer_images/layer_0.pw0Img", 7),
        zip_entry(2, "alt_layers/slice_0.customImg", 11),
    ];

    let layers = spec
        .archive
        .list_layer_entries(&entries, spec)
        .expect("custom archive layers");

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].offset, 2);
    assert_eq!(
        spec.archive
            .describe_layer_entry(&ArchiveEncodeContext::new("ignored"), 3, spec)
            .name,
        "alt_layers/slice_3.customImg"
    );
}

#[test]
fn anycubic_zip_header_layout_uses_adapter_specifics_not_catalog_facts() {
    let spec = custom_anycubic_zip_spec_without_catalog_facts();
    let data = zip_fixture(vec![
        (
            "custom_settings.json",
            anycubic_zip_settings_json().as_bytes().to_vec(),
        ),
        (
            "custom_layers.json",
            anycubic_zip_layers_json().as_bytes().to_vec(),
        ),
        ("alt_layers/slice_0.customImg", vec![1, 2, 3]),
    ]);

    let (metadata, layers) = spec
        .header
        .parse_header(&data, spec)
        .expect("custom header parse");

    assert_eq!(metadata.printer.machine_name, "Photon Mono M7");
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].offset, 2);
}

#[test]
fn anycubic_zip_pipeline_parses_scene_and_calc_manifests() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let pixels0 = vec![0, 17, 34, 51, 68, 85, 102, 119];
    let pixels1 = vec![136, 153, 170, 187, 204, 221, 238, 255];
    let data = zip_fixture(vec![
        (
            "anycubic_photon_resins.pwsp",
            anycubic_zip_settings_json().as_bytes().to_vec(),
        ),
        (
            "layers_controller.conf",
            anycubic_zip_layers_json().as_bytes().to_vec(),
        ),
        ("scene.slice", anycubic_zip_scene_manifest()),
        ("calc_layer_volumes.data", anycubic_zip_calc_layer_volumes()),
        (
            "layer_images/layer_0.pw0Img",
            pw0_rle::encode_pw0_rle4(&pixels0),
        ),
        (
            "layer_images/layer_1.pw0Img",
            pw0_rle::encode_pw0_rle4(&pixels1),
        ),
    ]);

    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("anycubic zip document with binary manifests");

    assert_eq!(parsed.metadata.stats.volume_ml, 3.3);
    assert_eq!(parsed.metadata.stats.weight_g, 3.96);
    assert_eq!(parsed.metadata.per_layer_settings[0].position_z, 0.07);
    assert_eq!(parsed.metadata.per_layer_settings[1].position_z, 0.14);
}

static CUSTOM_ANYCUBIC_ARCHIVE_SPECIFICS: AnycubicArchiveSpec = AnycubicArchiveSpec {
    settings_manifest_entry: "custom_settings.json",
    layers_manifest_entry: "custom_layers.json",
    layer_entry_prefix: "alt_layers/slice_",
    default_layer_image_extension: "customImg",
    layer_image_extensions: &["customImg"],
    required_manifest_entries: &["custom_settings.json", "custom_layers.json"],
};

static CUSTOM_DETECT_ANYCUBIC_ARCHIVE_SPECIFICS: AnycubicArchiveSpec = AnycubicArchiveSpec {
    settings_manifest_entry: "custom_settings.json",
    layers_manifest_entry: "custom_layers.json",
    layer_entry_prefix: "layer_images/layer_",
    default_layer_image_extension: "pw0Img",
    layer_image_extensions: &["pw0Img"],
    required_manifest_entries: &["custom_settings.json", "custom_layers.json"],
};

static CUSTOM_DETECT_ANYCUBIC_ZIP_HEADER: AnycubicZipHeaderLayout = AnycubicZipHeaderLayout {
    specifics: &CUSTOM_DETECT_ANYCUBIC_ARCHIVE_SPECIFICS,
};

static CUSTOM_ANYCUBIC_ZIP_HEADER: AnycubicZipHeaderLayout = AnycubicZipHeaderLayout {
    specifics: &CUSTOM_ANYCUBIC_ARCHIVE_SPECIFICS,
};
static CUSTOM_ANYCUBIC_ZIP_CODEC: AnycubicZipLayerCodec = AnycubicZipLayerCodec;
static CUSTOM_ANYCUBIC_ZIP_ARCHIVE: AnycubicZipArchiveLayout = AnycubicZipArchiveLayout {
    specifics: &CUSTOM_ANYCUBIC_ARCHIVE_SPECIFICS,
};
static CUSTOM_ANYCUBIC_ZIP_SPEC: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "custom-anycubic-zip",
        aliases: &["custom-anycubic-zip"],
        extensions: &["customzip"],
    },
    family: FormatFamily::Anycubic,
    catalog: FormatCatalog {
        profile_id: "custom-anycubic-zip",
        display_name: "Custom Anycubic Zip",
        default_extension: "customzip",
        target_names: &["custom-anycubic-zip"],
        default_export: false,
    },
    container: ContainerKind::Archive,
    capabilities: FormatCapabilities::archive_import_export(),
    keys: FormatKeys {
        family_key: keys::FAMILY_ANYCUBIC,
        profile_key: FormatKey("profile/custom-anycubic-zip"),
        container_key: keys::CONTAINER_ARCHIVE,
        header_key: keys::HEADER_ANYCUBIC_ZIP,
        layer_codec_key: keys::ANYCUBIC_ZIP_RLE,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::ZIP_ARCHIVE,
    },
    header: &CUSTOM_ANYCUBIC_ZIP_HEADER,
    layer_codec: &CUSTOM_ANYCUBIC_ZIP_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &CUSTOM_ANYCUBIC_ZIP_ARCHIVE,
    pipeline: &DefaultArchivePipeline,
};

static CUSTOM_ANYCUBIC_ZIP_SPEC_WITHOUT_CATALOG_FACTS: FormatSpec = FormatSpec {
    identity: FormatIdentity {
        name: "custom-anycubic-zip-without-format-specifics",
        aliases: &["custom-anycubic-zip-without-format-specifics"],
        extensions: &["customzip"],
    },
    family: FormatFamily::Anycubic,
    catalog: FormatCatalog {
        profile_id: "custom-anycubic-zip-without-format-specifics",
        display_name: "Custom Anycubic Zip Without Format Specifics",
        default_extension: "customzip",
        target_names: &["custom-anycubic-zip-without-format-specifics"],
        default_export: false,
    },
    container: ContainerKind::Archive,
    capabilities: FormatCapabilities::archive_import_export(),
    keys: FormatKeys {
        family_key: keys::FAMILY_ANYCUBIC,
        profile_key: FormatKey("profile/custom-anycubic-zip-without-format-specifics"),
        container_key: keys::CONTAINER_ARCHIVE,
        header_key: keys::HEADER_ANYCUBIC_ZIP,
        layer_codec_key: keys::ANYCUBIC_ZIP_RLE,
        pixel_model_key: keys::PIXEL_GRAYSCALE_8,
        crypto_key: keys::NO_CRYPTO,
        footer_key: keys::NO_FOOTER,
        archive_key: keys::ZIP_ARCHIVE,
    },
    header: &CUSTOM_ANYCUBIC_ZIP_HEADER,
    layer_codec: &CUSTOM_ANYCUBIC_ZIP_CODEC,
    crypto: &NoCrypto,
    footer: &NoFooter,
    archive: &CUSTOM_ANYCUBIC_ZIP_ARCHIVE,
    pipeline: &DefaultArchivePipeline,
};

fn custom_anycubic_zip_spec() -> &'static FormatSpec {
    &CUSTOM_ANYCUBIC_ZIP_SPEC
}

fn custom_anycubic_zip_spec_without_catalog_facts() -> &'static FormatSpec {
    &CUSTOM_ANYCUBIC_ZIP_SPEC_WITHOUT_CATALOG_FACTS
}

#[test]
fn anycubic_zip_layer_codec_decodes_pwsz_contour_layers() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_rectangle_layer();

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 4, 2).with_display_size_mm(80.0, 40.0),
            &mut decoded,
        )
        .expect("decode pwszImg contour layer");

    assert_eq!(decoded, vec![0, 255, 255, 0, 0, 255, 255, 0]);
}

#[test]
fn anycubic_zip_layer_codec_decodes_pwsz_contour_holes() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_nested_rectangle_layer();

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 4, 4).with_display_size_mm(80.0, 80.0),
            &mut decoded,
        )
        .expect("decode pwszImg nested contour layer");

    assert_eq!(
        decoded,
        vec![
            255, 255, 255, 255, //
            255, 0, 0, 255, //
            255, 0, 0, 255, //
            255, 255, 255, 255,
        ]
    );
}

#[test]
fn anycubic_zip_layer_codec_unions_overlapping_pwsz_positive_contours() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_overlapping_positive_rectangles_layer();

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 6, 4).with_display_size_mm(60.0, 40.0),
            &mut decoded,
        )
        .expect("decode overlapping pwszImg positive contour records");

    assert_eq!(decoded, vec![255; 24]);
}

#[test]
fn anycubic_zip_layer_codec_decodes_pwsz_interleaved_contour_records() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_interleaved_nested_rectangle_layer();

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 4, 4).with_display_size_mm(80.0, 80.0),
            &mut decoded,
        )
        .expect("decode interleaved pwszImg contour records");

    assert_eq!(
        decoded,
        vec![
            255, 255, 255, 255, //
            255, 0, 0, 255, //
            255, 0, 0, 255, //
            255, 255, 255, 255,
        ]
    );
}

#[test]
fn anycubic_zip_layer_codec_decodes_pwsz_single_point_contours() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_single_point_layer(0.0, 0.0);

    let mut decoded = Vec::new();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 4, 4).with_display_size_mm(80.0, 80.0),
            &mut decoded,
        )
        .expect("decode pwszImg single point layer");

    assert_eq!(
        decoded,
        vec![
            0, 0, 0, 0, //
            0, 0, 0, 0, //
            0, 0, 255, 0, //
            0, 0, 0, 0,
        ]
    );
}

#[test]
fn anycubic_zip_layer_codec_decodes_dense_pwsz_contours_quickly() {
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let encoded = pwsz_dense_rectangle_layer(4096);

    let mut decoded = Vec::new();
    let started_at = Instant::now();
    spec.layer_codec
        .decode_layer_into(
            &encoded,
            &LayerDecodeContext::new(0, 512, 256).with_display_size_mm(80.0, 40.0),
            &mut decoded,
        )
        .expect("decode dense pwszImg contour layer");
    let elapsed = started_at.elapsed();

    assert!(
        elapsed.as_millis() < 1_000,
        "dense pwsz contour decode took {elapsed:?}"
    );
    assert_eq!(decoded[128 * 512 + 256], 255);
    assert_eq!(decoded[0], 0);
}

#[derive(Debug, Clone, Copy)]
struct SceneLayerMetric {
    area_mm2: f32,
}

fn configured_real_pwszimg_fixture_path() -> Option<PathBuf> {
    for key in ["SC_REAL_ANYCUBIC_PWSZIMG_PATH", "SC_REAL_PWSZIMG_PATH"] {
        if let Ok(value) = std::env::var(key) {
            return Some(PathBuf::from(value));
        }
    }

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust crate should live below repo root")
        .join("dist/tmp/public-fixtures/uvtools-970-pm7/extracted/Hex/Hex.pm7");
    default_path.exists().then_some(default_path)
}

fn zip_entry_bytes(data: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).expect("zip archive");
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap_or_else(|error| panic!("{name} should exist: {error}"))
        .read_to_end(&mut bytes)
        .expect("zip entry should be readable");
    bytes
}

fn parse_scene_layer_metrics(data: &[u8]) -> Vec<SceneLayerMetric> {
    let mut index = 0usize;
    index += 16;
    index += 64;
    index += 4 * 4;
    index += 4;
    let _layer_count = read_le_u32_at(data, &mut index, "scene layer count");
    index += 6 * 4;
    index += 4;
    index += 64 * 4;
    assert_eq!(&data[index..index + 4], b"<---", "scene layer marker");
    index += 4;
    let layer_def_count = read_le_u32_at(data, &mut index, "scene layer def count");
    let mut layers = Vec::with_capacity(layer_def_count as usize);
    for _ in 0..layer_def_count {
        let _height = read_le_f32_at(data, &mut index, "scene layer height");
        let area_mm2 = read_le_f32_at(data, &mut index, "scene layer area");
        index += 4 * 4;
        let _object_count = read_le_u32_at(data, &mut index, "scene object count");
        let _max_contour_area = read_le_f32_at(data, &mut index, "scene max contour area");
        index += 8 * 4;
        layers.push(SceneLayerMetric { area_mm2 });
    }
    assert_eq!(&data[index..index + 4], b"--->", "scene end marker");
    layers
}

#[test]
fn anycubic_zip_real_pwszimg_decode_area_tracks_scene_manifest_when_configured() {
    let Some(input) = configured_real_pwszimg_fixture_path() else {
        eprintln!("skipping real pwszImg scene-area comparison: set SC_REAL_ANYCUBIC_PWSZIMG_PATH");
        return;
    };
    assert!(
        input.exists(),
        "configured real pwszImg fixture should exist: {}",
        input.display()
    );

    let data = std::fs::read(&input).expect("real pwszImg fixture should be readable");
    let registry = formats::global_registry();
    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    let parsed = spec
        .pipeline
        .parse_document(spec, &data)
        .expect("real Anycubic Zip document");
    let scene = parse_scene_layer_metrics(&zip_entry_bytes(&data, "scene.slice"));

    let pixel_area_mm2 = parsed.metadata.printer.display_width_mm
        * parsed.metadata.printer.display_height_mm
        / (parsed.metadata.printer.resolution_x * parsed.metadata.printer.resolution_y) as f32;
    assert!(pixel_area_mm2 > 0.0, "pixel area should be available");

    let mut candidate_indices = scene
        .iter()
        .enumerate()
        .filter_map(|(index, metric)| (metric.area_mm2 > pixel_area_mm2).then_some(index))
        .collect::<Vec<_>>();
    assert!(
        candidate_indices.len() >= 3,
        "real pwszImg fixture should have enough non-empty layers"
    );
    candidate_indices.sort_unstable();
    let sample_positions = [
        0,
        candidate_indices.len() / 3,
        candidate_indices.len() * 2 / 3,
        candidate_indices.len() - 1,
    ];
    let sample_indices = sample_positions
        .into_iter()
        .map(|position| candidate_indices[position])
        .collect::<Vec<_>>();

    let mut archive = zip::ZipArchive::new(Cursor::new(&data)).expect("zip archive");
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    for layer_index in sample_indices {
        let layer_offset = parsed.layers[layer_index].offset as usize;
        archive
            .by_index(layer_offset)
            .unwrap_or_else(|error| panic!("layer {layer_index} entry should exist: {error}"))
            .read_to_end(&mut encoded)
            .expect("layer entry should be readable");
        spec.layer_codec
            .decode_layer_into(
                &encoded,
                &LayerDecodeContext::new(
                    layer_index as u32,
                    parsed.metadata.printer.resolution_x,
                    parsed.metadata.printer.resolution_y,
                )
                .with_display_size_mm(
                    parsed.metadata.printer.display_width_mm,
                    parsed.metadata.printer.display_height_mm,
                ),
                &mut decoded,
            )
            .unwrap_or_else(|error| panic!("layer {layer_index} should decode: {error:?}"));
        let decoded_area_mm2 =
            decoded.iter().filter(|&&pixel| pixel > 0).count() as f32 * pixel_area_mm2;
        let expected_area_mm2 = scene[layer_index].area_mm2;
        let relative_error =
            (decoded_area_mm2 - expected_area_mm2).abs() / expected_area_mm2.max(pixel_area_mm2);
        assert!(
            relative_error <= 0.05,
            "layer {layer_index} area mismatch: decoded={decoded_area_mm2:.3}mm^2 scene={expected_area_mm2:.3}mm^2 relative_error={relative_error:.3}"
        );
        encoded.clear();
    }
}

fn zip_fixture(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, bytes) in entries {
        zip.start_file(name, options).expect("start zip entry");
        zip.write_all(&bytes).expect("write zip entry");
    }

    zip.finish().expect("finish zip").into_inner()
}

fn png_preview(pixels: &[u8]) -> Vec<u8> {
    sl1::encode_layer_data(pixels, 4, 2).expect("png preview")
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

fn anycubic_zip_layers_json() -> &'static str {
    r#"{
  "paras": [
    {
      "exposure_time": 2.2,
      "layer_index": 0,
      "layer_minheight": 0.0,
      "layer_thickness": 0.05,
      "zup_height": 5.0,
      "zup_speed": 6.0
    },
    {
      "exposure_time": 2.4,
      "layer_index": 1,
      "layer_minheight": 0.05,
      "layer_thickness": 0.05,
      "zup_height": 5.5,
      "zup_speed": 6.5
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

fn anycubic_zip_scene_manifest() -> Vec<u8> {
    let mut bytes = Vec::new();
    fixed_string(&mut bytes, "ANYCUBIC-PWSZ", 16);
    fixed_string(&mut bytes, "slicer-converter-test", 64);
    for value in [3u32, 1, 0, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&1.0f32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    for value in [-20.0f32, -20.0, 0.0, 20.0, 20.0, 0.14] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for _ in 0..64 {
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes.extend_from_slice(b"<---");
    bytes.extend_from_slice(&2u32.to_le_bytes());
    scene_layer_def(&mut bytes, 0.07, 10.0);
    scene_layer_def(&mut bytes, 0.14, 20.0);
    bytes.extend_from_slice(b"--->");
    bytes
}

fn scene_layer_def(bytes: &mut Vec<u8>, height: f32, area: f32) {
    for value in [height, area, -20.0, -20.0, 20.0, 20.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&area.to_le_bytes());
    for _ in 0..8 {
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
}

fn anycubic_zip_calc_layer_volumes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x20251024u32.to_le_bytes());
    bytes.extend_from_slice(&0.2f32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    calc_layer_volume_entry(&mut bytes, 0, 0.0, 0.1, 1000.0);
    calc_layer_volume_entry(&mut bytes, 1, 0.1, 0.1, 2000.0);
    bytes
}

fn calc_layer_volume_entry(
    bytes: &mut Vec<u8>,
    layer_ref: u32,
    z: f32,
    thickness: f32,
    raw_volume: f32,
) {
    bytes.extend_from_slice(&layer_ref.to_le_bytes());
    bytes.extend_from_slice(&z.to_le_bytes());
    bytes.extend_from_slice(&thickness.to_le_bytes());
    for _ in 0..8 {
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes.extend_from_slice(&raw_volume.to_le_bytes());
}

fn fixed_string(bytes: &mut Vec<u8>, value: &str, len: usize) {
    let raw = value.as_bytes();
    let copy_len = raw.len().min(len.saturating_sub(1));
    bytes.extend_from_slice(&raw[..copy_len]);
    bytes.resize(bytes.len() + len - copy_len, 0);
}

fn pwsz_rectangle_layer() -> Vec<u8> {
    pwsz_layer_from_rectangles(&[(-20.0, -20.0, 20.0, 20.0)])
}

fn pwsz_nested_rectangle_layer() -> Vec<u8> {
    pwsz_layer_from_rectangles(&[(-40.0, -40.0, 40.0, 40.0), (-20.0, -20.0, 20.0, 20.0)])
}

fn pwsz_interleaved_nested_rectangle_layer() -> Vec<u8> {
    let outer = pwsz_rectangle_segments(-40.0, -40.0, 40.0, 40.0);
    let inner = pwsz_rectangle_segments(-20.0, -20.0, 20.0, 20.0);
    let mut records = Vec::new();
    for index in 0..4 {
        records.push(outer[index]);
        records.push(inner[index]);
    }
    pwsz_layer_from_records(2, &records)
}

fn pwsz_overlapping_positive_rectangles_layer() -> Vec<u8> {
    pwsz_layer_from_rectangles(&[(-30.0, -20.0, 10.0, 20.0), (-10.0, -20.0, 30.0, 20.0)])
}

fn pwsz_dense_rectangle_layer(segments_per_edge: u32) -> Vec<u8> {
    let mut records = Vec::new();
    let (x_min, y_min, x_max, y_max) = (-20.0f32, -10.0f32, 20.0f32, 10.0f32);
    for step in 0..segments_per_edge {
        let t0 = step as f32 / segments_per_edge as f32;
        let t1 = (step + 1) as f32 / segments_per_edge as f32;
        records.push((
            x_min + (x_max - x_min) * t0,
            y_min,
            x_min + (x_max - x_min) * t1,
            y_min,
        ));
        records.push((
            x_max,
            y_min + (y_max - y_min) * t0,
            x_max,
            y_min + (y_max - y_min) * t1,
        ));
        records.push((
            x_max - (x_max - x_min) * t0,
            y_max,
            x_max - (x_max - x_min) * t1,
            y_max,
        ));
        records.push((
            x_min,
            y_max - (y_max - y_min) * t0,
            x_min,
            y_max - (y_max - y_min) * t1,
        ));
    }
    pwsz_layer_from_records(1, &records)
}

fn pwsz_single_point_layer(x: f32, y: f32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"{==\0");
    for value in [0.0f32, x, y, x, y] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(b"[--\0");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    for value in [x, y, x, y] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.push(1);
    bytes.extend_from_slice(b"--]\0");
    bytes.extend_from_slice(b"==}\0");
    bytes
}

fn pwsz_layer_from_rectangles(rectangles: &[(f32, f32, f32, f32)]) -> Vec<u8> {
    let records = rectangles
        .iter()
        .flat_map(|&(x_min, y_min, x_max, y_max)| {
            pwsz_rectangle_segments(x_min, y_min, x_max, y_max)
        })
        .collect::<Vec<_>>();
    pwsz_layer_from_records(rectangles.len() as u32, &records)
}

fn pwsz_rectangle_segments(
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
) -> [(f32, f32, f32, f32); 4] {
    [
        (x_min, y_min, x_max, y_min),
        (x_max, y_min, x_max, y_max),
        (x_max, y_max, x_min, y_max),
        (x_min, y_max, x_min, y_min),
    ]
}

fn pwsz_layer_from_records(object_count: u32, records: &[(f32, f32, f32, f32)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"{==\0");
    for value in [1600.0f32, -40.0, -40.0, 40.0, 40.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&object_count.to_le_bytes());
    bytes.extend_from_slice(b"[--\0");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    for &(start_x, start_y, end_x, end_y) in records {
        for value in [start_x, start_y, end_x, end_y] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(1);
    }
    bytes.extend_from_slice(b"--]\0");
    bytes.extend_from_slice(b"==}\0");
    bytes
}
