import { readFile } from "node:fs/promises";

import { expect, test, type Page } from "@playwright/test";
import { inflateRaw } from "@slicer-converter/wasm";

import { loadFixtureFile } from "../support/fixture-files";
import { getCompressedData, parseZipDirectory } from "../../../../tests/support/zip-inspection";

async function selectChituboxCtbTarget(page: Page): Promise<void> {
    await page.selectOption("#slicerConverterFormat", "chitubox-manual");
    await page.selectOption("#slicerConverterSuffix", "ctb");
}

test.describe("convert and download / 转换与下载", () => {
    test.beforeEach(async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.goo", "GOO");
    });

    test("convert card and button become visible after loading / 转换 card 并且 button become visible 在之后 loading", async ({
        page
    }) => {
        await expect(page.locator("#convertCard")).toBeVisible();
        await expect(page.locator("#convertBtn")).toBeEnabled();
    });

    test("converts GOO to CTB and triggers download / converts GOO to CTB 并且 triggers 下载", async ({ page }) => {
        await selectChituboxCtbTarget(page);

        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;

        expect(download.suggestedFilename()).toMatch(/\.ctb$/);
    });

    test("converts GOO to SL1 and triggers download / converts GOO to SL1 并且 triggers 下载", async ({ page }) => {
        await page.selectOption("#slicerConverterFormat", "sl1");

        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;

        expect(download.suggestedFilename()).toMatch(/\.sl1$/);
    });

    test("converts GOO to VDT with UVtools-compatible zip layout / converts GOO to VDT 带有 UVtools-compatible zip layout", async ({
        page
    }, testInfo) => {
        await page.selectOption("#slicerConverterFormat", "vdt");

        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;

        expect(download.suggestedFilename()).toMatch(/\.vdt$/);

        const filePath = testInfo.outputPath(download.suggestedFilename());
        await download.saveAs(filePath);

        const archiveBytes = new Uint8Array(await readFile(filePath));
        const dir = parseZipDirectory(archiveBytes);
        const entryNames = dir.entries.map((entry) => entry.name);

        expect(entryNames[0]).toBe("manifest.json");
        expect(entryNames.slice(1, 4)).toEqual(["0.png", "1.png", "2.png"]);
        expect(entryNames.filter((name) => /^\d+\.png$/.test(name))).toHaveLength(dir.entries.length - 1);

        const manifestEntry = dir.entries[0];
        const manifestBytes =
            manifestEntry.compressionMethod === 0
                ? getCompressedData(archiveBytes, manifestEntry)
                : await inflateRaw(getCompressedData(archiveBytes, manifestEntry));
        const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));

        expect(manifest).toMatchObject({
            machine: {
                resolution_x: 64,
                resolution_y: 64
            },
            print: {
                layer_thickness: expect.any(Number),
                exposure_time: expect.any(Number)
            }
        });
        expect(manifest.layers).toHaveLength(entryNames.length - 1);
        expect(manifest.layers[0]).toMatchObject({
            height: expect.any(Number),
            exposure_time: expect.any(Number),
            lift_distance: expect.any(Number)
        });
    });
});

test.describe("new format conversions / 新格式转换", () => {
    test("converts PW0 to CTB / converts PW0 to CTB 用例", async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.pw0", "PW0");

        await selectChituboxCtbTarget(page);
        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toMatch(/\.ctb$/);
    });

    test("converts PWS to GOO / converts PWS to GOO 用例", async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.pws", "PWS");

        await page.selectOption("#slicerConverterFormat", "goo");
        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toMatch(/\.goo$/);
    });

    test("converts VDT to CTB / converts VDT to CTB 用例", async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.vdt", "VDT");

        await selectChituboxCtbTarget(page);
        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toMatch(/\.ctb$/);
    });

    test("converts GOO to PW0 / converts GOO to PW0 用例", async ({ page }) => {
        await page.goto("/");
        await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
        await loadFixtureFile(page, "sample.goo", "GOO");

        await page.selectOption("#slicerConverterFormat", "anycubic-manual");
        await page.selectOption("#slicerConverterSuffix", "pw0");
        await page.selectOption("#slicerConverterVersion", "1");
        const downloadPromise = page.waitForEvent("download", { timeout: 60_000 });
        await page.click("#convertBtn");
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toMatch(/\.pw0$/);
    });
});
