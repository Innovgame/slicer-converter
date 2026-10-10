use slicer_converter_core::formats::common::ZipEntryInfo;
use slicer_converter_core::formats::{
    self,
    catalog::CatalogProfile,
    common::{keys, ArchiveEncodeContext, ContainerKind, ExportTargetOptions, FormatFamily},
};

#[test]
fn global_registry_exposes_unified_family_specs() {
    let families = formats::global_registry().families();
    let names = families
        .iter()
        .map(|family| family.identity.name)
        .collect::<Vec<_>>();

    assert!(names.contains(&"chitubox"));
    assert!(names.contains(&"anycubic"));
    assert!(names.contains(&"goo"));
    assert!(names.contains(&"phrozen"));
}

#[test]
fn global_registry_all_specs_flattens_unified_families() {
    let registry = formats::global_registry();
    let all_specs: &'static [&'static slicer_converter_core::formats::common::FormatSpec] =
        registry.all_specs();
    let from_families = registry
        .families()
        .iter()
        .flat_map(|family| family.profiles.iter().copied())
        .collect::<Vec<_>>();

    assert_eq!(all_specs.len(), from_families.len());
    assert_eq!(
        all_specs
            .iter()
            .map(|spec| spec.identity.name)
            .collect::<Vec<_>>(),
        from_families
            .iter()
            .map(|spec| spec.identity.name)
            .collect::<Vec<_>>()
    );
}

#[test]
fn global_registry_catalog_profiles_project_typed_family_profiles() {
    let registry = formats::global_registry();
    let all_specs = registry.all_specs();
    let all_profiles = registry.catalog_profiles();

    assert_eq!(all_profiles.len(), all_specs.len());
    assert_eq!(
        all_profiles
            .iter()
            .map(|profile| profile.spec().identity.name)
            .collect::<Vec<_>>(),
        all_specs
            .iter()
            .map(|spec| spec.identity.name)
            .collect::<Vec<_>>()
    );

    let anycubic_zip = all_profiles
        .iter()
        .copied()
        .find(|profile| profile.spec().identity.name == "anycubic-zip")
        .expect("anycubic zip profile");
    let anycubic_archive = anycubic_zip
        .as_anycubic_archive()
        .expect("Anycubic Zip should publish archive facts through catalog projection");
    assert_eq!(anycubic_archive.layer_entry_prefix, "layer_images/layer_");

    let ctb = all_profiles
        .iter()
        .copied()
        .find(|profile| profile.spec().identity.name == "ctb")
        .expect("ctb profile");
    let chitubox = ctb
        .as_chitubox()
        .expect("CTB should publish Chitubox facts through catalog projection");
    assert_eq!(chitubox.ctb_version, Some(2));
}

#[test]
fn unified_registry_keeps_existing_name_lookup() {
    assert_eq!(
        formats::global_registry()
            .spec_by_name("ctb")
            .expect("ctb")
            .identity
            .name,
        "ctb"
    );
    assert_eq!(
        formats::global_registry()
            .spec_by_name("pw0")
            .expect("pw0")
            .identity
            .name,
        "pw0"
    );
}

#[test]
fn chitubox_specs_publish_typed_specifics() {
    let registry = slicer_converter_core::formats::global_registry();

    let ctb_v4 = registry.spec_by_name("ctb-v4").expect("ctb-v4");
    let specifics = registry
        .catalog_profile_for_spec(ctb_v4)
        .expect("ctb-v4 profile")
        .as_chitubox()
        .expect("chitubox specifics");
    assert_eq!(specifics.ctb_version, Some(4));
    assert_eq!(specifics.machine_profile, None);
    assert_eq!(specifics.layer_table_entry_size, Some(84));

    let gktwo = registry.spec_by_name("gktwo-ctb").expect("gktwo");
    let specifics = registry
        .catalog_profile_for_spec(gktwo)
        .expect("gktwo profile")
        .as_chitubox()
        .expect("gktwo specifics");
    assert_eq!(specifics.machine_profile, Some("gktwo"));
}

