import eslint from "@eslint/js";
import { defineConfig } from "eslint/config";
import tseslint from "typescript-eslint";

export default defineConfig([
    {
        ignores: [
            "**/dist/**",
            "**/node_modules/**",
            "packages/slicer-wasm/lib/**",
            "apps/slicer-app/dist/**",
            "apps/slicer-app/test-results/**",
            "apps/slicer-app/playwright-report/**",
            "www/pkg/**",
            "rust/target/**",
            "rust/data/**"
        ]
    },
    eslint.configs.recommended,
    ...tseslint.configs.recommended,
    {
        files: ["eslint.config.mjs"],
        languageOptions: {
            globals: {
                process: "readonly",
                console: "readonly",
                URL: "readonly"
            }
        }
    },
    {
        files: ["scripts/**/*.mjs"],
        languageOptions: {
            globals: {
                process: "readonly",
                console: "readonly"
            }
        }
    },
    {
        files: ["**/*.ts"],
        rules: {
            eqeqeq: ["error", "always"],
            "no-var": "error",
            "prefer-const": "warn",
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                    caughtErrorsIgnorePattern: "^_"
                }
            ],
            "@typescript-eslint/no-explicit-any": "warn",
            "@typescript-eslint/consistent-type-imports": "error"
        }
    }
]);
