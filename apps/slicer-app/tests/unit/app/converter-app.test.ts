import React from "react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../../src/app/providers";
import { ConverterPage } from "../../../src/app/routes/converter-page";

vi.mock("../../../../src/features/converter/model/use-converter-controller", () => ({
    useConverterController: () => ({
        wasmReady: true,
        status: { type: "success", message: "Ready - 4 worker(s) initialized." },
        sourceFile: null,
        parsedHeader: null,
        selectedLayer: 0,
        previewImage: null,
        progress: null,
        slicerConverterFormatId: "chitubox-manual",
        slicerConverterSuffixId: "ctb",
        slicerConverterVersionId: "ctb",
        outputFileName: "sample",
        converting: false,
        onOutputFileNameChange: vi.fn(),
        onFileSelect: vi.fn(),
        onSelectSlicerConverterFormat: vi.fn(),
        onSelectSlicerConverterSuffix: vi.fn(),
        onSelectSlicerConverterVersion: vi.fn(),
        onConvert: vi.fn(),
        onCancel: vi.fn(),
        onPreviewLayerChange: vi.fn()
    })
}));

describe("ConverterPage / 转换器页面", () => {
    it("keeps use-converter-controller as a thin facade over sub-controllers / 保持 使用-converter-controller as a thin facade over sub-controllers", async () => {
        const { useConverterController } =
            await import("../../../src/features/converter/model/use-converter-controller");
        expect(typeof useConverterController).toBe("function");
    });

    it("moves runtime bootstrap behind a dedicated runtime hook / moves runtime bootstrap behind a dedicated runtime hook 用例", async () => {
        const { useRuntimeController } = await import("../../../src/app/runtime/use-runtime-controller");
        expect(typeof useRuntimeController).toBe("function");
    });

    it("keeps the existing step structure and stable ids / 保持 the existing step structure 并且 stable ids", () => {
        render(React.createElement(ThemeProvider, null, React.createElement(ConverterPage)));

        expect(screen.getByText("Slicer File Converter")).toBeInTheDocument();
        expect(screen.getByRole("link", { name: "Open the slicer-converter GitHub repository" })).toHaveAttribute(
            "href",
            "https://github.com/Innovgame/slicer-converter"
        );
        expect(screen.getByRole("link", { name: "Report a bug" })).toHaveAttribute(
            "href",
            "https://github.com/Innovgame/slicer-converter/issues/new?template=bug_report.md"
        );
        expect(screen.getByRole("link", { name: "Request a feature" })).toHaveAttribute(
            "href",
            "https://github.com/Innovgame/slicer-converter/issues/new?template=feature_request.md"
        );
        expect(screen.queryByRole("button", { name: "Load sample.goo" })).not.toBeInTheDocument();
        expect(screen.queryByText("Quick test with sample files")).not.toBeInTheDocument();
        expect(document.getElementById("statusMsg")).not.toBeNull();
        expect(document.getElementById("convertBtn")).not.toBeNull();
        expect(document.getElementById("layerSlider")).not.toBeNull();
        expect(document.getElementById("layerNum")).not.toBeNull();
    });

    it("orders slicer-converter target controls as filename, format, suffix, version / 按文件名、格式、后缀、版本排序目标控件", () => {
        const source = readFileSync(resolve(process.cwd(), "src/features/converter/ui/convert-panel.tsx"), "utf8");
        const fileNameIndex = source.indexOf('htmlFor="slicerConverterFileName"');
        const formatIndex = source.indexOf('htmlFor="slicerConverterFormat"');
        const suffixIndex = source.indexOf('htmlFor="slicerConverterSuffix"');
        const versionIndex = source.indexOf('htmlFor="slicerConverterVersion"');

        expect(fileNameIndex).toBeGreaterThan(-1);
        expect(formatIndex).toBeGreaterThan(fileNameIndex);
        expect(suffixIndex).toBeGreaterThan(formatIndex);
        expect(versionIndex).toBeGreaterThan(suffixIndex);
        expect(source).toContain("hasSlicerConverterVersionOptions ? (");
        expect(source).toContain('option.id !== "default"');
        expect(source).toContain('option.name !== "默认"');
    });
});