#[test]
fn anycubic_specs_publish_typed_specifics_for_machine_and_archive_profiles() {
    let registry = slicer_converter_core::formats::global_registry();

    for spec in registry
        .all_specs()
        .iter()
        .copied()
        .filter(|spec| spec.family == FormatFamily::Anycubic)
    {
        if spec.identity.name == "anycubic-zip" {
            assert_eq!(spec.container, ContainerKind::Archive);
            let specifics = registry
                .catalog_profile_for_spec(spec)
                .expect("Anycubic Zip profile")
                .as_anycubic_archive()
                .expect("Anycubic Zip archive specifics");
            assert_eq!(
                specifics.settings_manifest_entry,
                "anycubic_photon_resins.pwsp"
            );
            assert_eq!(specifics.layers_manifest_entry, "layers_controller.conf");
            assert_eq!(specifics.layer_entry_prefix, "layer_images/layer_");
            assert_eq!(specifics.default_layer_image_extension, "pw0Img");
            assert!(specifics.layer_image_extensions.contains(&"pwszImg"));
            assert!(specifics
                .required_manifest_entries
                .contains(&"anycubic_photon_resins.pwsp"));
            continue;
        }

        assert_eq!(spec.container, ContainerKind::Binary);
        let specifics = registry
            .catalog_profile_for_spec(spec)
            .expect("Anycubic profile")
            .as_anycubic()
            .unwrap_or_else(|| {
                panic!(
                    "{} should publish Anycubic machine specifics",
                    spec.identity.name
                )
            });
        assert_eq!(specifics.extension, spec.identity.name);
        assert!(specifics.machine.resolution_x > 0);
        assert!(specifics.machine.resolution_y > 0);
    }
}

#[test]
fn anycubic_archive_profile_resolves_photon_workshop_extension_matrix() {
    let registry = slicer_converter_core::formats::global_registry();
    let archive_targets = ["anycubic-zip", "pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"];
    let archive_extensions = ["pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"];

    let spec = registry
        .spec_by_name("anycubic-zip")
        .expect("Anycubic Zip profile");
    assert_eq!(spec.identity.name, "anycubic-zip");
    assert_eq!(spec.catalog.target_names, archive_targets.as_slice());
    assert_eq!(spec.identity.extensions, archive_extensions.as_slice());

    for name in archive_targets {
        let resolved = registry
            .spec_by_name(name)
            .unwrap_or_else(|| panic!("{name} should resolve as an Anycubic archive target"));
        assert_eq!(resolved.identity.name, "anycubic-zip");
    }

    for extension in archive_extensions {
        let resolved = registry.spec_by_extension(extension).unwrap_or_else(|| {
            panic!("{extension} should resolve as an Anycubic archive extension")
        });
        assert_eq!(resolved.identity.name, "anycubic-zip");
    }

    let specifics = registry
        .catalog_profile_for_spec(spec)
        .expect("Anycubic Zip catalog profile")
        .as_anycubic_archive()
        .expect("Anycubic Zip archive specifics");
    assert_eq!(
        specifics.layer_image_extensions,
        ["pw0Img", "pwszImg"].as_slice()
    );
    assert_eq!(
        specifics.required_manifest_entries,
        ["anycubic_photon_resins.pwsp", "layers_controller.conf"].as_slice()
    );
}

#[test]
fn anycubic_export_resolver_reads_machine_facts_from_family_profiles() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resolver_rs =
        std::fs::read_to_string(manifest_dir.join("src/formats/anycubic/resolver.rs"))
            .expect("read anycubic resolver");

    assert!(
        resolver_rs.contains("specs::ANYCUBIC_PROFILES"),
        "Anycubic export resolution should read typed Anycubic family profiles"
    );
    assert!(
        resolver_rs.contains("profile.facts"),
        "Anycubic export resolution should select machine facts from FamilyProfile<AnycubicSpec>"
    );
    assert!(
        !resolver_rs.contains(&format!("{}::Anycubic", concat!("Format", "Facts"))),
        "Anycubic resolver should not read catalog projection facts"
    );
}

