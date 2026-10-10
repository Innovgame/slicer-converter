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
});
