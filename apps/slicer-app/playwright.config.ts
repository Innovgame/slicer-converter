import { defineConfig, devices } from "@playwright/test";

const baseURL = "http://localhost:8080";
const useExistingServer = process.env.PLAYWRIGHT_USE_EXISTING_SERVER === "1";

export default defineConfig({
    testDir: "./tests/e2e",
    fullyParallel: true,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 1 : 0,
    workers: process.env.CI ? 1 : undefined,
    reporter: "list",
    use: {
        baseURL,
        trace: "on-first-retry"
    },
    projects: [
        {
            name: "chromium",
            use: {
                ...devices["Desktop Chrome"],
                ...(process.env.CI ? {} : { channel: "chrome" })
            }
        },
        {
            name: "firefox",
            use: {
                ...devices["Desktop Firefox"]
            }
        }
    ],
    webServer: useExistingServer
        ? undefined
        : {
              command: "pnpm dev",
              url: baseURL,
              reuseExistingServer: !process.env.CI,
              timeout: 60_000
          }
});
