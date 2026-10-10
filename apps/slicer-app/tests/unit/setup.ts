import "@testing-library/jest-dom/vitest";

if (typeof globalThis.ResizeObserver === "undefined") {
    globalThis.ResizeObserver = class ResizeObserver {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
    };
}
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
    configurable: true,
    writable: true,
    value(contextId: string): CanvasRenderingContext2D | null {
        if (contextId !== "2d") {
            return null;
        }

        return {
            canvas: this,
            clearRect() {},
            createImageData(width: number, height: number): ImageData {
                return {
                    width,
                    height,
                    colorSpace: "srgb",
                    data: new Uint8ClampedArray(width * height * 4)
                } as ImageData;
            },
            drawImage() {},
            getImageData(_x: number, _y: number, width: number, height: number): ImageData {
                return {
                    width,
                    height,
                    colorSpace: "srgb",
                    data: new Uint8ClampedArray(width * height * 4)
                } as ImageData;
            },
            putImageData() {}
        } as unknown as CanvasRenderingContext2D;
    }
});

afterEach(cleanup);
