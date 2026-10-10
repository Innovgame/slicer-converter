import { rmSync } from "node:fs";

rmSync(new URL("../lib/.gitignore", import.meta.url), { force: true });
