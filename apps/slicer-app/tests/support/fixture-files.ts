import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, type Page } from "@playwright/test";

const appTestRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(appTestRoot, "../../..");
const rustFixtureRoot = path.resolve(repoRoot, "rust/crates/core/tests/fixtures");

export function repoFilePath(...segments: string[]): string {
    return path.join(repoRoot, ...segments);
}

export function fixtureFilePath(filename: string): string {
    return path.join(rustFixtureRoot, filename);
}

export async function loadFilePath(page: Page, filePath: string, expectedFormat: string): Promise<void> {
    await page.setInputFiles("#fileInput", filePath);
    await expect(page.locator("#fileFormat")).toHaveText(expectedFormat, { timeout: 15_000 });
}

export async function loadFixtureFile(page: Page, filename: string, expectedFormat: string): Promise<void> {
    await loadFilePath(page, fixtureFilePath(filename), expectedFormat);
}
