import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { I18nextProvider } from "react-i18next";

import { ThemeProvider } from "../../../src/app/providers";
import { AppShell } from "../../../src/features/converter/ui/app-shell";
import { createAppI18n } from "../../../src/shared/i18n/i18n";
import { PreviewPanel } from "../../../src/features/converter/ui/preview-panel";

function viewport(mobile: boolean) {
    vi.stubGlobal("matchMedia", () => ({
        matches: mobile,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn()
    }));
}

function mount() {
    return render(
        <ThemeProvider>
            <AppShell
                title="Converter"
                documentStatus="sample.goo"
                documentLoaded
                sourceSummary="GOO · 10 layers"
                help={<button>Quick tour</button>}
                source={<input aria-label="Source file" defaultValue="sample.goo" />}
                viewer={<canvas aria-label="Exposure canvas" />}
                inspector={<input aria-label="Output filename" defaultValue="sample" />}
                status={<p role="status">Converting 40%</p>}
            />
        </ThemeProvider>
    );
}

describe("Mobile AppShell / 移动工作台", () => {
    afterEach(() => vi.unstubAllGlobals());

    it("preserves the preview and export draft across page switches / 切换页面保留预览与导出草稿", () => {
        viewport(true);
        mount();
        const canvas = screen.getByLabelText("Exposure canvas");
        fireEvent.click(screen.getByRole("button", { name: "Open export settings" }));
        const output = screen.getByRole("textbox", { name: "Output filename" });
        fireEvent.change(output, { target: { value: "custom-output" } });
        fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        expect(screen.getByRole("tabpanel", { name: "Settings" })).toBeVisible();
        expect(canvas.closest("[aria-hidden]")).toHaveAttribute("aria-hidden", "true");
        fireEvent.click(screen.getByRole("tab", { name: "Preview" }));
        expect(screen.getByLabelText("Exposure canvas")).toBe(canvas);
        expect(screen.getByRole("textbox", { name: "Output filename" })).toBe(output);
        expect(output).toHaveValue("custom-output");
        expect(screen.getByRole("status")).toHaveTextContent("Converting 40%");
    });

    it("restores navigation after immersive mode and Escape / 纯画布和 Escape 后恢复导航", () => {
        viewport(true);
        mount();
        fireEvent.click(screen.getByRole("button", { name: "Enter pure canvas" }));
        expect(screen.queryByRole("tab", { name: "Settings" })).not.toBeInTheDocument();
        fireEvent.click(screen.getByRole("button", { name: "Exit pure canvas" }));
        expect(screen.getByRole("tab", { name: "Settings" })).toBeInTheDocument();
        fireEvent.click(screen.getByRole("button", { name: "Enter pure canvas" }));
        fireEvent.keyDown(document, { key: "Escape" });
        expect(screen.getByRole("tab", { name: "Preview" })).toBeInTheDocument();
    });

    it("closes file details and returns focus to their trigger / 关闭文件详情后焦点回到入口", () => {
        viewport(true);
        mount();
        const trigger = screen.getByRole("button", { name: "File details" });
        expect(screen.getByText("sample.goo", { selector: "strong" })).toBeVisible();
        expect(screen.getByText("GOO · 10 layers")).toBeVisible();
        expect(trigger).toHaveAttribute("aria-expanded", "false");
        fireEvent.click(trigger);
        expect(trigger).toHaveAttribute("aria-expanded", "true");
        const drawer = screen.getByRole("dialog", { name: "File details" });
        expect(within(drawer).getByRole("textbox", { name: "Source file" })).toHaveValue("sample.goo");
        fireEvent.click(within(drawer).getByRole("button", { name: "Close panel" }));
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        expect(trigger).toHaveFocus();
    });

    it("opens the file picker from the empty header entry / 空文件标题入口直接打开文件选择器", () => {
        viewport(true);
        const browse = vi.fn();
        render(
            <ThemeProvider>
                <AppShell
                    title="Converter"
                    documentStatus="No file open"
                    documentLoaded={false}
                    help={null}
                    source={null}
                    viewer={<canvas />}
                    inspector={null}
                    status={null}
                    onBrowseFile={browse}
                />
            </ThemeProvider>
        );
        const trigger = screen.getByRole("button", { name: "Open file" });
        expect(trigger).toHaveAccessibleName("Open file");
        expect(screen.queryByRole("button", { name: "File details" })).not.toBeInTheDocument();
        fireEvent.click(trigger);
        expect(browse).toHaveBeenCalledOnce();
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("exits pure canvas with Escape from the focused real viewport / 实际画布获得焦点后 Escape 退出纯画布", () => {
        viewport(true);
        render(
            <ThemeProvider>
                <AppShell
                    title="Converter"
                    documentStatus="sample.goo"
                    documentLoaded
                    help={null}
                    source={null}
                    inspector={null}
                    status={null}
                    viewer={(mobile) => (
                        <PreviewPanel
                            parsedHeader={{
                                format: "goo",
                                metadata: {
                                    resolution_x: 100,
                                    resolution_y: 50,
                                    anti_alias_level: 1,
                                    encryption_key: 0
                                },
                                layers: [{ offset: 0, size: 1 }]
                            }}
                            selectedLayer={0}
                            previewImage={null}
                            onPreviewLayerChange={async () => undefined}
                            mobile={mobile}
                        />
                    )}
                />
            </ThemeProvider>
        );
        fireEvent.click(screen.getByRole("button", { name: "Enter pure canvas" }));
        const canvas = screen.getByLabelText("Layer preview");
        fireEvent.pointerDown(canvas);
        fireEvent.keyDown(canvas, { key: "Escape" });
        expect(screen.getByRole("tab", { name: "Preview" })).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Enter pure canvas" })).toHaveFocus();
    });

    it("keeps desktop panels available without mobile tabs / 桌面保留各面板且不显示移动标签", () => {
        viewport(false);
        mount();
        expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
        expect(screen.getByRole("textbox", { name: "Source file" })).toBeVisible();
        expect(screen.getByRole("textbox", { name: "Output filename" })).toBeVisible();
    });

    it("applies real theme and language preferences from the settings page / 设置页应用真实主题与语言偏好", async () => {
        viewport(true);
        const i18n = createAppI18n("en");
        render(
            <I18nextProvider i18n={i18n}>
                <ThemeProvider>
                    <AppShell
                        title="Converter"
                        documentStatus="sample.goo"
                        documentLoaded
                        help={<button>Guide</button>}
                        source={null}
                        viewer={<canvas />}
                        inspector={null}
                        status={null}
                    />
                </ThemeProvider>
            </I18nextProvider>
        );
        fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        fireEvent.click(
            within(screen.getByRole("group", { name: "Appearance" })).getByRole("button", { name: "Dark" })
        );
        expect(document.documentElement).toHaveAttribute("data-theme", "dark");
        fireEvent.click(within(screen.getByRole("group", { name: "Language" })).getByRole("button", { name: "中文" }));
        expect(await screen.findByRole("tab", { name: "设置" })).toHaveAttribute("aria-selected", "true");
        fireEvent.click(screen.getByRole("tab", { name: "预览" }));
        expect(screen.getByRole("tab", { name: "预览" })).toHaveAttribute("aria-selected", "true");
    });
});
