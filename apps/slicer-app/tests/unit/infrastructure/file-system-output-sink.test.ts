import { existsSync } from "node:fs";

import { describe, expect, it } from "vitest";

describe("file-system-output-sink boundary / File System output sink 边界", () => {
    it("does not keep an app-local File System sink wrapper / 不保留 app 本地 File System sink wrapper", () => {
        expect(existsSync("src/infrastructure/output/file-system-output-sink.ts")).toBe(false);
    });
});
