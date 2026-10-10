import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const downloadRoot = join(repoRoot, "apps", "slicer-app", "test-results", "roundtrip-downloads");
const reportPath = join(downloadRoot, "compat-report.json");
const markdownReportPath = join(downloadRoot, "compat-report.md");

export function getRoundtripCompatReportPath() {
    return reportPath;
}

export function getRoundtripCompatMarkdownReportPath() {
    return markdownReportPath;
}

export async function collectRoundtripCompatReport() {
    const relativePath = relative(repoRoot, downloadRoot).replaceAll("\\", "/");
    if (relativePath !== "apps/slicer-app/test-results/roundtrip-downloads") {
        throw new Error(`Refusing to collect unexpected path: ${downloadRoot}`);
    }

    await mkdir(downloadRoot, { recursive: true });
    const reports = await collectCompatFiles(downloadRoot);
    reports.sort((left, right) => String(left.label).localeCompare(String(right.label)));

    const summary = {
        generated_at: new Date().toISOString(),
        total: reports.length,
        entries: reports,
        qualityGate: evaluateQualityGate(reports)
    };
    await writeFile(reportPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
    await writeFile(markdownReportPath, `${formatMarkdownReport(summary)}\n`, "utf8");
    return summary;
}

async function collectCompatFiles(root) {
    const entries = await readdir(root, { withFileTypes: true });
    const reports = [];

    for (const entry of entries) {
        const fullPath = join(root, entry.name);
        if (entry.isDirectory()) {
            reports.push(...(await collectCompatFiles(fullPath)));
            continue;
        }
        if (!entry.name.endsWith(".compat.json")) {
            continue;
        }
        reports.push(JSON.parse(await readFile(fullPath, "utf8")));
    }

    return reports;
}

function formatMarkdownReport(summary) {
    const rows = [
        "# Roundtrip Compatibility Report",
        "",
        ...formatMarkdownSummary(summary),
        "",
        "| risk | data | family | target | ext | size | decode | encode | sq | res | lcd | layers | src | machine | notes |",
        "| --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- | --- | ---: | --- | --- | --- |"
    ];

    for (const entry of summary.entries) {
        const uvtools = entry.uvtools ?? {};
        const notes = compatibilityNotes(entry);
        const displayNotes = [...formatWarningList(uvtools), ...notes].join("; ");
        rows.push(
            [
                compatibilityRisk(entry, notes),
                formatDataTag(entry.tag),
                entry.outputFamily,
                formatTargetLabel(entry),
                uvtools.fileExtension,
                formatBytes(entry.fileSizeBytes),
                formatShortBool(uvtools.canDecode),
                formatShortBool(uvtools.canEncode),
                formatShortBool(uvtools.usingSquarePixels),
                formatResolution(uvtools),
                formatDisplay(uvtools),
                uvtools.layerCount,
                entry.sourceMachineName,
                uvtools.machineName,
                displayNotes
            ]
                .map(markdownCell)
                .join(" | ")
                .replace(/^/u, "| ")
                .replace(/$/u, " |")
        );
    }

    return rows.join("\n");
}

function formatMarkdownSummary(summary) {
    const riskCounts = countBy(summary.entries, (entry) => compatibilityRisk(entry, compatibilityNotes(entry)));
    const familyCounts = countBy(summary.entries, (entry) => entry.outputFamily ?? "-");
    const sampleNoteCount = summary.entries.filter((entry) => hasSampleFixtureNotes(entry, compatibilityNotes(entry))).length;
    return [
        `**Total**: ${summary.total}`,
        `**Risk**: fail ${riskCounts.fail ?? 0}, warning ${riskCounts.warning ?? 0}, note ${riskCounts.note ?? 0}, ok ${riskCounts.ok ?? 0}`,
        `**Sample notes**: ${sampleNoteCount}`,
        `**Quality gate**: ${summary.qualityGate?.ok ? "pass" : "fail"}`,
        `**Family**: ${formatCountMap(familyCounts)}`
    ];
}

function evaluateQualityGate(entries) {
    const violations = [];
    for (const entry of entries) {
        const risk = compatibilityRisk(entry, compatibilityNotes(entry));
        if (risk === "fail") {
            violations.push(formatGateViolation(entry, risk, "compatibility failure"));
            continue;
        }
        if (entry.tag === "@sample" && risk !== "ok") {
            violations.push(formatGateViolation(entry, risk, `sample compatibility ${risk}`));
        }
    }
    return {
        ok: violations.length === 0,
        violations
    };
}

function formatGateViolation(entry, risk, reason) {
    return {
        label: String(entry.label ?? formatTargetLabel(entry)),
        risk,
        reason
    };
}

function countBy(entries, keyForEntry) {
    const counts = {};
    for (const entry of entries) {
        const key = keyForEntry(entry);
        counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
}

function formatCountMap(counts) {
    const entries = Object.entries(counts).sort(([left], [right]) => left.localeCompare(right));
    if (entries.length === 0) {
        return "-";
    }
    return entries.map(([key, count]) => `${key} ${count}`).join(", ");
}

function compatibilityRisk(entry, notes) {
    const uvtools = entry.uvtools ?? {};
    const warnings = uvtools.warnings ?? [];

    if (uvtools.issue || uvtools.canDecode === false || uvtools.canEncode === false) {
        return "fail";
    }
    if (warnings.some((warning) => /incorrect image ratio/iu.test(warning))) {
        return "warning";
    }
    if (notes.length > 0 && !hasSampleFixtureNotes(entry, notes)) {
        return "note";
    }
    return "ok";
}

function compatibilityNotes(entry) {
    const uvtools = entry.uvtools ?? {};
    const notes = [];
    const isSampleFixtureResolution = entry.tag === "@sample" && uvtools.resolutionX === 64 && uvtools.resolutionY === 64;

    if (uvtools.canDecode === false) {
        notes.push("decode unsupported");
    }
    if (uvtools.canEncode === false) {
        notes.push("encode unsupported");
    }
    if (uvtools.machineName === "Unknown" || uvtools.machineName === "TestPrinter") {
        notes.push("legacy or generic machine metadata");
    }
    if (uvtools.usingSquarePixels === false && !isSampleFixtureResolution) {
        notes.push("target machine geometry differs from source");
    }
    if (isSampleFixtureResolution) {
        notes.push("sample fixture resolution");
    }

    return notes;
}

function hasSampleFixtureNotes(entry, notes) {
    return entry.tag === "@sample" && notes.includes("sample fixture resolution");
}

function formatBytes(value) {
    if (!Number.isFinite(value)) {
        return "-";
    }
    if (value < 1024) {
        return `${value} B`;
    }
    if (value < 1024 * 1024) {
        return `${(value / 1024).toFixed(1)} KB`;
    }
    return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatResolution(uvtools) {
    if (!Number.isFinite(uvtools.resolutionX) || !Number.isFinite(uvtools.resolutionY)) {
        return "-";
    }
    return `${uvtools.resolutionX}x${uvtools.resolutionY}`;
}

function formatDisplay(uvtools) {
    if (!Number.isFinite(uvtools.displayWidth) || !Number.isFinite(uvtools.displayHeight)) {
        return "-";
    }
    return `${uvtools.displayWidth}x${uvtools.displayHeight}`;
}

function formatWarningList(uvtools) {
    return [
        ...(uvtools.warnings ?? []).filter((warning) => !/^UsingSquarePixels:\s*False$/iu.test(warning)),
        uvtools.issue ? `issue: ${uvtools.issue}` : undefined
    ].filter(Boolean);
}

function formatDataTag(tag) {
    if (tag === "@sample") {
        return "sample";
    }
    if (tag === "@real" || tag === "@smoke") {
        return "real";
    }
    return "-";
}

function formatTargetLabel(entry) {
    if (entry.targetLabel) {
        return entry.targetLabel;
    }
    return String(entry.label ?? "")
        .replace(/^sample-goo-to-/u, "")
        .replace(/^real-.+?-to-/u, "");
}

function formatShortBool(value) {
    if (value === true) {
        return "Y";
    }
    if (value === false) {
        return "N";
    }
    return "-";
}

function markdownCell(value) {
    if (value === undefined || value === null || value === "") {
        return "-";
    }
    return String(value).replaceAll("|", "\\|").replaceAll(/\r?\n/gu, " ");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
    const summary = await collectRoundtripCompatReport();
    if (!summary.qualityGate.ok) {
        const details = summary.qualityGate.violations
            .map((violation) => `- ${violation.label}: ${violation.risk} (${violation.reason})`)
            .join("\n");
        console.error(`Roundtrip compatibility gate failed:\n${details}`);
        process.exitCode = 1;
    }
}
