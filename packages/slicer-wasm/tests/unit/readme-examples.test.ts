import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const readme = readFileSync(resolve(process.cwd(), "README.md"), "utf8");

describe("README browser API examples / README 浏览器 API 示例", () => {
    it("documents the external Goo-to-target conversion flow / 记录外部 Goo 转目标格式流程", () => {
        expect(readme).toContain("## External Browser App: Goo To Target");
        expect(readme).toContain("initSlicerConverter");
        expect(readme).toContain("syncFormatFacts(api.getSupportedFormats())");
        expect(readme).toContain("prepareSlicerExportTargetFromMatrix");
        expect(readme).toContain('sourceFormat: "goo"');
        expect(readme).toContain("fileNameStem: input.fileNameStem");
        expect(readme).toContain("convertSlicerFileToBlob");
        expect(readme).toContain("new Worker(new URL");
        expect(readme).toContain("result.blob");
    });
});
