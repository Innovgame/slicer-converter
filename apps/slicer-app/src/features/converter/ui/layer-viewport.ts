import type { PreviewImageState } from "../model/use-converter-controller";

/**
 * @zh 层图像素对应的构建板毫米参数。原点在板心，图像 Y=0 为顶边，对齐 PrintLab 切片预览。
 * @en Plate millimetre frame for a layer image. Origin is the plate center and image Y=0 is the top edge, matching the PrintLab slice preview.
 */
export interface LayerPlate {
    nativeWidth: number;
    nativeHeight: number;
    platformWidth: number;
    platformHeight: number;
    mirrorX: boolean;
    mirrorY: boolean;
}

/**
 * @zh 工具栏和像素读数使用的视口状态。
 * @en Viewport state used by the toolbar and pixel readout.
 */
export interface LayerViewportState {
    readonly zoom: number;
    readonly grid: boolean;
    readonly ready: boolean;
    readonly settled: boolean;
    readonly pixel?: { readonly x: number; readonly y: number; readonly grey?: number };
    readonly error?: string;
}

interface DisplayedLayer {
    readonly bitmap: PreviewImageState["bitmap"];
    readonly bitmapWidth: number;
    readonly bitmapHeight: number;
    readonly layerIndex: number;
    readonly plate: LayerPlate;
    readonly settled: boolean;
}

/**
 * @zh 视口还没有层图时的工具栏状态。
 * @en Toolbar state before a layer image is ready.
 */
export const EMPTY_LAYER_VIEW: LayerViewportState = {
    zoom: 1,
    grid: true,
    ready: false,
    settled: false
};

/**
 * @zh 把原生像素中心换成板心原点的毫米坐标。没有有效板面尺寸时返回空。
 * @en Converts a native pixel center into millimetres around the plate center. Returns null without a usable plate size.
 */
export function layerPixelToMm(
    plate: LayerPlate,
    pixelX: number,
    pixelY: number
): { readonly x: number; readonly y: number } | null {
    if (plate.platformWidth <= 0 || plate.platformHeight <= 0 || plate.nativeWidth < 1 || plate.nativeHeight < 1) {
        return null;
    }
    let column = pixelX;
    let row = pixelY;
    if (plate.mirrorX) {
        column = plate.nativeWidth - 1 - column;
    }
    if (plate.mirrorY) {
        row = plate.nativeHeight - 1 - row;
    }
    const pitchX = plate.platformWidth / plate.nativeWidth;
    const pitchY = plate.platformHeight / plate.nativeHeight;
    return {
        x: -plate.platformWidth / 2 + (column + 0.5) * pitchX,
        y: -plate.platformHeight / 2 + (plate.nativeHeight - 0.5 - row) * pitchY
    };
}

/**
 * @zh 按均匀层厚估算层顶 Z。文件没有层厚时用 0.05 mm。
 * @en Estimates the layer-top Z from a uniform thickness. Uses 0.05 mm when the file has no thickness.
 */
export function layerTopMm(index: number, layerHeightMm: number | undefined): number {
    const thickness = layerHeightMm && layerHeightMm > 0 ? layerHeightMm : 0.05;
    return (index + 1) * thickness;
}

/**
 * @zh 保留薄层精度并收起浮点尾差。
 * @en Keeps thin-layer precision and hides floating-point noise.
 */
export function formatLayerHeight(value: number): string {
    return value.toLocaleString("en-US", {
        useGrouping: false,
        minimumFractionDigits: 2,
        maximumFractionDigits: 4
    });
}

/**
 * @zh 单个层图视口的绘制、缩放和平移。不请求解码，也不改文档。
 * @en Owns drawing, zoom and pan for one layer view. It does not request decodes or mutate the document.
 */
export class LayerViewport {
    private readonly source = document.createElement("canvas");
    private readonly events = new AbortController();
    private readonly observer: ResizeObserver | null;
    private image: DisplayedLayer | undefined;
    private frame: number | undefined;
    private disposed = false;
    private width = 0;
    private height = 0;
    private dpr = 1;
    private zoom = 1;
    private offsetX = 0;
    private offsetY = 0;
    private fitting = true;
    private grid = true;
    private space = false;
    private pointer: { x: number; y: number } | undefined;
    private drag: { id: number; x: number; y: number } | undefined;
    /**
     * @zh 捕获中的触点与双指缩放锚点；离开画布或失焦时清除。
     * @en Captured touch points and pinch anchor; cleared when capture ends or the viewport loses focus.
     */
    private readonly touches = new Map<number, { x: number; y: number }>();
    private pinch:
        | { distance: number; zoom: number; x: number; y: number; offsetX: number; offsetY: number }
        | undefined;
    private error: string | undefined;

