import path from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vitest/config";

const appRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
    resolve: {
        alias: {
            "@": path.resolve(appRoot, "src"),
            "@slicer-converter/wasm": path.resolve(appRoot, "../../packages/slicer-wasm/src/index.ts")
        }
    },
    test: {
        environment: "jsdom",
        setupFiles: ["./tests/unit/setup.ts"],
        include: ["tests/**/*.test.ts", "tests/**/*.test.tsx"],
        coverage: {
            provider: "v8",
            reporter: ["text-summary", "html", "lcov"],
            reportsDirectory: "./coverage",
            exclude: ["tests/**", "**/*.d.ts"],
            thresholds: {
                statements: 25,
                branches: 20,
                functions: 25,
                lines: 25
            }
        }
    }
});
