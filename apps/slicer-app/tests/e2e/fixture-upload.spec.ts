import { expect, test } from "@playwright/test";

import { loadFixtureFile } from "../support/fixture-files";

test.describe("fixture upload / fixture 上传", () => {
    test("uploads sample.goo fixture and shows metadata / 上传 sample.goo fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.goo", "GOO");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
        await expect(page.locator("#fileName")).toHaveText("sample.goo");
    });

    test("uploads sample.ctb fixture and shows metadata / 上传 sample.ctb fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.ctb", "CTB");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
    });

    test("uploads sample.sl1 fixture and shows metadata / 上传 sample.sl1 fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.sl1", "SL1");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
    });

    test("uploads sample.pw0 fixture and shows metadata / 上传 sample.pw0 fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.pw0", "PW0");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
    });

    test("uploads sample.pws fixture and shows metadata / 上传 sample.pws fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.pws", "PWS");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
    });

    test("uploads sample.vdt fixture and shows metadata / 上传 sample.vdt fixture 并且 shows metadata", async ({
        page
    }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });

        await loadFixtureFile(page, "sample.vdt", "VDT");
        await expect(page.locator("#fileLayers")).not.toHaveText("-");
    });
});
