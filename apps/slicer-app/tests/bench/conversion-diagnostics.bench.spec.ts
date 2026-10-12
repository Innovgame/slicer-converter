import { existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test, type ConsoleMessage, type Page } from "@playwright/test";

import {
    assertConversionDiagnosticsGate,
    buildConversionDiagnosticsGate,
    type ConversionDiagnosticsRun,
    type ConversionDiagnosticsThresholdMap,
    type DiagnosticEvent,
    type FileSystemSinkStats
} from "./conversion-diagnostics-gate";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const DEFAULT_SAMPLE = path.resolve(__dirname, "../../../../rust/data/球体_1_202604141902.goo");
const DEFAULT_DIAGNOSTIC_THRESHOLDS: ConversionDiagnosticsThresholdMap = {
    prz: {
        maxFinalizeOnlyWallMs: 1500
    },
    vdt: {
        maxFinalizeOnlyWallMs: 1500
    }
};
const SAMPLE_PATH = process.env.BENCH_SAMPLE_PATH ?? DEFAULT_SAMPLE;
const ITERATIONS = Number(process.env.BENCH_ITERATIONS ?? 3);
const TARGETS = (process.env.BENCH_TARGETS ?? process.env.BENCH_TARGET ?? "prz,vdt")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
const SINK_MODE = process.env.BENCH_SINK ?? "opfs";

const DIAGNOSTIC_THRESHOLDS = parseDiagnosticThresholds(process.env.BENCH_DIAGNOSTIC_THRESHOLDS);

function parseDiagnosticConsole(text: string): DiagnosticEvent | null {
    const match = /^\[(slicer-(?:output|zip))\]\s+(\S+)\s+(.+)$/.exec(text);
    if (!match) {
        return null;
    }

    return {
        channel: match[1] as DiagnosticEvent["channel"],
        event: match[2],
        details: JSON.parse(match[3]) as Record<string, unknown>,
        raw: text
    };
}

function installFakeFileSystemSink(): string {
    return `
        (() => {
            const stats = {
                writeCount: 0,
                writeBytes: 0,
                writeWallMs: 0,
                maxWriteWallMs: 0,
                seekCount: 0,
                seekWallMs: 0,
                maxSeekWallMs: 0,
                closeCount: 0,
                closeWallMs: 0,
                maxCloseWallMs: 0,
                abortCount: 0
            };
            Object.defineProperty(window, "__benchFileSystemSinkStats", {
                value: stats,
                configurable: true
            });
            Object.defineProperty(window, "showSaveFilePicker", {
                configurable: true,
                value: async () => ({
                    createWritable: async () => ({
                        write: async (data) => {
                            const startedAt = performance.now();
                            stats.writeCount += 1;
                            stats.writeBytes += typeof data === "string"
                                ? new TextEncoder().encode(data).byteLength
                                : data instanceof Blob
                                    ? data.size
                                    : data?.byteLength ?? data?.length ?? 0;
                            const elapsed = performance.now() - startedAt;
                            stats.writeWallMs += elapsed;
                            stats.maxWriteWallMs = Math.max(stats.maxWriteWallMs, elapsed);
                        },
                        seek: async () => {
                            const startedAt = performance.now();
                            stats.seekCount += 1;
                            const elapsed = performance.now() - startedAt;
                            stats.seekWallMs += elapsed;
                            stats.maxSeekWallMs = Math.max(stats.maxSeekWallMs, elapsed);
                        },
                        close: async () => {
                            const startedAt = performance.now();
                            stats.closeCount += 1;
                            const elapsed = performance.now() - startedAt;
                            stats.closeWallMs += elapsed;
                            stats.maxCloseWallMs = Math.max(stats.maxCloseWallMs, elapsed);
                        },
                        abort: async () => {
                            stats.abortCount += 1;
                        }
                    })
                })
            });
        })();
    `;
}

function installOpfsFileSystemSink(): string {
    return `
        (() => {
            const stats = {
                writeCount: 0,
                writeBytes: 0,
                writeWallMs: 0,
                maxWriteWallMs: 0,
                seekCount: 0,
                seekWallMs: 0,
                maxSeekWallMs: 0,
                closeCount: 0,
                closeWallMs: 0,
                maxCloseWallMs: 0,
                abortCount: 0
            };
            const measure = async (record, operation) => {
                const startedAt = performance.now();
                try {
                    return await operation();
                } finally {
                    const elapsed = performance.now() - startedAt;
                    record(elapsed);
                }
            };
            Object.defineProperty(window, "__benchFileSystemSinkStats", {
                value: stats,
                configurable: true
            });
            Object.defineProperty(window, "showSaveFilePicker", {
                configurable: true,
                value: async (opts) => {
                    const root = await navigator.storage.getDirectory();
                    const name = opts?.suggestedName ?? "slicer-bench-output.bin";
                    try {
                        await root.removeEntry(name);
                    } catch {}
                    const handle = await root.getFileHandle(name, { create: true });
                    return {
                        createWritable: async () => {
                            const writable = await handle.createWritable();
                            return {
                                write: async (data) => {
                                    const bytes = typeof data === "string"
                                        ? new TextEncoder().encode(data).byteLength
                                        : data instanceof Blob
                                            ? data.size
                                            : data?.byteLength ?? data?.length ?? 0;
                                    await measure((elapsed) => {
                                        stats.writeCount += 1;
                                        stats.writeBytes += bytes;
                                        stats.writeWallMs += elapsed;
                                        stats.maxWriteWallMs = Math.max(stats.maxWriteWallMs, elapsed);
                                    }, () => writable.write(data));
                                },
                                seek: async (position) => {
                                    await measure((elapsed) => {
                                        stats.seekCount += 1;
                                        stats.seekWallMs += elapsed;
                                        stats.maxSeekWallMs = Math.max(stats.maxSeekWallMs, elapsed);
                                    }, () => writable.seek(position));
                                },
                                close: async () => {
                                    await measure((elapsed) => {
                                        stats.closeCount += 1;
                                        stats.closeWallMs += elapsed;
                                        stats.maxCloseWallMs = Math.max(stats.maxCloseWallMs, elapsed);
                                    }, () => writable.close());
                                },
                                abort: async (reason) => {
                                    stats.abortCount += 1;
                                    await writable.abort(reason);
                                }
                            };
                        }
                    };
                }
            });
        })();
    `;
}

