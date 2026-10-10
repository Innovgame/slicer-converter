import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { expect, test, type Page } from "@playwright/test";

import {
    ANYCUBIC_ARCHIVE_EXTENSIONS,
    ANYCUBIC_BINARY_EXTENSIONS,
    ANYCUBIC_UVTOOLS_STRICT_EXTENSION_SET
} from "../support/anycubic-targets";
import { fixtureFilePath, loadFilePath, repoFilePath } from "../support/fixture-files";
import { expectUvtoolsCanOpen, type UvtoolsCompatibilitySummary } from "../support/uvtools";
import { getCompressedData, parseZipDirectory } from "../../../../tests/support/zip-inspection";

interface RoundtripCase {
    tag: "@sample" | "@real" | "@smoke";
    label: string;
    sourcePath: string;
    sourceFormat: string;
    expectedLayers?: string;
    target: ExportTargetSelection;
    targetLabel: string;
    outputFamily: string;
    importedFormat: string;
    downloadedFilePattern: RegExp;
    verifyArchive?: (archiveBytes: Uint8Array) => void;
    requireThumbnail?: boolean;
    requireUvtoolsCompatibility?: boolean;
}

interface RoundtripTarget {
    target: ExportTargetSelection;
    version?: string;
    label: string;
    targetLabel?: string;
    outputFamily: string;
    importedFormat: string;
    downloadedFilePattern: RegExp;
    verifyArchive?: (archiveBytes: Uint8Array) => void;
    requireThumbnail?: boolean;
    requireUvtoolsCompatibility?: boolean;
}

type ExportTargetSelection =
    | { kind: "primary"; value: string }
    | { kind: "anycubic-manual"; value: string }
    | { kind: "chitubox-manual"; group: string; version?: string };

const REAL_SMOKE_TARGET_LABELS = new Set(["ctb-v5", "chitubox-zip", "pm5", "sl1", "vdt"]);

test.describe("browser export-import roundtrip / 浏览器导出再导入闭环", () => {
    for (const roundtripCase of roundtripCases()) {
        test(`${roundtripCase.tag} ${roundtripCase.label} / ${roundtripCase.label} 用例`, async ({ page }) => {
            test.setTimeout(roundtripCase.sourcePath.includes(`${path.sep}rust${path.sep}data${path.sep}`) ? 300_000 : 150_000);

            await disableNativeFileSystemSavePicker(page);
            await page.goto("/");
            await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

            const sourceLayers = await loadSourceAndReadLayerCount(page, roundtripCase);
            const sourceMachineName = await readSourceMachineName(roundtripCase.sourcePath);
            const downloadedPath = await exportAndSave(page, roundtripCase);
            const archiveBytes = new Uint8Array(await readFile(downloadedPath));
            roundtripCase.verifyArchive?.(archiveBytes);
            let uvtoolsSummary: UvtoolsCompatibilitySummary | null = null;
            if (roundtripCase.requireUvtoolsCompatibility ?? true) {
                await test.step("UVTools can open exported file / UVTools 可打开导出文件", async () => {
                    uvtoolsSummary = await expectUvtoolsCanOpen(downloadedPath);
                });
            }
            await writeCompatibilityReport({
                roundtripCase,
                downloadedPath,
                archiveBytes,
                sourceMachineName,
                uvtoolsSummary
            });

            await loadFilePath(page, downloadedPath, roundtripCase.importedFormat);
            await expect(page.locator("#statusMsg")).not.toContainText(/Parse error|failed/i);
            await expect(page.locator("#fileLayers")).toHaveText(sourceLayers);
            await expect(page.locator("#previewCard")).toBeVisible();
            await expect(page.locator("#previewCanvas")).toBeVisible();
            await verifyPreviewLayerSliderResponds(page);
            await expectPreviewCanvasToContainNonWhitePixels(page);

            if (roundtripCase.requireThumbnail) {
                await expect(page.locator("#thumbnailContainer")).toBeVisible();
            }
        });
    }
});

