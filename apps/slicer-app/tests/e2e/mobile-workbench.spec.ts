import { expect, test } from "@playwright/test";
import { loadFixtureFile } from "../support/fixture-files";

test.use({ viewport: { width: 390, height: 844 } });

test("keeps preview geometry and export choices across mobile tabs / 移动标签切换保留预览几何与导出选择", async ({
    page
}) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    await loadFixtureFile(page, "sample.goo", "GOO");
    await expect(page.locator("#previewCanvas")).toHaveAttribute("data-preview-layer", "0");
    const canvas = page.locator("#previewCanvas");
    const bounds = await canvas.boundingBox();
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    const zoom = await canvas.getAttribute("data-zoom");
    await page.getByRole("button", { name: "Open export settings" }).click();
    await expect(page.getByRole("dialog", { name: "Export", exact: true })).toBeVisible();
    await page.locator("#slicerConverterFileName").fill("mobile-output");
    expect(await canvas.boundingBox()).toEqual(bounds);
    await page.getByRole("tab", { name: "Settings", exact: true }).click();
    await expect(page.getByRole("tabpanel", { name: "Settings", exact: true })).toBeVisible();
    await expect(page.getByRole("group", { name: "Appearance" })).toBeVisible();
    await page.getByRole("tab", { name: "Preview", exact: true }).click();
    await expect(page.locator("#slicerConverterFileName")).toHaveValue("mobile-output");
    await page
        .getByRole("dialog", { name: "Export", exact: true })
        .getByRole("button", { name: "Close panel" })
        .click();
    await expect(canvas).toHaveAttribute("data-zoom", zoom!);
    await page.getByRole("button", { name: "Enter pure canvas" }).click();
    await expect(page.getByRole("tablist")).toHaveCount(0);
    expect(await canvas.boundingBox()).toEqual(bounds);
    await page.getByRole("button", { name: "Exit pure canvas" }).click();
    await expect(page.getByRole("tab", { name: "Preview", exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
});

test("opens a narrow standalone settings page and localizes navigation / 窄屏独立设置页与本地化导航", async ({
    page
}) => {
    await page.setViewportSize({ width: 320, height: 640 });
    await page.goto("/");
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await page.getByRole("tab", { name: "Settings", exact: true }).click();
    await page.getByRole("group", { name: "Language" }).getByRole("button", { name: "中文" }).click();
    await expect(page.getByRole("tab", { name: "设置", exact: true })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("group", { name: "语言" }).getByRole("button", { name: "中文" })).toHaveAttribute(
        "aria-pressed",
        "true"
    );
    const appearance = page.getByRole("group", { name: "外观" });
    await appearance.getByRole("button", { name: "浅色" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await appearance.getByRole("button", { name: "深色" }).click();
    await expect(appearance.getByRole("button", { name: "深色" })).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.getByRole("group", { name: "语言" }).getByRole("button", { name: "EN" }).click();
    await expect(page.getByRole("group", { name: "Language" }).getByRole("button", { name: "EN" })).toHaveAttribute(
        "aria-pressed",
        "true"
    );
    await expect(page.getByRole("tab", { name: "Settings", exact: true })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("group", { name: "Language" }).getByRole("button", { name: "中文" }).click();
    await expect(page.getByRole("link", { name: "报告问题", exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
});
