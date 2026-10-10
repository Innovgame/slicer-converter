//! Shared export diagnostics.
//!
//! Family modules keep their own mismatch rules. This module is the single
//! entry that WASM and the CLI call before writing a target file.
//!
//! 共享导出诊断。各 family 保留自己的 mismatch 规则；WASM 与 CLI 在写出目标文件前
//! 都走这里的统一入口。

use crate::common::FileMetadata;

use super::{
    anycubic::diagnostics::{
        diagnose_anycubic_export, ExportDiagnosticReport, ExportDiagnosticWarning,
        ExportMachineSnapshot,
    },
    chitubox::photons,
    common::{FormatFamily, FormatSpec},
};

const DISPLAY_SIZE_TOLERANCE_MM: f32 = 0.05;

/// Diagnose non-fatal export issues for a resolved target profile.
///
/// Warnings do not block export. Callers should show them before conversion
/// starts so the user can see geometry changes.
///
/// 诊断已解析目标 profile 的非致命导出问题。warning 不阻止导出。调用方应在转换开始前
/// 展示这些 warning，让用户看到几何变化。
pub fn diagnose_export(
    metadata: &FileMetadata,
    target_spec: &'static FormatSpec,
) -> ExportDiagnosticReport {
    if target_spec.identity.name == "photons" {
        return diagnose_photons_export(metadata, target_spec);
    }
    if target_spec.family == FormatFamily::Anycubic {
        return diagnose_anycubic_export(metadata, target_spec);
    }
    ExportDiagnosticReport {
        warnings: Vec::new(),
    }
}

fn diagnose_photons_export(
    metadata: &FileMetadata,
    target_spec: &'static FormatSpec,
) -> ExportDiagnosticReport {
    let (resolution_x, resolution_y, display_width_mm, display_height_mm, display_name) =
        photons::export_machine_geometry();
    let resolution_matches = metadata.printer.resolution_x == resolution_x
        && metadata.printer.resolution_y == resolution_y;
    let display_size_matches = nearly_equal(metadata.printer.display_width_mm, display_width_mm)
        && nearly_equal(metadata.printer.display_height_mm, display_height_mm);
    if resolution_matches && display_size_matches {
        return ExportDiagnosticReport {
            warnings: Vec::new(),
        };
    }

    let source_machine_name = metadata.printer.machine_name.trim();
    let source_machine_display_name =
        (!source_machine_name.is_empty()).then(|| source_machine_name.to_string());
    let message = if resolution_matches {
        format!(
            "Warning: PhotonS export keeps the source layer pixels but rewrites the bed size from {}x{} mm to {}x{} mm. UVTools may report a changed image ratio.",
            format_mm(metadata.printer.display_width_mm),
            format_mm(metadata.printer.display_height_mm),
            format_mm(display_width_mm),
            format_mm(display_height_mm),
        )
    } else {
        format!(
            "Warning: PhotonS export resamples layers from {}x{} to {}x{} and rewrites the bed size to {}x{} mm. Source bed is {}x{} mm. Nearest-neighbor resampling can change edges and printed proportions.",
            metadata.printer.resolution_x,
            metadata.printer.resolution_y,
            resolution_x,
            resolution_y,
            format_mm(display_width_mm),
            format_mm(display_height_mm),
            format_mm(metadata.printer.display_width_mm),
            format_mm(metadata.printer.display_height_mm),
        )
    };

    ExportDiagnosticReport {
        warnings: vec![ExportDiagnosticWarning {
            code: "PHOTONS_MACHINE_GEOMETRY_REWRITE",
            severity: "warning",
            message,
            source: ExportMachineSnapshot {
                format: None,
                display_name: source_machine_display_name,
                extension: None,
                resolution_x: metadata.printer.resolution_x,
                resolution_y: metadata.printer.resolution_y,
                display_width_mm: metadata.printer.display_width_mm,
                display_height_mm: metadata.printer.display_height_mm,
            },
            target: ExportMachineSnapshot {
                format: Some(target_spec.identity.name.to_string()),
                display_name: Some(display_name.to_string()),
                extension: Some("photons".to_string()),
                resolution_x,
                resolution_y,
                display_width_mm,
                display_height_mm,
            },
        }],
    }
}

fn nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= DISPLAY_SIZE_TOLERANCE_MM
}

fn format_mm(value: f32) -> String {
    if value.is_finite() {
        let rendered = format!("{value:.3}");
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        "?".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::diagnose_export;
    use crate::formats;

    #[test]
    fn photons_export_warns_when_source_resolution_differs() {
        let metadata = crate::common::FileMetadata::builder()
            .resolution(64, 64)
            .display_size_mm(68.04, 120.96)
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()
            .expect("metadata");
        let spec = formats::get_format_spec_by_name("photons").expect("photons spec");

        let report = diagnose_export(&metadata, spec);

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "PHOTONS_MACHINE_GEOMETRY_REWRITE");
        assert!(report.warnings[0]
            .message
            .contains("resamples layers from 64x64 to 1440x2560"));
    }

    #[test]
    fn photons_export_is_quiet_for_native_geometry() {
        let metadata = crate::common::FileMetadata::builder()
            .resolution(1440, 2560)
            .display_size_mm(68.04, 120.96)
            .layer_height_mm(0.05)
            .layer_count(1)
            .build()
            .expect("metadata");
        let spec = formats::get_format_spec_by_name("photons").expect("photons spec");

        let report = diagnose_export(&metadata, spec);

        assert!(report.warnings.is_empty());
    }
}
