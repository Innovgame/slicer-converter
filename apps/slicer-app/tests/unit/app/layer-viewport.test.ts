import { describe, expect, it, vi } from "vitest";

import {
    LayerViewport,
    layerPixelToMm,
    layerTopMm,
    type LayerPlate
} from "../../../src/features/converter/ui/layer-viewport";

const plate: LayerPlate = {
    nativeWidth: 100,
    nativeHeight: 50,
    platformWidth: 200,
    platformHeight: 100,
    mirrorX: false,
    mirrorY: false
};

describe("layer viewport plate mapping / 层图板面换算", () => {
    it("converts a pixel center around the plate origin / 把像素中心换到板心原点", () => {
        expect(layerPixelToMm(plate, 0, 0)).toEqual({ x: -99, y: 49 });
        expect(layerPixelToMm(plate, 99, 49)).toEqual({ x: 99, y: -49 });
    });

    it("mirrors axes before converting to millimetres / 换算毫米前镜像坐标轴", () => {
        expect(layerPixelToMm({ ...plate, mirrorX: true, mirrorY: true }, 0, 0)).toEqual({ x: 99, y: -49 });
    });

    it("returns null without a plate size / 没有板面尺寸时返回空", () => {
        expect(layerPixelToMm({ ...plate, platformWidth: 0 }, 1, 1)).toBeNull();
    });

    it("estimates layer-top Z from uniform thickness / 按均匀层厚估算层顶 Z", () => {
        expect(layerTopMm(0, 0.05)).toBeCloseTo(0.05);
        expect(layerTopMm(3, undefined)).toBeCloseTo(0.2);
    });
});

describe("LayerViewport touch and visibility / 层图触摸与可见性", () => {
    function createViewport() {
        const canvas = document.createElement("canvas");
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect() {},
            drawImage() {},
            setTransform() {}
        } as unknown as CanvasRenderingContext2D);
        let visible = true;
        vi.spyOn(canvas, "getBoundingClientRect").mockImplementation(() => ({
            x: 0,
            y: 0,
            left: 0,
            top: 0,
            right: visible ? 400 : 0,
            bottom: visible ? 300 : 0,
            width: visible ? 400 : 0,
            height: visible ? 300 : 0,
            toJSON: () => ({})
        }));
        const viewport = new LayerViewport(canvas, () => undefined);
        viewport.setImage({ bitmap: {} as ImageBitmap, bitmapWidth: 100, bitmapHeight: 50, layerIndex: 0, plate });
        viewport.setZoom(2);
        return {
            canvas,
            viewport,
            hide: () => {
                visible = false;
            },
            show: () => {
                visible = true;
            }
        };
    }

    function pointer(canvas: HTMLCanvasElement, type: string, id: number, x: number, y: number) {
        const event = new MouseEvent(type, { clientX: x, clientY: y, bubbles: true, cancelable: true });
        Object.defineProperties(event, { pointerId: { value: id }, pointerType: { value: "touch" } });
        canvas.dispatchEvent(event);
    }

    it("zooms around a two-finger midpoint / 围绕双指中点缩放", () => {
        const { canvas, viewport } = createViewport();
        try {
            pointer(canvas, "pointerdown", 1, 100, 150);
            pointer(canvas, "pointerdown", 2, 200, 150);
            pointer(canvas, "pointermove", 1, 50, 150);
            pointer(canvas, "pointermove", 2, 250, 150);
            expect(Number(canvas.dataset.zoom)).toBeCloseTo(4);
            pointer(canvas, "pointerup", 2, 250, 150);
            pointer(canvas, "pointermove", 1, 60, 150);
            pointer(canvas, "pointerup", 1, 60, 150);
            viewport.resize();
            expect(Number(canvas.dataset.zoom)).toBeCloseTo(4);
        } finally {
            viewport.dispose();
            vi.restoreAllMocks();
        }
    });

    it("continues zooming past 32x through controls and wheel up to 128x / 按钮与滚轮继续放大到 128 倍", () => {
        const { canvas, viewport } = createViewport();
        try {
            viewport.setZoom(32);
            viewport.zoomBy(2);
            expect(Number(canvas.dataset.zoom)).toBe(64);
            canvas.dispatchEvent(new WheelEvent("wheel", { deltaY: -600, clientX: 200, clientY: 150 }));
            viewport.resize();
            expect(Number(canvas.dataset.zoom)).toBe(128);
            viewport.setZoom(1024);
            expect(Number(canvas.dataset.zoom)).toBe(128);
        } finally {
            viewport.dispose();
            vi.restoreAllMocks();
        }
    });

    it("coalesces wheel frames and reads cached grey without Canvas readback / 合并滚轮帧并从缓存读取灰度而不读回画布", () => {
        const { canvas, viewport } = createViewport();
        const frames: FrameRequestCallback[] = [];
        vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
            frames.push(callback);
            return frames.length;
        });
        const read = vi.fn(() => {
            throw new Error("unexpected Canvas readback");
        });
        vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
            clearRect() {},
            drawImage() {},
            setTransform() {},
            getImageData: read
        } as unknown as CanvasRenderingContext2D);
        const publish = vi.fn();
        viewport.dispose();
        const cached = new LayerViewport(canvas, publish);
        const greys = new Uint8Array(100 * 50).fill(173);
        cached.setImage({
            bitmap: {} as ImageBitmap,
            bitmapWidth: 100,
            bitmapHeight: 50,
            layerIndex: 0,
            plate,
            greys: greys.buffer
        });
        publish.mockClear();
        try {
            for (let i = 0; i < 3; i++) {
                canvas.dispatchEvent(new WheelEvent("wheel", { deltaY: -28, clientX: 200, clientY: 150 }));
            }
            expect(publish).not.toHaveBeenCalled();
            expect(frames).toHaveLength(1);
            frames[0]!(0);
            expect(publish).toHaveBeenCalledTimes(1);
            expect(publish.mock.calls[0]![0].pixel).toMatchObject({ x: 50, y: 25, grey: 173 });
            expect(read).not.toHaveBeenCalled();
        } finally {
            cached.dispose();
            vi.restoreAllMocks();
        }
    });

    it("pinches past 32x with the same 128x limit / 双指缩放超过 32 倍并采用相同的 128 倍上限", () => {
        const { canvas, viewport } = createViewport();
        try {
            viewport.setZoom(32);
            pointer(canvas, "pointerdown", 1, 150, 150);
            pointer(canvas, "pointerdown", 2, 250, 150);
            pointer(canvas, "pointermove", 2, 350, 150);
            expect(Number(canvas.dataset.zoom)).toBe(64);
            pointer(canvas, "pointermove", 2, 650, 150);
            expect(Number(canvas.dataset.zoom)).toBe(128);
        } finally {
            viewport.dispose();
            vi.restoreAllMocks();
        }
    });

    it("keeps the view anchor through zero-size hidden resize / 隐藏后的零尺寸 resize 保留视图锚点", () => {
        const { canvas, viewport, hide, show } = createViewport();
        try {
            const offsetX = canvas.dataset.offsetX;
            const offsetY = canvas.dataset.offsetY;
            const width = canvas.width;
            hide();
            viewport.resize();
            show();
            viewport.resize();
            expect(canvas.dataset.offsetX).toBe(offsetX);
            expect(canvas.dataset.offsetY).toBe(offsetY);
            expect(canvas.width).toBe(width);
            expect(Number(canvas.dataset.zoom)).toBeCloseTo(2);
        } finally {
            viewport.dispose();
            vi.restoreAllMocks();
        }
    });
});