    constructor(
        private readonly canvas: HTMLCanvasElement,
        private readonly publish: (state: LayerViewportState) => void
    ) {
        const options = { signal: this.events.signal };
        canvas.addEventListener("wheel", this.onWheel, { ...options, passive: false });
        canvas.addEventListener("pointerdown", this.onPointerDown, options);
        canvas.addEventListener("pointermove", this.onPointerMove, options);
        canvas.addEventListener("pointerup", this.onPointerUp, options);
        canvas.addEventListener("pointercancel", this.onPointerUp, options);
        canvas.addEventListener("lostpointercapture", this.onPointerUp, options);
        canvas.addEventListener("pointerleave", this.onPointerLeave, options);
        canvas.addEventListener("keydown", this.onKeyDown, options);
        canvas.addEventListener("keyup", this.onKeyUp, options);
        canvas.addEventListener("blur", this.onBlur, options);
        canvas.addEventListener("auxclick", this.preventDefault, options);
        canvas.addEventListener("contextmenu", this.preventDefault, options);
        window.addEventListener("blur", this.onBlur, options);
        window.addEventListener("resize", this.resize, options);
        this.observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(this.resize);
        this.observer?.observe(canvas);
        this.resize();
    }

    resize = (): void => {
        if (this.disposed) {
            return;
        }
        const bounds = this.canvas.getBoundingClientRect();
        if (bounds.width < 2 || bounds.height < 2) return;
        const dpr = window.devicePixelRatio || 1;
        const centerX = this.width > 0 ? (this.width / 2 - this.offsetX) / this.scale : 0;
        const centerY = this.height > 0 ? (this.height / 2 - this.offsetY) / this.scale : 0;
        this.width = bounds.width;
        this.height = bounds.height;
        this.dpr = dpr;
        const bufferWidth = Math.max(1, Math.round(bounds.width * dpr));
        const bufferHeight = Math.max(1, Math.round(bounds.height * dpr));
        if (this.canvas.width !== bufferWidth) {
            this.canvas.width = bufferWidth;
        }
        if (this.canvas.height !== bufferHeight) {
            this.canvas.height = bufferHeight;
        }
        if (bounds.width < 2 || bounds.height < 2) {
            this.publish(this.snapshot());
            return;
        }
        if (this.fitting) {
            this.fit();
            return;
        }
        this.offsetX = bounds.width / 2 - centerX * this.scale;
        this.offsetY = bounds.height / 2 - centerY * this.scale;
        this.paintNow();
    };

    /**
     * @zh 显示与当前层匹配的位图。调用方在位图未到时不要调用，以便保留上一帧。
     * @en Shows a bitmap for the current layer. Callers skip this until the bitmap arrives so the previous frame stays.
     */
    setImage(input: {
        bitmap: PreviewImageState["bitmap"];
        bitmapWidth: number;
        bitmapHeight: number;
        layerIndex: number;
        plate: LayerPlate;
    }): void {
        if (this.disposed) {
            return;
        }
        const previous = this.image;
        const settled =
            input.bitmapWidth === input.plate.nativeWidth && input.bitmapHeight === input.plate.nativeHeight;
        this.error = undefined;
        try {
            if (this.source.width !== input.bitmapWidth) {
                this.source.width = input.bitmapWidth;
            }
            if (this.source.height !== input.bitmapHeight) {
                this.source.height = input.bitmapHeight;
            }
            const context = this.source.getContext("2d");
            if (!context || typeof context.drawImage !== "function") {
                throw new Error("preview_canvas_unavailable");
            }
            const drawStartedAt = performance.now();
            context.drawImage(input.bitmap, 0, 0, input.bitmapWidth, input.bitmapHeight);
            logPreviewCanvasPerf({
                layerIndex: input.layerIndex,
                bitmapWidth: input.bitmapWidth,
                bitmapHeight: input.bitmapHeight,
                displayWidth: input.plate.nativeWidth,
                displayHeight: input.plate.nativeHeight,
                drawMs: performance.now() - drawStartedAt
            });
        } catch {
            this.source.width = 0;
            this.source.height = 0;
            this.image = undefined;
            this.canvas.dataset.previewLayer = "";
            this.error = "Layer image could not be shown";
            this.publish(this.snapshot());
            return;
        }
        this.image = { ...input, settled };
        this.canvas.dataset.previewLayer = String(input.layerIndex);
        if (
            !previous ||
            previous.plate.nativeWidth !== input.plate.nativeWidth ||
            previous.plate.nativeHeight !== input.plate.nativeHeight
        ) {
            this.fitting = true;
        }
        if (this.fitting) {
            this.fit();
            return;
        }
        this.paintNow();
    }

