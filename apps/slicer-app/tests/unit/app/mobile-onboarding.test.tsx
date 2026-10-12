import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Config } from "driver.js";

import { ThemeProvider } from "../../../src/app/providers";
import { ConverterPage } from "../../../src/app/routes/converter-page";

const tour = vi.hoisted(() => ({ config: null as Config | null }));
vi.mock("driver.js", () => ({
    driver: (config: Config) => {
        tour.config = config;
        return { drive() {}, isActive: () => false, destroy() {} };
    }
}));
vi.mock("../../../src/features/converter/model/use-converter-controller", () => ({
    useConverterController: () => ({
        sourceFile: null,
        parsedHeader: null,
        selectedLayer: 0,
        previewImage: null,
        progress: null,
        status: null,
        converting: false,
        wasmReady: true,
        outputFileName: "",
        outputExtension: "",
        exportSummary: "",
        slicerConverterFormatId: "",
        slicerConverterSuffixId: "",
        slicerConverterVersionId: "",
        onFileSelect: vi.fn(),
        onPreviewLayerChange: vi.fn(),
        onOutputFileNameChange: vi.fn(),
        onSelectSlicerConverterFormat: vi.fn(),
        onSelectSlicerConverterSuffix: vi.fn(),
        onSelectSlicerConverterVersion: vi.fn(),
        onConvert: vi.fn(),
        onCancel: vi.fn()
    })
}));

describe("Mobile tour navigation / 移动引导导航", () => {
    afterEach(() => vi.unstubAllGlobals());

    it("reveals each tour target on its page or drawer / 在对应页面或抽屉显示每个引导目标", () => {
        vi.stubGlobal("matchMedia", () => ({ matches: true, addEventListener() {}, removeEventListener() {} }));
        render(
            <ThemeProvider>
                <ConverterPage />
            </ThemeProvider>
        );
        fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        fireEvent.click(screen.getByRole("button", { name: "Quick tour" }));
        const steps = tour.config!.steps!;
        const highlight = (index: number) => act(() => (steps[index].onHighlightStarted as () => void)());
        highlight(0);
        expect(screen.getByRole("dialog", { name: "File details" })).toBeInTheDocument();
        highlight(1);
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        expect(screen.getByRole("tab", { name: "Preview" })).toHaveAttribute("aria-selected", "true");
        highlight(2);
        expect(screen.getByRole("dialog", { name: "Export" })).toBeInTheDocument();
        highlight(3);
        expect(screen.getByRole("tabpanel", { name: "Settings" })).toBeVisible();
        expect(screen.getByRole("link", { name: "Report a bug" })).toBeInTheDocument();
        highlight(4);
        expect(screen.getByRole("combobox", { name: "Appearance" })).toBeInTheDocument();
    });
});
