import { readFileSync } from "node:fs";
import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ParsedHeader } from "@slicer-converter/wasm";
import { PreviewPanel } from "../../../src/features/converter/ui/preview-panel";

const parsedHeader = {
    format: "goo",
    metadata: {
        resolution_x: 3840,
        resolution_y: 2400,
        anti_alias_level: 1,
        encryption_key: 0
    },
    layers: [{ offset: 0, size: 10 }]
} satisfies ParsedHeader;

const multiLayerParsedHeader = {
    ...parsedHeader,
    layers: Array.from({ length: 10 }, (_, index) => ({ offset: index * 10, size: 10 }))
} satisfies ParsedHeader;

describe("PreviewPanel / 预览面板", () => {
    afterEach(() => {
        vi.restoreAllMocks();
        vi.useRealTimers();
    });

    it("copies the preview bitmap without sizing the viewport to native pixels / 复制 preview bitmap 且不把视口撑成原生像素", () => {
        const drawImage = vi.fn();
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect: vi.fn(),
            drawImage
        } as unknown as CanvasRenderingContext2D);

        const { container, rerender } = render(
            React.createElement(PreviewPanel, {
                parsedHeader,
                selectedLayer: 0,
                previewImage: createPreviewImage(512, 320),
                onPreviewLayerChange: vi.fn()
            })
        );

        const canvas = container.querySelector<HTMLCanvasElement>("#previewCanvas");
        expect(canvas).not.toBeNull();
        expect(canvas?.style.width).toBe("");
        expect(drawImage).toHaveBeenLastCalledWith(expect.anything(), 0, 0, 512, 320);

        rerender(
            React.createElement(PreviewPanel, {
                parsedHeader,
                selectedLayer: 0,
                previewImage: createPreviewImage(2048, 1280),
                onPreviewLayerChange: vi.fn()
            })
        );

        expect(canvas?.style.width).toBe("");
        expect(drawImage).toHaveBeenLastCalledWith(expect.anything(), 0, 0, 2048, 1280);
        expect(screen.getByRole("button", { name: "Fit window" })).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Pixel grid" })).toBeInTheDocument();
        expect(container.querySelector("#layerSlider")).toHaveAttribute("aria-orientation", "vertical");
    });

    it("coalesces dragging changes by animation frame and commits current slider value / 按 animation frame 合并 dragging change 并提交当前 slider 值", () => {
        vi.useFakeTimers();
        const onPreviewLayerChange = vi.fn(async () => undefined);
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect: vi.fn(),
            drawImage: vi.fn()
        } as unknown as CanvasRenderingContext2D);

        const { container, rerender } = render(
            React.createElement(PreviewPanel, {
                parsedHeader: multiLayerParsedHeader,
                selectedLayer: 0,
                previewImage: createPreviewImage(512, 320),
                onPreviewLayerChange
            })
        );

        const slider = container.querySelector<HTMLInputElement>("#layerSlider");
        expect(slider).not.toBeNull();

        fireEvent.pointerDown(slider!, { pointerId: 1 });
        expect(onPreviewLayerChange).toHaveBeenLastCalledWith(0, "dragging");

        fireEvent.change(slider!, { target: { value: "4" } });
        fireEvent.change(slider!, { target: { value: "5" } });

        expect(onPreviewLayerChange).toHaveBeenCalledTimes(1);

        vi.advanceTimersByTime(16);

        expect(onPreviewLayerChange).toHaveBeenCalledTimes(2);
        expect(onPreviewLayerChange).toHaveBeenLastCalledWith(5, "dragging");

        rerender(
            React.createElement(PreviewPanel, {
                parsedHeader: multiLayerParsedHeader,
                selectedLayer: 5,
                previewImage: createPreviewImage(512, 320),
                onPreviewLayerChange
            })
        );
        const updatedSlider = container.querySelector<HTMLInputElement>("#layerSlider");
        fireEvent.change(updatedSlider!, { target: { value: "6" } });
        fireEvent.pointerUp(updatedSlider!);

        expect(onPreviewLayerChange).toHaveBeenCalledTimes(3);
        expect(onPreviewLayerChange).toHaveBeenLastCalledWith(6, "committed");
        vi.advanceTimersByTime(16);
        expect(onPreviewLayerChange).toHaveBeenCalledTimes(3);
    });

    it("keeps the previous layer frame while the selected bitmap is in flight / 等待 selected layer bitmap 时保留上一帧", () => {
        const drawImage = vi.fn();
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect: vi.fn(),
            drawImage
        } as unknown as CanvasRenderingContext2D);

        const { container, rerender } = render(
            React.createElement(PreviewPanel, {
                parsedHeader: multiLayerParsedHeader,
                selectedLayer: 4,
                previewImage: createPreviewImage(512, 320, 4),
                onPreviewLayerChange: vi.fn()
            })
        );

        const canvas = container.querySelector<HTMLCanvasElement>("#previewCanvas");
        expect(canvas?.dataset.previewLayer).toBe("4");
        expect(drawImage).toHaveBeenCalledTimes(1);

        rerender(
            React.createElement(PreviewPanel, {
                parsedHeader: multiLayerParsedHeader,
                selectedLayer: 5,
                previewImage: createPreviewImage(512, 320, 4),
                onPreviewLayerChange: vi.fn()
            })
        );

        expect(canvas?.dataset.previewLayer).toBe("4");
        expect(drawImage).toHaveBeenCalledTimes(1);
    });

    it("keeps canvas backing size out of React props / 不通过 React props 设置 canvas backing 尺寸", () => {
        const source = readFileSync("src/features/converter/ui/preview-panel.tsx", "utf8");
        const canvasTagBody = source.match(/<canvas(?<body>[\s\S]*?)\/>/)?.groups?.body ?? "";
        expect(canvasTagBody).not.toMatch(/\bwidth=\{/);
        expect(canvasTagBody).not.toMatch(/\bheight=\{/);
    });

    it("draws preview bitmap after owning canvas backing size / 接管 canvas backing 尺寸后绘制 preview bitmap", () => {
        const drawImage = vi.fn();
        const info = vi.spyOn(console, "info").mockImplementation(() => undefined);
        vi.spyOn(performance, "now").mockReturnValueOnce(10).mockReturnValueOnce(12.5);
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect: vi.fn(),
            drawImage
        } as unknown as CanvasRenderingContext2D);

        const { container } = render(
            React.createElement(PreviewPanel, {
                parsedHeader,
                selectedLayer: 0,
                previewImage: createPreviewImage(512, 320),
                onPreviewLayerChange: vi.fn()
            })
        );

        const canvas = container.querySelector<HTMLCanvasElement>("#previewCanvas");
        expect(canvas).not.toBeNull();
        expect(drawImage).toHaveBeenLastCalledWith(expect.anything(), 0, 0, 512, 320);
        expect(info).toHaveBeenCalledWith(
            expect.stringMatching(
                /^\[preview-perf\] app canvas layer=0 bitmap=512x320 display=3840x2400 canvasDraw=\d+\.\dms$/
            )
        );
    });
});

function createPreviewImage(width: number, height: number, layerIndex = 0) {
    return {
        bitmap: { close: vi.fn() } as unknown as ImageBitmap,
        layerIndex,
        width,
        height
    };
}