    clear(): void {
        if (this.disposed || !this.image) {
            return;
        }
        this.image = undefined;
        this.error = undefined;
        this.pointer = undefined;
        this.source.width = 0;
        this.source.height = 0;
        this.canvas.dataset.previewLayer = "";
        this.fitting = true;
        this.paintNow();
    }

    fit(): void {
        const image = this.image;
        if (!image || this.width < 2 || this.height < 2) {
            this.publish(this.snapshot());
            return;
        }
        this.fitting = true;
        this.zoom = Math.min(
            32,
            Math.min(
                Math.max(1, this.width - 32) / image.plate.nativeWidth,
                Math.max(1, this.height - 32) / image.plate.nativeHeight
            ) * this.dpr
        );
        this.offsetX = (this.width - image.plate.nativeWidth * this.scale) / 2;
        this.offsetY = (this.height - image.plate.nativeHeight * this.scale) / 2;
        this.paintNow();
    }

    setZoom(zoom: number, x = this.width / 2, y = this.height / 2): void {
        if (!this.image || !Number.isFinite(zoom) || zoom <= 0) {
            return;
        }
        const next = Math.max(this.minimumZoom, Math.min(32, zoom));
        const ratio = next / this.zoom;
        this.offsetX = x - (x - this.offsetX) * ratio;
        this.offsetY = y - (y - this.offsetY) * ratio;
        this.zoom = next;
        this.fitting = false;
        this.paintNow();
    }

    zoomBy(factor: number): void {
        this.setZoom(this.zoom * factor);
    }

    setGrid(grid: boolean): void {
        this.grid = grid;
        this.paintNow();
    }

    dispose(): void {
        if (this.disposed) {
            return;
        }
        this.disposed = true;
        this.events.abort();
        this.observer?.disconnect();
        this.endDrag();
        if (this.frame !== undefined) {
            window.cancelAnimationFrame(this.frame);
        }
        this.source.width = 0;
        this.source.height = 0;
        this.image = undefined;
    }

    private get scale(): number {
        return this.zoom / this.dpr;
    }

    private get minimumZoom(): number {
        const image = this.image;
        if (!image) {
            return 0.01;
        }
        return Math.min(
            0.01,
            this.dpr *
                Math.min(
                    Math.max(1, this.width - 32) / image.plate.nativeWidth,
                    Math.max(1, this.height - 32) / image.plate.nativeHeight
                )
        );
    }

    private paintNow(): void {
        if (this.disposed) {
            return;
        }
        if (this.frame !== undefined) {
            window.cancelAnimationFrame(this.frame);
            this.frame = undefined;
        }
        this.draw();
    }

    private invalidate(): void {
        if (this.disposed || this.frame !== undefined) {
            return;
        }
        this.frame = window.requestAnimationFrame(() => {
            this.frame = undefined;
            this.draw();
        });
    }

    private draw(): void {
        const context = this.canvas.getContext("2d");
        if (!context || typeof context.setTransform !== "function" || typeof context.clearRect !== "function") {
            this.publish(this.snapshot());
            return;
        }
        context.setTransform(1, 0, 0, 1, 0, 0);
        context.clearRect(0, 0, this.canvas.width, this.canvas.height);
        const image = this.image;
        const scale = this.scale;
        const gridVisible = Boolean(image?.settled && this.grid && scale >= 8);
        if (image && this.width >= 2 && this.height >= 2 && typeof context.drawImage === "function") {
            context.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
            context.imageSmoothingEnabled = this.zoom < 1;
            context.drawImage(
                this.source,
                this.offsetX,
                this.offsetY,
                image.plate.nativeWidth * scale,
                image.plate.nativeHeight * scale
            );
            if (gridVisible) {
                this.drawGrid(context, image);
            }
        }
        this.canvas.dataset.zoom = String(this.zoom);
        this.canvas.dataset.offsetX = String(this.offsetX);
        this.canvas.dataset.offsetY = String(this.offsetY);
        this.canvas.dataset.gridVisible = String(gridVisible);
        this.publish(this.snapshot());
    }

