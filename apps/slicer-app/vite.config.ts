import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";
import { resolveBuildCommit } from "./build/build-metadata";

const appRoot = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(appRoot, "../..");

export default defineConfig(({ mode }) => ({
    define: {
        "import.meta.env.VITE_BUILD_COMMIT": JSON.stringify(
            resolveBuildCommit({
                projectRoot,
                env: { ...process.env, ...loadEnv(mode, appRoot, "VITE_") }
            })
        )
    },
    plugins: [react(), tailwindcss()],
    resolve: {
        alias: {
            "@": path.resolve(appRoot, "src"),
            "@innovgame/slicer-converter": path.resolve(projectRoot, "packages/slicer-wasm/src/index.ts")
        }
    },
    root: appRoot,
    base: process.env.GITHUB_PAGES === "true" ? "/slicer-converter/" : "/",
    build: {
        rollupOptions: {
            input: {
                main: path.resolve(appRoot, "index.html")
            }
        }
    },
    server: {
        port: 8080,
        fs: {
            allow: [projectRoot]
        }
    },
    preview: {
        port: 8080
    },
    worker: {
        format: "es"
    }
}));
