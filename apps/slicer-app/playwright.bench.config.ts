import { defineConfig, devices } from "@playwright/test";

const baseURL = "http://localhost:8080";
const useExistingServer = process.env.PLAYWRIGHT_USE_EXISTING_SERVER === "1";

export default defineConfig({
    testDir: "./tests/bench",
    testMatch: /.*\.bench\.spec\.ts$/,
    fullyParallel: false,
    forbidOnly: !!process.env.CI,
    retries: 0,
    workers: 1,
    timeout: 15 * 60_000,
    reporter: "list",
    use: {
        baseURL,
        trace: "off",
        actionTimeout: 60_000,
        launchOptions: {
            args: ["--enable-precise-memory-info"]
        }
    },
    projects: [
        {
            name: "chromium-bench",
            use: {
                ...devices["Desktop Chrome"],
                ...(process.env.CI ? {} : { channel: "chrome" })
            }
        }
    ],
    webServer: useExistingServer
        ? undefined
        : {
              command: "pnpm dev",
              url: baseURL,
              reuseExistingServer: !process.env.CI,
              timeout: 120_000
          }
});