    private drawGrid(context: CanvasRenderingContext2D, image: DisplayedLayer): void {
        if (typeof context.save !== "function") {
            return;
        }
        const scale = this.scale;
        const left = Math.max(0, Math.ceil(-this.offsetX / scale));
        const right = Math.min(image.plate.nativeWidth, Math.floor((this.width - this.offsetX) / scale));
        const top = Math.max(0, Math.ceil(-this.offsetY / scale));
        const bottom = Math.min(image.plate.nativeHeight, Math.floor((this.height - this.offsetY) / scale));
        context.save();
        context.beginPath();
        context.rect(this.offsetX, this.offsetY, image.plate.nativeWidth * scale, image.plate.nativeHeight * scale);
        context.clip();
        context.beginPath();
        for (let x = left; x <= right; x += 1) {
            const screen = (Math.round((this.offsetX + x * scale) * this.dpr) + 0.5) / this.dpr;
            context.moveTo(screen, 0);
            context.lineTo(screen, this.height);
        }
        for (let y = top; y <= bottom; y += 1) {
            const screen = (Math.round((this.offsetY + y * scale) * this.dpr) + 0.5) / this.dpr;
            context.moveTo(0, screen);
            context.lineTo(this.width, screen);
        }
        context.strokeStyle = "rgba(128, 128, 128, 0.65)";
        context.lineWidth = 1 / this.dpr;
        context.stroke();
        context.restore();
    }

    private snapshot(): LayerViewportState {
        return {
            zoom: this.zoom,
            grid: this.grid,
            ready: Boolean(this.image),
            settled: Boolean(this.image?.settled),
            pixel: this.pixel(),
            error: this.error
        };
    }

    private pixel(): LayerViewportState["pixel"] {
        const image = this.image;
        if (!this.pointer || !image) {
            return undefined;
        }
        const x = Math.floor((this.pointer.x - this.offsetX) / this.scale);
        const y = Math.floor((this.pointer.y - this.offsetY) / this.scale);
        if (x < 0 || y < 0 || x >= image.plate.nativeWidth || y >= image.plate.nativeHeight) {
            return undefined;
        }
        if (!image.settled) {
            return { x, y };
        }
        const context = this.source.getContext("2d");
        if (!context || typeof context.getImageData !== "function") {
            return { x, y };
        }
        try {
            const sample = context.getImageData(x, y, 1, 1).data;
            return { x, y, grey: sample[0] };
        } catch {
            return { x, y };
        }
    }

    private localPoint(event: MouseEvent): { x: number; y: number } {
        const bounds = this.canvas.getBoundingClientRect();
        return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
    }

    private readonly onWheel = (event: WheelEvent): void => {
        event.preventDefault();
        event.stopPropagation();
        if (this.drag) {
            return;
        }
        this.pointer = this.localPoint(event);
        const line = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? this.height : 1;
        const delta = event.deltaY * line;
        this.setZoom(
            this.zoom * Math.exp(-Math.max(-600, Math.min(600, delta)) * 0.002),
            this.pointer.x,
            this.pointer.y
        );
    };

    private readonly onPointerDown = (event: PointerEvent): void => {
        event.stopPropagation();
        this.canvas.focus({ preventScroll: true });
        if (!this.image || event.button > 2) {
            return;
        }
        if (event.pointerType === "touch") {
            event.preventDefault();
            this.touches.set(event.pointerId, this.localPoint(event));
            this.canvas.setPointerCapture?.(event.pointerId);
            this.fitting = false;
            if (this.touches.size >= 2) {
                const [a, b] = Array.from(this.touches.values());
                this.drag = undefined;
                this.pinch = {
                    distance: Math.max(1, Math.hypot(a.x - b.x, a.y - b.y)),
                    zoom: this.zoom,
                    x: (a.x + b.x) / 2,
                    y: (a.y + b.y) / 2,
                    offsetX: this.offsetX,
                    offsetY: this.offsetY
                };
                return;
            }
        } else if (this.drag) return;
        event.preventDefault();
        this.drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
        this.canvas.setPointerCapture?.(event.pointerId);
        this.canvas.style.cursor = "grabbing";
        this.fitting = false;
    };