#[test]
fn global_registry_resolves_export_targets_through_family_resolvers() {
    let registry = formats::global_registry();

    let spec = registry
        .resolve_export_spec("model.v4.ctb", None, ExportTargetOptions::default())
        .expect("ctb v4 export profile");
    assert_eq!(spec.identity.name, "ctb-v4");

    let spec = registry
        .resolve_export_spec("model.gktwo.ctb", None, ExportTargetOptions::default())
        .expect("GKtwo export profile");
    assert_eq!(spec.identity.name, "gktwo-ctb");

    let spec = registry
        .resolve_export_spec("model.pws", None, ExportTargetOptions::default())
        .expect("PWS export profile");
    assert_eq!(spec.identity.name, "pws");
}

#[test]
fn global_registry_rejects_import_only_profiles_as_export_targets() {
    let registry = formats::global_registry();

    let target = "cxdlp";
    let err = match registry.resolve_export_spec(target, None, ExportTargetOptions::default()) {
        Ok(spec) => panic!(
            "Creality {target} should not resolve as an export target, got {}",
            spec.identity.name
        ),
        Err(error) => error,
    };

    assert_eq!(err.to_string(), "Unknown file format");

    let export_spec = registry
        .resolve_export_spec("cxdlpv4", None, ExportTargetOptions::default())
        .expect("CXDLPv4 should resolve as an export target");
    assert_eq!(export_spec.identity.name, "cxdlpv4");

    for extension in ["cxdlp", "cxdlpv4"] {
        let import_spec = registry
            .spec_by_extension(extension)
            .unwrap_or_else(|| panic!("{extension} should remain import-visible"));
        assert_eq!(import_spec.identity.name, extension);
    }
}

#[test]
fn uvtools_archive_zip_specs_expose_entry_contracts() {
    let registry = formats::global_registry();

    let chitubox_zip = registry
        .spec_by_name("chitubox-zip")
        .expect("chitubox zip spec");
    assert_eq!(chitubox_zip.family, FormatFamily::Chitubox);
    assert_eq!(chitubox_zip.container, ContainerKind::Archive);
    assert!(chitubox_zip.capabilities.can_import);
    assert!(chitubox_zip.capabilities.can_export);
    assert_eq!(chitubox_zip.keys.header_key, keys::HEADER_CHITUBOX_ZIP);
    assert_eq!(chitubox_zip.keys.layer_codec_key, keys::PNG_GRAYSCALE);
    assert_eq!(chitubox_zip.keys.archive_key, keys::ZIP_ARCHIVE);
    assert_eq!(
        chitubox_zip
            .archive
            .describe_layer_entry(&ArchiveEncodeContext::new("ignored"), 0, chitubox_zip)
            .name,
        "1.png"
    );
    let entries = vec![
        zip_entry(0, "run.gcode", 5),
        zip_entry(1, "preview.png", 10),
        zip_entry(2, "1.png", 11),
        zip_entry(3, "2.png", 12),
    ];
    let layers = chitubox_zip
        .archive
        .list_layer_entries(&entries, chitubox_zip)
        .expect("chitubox zip layers");
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].offset, 2);
    assert_eq!(layers[1].offset, 3);

    let anycubic_zip = registry
        .spec_by_name("anycubic-zip")
        .expect("anycubic zip spec");
    assert_eq!(anycubic_zip.family, FormatFamily::Anycubic);
    assert_eq!(anycubic_zip.container, ContainerKind::Archive);
    assert!(anycubic_zip.capabilities.can_import);
    assert!(anycubic_zip.capabilities.can_export);
    assert_eq!(anycubic_zip.keys.header_key, keys::HEADER_ANYCUBIC_ZIP);
    assert_eq!(anycubic_zip.keys.layer_codec_key, keys::ANYCUBIC_ZIP_RLE);
    assert_eq!(anycubic_zip.keys.archive_key, keys::ZIP_ARCHIVE);
    assert_eq!(
        anycubic_zip
            .archive
            .describe_layer_entry(&ArchiveEncodeContext::new("ignored"), 0, anycubic_zip)
            .name,
        "layer_images/layer_0.pw0Img"
    );
    let entries = vec![
        zip_entry(0, "anycubic_photon_resins.pwsp", 5),
        zip_entry(1, "layers_controller.conf", 7),
        zip_entry(2, "layer_images/layer_0.pwszImg", 11),
        zip_entry(3, "layer_images/layer_1.pw0Img", 12),
    ];
    let layers = anycubic_zip
        .archive
        .list_layer_entries(&entries, anycubic_zip)
        .expect("anycubic zip layers");
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].offset, 2);
    assert_eq!(layers[1].offset, 3);
}

