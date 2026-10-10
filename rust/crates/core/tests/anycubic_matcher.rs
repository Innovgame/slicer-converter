use slicer_converter_core::common::{ConvertError, FileMetadataBuilder};
use slicer_converter_core::formats;
use slicer_converter_core::formats::anycubic::AnycubicMachineId;
use slicer_converter_core::formats::common::ExportTargetOptions;

#[test]
fn matcher_uses_machine_name_hint_within_exact_resolution_candidates() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .machine_name("Anycubic Photon Mono M5s")
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec("anycubic", Some(&metadata), ExportTargetOptions::default())
        .expect("profile");
    let specifics = formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("profile facts")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(
        specifics.machine.machine_id,
        AnycubicMachineId::PhotonMonoM5s
    );
}

#[test]
fn matcher_rejects_non_anycubic_resolution_without_misdetecting_pm4n() {
    let metadata = FileMetadataBuilder::new()
        .resolution(8520, 4320)
        .display_size_mm(153.36, 77.76)
        .machine_name("ELEGOO MARS 4 Ultra")
        .build()
        .expect("valid fixture");

    let error = match formats::global_registry().resolve_export_spec(
        "anycubic",
        Some(&metadata),
        ExportTargetOptions::default(),
    ) {
        Ok(spec) => panic!("expected no matching profile, got {}", spec.identity.name),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ConvertError::NoMatchingAnycubicProfile { .. }
    ));
}

#[test]
fn matcher_rejects_ambiguous_profiles_when_metadata_cannot_disambiguate() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .build()
        .expect("valid fixture");

    let error = match formats::global_registry().resolve_export_spec(
        "anycubic",
        Some(&metadata),
        ExportTargetOptions::default(),
    ) {
        Ok(spec) => panic!("expected ambiguous profile, got {}", spec.identity.name),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ConvertError::AmbiguousAnycubicProfile { .. }
    ));
}

#[test]
fn export_plan_returns_v518_plan_for_supported_modern_profile() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .machine_name("Anycubic Photon Mono M5s")
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec("anycubic", Some(&metadata), ExportTargetOptions::default())
        .expect("pm5s should be exportable through v518");
    assert_eq!(spec.identity.name, "pm5s");
    let specifics = formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("profile facts")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(specifics.extension, "pm5s");
    assert_eq!(specifics.default_version.as_str(), "v518");
    assert_eq!(specifics.default_version.as_u32(), 518);
}

#[test]
fn export_plan_returns_v1_plan_for_supported_legacy_profile() {
    let metadata = FileMetadataBuilder::new()
        .resolution(1440, 2560)
        .display_size_mm(68.04, 120.96)
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec("anycubic", Some(&metadata), ExportTargetOptions::default())
        .expect("pws should be exportable through v1");
    assert_eq!(spec.identity.name, "pws");
    let specifics = formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("profile facts")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(specifics.extension, "pws");
    assert_eq!(specifics.default_version.as_str(), "v1");
    assert_eq!(specifics.default_version.as_u32(), 1);
}

#[test]
fn export_plan_returns_v516_plan_for_supported_modern_profile() {
    let metadata = FileMetadataBuilder::new()
        .resolution(1620, 2560)
        .display_size_mm(82.62, 130.56)
        .machine_name("Anycubic Photon Mono")
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec("anycubic", Some(&metadata), ExportTargetOptions::default())
        .expect("pwmo should be exportable through v516");
    assert_eq!(spec.identity.name, "pwmo");
    let specifics = formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("profile facts")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(specifics.extension, "pwmo");
    assert_eq!(specifics.default_version.as_str(), "v516");
    assert_eq!(specifics.default_version.as_u32(), 516);
}

#[test]
fn broad_anycubic_target_accepts_supported_explicit_version_after_metadata_match() {
    let metadata = FileMetadataBuilder::new()
        .resolution(1620, 2560)
        .display_size_mm(82.62, 130.56)
        .machine_name("Anycubic Photon Mono")
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec(
            "anycubic",
            Some(&metadata),
            ExportTargetOptions::default().with_format_version(516),
        )
        .expect("pwmo should accept supported explicit v516");
    assert_eq!(spec.identity.name, "pwmo");
}