    private readonly onPointerMove = (event: PointerEvent): void => {
        event.stopPropagation();
        this.pointer = this.localPoint(event);
        if (this.touches.has(event.pointerId)) {
            this.touches.set(event.pointerId, this.pointer);
            if (this.pinch && this.touches.size >= 2) {
                const [a, b] = Array.from(this.touches.values());
                const zoom = Math.max(
                    this.minimumZoom,
                    Math.min(32, (this.pinch.zoom * Math.hypot(a.x - b.x, a.y - b.y)) / this.pinch.distance)
                );
                const ratio = zoom / this.pinch.zoom;
                this.offsetX = (a.x + b.x) / 2 - (this.pinch.x - this.pinch.offsetX) * ratio;
                this.offsetY = (a.y + b.y) / 2 - (this.pinch.y - this.pinch.offsetY) * ratio;
                this.zoom = zoom;
                this.paintNow();
                return;
            }
        }
        if (this.drag?.id === event.pointerId) {
            this.offsetX += event.clientX - this.drag.x;
            this.offsetY += event.clientY - this.drag.y;
            this.drag.x = event.clientX;
            this.drag.y = event.clientY;
        }
        this.invalidate();
    };

    private readonly onPointerUp = (event: PointerEvent): void => {
        event.stopPropagation();
        if (this.touches.delete(event.pointerId)) {
            this.pinch = undefined;
            const remaining = this.touches.entries().next().value;
            if (remaining) {
                const bounds = this.canvas.getBoundingClientRect();
                this.drag = { id: remaining[0], x: remaining[1].x + bounds.left, y: remaining[1].y + bounds.top };
            } else this.endDrag();
            return;
        }
        if (this.drag?.id === event.pointerId) {
            this.endDrag();
        }
    };

    private endDrag(): void {
        const drag = this.drag;
        this.drag = undefined;
        if (drag && this.canvas.hasPointerCapture?.(drag.id)) {
            this.canvas.releasePointerCapture(drag.id);
        }
        this.canvas.style.cursor = this.space ? "grab" : "crosshair";
    }

    private readonly onPointerLeave = (): void => {
        if (!this.drag) {
            this.pointer = undefined;
        }
        this.invalidate();
    };

    private readonly onBlur = (): void => {
        this.space = false;
        this.touches.clear();
        this.pinch = undefined;
        this.endDrag();
        this.pointer = undefined;
        this.invalidate();
    };

    private readonly onKeyDown = (event: KeyboardEvent): void => {
        event.stopPropagation();
        if (event.ctrlKey || event.metaKey || event.altKey) {
            return;
        }
        if (event.code === "Space") {
            event.preventDefault();
            this.space = true;
            if (!this.drag) {
                this.canvas.style.cursor = "grab";
            }
            return;
        }
        if (event.key === "Escape") {
            event.preventDefault();
            this.onBlur();
            return;
        }
        if (event.key === "+" || event.key === "=") {
            event.preventDefault();
            this.zoomBy(2);
            return;
        }
        if (event.key === "-") {
            event.preventDefault();
            this.zoomBy(0.5);
            return;
        }
        if (event.key === "0") {
            event.preventDefault();
            this.fit();
            return;
        }
        if (event.key === "1") {
            event.preventDefault();
            this.setZoom(1);
            return;
        }
        if (!this.image || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
            return;
        }
        event.preventDefault();
        const step = event.shiftKey ? 100 : 24;
        this.offsetX += event.key === "ArrowLeft" ? step : event.key === "ArrowRight" ? -step : 0;
        this.offsetY += event.key === "ArrowUp" ? step : event.key === "ArrowDown" ? -step : 0;
        this.fitting = false;
        this.invalidate();
    };

    private readonly onKeyUp = (event: KeyboardEvent): void => {
        event.stopPropagation();
        if (event.code !== "Space") {
            return;
        }
        event.preventDefault();
        this.space = false;
        this.endDrag();
    };

    private readonly preventDefault = (event: Event): void => {
        event.preventDefault();
        event.stopPropagation();
    };
}

function logPreviewCanvasPerf(input: {
    layerIndex: number;
    bitmapWidth: number;
    bitmapHeight: number;
    displayWidth: number;
    displayHeight: number;
    drawMs: number;
}): void {
    console.info(
        `[preview-perf] app canvas layer=${input.layerIndex} bitmap=${input.bitmapWidth}x${input.bitmapHeight} display=${input.displayWidth}x${input.displayHeight} canvasDraw=${formatMs(input.drawMs)}`
    );
}

function formatMs(value: number): string {
    return `${value.toFixed(1)}ms`;
}
