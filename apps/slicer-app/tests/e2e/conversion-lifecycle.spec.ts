import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { fixtureFilePath, loadFixtureFile } from "../support/fixture-files";

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("slicer-language", "en");
        localStorage.setItem("slicer-onboarding-v1", "dismissed");
        Object.defineProperty(window, "showSaveFilePicker", { configurable: true, value: undefined });
    });
});

test("cancels conversion and immediately retries with a working pool / 取消后立即重试且 worker 池可恢复", async ({
    page
}) => {
    await page.addInitScript(() => {
        const original = Worker.prototype.postMessage;
        const state = window as typeof window & {
            conversionDispatchEntered?: boolean;
            releaseConversionDispatch?: () => void;
        };
        let held = true;
        const pending: (() => void)[] = [];
        state.releaseConversionDispatch = () => {
            held = false;
            pending.splice(0).forEach((send) => send());
        };
        Worker.prototype.postMessage = function (
            message: unknown,
            options?: Transferable[] | StructuredSerializeOptions
        ) {
            const send = () =>
                original.call(this, message, { transfer: Array.isArray(options) ? options : options?.transfer });
            const request = message as { type?: string };
            if (held && request.type?.includes("decode-encode")) {
                state.conversionDispatchEntered = true;
                pending.push(send);
            } else send();
        };
    });
    await page.goto("/");
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    await loadFixtureFile(page, "sample.goo", "GOO");
    await page.selectOption("#slicerConverterFormat", "chitubox-manual");
    await page.selectOption("#slicerConverterSuffix", "ctb");
    let downloads = 0;
    page.on("download", () => downloads++);
    await page.click("#convertBtn");
    await page.waitForFunction(
        () => (window as typeof window & { conversionDispatchEntered?: boolean }).conversionDispatchEntered === true
    );
    await page.click("#cancelBtn");
    await expect(page.locator("#statusMsg")).toContainText("cancelled");
    expect(downloads).toBe(0);
    await page.evaluate(() =>
        (window as typeof window & { releaseConversionDispatch?: () => void }).releaseConversionDispatch?.()
    );
    await expect(page.locator("#convertBtn")).toBeEnabled();
    const download = page.waitForEvent("download");
    await page.click("#convertBtn");
    expect((await download).suggestedFilename()).toMatch(/\.ctb$/);
    await expect(page.locator("#statusMsg")).toContainText("Converted");
    expect(downloads).toBe(1);
});

test("keeps the second file when the first file finishes reading later / 首个文件后完成读取时保留第二个文件", async ({
    page
}) => {
    await page.addInitScript(() => {
        const state = window as typeof window & { releaseOlderRead?: () => void; olderReads?: number };
        let released = false;
        const waiting: (() => void)[] = [];
        state.olderReads = 0;
        state.releaseOlderRead = () => {
            released = true;
            waiting.splice(0).forEach((resolve) => resolve());
        };
        const slice = File.prototype.slice;
        File.prototype.slice = function (...args: Parameters<File["slice"]>) {
            const blob = slice.apply(this, args);
            if (this.name === "older.goo") {
                const arrayBuffer = blob.arrayBuffer.bind(blob);
                blob.arrayBuffer = async () => {
                    if (!released) await new Promise<void>((resolve) => waiting.push(resolve));
                    const bytes = await arrayBuffer();
                    state.olderReads = (state.olderReads ?? 0) + 1;
                    return bytes;
                };
            }
            return blob;
        };
    });
    await page.goto("/");
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    await page.setInputFiles("#fileInput", {
        name: "older.goo",
        mimeType: "application/octet-stream",
        buffer: await readFile(fixtureFilePath("sample.goo"))
    });
    await loadFixtureFile(page, "sample.ctb", "CTB");
    await page.evaluate(() => (window as typeof window & { releaseOlderRead?: () => void }).releaseOlderRead?.());
    await page.waitForFunction(() => ((window as typeof window & { olderReads?: number }).olderReads ?? 0) >= 11);
    await expect(page.locator("#fileName")).toHaveText("sample.ctb");
    await expect(page.locator("#fileFormat")).toHaveText("CTB");
    await expect(page.locator("#statusMsg")).toContainText("Parsed CTB");
});