#[test]
fn uvtools_archive_zip_specs_detect_manifest_markers() {
    let registry = formats::global_registry();

    let chitubox_data = b"PK\x03\x04 run.gcode preview.png 1.png";
    let spec = registry
        .detect_spec(chitubox_data, Some("zip"))
        .expect("chitubox zip detection");
    assert_eq!(spec.identity.name, "chitubox-zip");

    let anycubic_data = b"PK\x03\x04 anycubic_photon_resins.pwsp layers_controller.conf layer_images/layer_0.pw0Img";
    let spec = registry
        .detect_spec(anycubic_data, Some("pm7"))
        .expect("anycubic zip detection");
    assert_eq!(spec.identity.name, "anycubic-zip");

    let by_extension = registry
        .spec_by_name("pwsz")
        .expect("anycubic zip extension alias");
    assert_eq!(by_extension.identity.name, "anycubic-zip");
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
fn unified_registry_can_detect_profile_without_changing_public_format_identity() {
    let ctb = include_bytes!("fixtures/sample.ctb");
    let spec = formats::global_registry()
        .detect_spec(ctb, Some("ctb"))
        .expect("ctb spec");

    assert_eq!(spec.family, FormatFamily::Chitubox);
    assert!(spec.catalog.profile_id.starts_with("chitubox-ctb"));

    let pw0 = include_bytes!("fixtures/sample.pw0");
    let spec = formats::global_registry()
        .detect_spec(pw0, Some("pw0"))
        .expect("pw0 spec");

    assert_eq!(spec.family, FormatFamily::Anycubic);
    assert_eq!(
        formats::global_registry()
            .catalog_profile_for_spec(spec)
            .expect("anycubic profile")
            .as_anycubic()
            .expect("anycubic specifics")
            .extension,
        "pw0"
    );
}

#[test]
fn registered_format_specs_publish_consistent_profile_metadata() {
    let registry = formats::global_registry();

    for spec in registry.all_specs() {
        assert_eq!(spec.keys.family_key, spec.family.as_key());
        assert!(!spec.catalog.profile_id.is_empty());
        assert!(!spec.catalog.display_name.is_empty());
        assert!(
            spec.keys.profile_key.0.starts_with("profile/"),
            "{} profile key should use the profile namespace",
            spec.identity.name
        );
        assert!(
            spec.catalog.target_names.contains(&spec.identity.name),
            "{} target names should include the canonical format id",
            spec.identity.name
        );
    }
}

#[test]
fn goo_family_specs_publish_shared_strategy_keys() {
    let registry = formats::global_registry();

    for name in ["goo", "prz"] {
        let spec = registry.spec_by_name(name).expect("GOO-family spec");
        assert_eq!(spec.keys.header_key, keys::HEADER_GOO_FAMILY);
        assert_eq!(spec.keys.layer_codec_key, keys::GOO_RLE);
        assert_eq!(spec.keys.footer_key, keys::GOO_FAMILY_FOOTER);
        assert_eq!(spec.keys.archive_key, keys::NO_ARCHIVE);
    }
}

#[test]
fn goo_prz_profiles_stay_generic_until_typed_family_facts_exist() {
    let registry = formats::global_registry();

    for name in ["goo", "prz"] {
        let spec = registry.spec_by_name(name).expect("GOO-family spec");
        let profile = registry
            .catalog_profile_for_spec(spec)
            .unwrap_or_else(|| panic!("{name} catalog profile"));

        match *profile {
            CatalogProfile::Generic(profile_spec) => {
                assert!(
                    std::ptr::eq(profile_spec, spec),
                    "{name} generic catalog profile should point at its FormatSpec"
                );
            }
            CatalogProfile::Chitubox(_)
            | CatalogProfile::Anycubic(_)
            | CatalogProfile::AnycubicArchive(_)
            | CatalogProfile::Creality(_) => {
                panic!("{name} should not borrow another family's typed facts")
            }
        }
    }
}

#[test]
fn creality_cxdlpv4_is_registered_as_independent_family() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("cxdlpv4").expect("cxdlpv4 spec");

    assert_eq!(spec.identity.name, "cxdlpv4");
    assert_eq!(spec.family, FormatFamily::Creality);
    assert_eq!(spec.container, ContainerKind::Binary);
    assert!(spec.capabilities.can_import);
    assert!(spec.capabilities.can_export);
    assert!(spec.capabilities.supports_preview);
    assert!(spec.capabilities.supports_per_layer_settings);
    assert!(spec.capabilities.supports_encryption);
    assert_eq!(spec.keys.family_key, keys::FAMILY_CREALITY);
    assert_eq!(spec.keys.profile_key.0, "profile/creality-cxdlpv4");
    assert_eq!(spec.keys.header_key, keys::HEADER_CREALITY_CXDLP_V4);
    assert_eq!(spec.keys.layer_codec_key, keys::CREALITY_CXDLP_RLE);

    let by_extension = registry
        .spec_by_extension("cxdlpv4")
        .expect("cxdlpv4 extension");
    assert_eq!(by_extension.identity.name, "cxdlpv4");
}

