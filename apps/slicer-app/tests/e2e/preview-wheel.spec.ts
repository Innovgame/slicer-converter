import { expect, test } from "@playwright/test";
import { existsSync } from "node:fs";
import type * as LayerViewportModule from "../../src/features/converter/ui/layer-viewport";
import { loadFixtureFile, repoFilePath } from "../support/fixture-files";

for (const mobile of [false, true]) {
    test(`opens a real 8K file without blocking React on ${mobile ? "mobile" : "desktop"} / ${mobile ? "移动端" : "桌面端"}打开真实 8K 文件不会阻塞 React`, async ({
        browser,
        browserName,
        baseURL
    }) => {
        test.skip(browserName !== "chromium", "Long-task timing requires Chromium / 长任务测量需要 Chromium");
        const sample = repoFilePath("rust", "data", "球体_1_202604141902.goo");
        test.skip(!existsSync(sample), "Local real 8K sample is unavailable / 本地真实 8K 样本不存在");
        const context = await browser.newContext({
            viewport: mobile ? { width: 393, height: 852 } : { width: 1280, height: 800 },
            deviceScaleFactor: mobile ? 3 : 1,
            isMobile: mobile,
            hasTouch: mobile
        });
        try {
            const page = await context.newPage();
            await page.goto(baseURL!);
            await page.getByRole("button", { name: "Not now", exact: true }).click();
            await expect(page.locator("#statusMsg")).toContainText("Ready");
            await page.evaluate(() => {
                const tasks: number[] = [];
                const observer = new PerformanceObserver((list) =>
                    tasks.push(...list.getEntries().map((entry) => entry.duration))
                );
                observer.observe({ type: "longtask" });
                Object.assign(window, { previewLoadTasks: tasks, previewLoadObserver: observer });
            });
            const settled = page.waitForEvent("console", {
                predicate: (message) =>
                    message.text().includes("[preview-perf] app canvas") && message.text().includes("bitmap=8520x4320"),
                timeout: 60_000
            });
            await page.setInputFiles("#fileInput", sample);
            await settled;
            const tasks = await page.evaluate(async () => {
                await new Promise<void>((resolve) =>
                    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
                );
                const tracked = window as typeof window & {
                    previewLoadTasks: number[];
                    previewLoadObserver: PerformanceObserver;
                };
                tracked.previewLoadObserver.disconnect();
                return tracked.previewLoadTasks;
            });
            expect(Math.max(0, ...tasks)).toBeLessThan(1000);
            await page.getByRole("button", { name: "Zoom in", exact: true }).click();
            await expect(page.locator("#previewCanvas")).toHaveAttribute("data-preview-layer", "0");
        } finally {
            await context.close();
        }
    });
}

test("keeps grey readouts through the real preview worker pipeline / 真实预览 Worker 链路保留灰度读数", async ({
    page
}) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await expect(page.locator("#statusMsg")).toContainText("Ready");
    await loadFixtureFile(page, "sample.goo", "GOO");
    const canvas = page.locator("#previewCanvas");
    await expect(canvas).toHaveAttribute("data-preview-layer", "0");
    const bounds = (await canvas.boundingBox())!;
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await expect(page.locator(".sc-pixel-readout")).toHaveText(/ · \D+ \d+$/);
    const previousZoom = (await canvas.getAttribute("data-zoom"))!;
    await page.mouse.wheel(0, -28);
    await expect(canvas).not.toHaveAttribute("data-zoom", previousZoom);
    await expect(page.locator(".sc-pixel-readout")).toHaveText(/ · \D+ \d+$/);
});

test("samples an 8K layer without Canvas readback during wheel zoom / 滚轮缩放 8K 层时无需画布读回取样", async ({
    page
}) => {
    await page.goto("/");
    const result = await page.evaluate(async () => {
        const modulePath = "/src/features/converter/ui/layer-viewport.ts";
        const { LayerViewport } = (await import(modulePath)) as typeof LayerViewportModule;
        const canvas = document.createElement("canvas");
        canvas.style.cssText = "position:fixed;left:0;top:0;width:1000px;height:600px";
        document.body.replaceChildren(canvas);
        const source = document.createElement("canvas");
        source.width = 8520;
        source.height = 4320;
        source.getContext("2d")!.fillRect(0, 0, source.width, source.height);
        const bitmap = await createImageBitmap(source);
        const greys = new Uint8Array(source.width * source.height).fill(173);
        let frames = 0;
        let grey: number | undefined;
        const viewport = new LayerViewport(canvas, (state) => {
            frames++;
            grey = state.pixel?.grey;
        });
        const originalRead = CanvasRenderingContext2D.prototype.getImageData;
        let readbacks = 0;
        CanvasRenderingContext2D.prototype.getImageData = function (...args) {
            readbacks++;
            return originalRead.apply(this, args);
        };
        try {
            viewport.setImage({
                bitmap,
                greys: greys.buffer,
                bitmapWidth: source.width,
                bitmapHeight: source.height,
                layerIndex: 0,
                plate: {
                    nativeWidth: source.width,
                    nativeHeight: source.height,
                    platformWidth: 200,
                    platformHeight: 100,
                    mirrorX: false,
                    mirrorY: false
                }
            });
            // Allow the initial ResizeObserver notification to finish before measuring wheel frames.
            await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
            frames = 0;
            const durations: number[] = [];
            for (let i = 0; i < 5; i++) {
                const start = performance.now();
                canvas.dispatchEvent(
                    new WheelEvent("wheel", { deltaY: -28, clientX: 500, clientY: 300, cancelable: true })
                );
                durations.push(performance.now() - start);
            }
            const synchronousFrames = frames;
            await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
            return { durations, synchronousFrames, frames, readbacks, grey };
        } finally {
            viewport.dispose();
            bitmap.close();
            CanvasRenderingContext2D.prototype.getImageData = originalRead;
        }
    });
    expect(result.synchronousFrames).toBe(0);
    expect(result.frames).toBe(1);
    expect(result.readbacks).toBe(0);
    expect(result.grey).toBe(173);
    expect(Math.max(...result.durations)).toBeLessThan(50);
});