async function readFileSystemSinkStats(page: Page) {
    return page.evaluate(() => {
        return (window as unknown as { __benchFileSystemSinkStats?: FileSystemSinkStats }).__benchFileSystemSinkStats;
    });
}

test.describe("conversion diagnostics bench / 转换诊断 bench", () => {
    test(`GOO -> ${TARGETS.join(",")} diagnostics via ${SINK_MODE}`, async ({ page }, testInfo) => {
        test.skip(
            !existsSync(SAMPLE_PATH),
            `Bench sample missing: ${SAMPLE_PATH}. Set BENCH_SAMPLE_PATH env var to override.`
        );
        test.skip(TARGETS.length === 0, "No BENCH_TARGETS specified.");
        test.skip(!["blob", "fake-fs", "opfs"].includes(SINK_MODE), "BENCH_SINK must be blob, fake-fs, or opfs.");

        const sampleSizeBytes = statSync(SAMPLE_PATH).size;
        const summaries: ConversionDiagnosticsRun[] = [];

        console.log(`\n[bench] sample=${SAMPLE_PATH} (${(sampleSizeBytes / 1048576).toFixed(1)} MB)`);
        console.log(`[bench] targets=${TARGETS.join(",")} iterations=${ITERATIONS} sink=${SINK_MODE}`);

        for (const target of TARGETS) {
            for (let iteration = 0; iteration < ITERATIONS; iteration += 1) {
                const diagnostics: DiagnosticEvent[] = [];
                const onConsole = (message: ConsoleMessage) => {
                    const parsed = parseDiagnosticConsole(message.text());
                    if (parsed) {
                        diagnostics.push(parsed);
                    }
                };
                page.on("console", onConsole);

                if (SINK_MODE === "fake-fs") {
                    await page.addInitScript(installFakeFileSystemSink());
                } else if (SINK_MODE === "opfs") {
                    await page.addInitScript(installOpfsFileSystemSink());
                } else {
                    await page.addInitScript(() => {
                        Reflect.deleteProperty(window, "showSaveFilePicker");
                    });
                }

                await page.goto("/");
                await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 60_000 });
                await page.setInputFiles("#fileInput", SAMPLE_PATH);
                await expect(page.locator("#fileFormat")).toHaveText("GOO", { timeout: 120_000 });
                await page.selectOption("#slicerConverterFormat", target);

                const startedAt = Date.now();
                const downloadPromise =
                    SINK_MODE === "blob" ? page.waitForEvent("download", { timeout: 10 * 60_000 }) : null;
                await page.click("#convertBtn");
                const download = await downloadPromise;
                if (download) {
                    await download.path();
                    await download.delete().catch(() => {});
                }

                await expect(page.locator("#statusMsg")).toContainText("Converted", { timeout: 10 * 60_000 });
                const elapsedMs = Date.now() - startedAt;
                const statusText = (await page.locator("#statusMsg").textContent())?.trim() ?? "";
                const fileSystemSink =
                    SINK_MODE === "fake-fs" || SINK_MODE === "opfs" ? await readFileSystemSinkStats(page) : undefined;

                const complete = diagnostics.findLast((item) => item.event === "complete");
                const fileSystemSinkText = fileSystemSink
                    ? ` fsWrites=${fileSystemSink.writeCount} fsBytes=${fileSystemSink.writeBytes} fsClose=${fileSystemSink.closeWallMs.toFixed(1)}ms`
                    : "";
                console.log(
                    `[bench] target=${target} run=${iteration + 1}/${ITERATIONS} elapsed=${(elapsedMs / 1000).toFixed(
                        2
                    )}s${fileSystemSinkText} complete=${complete?.raw ?? "missing"}`
                );

                summaries.push({
                    target,
                    iteration: iteration + 1,
                    statusText,
                    elapsedMs,
                    diagnostics,
                    completionEvidence: "public-status",
                    fileSystemSink
                });
                page.off("console", onConsole);
            }
        }

        await testInfo.attach("conversion-diagnostics-summary.json", {
            body: JSON.stringify(
                {
                    sample: SAMPLE_PATH,
                    sampleSizeMB: sampleSizeBytes / 1048576,
                    iterations: ITERATIONS,
                    targets: TARGETS,
                    sinkMode: SINK_MODE,
                    runs: summaries
                },
                null,
                2
            ),
            contentType: "application/json"
        });

        const gate = buildConversionDiagnosticsGate(summaries, DIAGNOSTIC_THRESHOLDS);
        console.log(`[bench] diagnostic gate ${JSON.stringify(gate, null, 2)}`);

        for (const summary of summaries) {
            expect(summary.statusText).toContain("Converted");
        }
        assertConversionDiagnosticsGate(gate);
    });
});

function parseDiagnosticThresholds(value: string | undefined): ConversionDiagnosticsThresholdMap {
    if (!value) {
        return DEFAULT_DIAGNOSTIC_THRESHOLDS;
    }
    return JSON.parse(value) as ConversionDiagnosticsThresholdMap;
}