#[test]
fn creality_cxdlp_v3_is_registered_as_read_only_profile() {
    let registry = formats::global_registry();
    let spec = registry.spec_by_name("cxdlp").expect("cxdlp spec");

    assert_eq!(spec.identity.name, "cxdlp");
    assert_eq!(spec.family, FormatFamily::Creality);
    assert_eq!(spec.container, ContainerKind::Binary);
    assert!(spec.capabilities.can_import);
    assert!(!spec.capabilities.can_export);
    assert!(spec.capabilities.supports_preview);
    assert!(!spec.capabilities.supports_encryption);
    assert_eq!(spec.keys.family_key, keys::FAMILY_CREALITY);
    assert_eq!(spec.keys.header_key, keys::HEADER_CREALITY_CXDLP);
    assert_eq!(spec.keys.layer_codec_key, keys::CREALITY_CXDLP_LINES);

    let by_extension = registry
        .spec_by_extension("cxdlp")
        .expect("cxdlp extension");
    assert_eq!(by_extension.identity.name, "cxdlp");

    let v2 = registry.spec_by_name("cxdlp-v2").expect("cxdlp v2 spec");
    assert_eq!(v2.identity.name, "cxdlp-v2");
    assert_eq!(v2.family, FormatFamily::Creality);
    assert!(v2.capabilities.can_import);
    assert!(!v2.capabilities.can_export);
    assert_eq!(v2.keys.header_key, keys::HEADER_CREALITY_CXDLP);
    assert_eq!(v2.keys.layer_codec_key, keys::CREALITY_CXDLP_LINES);

    let v1 = registry.spec_by_name("cxdlp-v1").expect("cxdlp v1 spec");
    assert_eq!(v1.identity.name, "cxdlp-v1");
    assert_eq!(v1.family, FormatFamily::Creality);
    assert!(v1.capabilities.can_import);
    assert!(!v1.capabilities.can_export);
    assert_eq!(v1.catalog.default_extension, "v1.cxdlp");
    assert_eq!(v1.keys.header_key, keys::HEADER_CREALITY_CXDLP);
    assert_eq!(v1.keys.layer_codec_key, keys::CREALITY_CXDLP_LINES);

    let v1_by_extension = registry
        .spec_by_extension("v1.cxdlp")
        .expect("v1.cxdlp extension");
    assert_eq!(v1_by_extension.identity.name, "cxdlp-v1");
}

#[test]
fn unified_registry_disambiguates_goo_family_specs_by_hint() {
    let goo = include_bytes!("fixtures/sample.goo");

    let spec = formats::global_registry()
        .detect_spec(goo, Some("goo"))
        .expect("goo spec");
    assert_eq!(spec.family, FormatFamily::Goo);
    assert_eq!(spec.identity.name, "goo");

    let spec = formats::global_registry()
        .detect_spec(goo, Some("prz"))
        .expect("prz spec");
    assert_eq!(spec.family, FormatFamily::Phrozen);
    assert_eq!(spec.identity.name, "prz");
}