#[test]
fn broad_anycubic_target_rejects_unsupported_explicit_version_after_metadata_match() {
    let metadata = FileMetadataBuilder::new()
        .resolution(1620, 2560)
        .display_size_mm(82.62, 130.56)
        .machine_name("Anycubic Photon Mono")
        .build()
        .expect("valid fixture");

    let error = match formats::global_registry().resolve_export_spec(
        "anycubic",
        Some(&metadata),
        ExportTargetOptions::default().with_format_version(517),
    ) {
        Ok(spec) => panic!("pwmo should reject v517, got {}", spec.identity.name),
        Err(error) => error,
    };

    let structured = error.to_structured();
    assert_eq!(structured.code, "E_UNSUPPORTED_FORMAT_VERSION");
    assert!(structured
        .message
        .contains("Anycubic target pwmo does not support version 517"));
    assert!(structured.message.contains("1, 515, 516"));
}

#[test]
fn export_plan_returns_v517_plan_for_pm4n() {
    let metadata = FileMetadataBuilder::new()
        .resolution(9024, 5120)
        .display_size_mm(153.408, 87.04)
        .machine_name("Anycubic Photon Mono 4 Ultra")
        .build()
        .expect("valid fixture");

    let spec = formats::global_registry()
        .resolve_export_spec("anycubic", Some(&metadata), ExportTargetOptions::default())
        .expect("pm4n should be exportable through v517");
    assert_eq!(spec.identity.name, "pm4n");
    let specifics = formats::global_registry()
        .catalog_profile_for_spec(spec)
        .expect("profile facts")
        .as_anycubic()
        .expect("anycubic specifics");
    assert_eq!(specifics.extension, "pm4n");
    assert_eq!(specifics.default_version.as_str(), "v517");
    assert_eq!(specifics.default_version.as_u32(), 517);
}

#[test]
fn direct_anycubic_suffix_accepts_supported_explicit_versions() {
    let metadata = FileMetadataBuilder::new()
        .machine_name("Anycubic Photon Mono SE")
        .resolution(1620, 2560)
        .display_size_mm(82.62, 130.56)
        .machine_z_mm(160.0)
        .mirror(false, true)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .transition_layer_count(0)
        .anti_alias_level(1)
        .exposure(2.0, 30.0)
        .light_pwm(255, 255)
        .build()
        .expect("metadata");

    for version in [1, 515, 516] {
        let spec = formats::global_registry()
            .resolve_export_spec(
                "pwms",
                Some(&metadata),
                ExportTargetOptions::default().with_format_version(version),
            )
            .expect("pwms should accept supported version");
        assert_eq!(spec.identity.name, "pwms");
    }
}

#[test]
fn rejects_unsupported_anycubic_export_version_for_suffix() {
    let metadata = FileMetadataBuilder::new()
        .machine_name("Anycubic Photon Mono")
        .resolution(2560, 1620)
        .display_size_mm(120.0, 80.0)
        .machine_z_mm(165.0)
        .mirror(false, true)
        .layer_height_mm(0.05)
        .layer_count(1)
        .bottom_layer_count(1)
        .transition_layer_count(0)
        .anti_alias_level(1)
        .exposure(2.0, 30.0)
        .light_pwm(255, 255)
        .build()
        .expect("metadata");
    let error = match formats::global_registry().resolve_export_spec(
        "pwms",
        Some(&metadata),
        ExportTargetOptions::default().with_format_version(517),
    ) {
        Ok(spec) => panic!("pwms should reject v517, got {}", spec.identity.name),
        Err(error) => error,
    };

    let structured = error.to_structured();
    assert_eq!(structured.code, "E_UNSUPPORTED_FORMAT_VERSION");
    assert!(structured
        .message
        .contains("Anycubic target pwms does not support version 517"));
    assert!(structured.message.contains("1, 515, 516"));
}

#[test]
fn matcher_rejects_non_anycubic_resolution_before_explicit_version_check() {
    let metadata = FileMetadataBuilder::new()
        .resolution(8520, 4320)
        .display_size_mm(153.36, 77.76)
        .machine_name("ELEGOO MARS 4 Ultra")
        .build()
        .expect("valid fixture");

    let error = match formats::global_registry().resolve_export_spec(
        "anycubic",
        Some(&metadata),
        ExportTargetOptions::default().with_format_version(517),
    ) {
        Ok(spec) => panic!("expected no matching profile, got {}", spec.identity.name),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ConvertError::NoMatchingAnycubicProfile { .. }
    ));
}

#[test]
fn matcher_rejects_ambiguous_profiles_before_explicit_version_check() {
    let metadata = FileMetadataBuilder::new()
        .resolution(11520, 5120)
        .display_size_mm(218.88, 122.88)
        .build()
        .expect("valid fixture");

    let error = match formats::global_registry().resolve_export_spec(
        "anycubic",
        Some(&metadata),
        ExportTargetOptions::default().with_format_version(517),
    ) {
        Ok(spec) => panic!("expected ambiguous profile, got {}", spec.identity.name),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ConvertError::AmbiguousAnycubicProfile { .. }
    ));
}