async function disableNativeFileSystemSavePicker(page: Page): Promise<void> {
    await page.addInitScript(() => {
        Object.defineProperty(window, "showSaveFilePicker", {
            configurable: true,
            value: undefined
        });
    });
}

async function loadSourceAndReadLayerCount(page: Page, roundtripCase: RoundtripCase): Promise<string> {
    await loadFilePath(page, roundtripCase.sourcePath, roundtripCase.sourceFormat);
    await expect(page.locator("#fileLayers")).not.toHaveText("-");
    const layerCount = (await page.locator("#fileLayers").textContent())?.trim() ?? "";
    if (roundtripCase.expectedLayers) {
        expect(layerCount).toBe(roundtripCase.expectedLayers);
    }
    return layerCount;
}

async function exportAndSave(page: Page, roundtripCase: RoundtripCase): Promise<string> {
    await selectTarget(page, roundtripCase.target);

    const downloadPromise = page.waitForEvent("download", { timeout: 90_000 });
    await page.click("#convertBtn");
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toMatch(roundtripCase.downloadedFilePattern);
    const outputDir = path.join(process.cwd(), "test-results", "roundtrip-downloads", roundtripCase.outputFamily);
    await mkdir(outputDir, { recursive: true });
    const filePath = path.join(outputDir, `${roundtripCase.label}-${download.suggestedFilename()}`);
    await download.saveAs(filePath);
    return filePath;
}

async function writeCompatibilityReport(input: {
    roundtripCase: RoundtripCase;
    downloadedPath: string;
    archiveBytes: Uint8Array;
    sourceMachineName: string | null;
    uvtoolsSummary: UvtoolsCompatibilitySummary | null;
}): Promise<void> {
    const reportPath = `${input.downloadedPath}.compat.json`;
    const report = {
        tag: input.roundtripCase.tag,
        label: input.roundtripCase.label,
        targetLabel: input.roundtripCase.targetLabel,
        sourceMachineName: input.sourceMachineName,
        sourceFormat: input.roundtripCase.sourceFormat,
        importedFormat: input.roundtripCase.importedFormat,
        outputFamily: input.roundtripCase.outputFamily,
        outputPath: input.downloadedPath,
        fileSizeBytes: input.archiveBytes.byteLength,
        uvtools: input.uvtoolsSummary,
        uvtoolsCompatibilityRequired: input.roundtripCase.requireUvtoolsCompatibility ?? true
    };
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
}

const sourceMachineNameCache = new Map<string, Promise<string | null>>();

function readSourceMachineName(sourcePath: string): Promise<string | null> {
    let cached = sourceMachineNameCache.get(sourcePath);
    if (!cached) {
        cached = expectUvtoolsCanOpen(sourcePath).then((summary) => summary.machineName);
        sourceMachineNameCache.set(sourcePath, cached);
    }
    return cached;
}

async function selectTarget(page: Page, target: ExportTargetSelection): Promise<void> {
    if (target.kind === "primary") {
        await page.selectOption("#slicerConverterFormat", target.value);
        return;
    }

    if (target.kind === "anycubic-manual") {
        await page.selectOption("#slicerConverterFormat", "anycubic-manual");
        await page.selectOption("#slicerConverterSuffix", target.value);
        return;
    }

    await page.selectOption("#slicerConverterFormat", "chitubox-manual");
    await page.selectOption("#slicerConverterSuffix", target.group);
    if (target.version) {
        await page.selectOption("#slicerConverterVersion", target.version);
    }
}

async function verifyPreviewLayerSliderResponds(page: Page): Promise<void> {
    const slider = page.locator("#layerSlider");
    const max = Number(await slider.getAttribute("max"));
    if (!Number.isFinite(max) || max <= 0) {
        return;
    }

    const targetLayer = Math.max(1, Math.floor(max / 2));
    await slider.fill(String(targetLayer));
    await slider.dispatchEvent("input");
    await expect(page.locator("#layerNum")).toHaveText(String(targetLayer));
    await expect(page.locator("#statusMsg")).not.toContainText(/Parse error|failed/i);
}

