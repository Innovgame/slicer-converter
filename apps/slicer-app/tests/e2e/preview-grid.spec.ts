import { expect, test } from "@playwright/test";
import { loadFixtureFile } from "../support/fixture-files";

for (const dpr of [1, 2, 3]) {
    test.describe(`Pixel grid at DPR ${dpr} / DPR ${dpr} 像素网格`, () => {
        test.use({ deviceScaleFactor: dpr, viewport: { width: 390, height: 844 } });

        test("aligns pixel edges after zooming and switching viewport size / 缩放与切换视口尺寸后像素边界仍对齐", async ({
            page
        }) => {
            await page.goto("/");
            await page.getByRole("button", { name: "Not now", exact: true }).click();
            await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
            await loadFixtureFile(page, "sample.goo", "GOO");
            const canvas = page.locator("#previewCanvas");
            await expect(canvas).toHaveAttribute("data-preview-layer", "0");

            for (const width of [390, 1280]) {
                await page.setViewportSize({ width, height: 844 });
                await canvas.focus();
                await canvas.press("1");
                for (let i = 0; i < (dpr === 1 ? 4 : 5); i++) await canvas.press("+");
                for (const fractional of [false, true]) {
                    if (fractional)
                        await canvas.dispatchEvent("wheel", { deltaY: 28, clientX: width / 2, clientY: 422 });
                    await expect(canvas).toHaveAttribute("data-grid-visible", "true");
                    const measurements = await canvas.evaluate((element: HTMLCanvasElement) => {
                        const context = element.getContext("2d")!;
                        const zoom = Number(element.dataset.zoom);
                        const originX = Number(element.dataset.offsetX) * devicePixelRatio;
                        const originY = Number(element.dataset.offsetY) * devicePixelRatio;
                        let y = Math.floor(element.height / 2);
                        while (Math.abs((y - originY) / zoom - Math.round((y - originY) / zoom)) < 0.15) y++;
                        const readRow = () => context.getImageData(0, y, element.width, 1).data;
                        const grid = readRow();
                        const gridLines: number[] = [];
                        for (let x = 0; x < element.width; x++) {
                            if (grid[x * 4] > 0 && grid[x * 4] < 255) gridLines.push(x);
                        }
                        return { gridLines, zoom, originX, y };
                    });
                    expect(measurements.gridLines.length).toBeGreaterThan(3);
                    for (const x of measurements.gridLines) {
                        const pixel = Math.round((x - measurements.originX) / measurements.zoom);
                        expect(
                            Math.abs(x - Math.round(measurements.originX + pixel * measurements.zoom))
                        ).toBeLessThanOrEqual(dpr);
                    }
                    await page.getByRole("button", { name: "Pixel grid", exact: true }).click();
                    const edges = await canvas.evaluate((element: HTMLCanvasElement, y) => {
                        const row = element.getContext("2d")!.getImageData(0, y, element.width, 1).data;
                        const result: number[] = [];
                        for (let x = 1; x < element.width; x++)
                            if (Math.abs(row[x * 4] - row[(x - 1) * 4]) > 128) result.push(x);
                        return result;
                    }, measurements.y);
                    expect(edges.length).toBeGreaterThan(0);
                    for (const edge of edges) {
                        expect(
                            Math.min(...measurements.gridLines.map((line) => Math.abs(line - edge)))
                        ).toBeLessThanOrEqual(1);
                    }
                    await page.getByRole("button", { name: "Pixel grid", exact: true }).click();
                }
            }
        });
    });
}

