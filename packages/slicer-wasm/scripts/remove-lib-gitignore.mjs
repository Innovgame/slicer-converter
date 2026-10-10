import { rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const libGitignore = join(dirname(fileURLToPath(import.meta.url)), "../lib/.gitignore");

rmSync(libGitignore, { force: true });