async function expectPreviewCanvasToContainNonWhitePixels(page: Page): Promise<void> {
    await expect
        .poll(
            () =>
                page.locator("#previewCanvas").evaluate((canvas) => {
                    if (!(canvas instanceof HTMLCanvasElement)) {
                        return false;
                    }
                    const context = canvas.getContext("2d");
                    if (!context || canvas.width === 0 || canvas.height === 0) {
                        return false;
                    }
                    const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
                    for (let index = 0; index < data.length; index += 4) {
                        const alpha = data[index + 3] ?? 0;
                        const red = data[index] ?? 255;
                        const green = data[index + 1] ?? 255;
                        const blue = data[index + 2] ?? 255;
                        if (alpha > 0 && (red < 250 || green < 250 || blue < 250)) {
                            return true;
                        }
                    }
                    return false;
                }),
            { timeout: 10_000 }
        )
        .toBe(true);
}

function roundtripCases(): RoundtripCase[] {
    return [...sampleDataRoundtripCases(), ...realDataRoundtripCases()];
}

function sampleDataRoundtripCases(): RoundtripCase[] {
    return [
        ...buildRoundtripCasesForSource({
            tag: "@sample",
            sourceLabel: "sample-goo",
            sourcePath: fixtureFilePath("sample.goo"),
            sourceFormat: "GOO",
            expectedLayers: "10",
            includeSameFormatTarget: false
        }),
        ...buildRoundtripCasesForSource({
            tag: "@sample",
            sourceLabel: "sample-pws",
            sourcePath: fixtureFilePath("sample.pws"),
            sourceFormat: "PWS",
            expectedLayers: "10",
            onlyTargetLabels: new Set(["goo"])
        })
    ];
}

function realDataRoundtripCases(): RoundtripCase[] {
    const fileName = "球体_1_202604141902.goo";
    return buildRoundtripCasesForSource({
        tag: "@real",
        sourceLabel: `real-${sanitizeLabel(fileName)}`,
        sourcePath: repoFilePath("rust", "data", fileName),
        sourceFormat: "GOO",
        includeSameFormatTarget: false
    }).map((roundtripCase) =>
        REAL_SMOKE_TARGET_LABELS.has(roundtripCase.targetLabel)
            ? { ...roundtripCase, tag: "@smoke" }
            : roundtripCase
    );
}

function buildRoundtripCasesForSource(input: {
    tag: "@sample" | "@real" | "@smoke";
    sourceLabel: string;
    sourcePath: string;
    sourceFormat: string;
    expectedLayers?: string;
    includeSameFormatTarget?: boolean;
    onlyTargetLabels?: ReadonlySet<string>;
}): RoundtripCase[] {
    return roundtripTargets()
        .filter((target) => !input.onlyTargetLabels || input.onlyTargetLabels.has(target.targetLabel ?? target.label))
        .filter(
            (target) =>
                input.includeSameFormatTarget !== false ||
                (target.target.kind === "primary" && target.target.value !== input.sourceFormat.toLowerCase()) ||
                target.target.kind !== "primary"
        )
        .map((target) => ({
            tag: input.tag,
            label: `${input.sourceLabel}-to-${target.label}`,
            sourcePath: input.sourcePath,
            sourceFormat: input.sourceFormat,
            expectedLayers: input.expectedLayers,
            target: target.target,
            targetLabel: target.targetLabel ?? target.label,
            outputFamily: target.outputFamily,
            importedFormat: target.importedFormat,
            downloadedFilePattern: target.downloadedFilePattern,
            verifyArchive: target.verifyArchive,
            requireThumbnail: target.requireThumbnail,
            requireUvtoolsCompatibility: target.requireUvtoolsCompatibility
        }));
}