test("keeps a clear pixel grid and complete frames while dragging at 128x / 128 倍拖动时网格清晰且不出现空白帧", async ({
    browser,
    baseURL
}) => {
    const context = await browser.newContext({ viewport: { width: 393, height: 852 }, deviceScaleFactor: 3 });
    try {
        const page = await context.newPage();
        await page.addInitScript(() => {
            const clear = CanvasRenderingContext2D.prototype.clearRect;
            CanvasRenderingContext2D.prototype.clearRect = function (x, y, width, height) {
                if (this.canvas instanceof HTMLCanvasElement && this.canvas.id === "previewCanvas") {
                    this.canvas.dataset.cleared = "true";
                }
                clear.call(this, x, y, width, height);
            };
        });
        await page.goto(baseURL!);
        await page.getByRole("button", { name: "Not now", exact: true }).click();
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.goo", "GOO");
        const canvas = page.locator("#previewCanvas");
        await expect(canvas).toHaveAttribute("data-preview-layer", "0");
        await canvas.focus();
        await canvas.press("1");
        for (let i = 0; i < 7; i++) await canvas.press("+");
        await expect(canvas).toHaveAttribute("data-zoom", "128");
        await page.mouse.move(90, 300);
        await page.mouse.down();
        for (let step = 1; step <= 10; step++) {
            await page.mouse.move(90 + step * 12.3, 300 + step * 2.7);
            await page.evaluate(
                () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
            );
            const frame = await canvas.evaluate((element: HTMLCanvasElement) => {
                const ctx = element.getContext("2d")!;
                const zoom = Number(element.dataset.zoom);
                const originX = Number(element.dataset.offsetX) * devicePixelRatio;
                const originY = Number(element.dataset.offsetY) * devicePixelRatio;
                const column = Math.round((element.width / 2 - originX) / zoom);
                const boundary = Math.round(originX + column * zoom);
                const row = Math.floor((element.height / 2 - originY) / zoom);
                const y = Math.round(originY + (row + 0.5) * zoom);
                const grid = ctx.getImageData(boundary, y, 3, 1).data;
                const background = ctx.getImageData(boundary + 10, y, 1, 1).data;
                return {
                    grid: Array.from({ length: 3 }, (_, index) => ({
                        value: grid[index * 4],
                        alpha: grid[index * 4 + 3]
                    })),
                    background: background[0],
                    alpha: background[3],
                    cleared: element.dataset.cleared
                };
            });
            expect(frame.cleared).toBeUndefined();
            expect(frame.alpha).toBe(255);
            expect(frame.background).toBe(255);
            for (const pixel of frame.grid) {
                expect(pixel.alpha).toBe(255);
                expect(pixel.value).toBeLessThan(150);
                expect(pixel.value).toBeGreaterThan(100);
            }
        }
        await page.mouse.up();
    } finally {
        await context.close();
    }
});

test("keeps grid and bitmap aligned in a Chrome device-mode capture / Chrome 设备模拟截取时网格与位图保持对齐", async ({
    browser,
    browserName,
    baseURL
}) => {
    test.skip(
        browserName !== "chromium",
        "Device-mode capture uses Chrome DevTools Protocol / 设备模拟截取使用 Chrome DevTools 协议"
    );
    const context = await browser.newContext({ viewport: null, deviceScaleFactor: undefined });
    try {
        const page = await context.newPage();
        await page.goto(baseURL!);
        await page.getByRole("button", { name: "Not now", exact: true }).click();
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.goo", "GOO");
        const canvas = page.locator("#previewCanvas");
        await expect(canvas).toHaveAttribute("data-preview-layer", "0");
        const session = await context.newCDPSession(page);
        await session.send("Emulation.setDeviceMetricsOverride", {
            width: 393,
            height: 852,
            deviceScaleFactor: 3,
            mobile: true
        });
        await expect(page.locator(".sc-mobile-preview-tools")).toBeVisible();
        await expect.poll(() => canvas.evaluate((element: HTMLCanvasElement) => element.width)).toBe(1179);
        await canvas.dispatchEvent("keydown", { key: "1" });
        for (let i = 0; i < 5; i++) await canvas.dispatchEvent("keydown", { key: "+" });
        await expect(canvas).toHaveAttribute("data-zoom", "32");
        await expect(canvas).toHaveAttribute("data-grid-visible", "true");
        await page.evaluate(
            () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
        );
        const withGrid = await page.screenshot();
        const inspection = await context.newPage();
        const rows = await inspection.evaluate(async (grid) => {
            const image = new Image();
            image.src = `data:image/png;base64,${grid}`;
            await image.decode();
            const output = document.createElement("canvas");
            output.width = image.width;
            output.height = image.height;
            const ctx = output.getContext("2d")!;
            ctx.drawImage(image, 0, 0);
            return [0.36, 0.43, 0.51, 0.58].map((ratio) => {
                const y = Math.floor(output.height * ratio);
                const a = ctx.getImageData(0, y, output.width, 1).data;
                const lines: number[] = [],
                    edges: number[] = [];
                let previous: { x: number; value: number } | undefined;
                for (let x = 1; x < output.width * 0.78; x++) {
                    const value = a[x * 4];
                    if (value > 0 && value < 255) lines.push(x);
                    if (value === 0 || value === 255) {
                        if (previous && value !== previous.value && x - previous.x <= 3) edges.push(x);
                        previous = { x, value };
                    }
                }
                return { lines, edges };
            });
        }, withGrid.toString("base64"));
        expect(rows.flatMap((row) => row.edges).length).toBeGreaterThan(2);
        for (const row of rows)
            for (const edge of row.edges) {
                expect(Math.min(...rows[0].lines.map((line) => Math.abs(line - edge)))).toBeLessThanOrEqual(2);
            }
        await inspection.close();
        await session.detach();
    } finally {
        await context.close();
    }
});
