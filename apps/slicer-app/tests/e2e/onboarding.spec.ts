import { expect, test, type Page } from "@playwright/test";

import { loadFixtureFile } from "../support/fixture-files";

const dialog = (page: Page) => page.locator(".sc-tour-popover");

test("offers an optional first-visit tour and remembers dismissal / 首次可选引导并记住跳过", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("region", { name: "Welcome to Slicer File Converter" })).toBeVisible();
    await expect(dialog(page)).toHaveCount(0);
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await page.reload();
    await expect(page.getByRole("region", { name: "Welcome to Slicer File Converter" })).toHaveCount(0);
    await page.getByRole("button", { name: "Quick tour", exact: true }).click();
    await expect(dialog(page)).toHaveAttribute("role", "dialog");
    await expect(dialog(page)).toContainText("Open your slicer file");
    await page.keyboard.press("Escape");
    await expect(dialog(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Quick tour", exact: true })).toBeFocused();
});

test("walks through empty-state anchors with back and finish / 空态引导支持返回与完成", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Start tour", exact: true }).click();
    await expect(dialog(page)).toContainText("1 of 5");
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await expect(dialog(page)).toContainText("Inspect the layers");
    await dialog(page).getByRole("button", { name: "Back", exact: true }).click();
    await expect(dialog(page)).toContainText("Open your slicer file");
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await expect(dialog(page)).toContainText("Choose an export format");
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await expect(dialog(page)).toContainText("Report issues and share ideas");
    await expect(page.locator('[data-tour="feedback"]')).toHaveClass(/driver-active-element/);
    await expect(dialog(page)).toContainText("Report a bug");
    await expect(dialog(page)).toContainText("Request a feature");
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await expect(dialog(page)).toContainText("Make it yours");
    await dialog(page).getByRole("button", { name: "Finish", exact: true }).click();
    await expect(dialog(page)).toHaveCount(0);
    await page.reload();
    await expect(page.getByRole("region", { name: "Welcome to Slicer File Converter" })).toHaveCount(0);
});

test("highlights loaded controls without changing the document or converting / 已打开文件引导不改变文档或触发转换", async ({
    page
}) => {
    let downloads = 0;
    page.on("download", () => downloads++);
    await page.goto("/");
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    await loadFixtureFile(page, "sample.goo", "GOO");
    await page.locator("#layerSlider").fill("1");
    await page.getByRole("button", { name: "Quick tour", exact: true }).click();
    await expect(dialog(page)).toContainText("1 of 7");
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await expect(dialog(page)).toContainText("Jump to a layer");
    await expect(page.locator(".sc-rail-copy")).toHaveClass(/driver-active-element/);
    for (let step = 0; step < 4; step++) await dialog(page).getByRole("button", { name: "Next", exact: true }).click();
    await dialog(page).getByRole("button", { name: "Finish", exact: true }).click();
    await expect(page.locator("#fileFormat")).toHaveText("GOO");
    await expect(page.locator("#layerNum")).toHaveText("1");
    expect(downloads).toBe(0);
});

test("localizes the tour and fits small screens / 中文引导适配窄屏", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
    await page.goto("/");
    await page.getByRole("button", { name: "Not now", exact: true }).click();
    await page.getByRole("tab", { name: "Settings", exact: true }).click();
    await page.getByRole("group", { name: "Language" }).getByRole("button", { name: "中文" }).click();
    await page.getByRole("button", { name: "新手引导", exact: true }).click();
    await expect(dialog(page)).toContainText("打开切片文件");
    await expect(dialog(page)).toContainText("第 1 / 5 步");
    for (let step = 0; step < 5; step++) {
        const bounds = (await dialog(page).boundingBox())!;
        expect(bounds.x).toBeGreaterThanOrEqual(0);
        expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
        expect(bounds.y).toBeGreaterThanOrEqual(0);
        expect(bounds.y + bounds.height).toBeLessThanOrEqual(844);
        if (step === 3) {
            await expect(dialog(page)).toContainText("报告问题与反馈建议");
            await expect(dialog(page)).toContainText("功能建议");
            await expect(page.locator('[data-tour="feedback"]')).toHaveClass(/driver-active-element/);
        }
        const button = step === 4 ? "完成" : "下一步";
        await dialog(page).getByRole("button", { name: button, exact: true }).click();
    }
    await expect(dialog(page)).toHaveCount(0);
});

test("keeps keyboard focus in the tour and restores it on exit / 键盘焦点限制在引导内并在退出后恢复", async ({
    page
}) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Quick tour", exact: true }).click();
    await expect(dialog(page)).toBeVisible();
    for (let tab = 0; tab < 6; tab++) {
        await page.keyboard.press("Tab");
        expect(await dialog(page).evaluate((element) => element.contains(document.activeElement))).toBe(true);
    }
    await page.keyboard.press("ArrowRight");
    await expect(dialog(page)).toContainText("Inspect the layers");
    await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
    await page.keyboard.press("ArrowLeft");
    await expect(dialog(page)).toContainText("Open your slicer file");
    await dialog(page).getByRole("button", { name: "Skip tour" }).click();
    await expect(dialog(page)).toHaveCount(0);
    await expect(page.locator(".driver-overlay")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Quick tour", exact: true })).toBeFocused();
});

test("remains usable when browser preference storage is unavailable / 浏览器存储不可用时仍能使用引导", async ({
    page
}) => {
    await page.addInitScript(() => {
        Storage.prototype.getItem = () => {
            throw new Error("Storage unavailable");
        };
        Storage.prototype.setItem = () => {
            throw new Error("Storage unavailable");
        };
    });
    await page.goto("/");
    await page.getByRole("button", { name: "Start tour", exact: true }).click();
    await expect(dialog(page)).toContainText("Open your slicer file");
    await page.keyboard.press("Escape");
    await expect(dialog(page)).toHaveCount(0);
    await page.getByRole("button", { name: "Quick tour", exact: true }).click();
    await expect(dialog(page)).toContainText("Open your slicer file");
});
