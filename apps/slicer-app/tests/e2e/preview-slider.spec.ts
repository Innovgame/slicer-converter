import { expect, test } from "@playwright/test";

import { loadFixtureFile } from "../support/fixture-files";

test.describe("preview and layer slider / 预览与图层滑块", () => {
    test.beforeEach(async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.goo", "GOO");
    });

    test("preview card and canvas are visible / preview card 并且 canvas are visible", async ({ page }) => {
        await expect(page.locator("#previewCard")).toBeVisible();
        await expect(page.locator("#previewCanvas")).toBeVisible();
    });

    test("layer slider updates layer number / layer slider updates layer number 用例", async ({ page }) => {
        const slider = page.locator("#layerSlider");
        const max = await slider.getAttribute("max");
        expect(Number(max)).toBeGreaterThan(0);
        const midLayer = Math.floor(Number(max) / 2);
        await slider.fill(String(midLayer));
        await slider.dispatchEvent("input");

        await expect(page.locator("#layerNum")).toHaveText(String(midLayer));
        await expect(page.locator("#previewCanvas")).toBeVisible();
    });

    test("jumps from an editable layer number and keeps the readout attached / 输入层数跳层且读数保持随动", async ({
        page
    }) => {
        const layerNumber = page.getByRole("textbox", { name: "Layer number" });
        const max = Number(await page.locator("#layerSlider").getAttribute("max"));
        const target = Math.floor(max / 2);
        await layerNumber.fill(String(target + 1));
        await layerNumber.press("Enter");
        await expect(page.locator("#layerSlider")).toHaveValue(String(target));
        await expect(page.locator("#layerNum")).toHaveText(String(target));
        await expect(page.locator("#previewCanvas")).toHaveAttribute("data-preview-layer", String(target));

        await layerNumber.fill("99999");
        await layerNumber.press("Tab");
        await expect(page.locator("#layerNum")).toHaveText(String(max));

        const settledLayer = await layerNumber.inputValue();
        await layerNumber.fill("invalid");
        await layerNumber.press("Enter");
        await expect(layerNumber).toHaveValue(settledLayer);
        await expect(page.locator("#layerNum")).toHaveText(String(max));
        await layerNumber.fill("0");
        await layerNumber.press("Escape");
        await expect(layerNumber).toHaveValue(settledLayer);
        await expect(page.locator("#layerNum")).toHaveText(String(max));
    });

    test("updates the layer readout before queued previews run / 排队预览执行前立即更新拖动层数", async ({ page }) => {
        const slider = page.locator("#layerSlider");
        const bounds = (await slider.boundingBox())!;
        await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height - 7);
        await page.mouse.down();

        const samples = await page.evaluate(() => {
            const input = document.querySelector<HTMLInputElement>("#layerSlider")!;
            const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
            const max = Number(input.max);
            return [max, 0, Math.floor(max / 2)].map((layer) => {
                setValue.call(input, String(layer));
                input.dispatchEvent(new Event("input", { bubbles: true }));
                return {
                    requested: layer,
                    slider: Number(input.value),
                    readout: Number(document.querySelector("#layerNum")!.textContent)
                };
            });
        });
        await page.mouse.up();

        for (const sample of samples) {
            expect(sample.slider).toBe(sample.requested);
            expect(sample.readout).toBe(sample.requested);
        }
    });

    test("keeps the layer readout beside the thumb across the full rail / 层数读数在整条滑轨上贴合滑块", async ({
        page
    }) => {
        const slider = page.locator("#layerSlider");
        const max = Number(await slider.getAttribute("max"));

        for (const layer of [0, Math.floor(max / 2), max]) {
            await slider.fill(String(layer));
            await expect(page.locator("#layerNum")).toHaveText(String(layer));

            const offset = await page.evaluate(() => {
                const input = document.querySelector<HTMLInputElement>("#layerSlider")!;
                const bounds = input.getBoundingClientRect();
                const readout = document.querySelector(".sc-rail-copy")!.getBoundingClientRect();
                const thumbHeight = 14;
                const ratio = Number(input.value) / Number(input.max);
                const thumbY = bounds.bottom - thumbHeight / 2 - ratio * (bounds.height - thumbHeight);
                return Math.abs((readout.top + readout.bottom) / 2 - thumbY);
            });

            expect(offset).toBeLessThan(2);
        }
    });
});
