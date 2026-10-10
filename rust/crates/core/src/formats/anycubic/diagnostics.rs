use serde::Serialize;

use crate::common::FileMetadata;
use crate::formats::common::FormatSpec;

use super::{specs, AnycubicSpec};

const DISPLAY_SIZE_TOLERANCE_MM: f32 = 0.05;

/// Export diagnostics for an Anycubic target.
///
/// Anycubic export diagnostics only warns about issues that can make external
/// validators report stretched output; it does not block export.
///
/// Anycubic 导出诊断。这里只报告可能导致外部工具提示模型拉伸的问题，不阻止导出。
#[derive(Debug, Serialize)]
pub struct ExportDiagnosticReport {
    pub warnings: Vec<ExportDiagnosticWarning>,
}

/// A non-fatal export warning.
///
/// 非致命导出 warning。
#[derive(Debug, Serialize)]
pub struct ExportDiagnosticWarning {
    pub code: &'static str,
    pub severity: &'static str,
    pub message: String,
    pub source: ExportMachineSnapshot,
    pub target: ExportMachineSnapshot,
}

/// Machine geometry snapshot included in diagnostics.
///
/// 诊断中携带的机器几何快照。
#[derive(Debug, Serialize)]
pub struct ExportMachineSnapshot {
    pub format: Option<String>,
    pub display_name: Option<String>,
    pub extension: Option<String>,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub display_width_mm: f32,
    pub display_height_mm: f32,
}

pub fn diagnose_anycubic_export(
    metadata: &FileMetadata,
    target_spec: &'static FormatSpec,
) -> ExportDiagnosticReport {
    let warnings = anycubic_facts_for_spec(target_spec)
        .and_then(|facts| machine_profile_mismatch_warning(metadata, target_spec, facts))
        .into_iter()
        .collect();

    ExportDiagnosticReport { warnings }
}

fn anycubic_facts_for_spec(spec: &'static FormatSpec) -> Option<&'static AnycubicSpec> {
    specs::ANYCUBIC_PROFILES
        .iter()
        .copied()
        .find(|profile| std::ptr::eq(profile.spec, spec))
        .map(|profile| profile.facts)
}

fn machine_profile_mismatch_warning(
    metadata: &FileMetadata,
    target_spec: &'static FormatSpec,
    facts: &AnycubicSpec,
) -> Option<ExportDiagnosticWarning> {
    let resolution_matches = metadata.printer.resolution_x == facts.machine.resolution_x
        && metadata.printer.resolution_y == facts.machine.resolution_y;
    let display_size_matches = nearly_equal(
        metadata.printer.display_width_mm,
        facts.machine.display_width_mm,
    ) && nearly_equal(
        metadata.printer.display_height_mm,
        facts.machine.display_height_mm,
    );

    if resolution_matches && display_size_matches {
        return None;
    }

    let source_machine_name = metadata.printer.machine_name.trim();
    let source_machine_display_name =
        (!source_machine_name.is_empty()).then(|| source_machine_name.to_string());
    let source_machine_label = source_machine_display_name
        .as_deref()
        .unwrap_or("unknown source machine");

    Some(ExportDiagnosticWarning {
        code: "ANYCUBIC_MACHINE_PROFILE_MISMATCH",
        severity: "warning",
        message: format!(
            "Warning: Source machine {source_machine_label} geometry differs from target machine {} ({}). Export continues without layer resampling. Source is {}x{} at {}x{} mm; target profile is {}x{} at {}x{} mm. UVTools may report non-square pixels or an incorrect image ratio, and the model may print with stretched proportions.",
            facts.machine.display_name,
            facts.extension.to_ascii_uppercase(),
            metadata.printer.resolution_x,
            metadata.printer.resolution_y,
            format_mm(metadata.printer.display_width_mm),
            format_mm(metadata.printer.display_height_mm),
            facts.machine.resolution_x,
            facts.machine.resolution_y,
            format_mm(facts.machine.display_width_mm),
            format_mm(facts.machine.display_height_mm),
        ),
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
            display_name: Some(facts.machine.display_name.to_string()),
            extension: Some(facts.extension.to_string()),
            resolution_x: facts.machine.resolution_x,
            resolution_y: facts.machine.resolution_y,
            display_width_mm: facts.machine.display_width_mm,
            display_height_mm: facts.machine.display_height_mm,
        },
    })
}

fn nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= DISPLAY_SIZE_TOLERANCE_MM
}

fn format_mm(value: f32) -> String {
    if value.is_finite() {
        trim_float(format!("{value:.3}"))
    } else {
        "?".to_string()
    }
}

fn trim_float(value: String) -> String {
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
