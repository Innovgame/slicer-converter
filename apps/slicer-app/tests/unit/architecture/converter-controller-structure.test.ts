import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("converter controller structure / 转换器控制器结构", () => {
    it("keeps app modules grouped by standardized feature roles / 按标准 feature 角色组织 app modules", () => {
        expect(existsSync(path.join(process.cwd(), "src/core"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/app/routes"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/app/providers"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/app/runtime"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/features/converter/model"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/features/converter/ui"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/features/conversion/model"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/features/document-opening/model"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/features/preview/model"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/application/conversion"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/domain/conversion"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/application/preview"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/domain/format"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/archive"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/output/download.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/wasm"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/wasm/wasm-format-facts.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/infrastructure/worker"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/shared/lib"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/shared/errors.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/shared/format.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/shared/runtime-metrics.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/shared/lib/runtime-metrics.ts"))).toBe(false);
    });

    it("keeps shared controller contracts outside React hooks / 共享控制器契约不放在 hooks 中", () => {
        expect(existsSync(path.join(process.cwd(), "src/app/hooks/converter-controller-shared.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/features/converter/model/converter-view-state.ts"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/app/controller/preview-cache.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/app/controller/runtime-bootstrap-state.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/app/hooks/use-runtime-controller.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/app/hooks/use-document-controller.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/app/hooks/use-preview-controller.ts"))).toBe(false);
    });

    it("keeps the converter feature barrel focused on public page composition / converter feature barrel 只暴露页面组合边界", () => {
        const converterIndexSource = readFileSync(path.join(process.cwd(), "src/features/converter/index.ts"), "utf8");

        expect(converterIndexSource).toContain("export { useConverterController }");
        expect(converterIndexSource).toContain("export type { ConverterController }");
        expect(converterIndexSource).not.toContain("PreviewLayerInteraction");
        expect(converterIndexSource).not.toContain("PreviewImageState");
        expect(converterIndexSource).not.toContain("ProgressState");
        expect(converterIndexSource).not.toContain("StatusState");
    });

    it("keeps unit tests grouped by tested layer / 单元测试按被测层分组", () => {
        const unitTestRoot = path.join(process.cwd(), "tests/unit");
        const entries = readdirSync(unitTestRoot, { withFileTypes: true });

        expect(
            entries.filter((entry) => entry.isFile() && /\.test\.tsx?$/.test(entry.name)).map((entry) => entry.name)
        ).toEqual([]);
        expect(existsSync(path.join(unitTestRoot, "application"))).toBe(false);
        expect(
            entries
                .filter((entry) => entry.isDirectory())
                .map((entry) => entry.name)
                .sort()
        ).toEqual(["app", "architecture", "domain", "features", "infrastructure", "shared", "support", "worker"]);
    });
});