function roundtripTargets(): RoundtripTarget[] {
    return [
        {
            target: { kind: "primary", value: "goo" },
            label: "goo",
            outputFamily: "goo",
            importedFormat: "GOO",
            downloadedFilePattern: /\.goo$/i
        },
        {
            target: { kind: "primary", value: "prz" },
            label: "prz",
            outputFamily: "phrozen",
            importedFormat: "PRZ",
            downloadedFilePattern: /\.prz$/i
        },
        ...["ctb-v1", "ctb", "ctb-v3", "ctb-v4", "ctb-v5"].map((version) => ({
            target: { kind: "chitubox-manual" as const, group: "ctb", version },
            version,
            label: version,
            outputFamily: "chitubox",
            importedFormat: version.toUpperCase(),
            downloadedFilePattern: /\.ctb$/i
        })),
        {
            target: { kind: "chitubox-manual", group: "cbddlp", version: "cbddlp-v1" },
            version: "cbddlp-v1",
            label: "cbddlp-v1",
            outputFamily: "chitubox",
            importedFormat: "CBDDLP-V1",
            downloadedFilePattern: /\.cbddlp$/i
        },
        {
            target: { kind: "chitubox-manual", group: "cbddlp", version: "cbddlp" },
            version: "cbddlp",
            label: "cbddlp",
            outputFamily: "chitubox",
            importedFormat: "CBDDLP",
            downloadedFilePattern: /\.cbddlp$/i
        },
        ...["gktwo-ctb-v1", "gktwo-ctb-v2", "gktwo-ctb-v3", "gktwo-ctb", "gktwo-ctb-v5"].map((version) => ({
            target: { kind: "chitubox-manual" as const, group: "gktwo-ctb", version },
            version,
            label: version,
            outputFamily: "chitubox",
            importedFormat: version.toUpperCase(),
            downloadedFilePattern: /\.gktwo\.ctb$/i
        })),
        {
            target: { kind: "primary", value: "sl1" },
            label: "sl1",
            outputFamily: "archive",
            importedFormat: "SL1",
            downloadedFilePattern: /\.sl1$/i,
            verifyArchive: verifySl1Archive
        },
        {
            target: { kind: "primary", value: "vdt" },
            label: "vdt",
            outputFamily: "archive",
            importedFormat: "VDT",
            downloadedFilePattern: /\.vdt$/i,
            verifyArchive: verifyVdtArchive
        },
        {
            target: { kind: "primary", value: "cxdlpv4" },
            label: "cxdlpv4",
            outputFamily: "creality",
            importedFormat: "CXDLPV4",
            downloadedFilePattern: /\.cxdlpv4$/i
        },
        {
            target: { kind: "chitubox-manual", group: "zip" },
            label: "chitubox-zip",
            targetLabel: "chitubox-zip",
            outputFamily: "chitubox",
            importedFormat: "CHITUBOX-ZIP",
            downloadedFilePattern: /\.zip$/i,
            verifyArchive: verifyChituboxZipArchive,
            requireThumbnail: true
        },
        {
            target: { kind: "chitubox-manual", group: "photon" },
            label: "photon",
            outputFamily: "chitubox",
            importedFormat: "PHOTON",
            downloadedFilePattern: /\.photon$/i
        },
        {
            target: { kind: "chitubox-manual", group: "phz" },
            label: "phz",
            outputFamily: "chitubox",
            importedFormat: "PHZ",
            downloadedFilePattern: /\.phz$/i
        },
        {
            target: { kind: "chitubox-manual", group: "fdg" },
            label: "fdg",
            outputFamily: "chitubox",
            importedFormat: "FDG",
            downloadedFilePattern: /\.fdg$/i
        },
        {
            target: { kind: "chitubox-manual", group: "photons" },
            label: "photons",
            outputFamily: "chitubox",
            importedFormat: "PHOTONS",
            downloadedFilePattern: /\.photons$/i
        },
        {
            target: { kind: "chitubox-manual", group: "encrypted-ctb", version: "encrypted-ctb-v4" },
            version: "encrypted-ctb-v4",
            label: "encrypted-ctb-v4",
            outputFamily: "chitubox",
            importedFormat: "ENCRYPTED-CTB-V4",
            downloadedFilePattern: /\.encrypted\.ctb$/i
        },
        {
            target: { kind: "chitubox-manual", group: "encrypted-ctb", version: "encrypted-ctb" },
            version: "encrypted-ctb",
            label: "encrypted-ctb",
            outputFamily: "chitubox",
            importedFormat: "ENCRYPTED-CTB",
            downloadedFilePattern: /\.encrypted\.ctb$/i
        },
        ...ANYCUBIC_ARCHIVE_EXTENSIONS.map((extension) => ({
            target: { kind: "anycubic-manual" as const, value: extension },
            label: `anycubic-zip-${extension}`,
            targetLabel: `zip-${extension}`,
            outputFamily: "anycubic",
            importedFormat: "ANYCUBIC-ZIP",
            downloadedFilePattern: new RegExp(`\\.${escapeRegExp(extension)}$`, "i"),
            verifyArchive: verifyAnycubicZipArchive,
            requireUvtoolsCompatibility: false
        })),
        ...ANYCUBIC_BINARY_EXTENSIONS.map((extension) => ({
            target: { kind: "anycubic-manual" as const, value: extension },
            label: `anycubic-${extension}`,
            targetLabel: extension,
            outputFamily: "anycubic",
            importedFormat: extension.toUpperCase(),
            downloadedFilePattern: new RegExp(`\\.${escapeRegExp(extension)}$`, "i"),
            requireUvtoolsCompatibility: ANYCUBIC_UVTOOLS_STRICT_EXTENSION_SET.has(extension)
        }))
    ];
}

