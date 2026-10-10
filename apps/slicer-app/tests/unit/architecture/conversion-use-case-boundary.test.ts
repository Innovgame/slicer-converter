import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";

import { describe, expect, it } from "vitest";

describe("conversion use case boundary / 转换 use case 边界", () => {
    it("keeps the React conversion hook as a state and progress adapter / 保持 React 转换 hook 作为状态与进度适配器", async () => {
        const hookSource = await readFile("src/features/conversion/model/use-conversion-controller.ts", "utf8");
        const forbiddenHookDependencies = [
            "buildConversionStatusMessage",
            "createConversionSession",
            "buildZipOutputWithWorkers",
            "convertLayersWithWorkers",
            "buildDownloadFilename",
            "openFileSystemSink",
            "getFormatFacts",
            "createOrderedLayerQueue",
            "buildLayerMetadataJson",
            "createBlobSink",
            "createBufferedOutputSink",
            "snapshotBufferedOutputSinkStats",
            "createRuntimeMetrics",
            "assembleWasmBinaryOutputFromLayerStream",
            "resolveAnycubicExportTarget"
        ];

        expect(hookSource).toContain("runConversionUseCase");
        expect(hookSource).toContain("conversionTarget");
        expect(hookSource).not.toContain("resolveSlicerExportTarget");
        for (const dependency of forbiddenHookDependencies) {
            expect(hookSource).not.toContain(dependency);
        }
    });

    it("delegates reusable conversion mechanics to the wasm package / 将可复用转换机制委托给 wasm package", async () => {
        const useCaseSource = await readFile("src/features/conversion/model/conversion-use-case.ts", "utf8");
        const forbiddenUseCaseDependencies = [
            "runConversionJob",
            "runConversionStrategy",
            "createConversionSession",
            "buildZipOutputWithWorkers",
            "convertLayersWithWorkers",
            "createOrderedLayerQueue",
            "assembleWasmBinaryOutputFromLayerStream",
            "buildLayerMetadataJson",
            "createRuntimeMetrics"
        ];

        expect(useCaseSource).toContain("convertSlicerFile");
        expect(useCaseSource).toContain("openConversionOutputTarget");
        for (const dependency of forbiddenUseCaseDependencies) {
            expect(useCaseSource).not.toContain(dependency);
        }
    });

    it("does not keep app-local conversion pipeline modules after sinkdown / 下沉后不保留 app 本地转换管线模块", () => {
        const removedModules = [
            "src/features/conversion/model/conversion-controller.ts",
            "src/features/conversion/model/conversion-job-runner.ts",
            "src/features/conversion/model/conversion-session.ts",
            "src/features/conversion/model/conversion-strategy-executor.ts"
        ];

        for (const modulePath of removedModules) {
            expect(existsSync(modulePath)).toBe(false);
        }
    });

    it("keeps File System Access output decisions in the conversion output adapter / 将 File System Access 输出决策保留在转换输出 adapter", async () => {
        const useCaseSource = await readFile("src/features/conversion/model/conversion-use-case.ts", "utf8");
        const outputTargetSource = await readFile("src/features/conversion/model/conversion-output-target.ts", "utf8");

        expect(useCaseSource).toContain("openConversionOutputTarget");
        expect(useCaseSource).toContain("outputTarget.mode");
        expect(useCaseSource).not.toContain("outputTarget.sink.streaming");
        expect(outputTargetSource).toContain("openFileSystemSink");
        expect(outputTargetSource).toContain("@slicer-converter/wasm");
        expect(outputTargetSource).not.toContain("../../infrastructure/output/file-system-output-sink");
        expect(outputTargetSource).toContain("createBufferedOutputSink");
        expect(outputTargetSource).toContain('mode: "streaming"');
        expect(outputTargetSource).toContain('mode: "blob"');
        expect(outputTargetSource).not.toContain("convertSlicerFile");
        expect(existsSync("src/infrastructure/output/file-system-output-sink.ts")).toBe(false);
    });

    it("consumes resolved conversion targets from the converter feature / 消费 converter feature 已解析的转换目标", async () => {
        const useCaseSource = await readFile("src/features/conversion/model/conversion-use-case.ts", "utf8");
        const hookSource = await readFile("src/features/conversion/model/use-conversion-controller.ts", "utf8");

        expect(useCaseSource).not.toContain("resolveAnycubicExportTarget");
        expect(useCaseSource).not.toContain("export function resolveConversionTarget");
        expect(existsSync("src/domain/conversion/conversion-target.ts")).toBe(false);
        expect(hookSource).not.toContain("resolveSlicerExportTarget");
        expect(hookSource).toContain("conversionTarget: ResolvedSlicerExportTarget | null");
        expect(hookSource).toContain("@slicer-converter/wasm");
        expect(hookSource).toContain("./conversion-use-case");
    });
});
