import { defineConfig, devices } from "@playwright/test";

const baseURL = "http://localhost:8080";
const useExistingServer = process.env.PLAYWRIGHT_USE_EXISTING_SERVER === "1";
// CI builds release WASM before starting Vite. `pnpm dev` would rebuild it inside predev.
const isCi = process.env.CI === "true";

export default defineConfig({
    testDir: "./tests/e2e",
    testIgnore: isCi ? [/uvtools-compat\.spec\.ts/, /roundtrip\.spec\.ts/] : undefined,
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
              command: isCi ? "pnpm exec vite" : "pnpm dev",
              url: baseURL,
              reuseExistingServer: !isCi,
              timeout: isCi ? 180_000 : 60_000
          }
});
