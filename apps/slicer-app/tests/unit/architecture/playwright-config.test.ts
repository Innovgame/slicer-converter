import { afterEach, describe, expect, it, vi } from "vitest";

async function loadPlaywrightConfig() {
    vi.resetModules();
    const mod = await import("../../../playwright.config");
    return mod.default;
}

describe("playwright config / playwright config 用例", () => {
    afterEach(() => {
        delete process.env.PLAYWRIGHT_USE_EXISTING_SERVER;
        delete process.env.CI;
    });

    it("keeps the managed web server by default / 保持 the managed web server by default", async () => {
        delete process.env.CI;
        const config = await loadPlaywrightConfig();

        expect(config.webServer).toMatchObject({
            command: "pnpm dev",
            url: "http://localhost:8080"
        });
    }, 15_000);

    it("starts Vite directly in CI / CI 中直接启动 Vite", async () => {
        process.env.CI = "true";
        const config = await loadPlaywrightConfig();

        expect(config.webServer).toMatchObject({
            command: "pnpm exec vite",
            url: "http://localhost:8080"
        });
        expect(config.testIgnore).toEqual([/uvtools-compat\.spec\.ts/, /roundtrip\.spec\.ts/]);
    }, 15_000);

    it("can skip the managed web server when reusing an external server / can skip the managed web server 当 reusing an external server", async () => {
        process.env.PLAYWRIGHT_USE_EXISTING_SERVER = "1";

        const config = await loadPlaywrightConfig();

        expect(config.webServer).toBeUndefined();
    }, 15_000);
});