function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function verifyChituboxZipArchive(archiveBytes: Uint8Array): void {
    const entries = parseZipDirectory(archiveBytes).entries;
    const entryNames = entries.map((entry) => entry.name);

    expect(entryNames).toContain("run.gcode");
    expect(entryNames).toContain("preview.png");
    expect(entryNames).toContain("preview_cropping.png");
    expect(entryNames).toContain("1.png");
    expect(entryNames.filter((name) => /^\d+\.png$/u.test(name))).not.toHaveLength(0);
}

function verifySl1Archive(archiveBytes: Uint8Array): void {
    const entries = parseZipDirectory(archiveBytes).entries;
    const entryNames = entries.map((entry) => entry.name);

    expect(entryNames).toContain("config.ini");
    expect(entryNames).toContain("prusaslicer.ini");
    expect(entryNames.filter((name) => /^.+\d{5}\.png$/u.test(name))).not.toHaveLength(0);
}

function verifyVdtArchive(archiveBytes: Uint8Array): void {
    const entries = parseZipDirectory(archiveBytes).entries;
    const entryNames = entries.map((entry) => entry.name);

    expect(entryNames).toContain("manifest.json");
    expect(entryNames).toContain("0.png");
    expect(entryNames.filter((name) => /^\d+\.png$/u.test(name))).not.toHaveLength(0);
}

function verifyAnycubicZipArchive(archiveBytes: Uint8Array): void {
    const entries = parseZipDirectory(archiveBytes).entries;
    const entryNames = entries.map((entry) => entry.name);

    for (const required of [
        "anycubic_photon_resins.pwsp",
        "layers_controller.conf",
        "print_info.json",
        "layer_images/layer_0.pw0Img"
    ]) {
        expect(entryNames).toContain(required);
    }

    const firstLayer = entries.find((entry) => entry.name === "layer_images/layer_0.pw0Img");
    expect(firstLayer).toBeDefined();
    if (firstLayer) {
        expect(getCompressedData(archiveBytes, firstLayer).byteLength).toBeGreaterThan(0);
    }
}

function sanitizeLabel(value: string): string {
    return value.replace(/\.[^.]+$/u, "").replace(/[^\p{Letter}\p{Number}]+/gu, "-").replace(/^-|-$/gu, "");
}
