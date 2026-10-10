import type { ExportDiagnosticReport } from "@innovgame/slicer-converter";

/**
 * @zh 从 Rust 导出诊断报告中提取第一条可展示 warning。
 * @en Extracts the first displayable warning from the Rust export diagnostic report.
 */
export function buildAnycubicExportWarning(report: ExportDiagnosticReport): string | null {
    return report.warnings.find((warning) => warning.severity === "warning")?.message ?? null;
}
