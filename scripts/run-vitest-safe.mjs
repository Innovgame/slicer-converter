import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";

const originalNativeRealpath = fs.realpathSync.native;
const currentDirectory = path.resolve("./");
const require = createRequire(import.meta.url);

fs.realpathSync.native = function patchedRealpath(target, ...args) {
    if (target === currentDirectory) {
        const error = new Error("EISDIR: illegal operation on a directory");
        throw error;
    }

    return originalNativeRealpath.call(this, target, ...args);
};

process.argv = ["node", "vitest", ...process.argv.slice(2)];

const vitestPackageEntrypoint = require.resolve("vitest", {
    paths: [process.cwd()]
});
const vitestEntrypoint = path.join(path.dirname(vitestPackageEntrypoint), "vitest.mjs");

await import(pathToFileURL(vitestEntrypoint).href);
