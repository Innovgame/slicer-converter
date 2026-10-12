import { act, fireEvent, render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { afterEach, describe, expect, it, vi } from "vitest";

import { StatusBanner } from "../../../src/features/converter/ui/status-banner";
import { createAppI18n } from "../../../src/shared/i18n/i18n";

const ready = { type: "success" as const, message: "Ready - prewarmed 1 worker(s), scales up to 8 on demand." };

describe("Mobile inline status / 移动端行内状态", () => {
    afterEach(() => vi.useRealTimers());

    it("keeps empty-file readiness quiet while still surfacing initialization and errors / 空文件就绪不重复展示但保留初始化与错误", () => {
        const view = render(
            <StatusBanner
                id="statusMsg"
                status={{ type: "info", message: "Loading WASM..." }}
                mobile
                documentLoaded={false}
            />
        );
        expect(screen.getByRole("button", { name: /Getting ready/ })).toBeVisible();
        view.rerender(<StatusBanner id="statusMsg" status={ready} mobile documentLoaded={false} />);
        expect(screen.queryByRole("button")).not.toBeInTheDocument();
        view.rerender(
            <StatusBanner
                id="statusMsg"
                status={{ type: "error", message: "Parse error: corrupt header" }}
                mobile
                documentLoaded={false}
            />
        );
        expect(screen.getByRole("button", { name: /Could not open file/ })).toBeVisible();
    });

    it("shows concise status and opens localized full details / 简短状态可展开本地化完整详情", () => {
        render(
            <I18nextProvider i18n={createAppI18n("zh")}>
                <StatusBanner id="statusMsg" status={ready} mobile />
            </I18nextProvider>
        );
        const trigger = screen.getByRole("button", { name: /就绪/ });
        expect(trigger).not.toHaveTextContent("处理线程");
        fireEvent.click(trigger);
        expect(trigger).toHaveAttribute("aria-expanded", "true");
        expect(screen.getByRole("region", { name: "状态详情" })).toHaveTextContent("已预热 1 个处理线程");
        fireEvent.keyDown(document, { key: "Escape" });
        expect(screen.queryByRole("region")).not.toBeInTheDocument();
        expect(trigger).toHaveFocus();
    });

    it("pauses dismissal while details are open and reveals a new error / 查看详情暂停淡出且新错误重新显示", () => {
        vi.useFakeTimers();
        const view = render(<StatusBanner id="statusMsg" status={ready} mobile />);
        fireEvent.click(screen.getByRole("button", { name: /Ready/ }));
        act(() => vi.advanceTimersByTime(6000));
        expect(screen.getByRole("region")).toBeVisible();
        fireEvent.click(screen.getByRole("button", { name: "Close status details" }));
        act(() => vi.advanceTimersByTime(5000));
        expect(screen.queryByRole("button", { name: /Ready/ })).not.toBeInTheDocument();
        view.rerender(
            <StatusBanner id="statusMsg" status={{ type: "error", message: "Parse error: corrupt header" }} mobile />
        );
        act(() => vi.advanceTimersByTime(10000));
        const trigger = screen.getByRole("button", { name: /Could not open file/ });
        fireEvent.click(trigger);
        expect(screen.getByRole("region")).toHaveTextContent("corrupt header");
    });

    it("keeps current conversion progress visible over stale success / 转换进度覆盖旧成功状态并持续显示", () => {
        vi.useFakeTimers();
        const view = render(<StatusBanner id="statusMsg" status={ready} mobile />);
        act(() => vi.advanceTimersByTime(5000));
        view.rerender(
            <StatusBanner
                id="statusMsg"
                status={ready}
                mobile
                converting
                progress={{ label: "Converting...", percent: 42 }}
            />
        );
        act(() => vi.advanceTimersByTime(10000));
        expect(screen.getByRole("button", { name: /Converting.*42%/ })).toBeVisible();
    });

    it("preserves the complete desktop status without dismissal / 桌面端完整状态不自动消失", () => {
        vi.useFakeTimers();
        render(<StatusBanner id="statusMsg" status={ready} />);
        act(() => vi.advanceTimersByTime(10000));
        expect(screen.getByText(/Ready - prewarmed/)).toBeVisible();
        expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
});
