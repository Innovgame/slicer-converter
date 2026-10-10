import { existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * @zh T3.1 ZIP 端到端流式组装的性能基线采集。
 * @en Performance baseline capture for T3.1 end-to-end streaming ZIP assemble.
 *
 * 样本来自外部磁盘（不入仓库）。未放置样本时自动 skip，不影响 CI。
 */
const DEFAULT_SAMPLE = path.resolve(__dirname, "../../../../rust/data/球体_1_202604141902.goo");
const SAMPLE_PATH = process.env.BENCH_SAMPLE_PATH ?? DEFAULT_SAMPLE;
const ITERATIONS = Number(process.env.BENCH_ITERATIONS ?? 5);
const TARGET_FORMAT = process.env.BENCH_TARGET ?? "vdt";

interface BenchRun {
    statusText: string;
    peakHeapBytes: number;
    totalElapsedSec: number | null;
    assembleSec: number | null;
    finalizeOnlySec: number | null;
    layersSec: number | null;
    workerSumSec: number | null;
}

function parseStatus(text: string): {
    totalElapsedSec: number | null;
    assembleSec: number | null;
    finalizeOnlySec: number | null;
    layersSec: number | null;
    workerSumSec: number | null;
} {
    const match = (pattern: RegExp): number | null => {
        const result = pattern.exec(text);
        if (!result) {
            return null;
        }
        return Number(result[1]);
    };
    const matchDuration = (key: RegExp): number | null => {
        /**
         * @zh 支持 "XX.Xms" 和 "XX.Xs" 两种格式，并归一化为秒。
         * @en Accepts both "XX.Xms" and "XX.Xs", then normalizes to seconds.
         */
        const value = key.exec(text);
        if (!value) {
            return null;
        }
        const num = Number(value[1]);
        return value[2] === "ms" ? num / 1000 : num;
    };

    return {
        totalElapsedSec: match(/in\s+([\d.]+)s\b/),
        layersSec: matchDuration(/layers\s+([\d.]+)(ms|s)\b/),
        workerSumSec: matchDuration(/worker sum\s+([\d.]+)(ms|s)\b/),
        assembleSec: matchDuration(/assemble\s+([\d.]+)(ms|s)\b/),
        finalizeOnlySec: matchDuration(/finalize-only\s+([\d.]+)(ms|s)\b/)
    };
}

function median(values: number[]): number {
    if (values.length === 0) {
        return 0;
    }
    const sorted = [...values].sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

test.describe("T3.1 ZIP streaming bench / T3.1 ZIP streaming bench", () => {
    test(`GOO -> ${TARGET_FORMAT.toUpperCase()} ${ITERATIONS}x / GOO -> ${TARGET_FORMAT.toUpperCase()} ${ITERATIONS}x 用例`, async ({
        page
    }, testInfo) => {
        test.skip(
            !existsSync(SAMPLE_PATH),
            `Bench sample missing: ${SAMPLE_PATH}. Set BENCH_SAMPLE_PATH env var to override.`
        );

        const sampleSizeBytes = statSync(SAMPLE_PATH).size;
        console.log(`\n[bench] sample=${SAMPLE_PATH} (${(sampleSizeBytes / 1048576).toFixed(1)} MB)`);
        console.log(`[bench] target=${TARGET_FORMAT} iterations=${ITERATIONS}`);

        const runs: BenchRun[] = [];

        for (let iteration = 0; iteration < ITERATIONS; iteration += 1) {
            await page.goto("/");
            await expect(page.locator("#statusMsg")).toContainText("Ready", {
                timeout: 60_000
            });

            /**
             * @zh 在实际转换开始前采样 peak heap。
             * @en Starts peak-heap polling before conversion work begins.
             */
            await page.evaluate(() => {
                const globalAny = window as unknown as {
                    __benchPeak: number;
                    __benchInterval: number;
                };
                globalAny.__benchPeak = 0;
                const memSource = performance as unknown as {
                    memory?: { usedJSHeapSize: number };
                };
                globalAny.__benchInterval = window.setInterval(() => {
                    const heap = memSource.memory?.usedJSHeapSize ?? 0;
                    if (heap > globalAny.__benchPeak) {
                        globalAny.__benchPeak = heap;
                    }
                }, 50);
            });

            await page.setInputFiles("#fileInput", SAMPLE_PATH);
            await expect(page.locator("#fileFormat")).toHaveText("GOO", {
                timeout: 120_000
            });

            await page.selectOption("#slicerConverterFormat", TARGET_FORMAT);
            const downloadPromise = page.waitForEvent("download", {
                timeout: 10 * 60_000
            });
            await page.click("#convertBtn");
            const download = await downloadPromise;
            /**
             * @zh 等待下载完全写入，确保状态消息反映转换完成。
             * @en Waits until the download is fully written so the status message reflects completed conversion.
             */
            await download.path();

            await expect(page.locator("#statusMsg")).toContainText("Converted", {
                timeout: 10 * 60_000
            });

            const statusText = (await page.locator("#statusMsg").textContent())?.trim() ?? "";
            const peakHeapBytes = await page.evaluate(() => {
                const globalAny = window as unknown as {
                    __benchPeak: number;
                    __benchInterval: number;
                };
                window.clearInterval(globalAny.__benchInterval);
                return globalAny.__benchPeak;
            });

            const parsed = parseStatus(statusText);
            runs.push({ statusText, peakHeapBytes, ...parsed });

            console.log(
                `[bench] run ${iteration + 1}/${ITERATIONS} peak=${(peakHeapBytes / 1048576).toFixed(
                    1
                )}MB status="${statusText}"`
            );

            /**
             * @zh 尽力清理下载文件，避免多轮 bench 占用过多磁盘。
             * @en Best-effort cleanup so repeated bench iterations do not grow disk usage.
             */
            await download.delete().catch(() => {});
        }

        const totals = runs.map((run) => run.totalElapsedSec).filter((value): value is number => value !== null);
        const assembles = runs.map((run) => run.assembleSec).filter((value): value is number => value !== null);
        const finalizes = runs.map((run) => run.finalizeOnlySec).filter((value): value is number => value !== null);
        const peaks = runs.map((run) => run.peakHeapBytes);

        const summary = {
            sample: SAMPLE_PATH,
            sampleSizeMB: sampleSizeBytes / 1048576,
            target: TARGET_FORMAT,
            iterations: ITERATIONS,
            medianTotalSec: median(totals),
            medianAssembleSec: median(assembles),
            medianFinalizeOnlySec: finalizes.length === ITERATIONS ? median(finalizes) : null,
            medianPeakHeapMB: median(peaks) / 1048576,
            runs
        };

        console.log("\n=== ZIP streaming bench summary ===");
        console.log(
            `median total=${summary.medianTotalSec.toFixed(2)}s, ` +
                `median assemble=${summary.medianAssembleSec.toFixed(2)}s, ` +
                (summary.medianFinalizeOnlySec !== null
                    ? `median finalize-only=${summary.medianFinalizeOnlySec.toFixed(2)}s, `
                    : "median finalize-only=N/A, ") +
                `median peak heap=${summary.medianPeakHeapMB.toFixed(1)}MB`
        );

        await testInfo.attach("bench-summary.json", {
            body: JSON.stringify(summary, null, 2),
            contentType: "application/json"
        });

        /**
         * @zh 软护栏：如果完全没有采集到指标则失败；§4 baseline 阈值仍手动判断。
         * @en Soft guard: fail loudly if metrics are not captured at all; §4 baseline thresholds are still checked manually.
         */
        expect(totals.length, "all runs must parse totalElapsedSec").toBe(ITERATIONS);
        expect(assembles.length, "all runs must parse assembleSec").toBe(ITERATIONS);
        expect(peaks.every((peak) => peak > 0)).toBe(true);
    });
});
