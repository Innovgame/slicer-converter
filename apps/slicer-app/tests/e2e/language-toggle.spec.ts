import { expect, test } from "@playwright/test";

import { loadFixtureFile } from "../support/fixture-files";

test("switches language without losing the file or selected layer / 切换语言保留文件与当前层", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/");
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    await loadFixtureFile(page, "sample.goo", "GOO");
    await page.locator("#layerSlider").fill("1");
    const filename = await page.locator("#fileName").textContent();
    await page.getByRole("button", { name: "Switch to Chinese" }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
    await expect(page.getByRole("heading", { name: "源文件", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "转换", exact: true })).toBeEnabled();
    await expect(page.locator("#statusMsg")).toContainText("已解析");
    await expect(page.locator("#fileName")).toHaveText(filename!);
    await expect(page.locator("#layerNum")).toHaveText("1");
    await expect(page.getByRole("textbox", { name: "层数" })).toHaveValue("2");
    await page.getByRole("button", { name: "切换到浅色主题" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await expect(page.getByRole("link", { name: "报告问题" })).toBeVisible();

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "转换", exact: true }).click();
    expect((await download).suggestedFilename()).toMatch(/\.ctb$/);
    await expect(page.locator("#statusMsg")).toContainText("已将");

    await page.reload();
    await expect(page.getByRole("heading", { name: "源文件", exact: true })).toBeVisible();
    await expect(page.locator("#statusMsg")).toContainText("就绪", { timeout: 30_000 });
    await page.getByRole("button", { name: "Switch to English" }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(page.getByRole("heading", { name: "Source", exact: true })).toBeVisible();
    await expect(page.locator("#statusMsg")).toContainText("Ready");
});

test("keeps language segments accessible on small screens / 窄屏语言分段控件保持可用", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await page.getByRole("tab", { name: "Settings", exact: true }).click();
    await page.getByRole("group", { name: "Language" }).getByRole("button", { name: "中文" }).click();
    const english = page.getByRole("group", { name: "语言" }).getByRole("button", { name: "EN" });
    await expect(english).toBeVisible();
    await english.click();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(page.getByRole("tab", { name: "Settings", exact: true })).toHaveAttribute("aria-selected", "true");
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
});
