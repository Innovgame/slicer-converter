import { expect, test, type ConsoleMessage, type Page } from "@playwright/test";

import {
    type ConversionDiagnosticsRun,
    type DiagnosticEvent,
    type FileSystemSinkStats
} from "../support/conversion-diagnostics";
import { loadFixtureFile } from "../support/fixture-files";

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
            Object.defineProperty(window, "__conversionTestFileSystemSinkStats", {
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

async function readFileSystemSinkStats(page: Page): Promise<FileSystemSinkStats | undefined> {
    return page.evaluate(() => {
        return (window as unknown as { __conversionTestFileSystemSinkStats?: FileSystemSinkStats })
            .__conversionTestFileSystemSinkStats;
    });
}

test.describe("conversion diagnostics / 转换诊断", () => {
    test("emits stable diagnostics for binary and ZIP conversions / emits stable 诊断 for binary 并且 ZIP conversions", async ({
        page
    }, testInfo) => {
        test.setTimeout(90_000);
        const summaries: ConversionDiagnosticsRun[] = [];

        for (const target of ["prz", "vdt"]) {
            const diagnostics: DiagnosticEvent[] = [];
            const onConsole = (message: ConsoleMessage) => {
                const parsed = parseDiagnosticConsole(message.text());
                if (parsed) {
                    diagnostics.push(parsed);
                }
            };
            page.on("console", onConsole);

            await page.addInitScript(installFakeFileSystemSink());
            await page.goto("/");
            await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
            await loadFixtureFile(page, "sample.goo", "GOO");
            await page.selectOption("#slicerConverterFormat", target);

            const startedAt = Date.now();
            await page.click("#convertBtn");
            await expect(page.locator("#statusMsg")).toContainText("Converted", { timeout: 60_000 });

            summaries.push({
                target,
                iteration: 1,
                statusText: (await page.locator("#statusMsg").textContent())?.trim() ?? "",
                elapsedMs: Date.now() - startedAt,
                diagnostics,
                fileSystemSink: await readFileSystemSinkStats(page)
            });

            page.off("console", onConsole);
        }

        await testInfo.attach("conversion-diagnostics-summary.json", {
            body: JSON.stringify(summaries, null, 2),
            contentType: "application/json"
        });

        expect(summaries.every((summary) => summary.statusText.includes("Converted"))).toBe(true);
        expect(summaries.every((summary) => summary.statusText.includes("streamed to disk"))).toBe(true);
        expect(summaries.every((summary) => (summary.fileSystemSink?.closeCount ?? 0) > 0)).toBe(true);
        expect(
            summaries.every((summary) => summary.diagnostics.some((item) => item.event === "file-system-close"))
        ).toBe(true);
    });
});
