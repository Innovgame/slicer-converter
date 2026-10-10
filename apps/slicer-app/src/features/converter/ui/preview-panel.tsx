import { useTranslation } from "../../../shared/i18n/i18n";

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";

import type { ParsedHeader } from "@innovgame/slicer-converter";

import type { PreviewLayerInteraction } from "../../preview";
import type { PreviewImageState } from "../model/use-converter-controller";
import {
    ChevronDownIcon,
    ChevronUpIcon,
    GridIcon,
    LayersIcon,
    MaximizeIcon,
    RotateCcwIcon,
    ScanIcon,
    ZoomInIcon,
    ZoomOutIcon
} from "./icons";
import {
    EMPTY_LAYER_VIEW,
    formatLayerHeight,
    layerPixelToMm,
    layerTopMm,
    LayerViewport,
    type LayerPlate,
    type LayerViewportState
} from "./layer-viewport";

interface PreviewPanelProps {
    parsedHeader: ParsedHeader | null;
    selectedLayer: number;
    previewImage: PreviewImageState | null;
    onPreviewLayerChange(layerIndex: number, interaction?: PreviewLayerInteraction): Promise<void>;
}

/**
 * @zh 切片层图视口：铺满的画布、缩放工具条、像素读数和纵向层轨。
 * @en Slice layer viewport: filled canvas, zoom toolbar, pixel readout and vertical layer rail.
 */
export function PreviewPanel({ parsedHeader, selectedLayer, previewImage, onPreviewLayerChange }: PreviewPanelProps) {
    const { t } = useTranslation();
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const viewportRef = useRef<LayerViewport | null>(null);
    const sliderDraggingRef = useRef(false);
    const pendingDragLayerRef = useRef<number | null>(null);
    const dragFrameRef = useRef<number | null>(null);
    const [view, setView] = useState<LayerViewportState>(EMPTY_LAYER_VIEW);
    const [zoomDraft, setZoomDraft] = useState<string | null>(null);
    const [layerDraft, setLayerDraft] = useState<string | null>(null);
    /**
     * @zh 拖动读数立即响应输入；预览请求仍按帧合并，父级确认当前层后清除草稿。
     * @en Drag readouts respond immediately; preview requests stay frame-coalesced until the parent acknowledges the layer.
     */
    const [dragLayer, setDragLayer] = useState<{ header: ParsedHeader | null; index: number } | null>(null);
    const displayedLayer = dragLayer?.header === parsedHeader ? dragLayer.index : selectedLayer;
    const totalLayers = parsedHeader?.layers.length ?? 0;
    const visible = totalLayers > 0;
    const plate = plateFrom(parsedHeader, previewImage);
    const sliderMax = Math.max(totalLayers - 1, 0);
    const railPosition = sliderMax === 0 ? 0 : (displayedLayer / sliderMax) * 100;
    const percent = Math.round(view.zoom * 1000) / 10;

    const cancelPendingDragFrame = (): void => {
        if (dragFrameRef.current !== null) {
            cancelAnimationFrame(dragFrameRef.current);
            dragFrameRef.current = null;
        }
        pendingDragLayerRef.current = null;
    };

    const queueDraggingLayer = (layerIndex: number): void => {
        pendingDragLayerRef.current = layerIndex;
        if (dragFrameRef.current !== null) {
            return;
        }

        dragFrameRef.current = requestAnimationFrame(() => {
            dragFrameRef.current = null;
            const pendingLayer = pendingDragLayerRef.current;
            pendingDragLayerRef.current = null;
            if (pendingLayer !== null && sliderDraggingRef.current) {
                void onPreviewLayerChange(pendingLayer, "dragging");
            }
        });
    };

    const commitSelectedLayer = (layerIndex: number): void => {
        if (!sliderDraggingRef.current) {
            return;
        }
        const committedLayer = pendingDragLayerRef.current ?? layerIndex;
        sliderDraggingRef.current = false;
        cancelPendingDragFrame();
        void onPreviewLayerChange(committedLayer, "committed");
    };

    const nudgeLayer = (delta: number): void => {
        const next = displayedLayer + delta;
        if (next < 0 || next > sliderMax) {
            return;
        }
        void onPreviewLayerChange(next, "committed");
    };

    const commitLayerNumber = (): void => {
        if (layerDraft === null) {
            return;
        }
        const text = layerDraft.trim();
        const layerNumber = /^\d+$/.test(text) ? Number(text) : Number.NaN;
        setLayerDraft(null);
        if (!visible || !Number.isSafeInteger(layerNumber)) {
            return;
        }
        const layerIndex = Math.min(sliderMax, Math.max(0, layerNumber - 1));
        sliderDraggingRef.current = false;
        cancelPendingDragFrame();
        void onPreviewLayerChange(layerIndex, "committed");
    };

    const commitZoom = (text: string): void => {
        const value = Number(text);
        if (Number.isFinite(value) && value > 0) {
            viewportRef.current?.setZoom(value / 100);
        }
        setZoomDraft(null);
    };

    useEffect(() => {
        sliderDraggingRef.current = false;
        cancelPendingDragFrame();
        setLayerDraft(null);
        return cancelPendingDragFrame;
    }, [parsedHeader]);

    useEffect(() => {
        if (dragLayer && (dragLayer.header !== parsedHeader || dragLayer.index === selectedLayer)) {
            setDragLayer(null);
        }
    }, [dragLayer, parsedHeader, selectedLayer]);

    useLayoutEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }
        const viewport = new LayerViewport(canvas, setView);
        viewportRef.current = viewport;
        return () => {
            viewport.dispose();
            viewportRef.current = null;
        };
    }, []);

    useLayoutEffect(() => {
        const viewport = viewportRef.current;
        if (!viewport) {
            return;
        }
        viewport.resize();
        if (previewImage && previewImage.layerIndex === selectedLayer) {
            viewport.setImage({
                bitmap: previewImage.bitmap,
                bitmapWidth: previewImage.width,
                bitmapHeight: previewImage.height,
                layerIndex: selectedLayer,
                plate
            });
            return;
        }
        if (!previewImage) {
            viewport.clear();
        }
    }, [
        plate.mirrorX,
        plate.mirrorY,
        plate.nativeHeight,
        plate.nativeWidth,
        plate.platformHeight,
        plate.platformWidth,
        previewImage,
        selectedLayer,
        visible
    ]);

    return (
        <div className="sc-viewport-fill">
            {visible ? null : (
                <div className="sc-slice-empty">
                    <div className="sc-empty-mark">
                        <LayersIcon />
                    </div>
                    <p className="sc-empty-title">{t("preview.title")}</p>
                    <p className="sc-empty-copy">{t("preview.empty")}</p>
                </div>
            )}
            <section
                id="previewCard"
                className={visible ? "sc-slice" : "sc-slice is-collapsed"}
                data-slice-layer-view=""
            >
                <div className="sc-slice-stage">
                    <div className="sc-canvas-surface">
                        <canvas
                            id="previewCanvas"
                            ref={canvasRef}
                            aria-label={t("preview.title")}
                            className="sc-slice-canvas"
                            tabIndex={0}
                            title={t("preview.canvasHelp")}
                        />
                        {view.error ? <p className="sc-slice-error">{t("preview.imageError")}</p> : null}
                    </div>
                    <div className="sc-layer-rail" aria-label={t("preview.layerHeight")} data-layer-rail="">
                        <div className="sc-layer-rail-stage">
                            <label className="sc-sr" htmlFor="layerSlider">
                                {t("preview.layer")}
                            </label>
                            <input
                                id="layerSlider"
                                aria-label={t("preview.layerHeight")}
                                aria-orientation="vertical"
                                className="sc-layer-slider"
                                disabled={!visible}
                                max={sliderMax}
                                min="0"
                                style={{ "--layer-fill": `${railPosition}%` } as CSSProperties}
                                type="range"
                                value={displayedLayer}
                                onPointerDown={(event) => {
                                    sliderDraggingRef.current = true;
                                    void onPreviewLayerChange(
                                        Number.parseInt(event.currentTarget.value, 10),
                                        "dragging"
                                    );
                                }}
                                onPointerUp={(event) =>
                                    commitSelectedLayer(Number.parseInt(event.currentTarget.value, 10))
                                }
                                onPointerCancel={(event) =>
                                    commitSelectedLayer(Number.parseInt(event.currentTarget.value, 10))
                                }
                                onBlur={(event) => commitSelectedLayer(Number.parseInt(event.currentTarget.value, 10))}
                                onChange={(event) => {
                                    const layerIndex = Number.parseInt(event.target.value, 10);
                                    if (sliderDraggingRef.current) {
                                        setDragLayer({ header: parsedHeader, index: layerIndex });
                                        queueDraggingLayer(layerIndex);
                                        return;
                                    }
                                    void onPreviewLayerChange(layerIndex, "committed");
                                }}
                            />
                            <div
                                className="sc-rail-readout"
                                data-clip-end="current"
                                style={{
                                    bottom: `calc(${railPosition}% + var(--sc-layer-thumb-size) * ${0.5 - railPosition / 100})`
                                }}
                            >
                                <button
                                    aria-label={t("preview.increase")}
                                    className="sc-rail-nudge"
                                    disabled={!visible || displayedLayer >= sliderMax}
                                    type="button"
                                    onClick={() => nudgeLayer(1)}
                                >
                                    <ChevronUpIcon />
                                </button>
                                <span className="sc-rail-copy">
                                    <span className="sc-rail-detail">
                                        <span id="layerNum" className="sc-sr">
                                            {displayedLayer}
                                        </span>
                                        <input
                                            aria-label={t("preview.layerNumber")}
                                            className="sc-layer-number-input"
                                            disabled={!visible}
                                            inputMode="numeric"
                                            type="text"
                                            title={t("preview.jumpHelp", { count: totalLayers })}
                                            value={layerDraft ?? String(displayedLayer + 1)}
                                            onFocus={(event) => event.currentTarget.select()}
                                            onChange={(event) => setLayerDraft(event.currentTarget.value)}
                                            onBlur={commitLayerNumber}
                                            onKeyDown={(event) => {
                                                if (event.key === "Enter") {
                                                    event.preventDefault();
                                                    commitLayerNumber();
                                                } else if (event.key === "Escape") {
                                                    event.preventDefault();
                                                    setLayerDraft(null);
                                                }
                                            }}
                                        />
                                        <span className="sc-rail-total">/ {totalLayers}</span>
                                    </span>
                                    <span className="sc-rail-value" aria-label={t("preview.zHeight")}>
                                        {formatLayerHeight(
                                            layerTopMm(displayedLayer, parsedHeader?.metadata.layer_height_mm)
                                        )}
                                        <span className="sc-rail-unit">mm</span>
                                    </span>
                                </span>
                                <button
                                    aria-label={t("preview.decrease")}
                                    className="sc-rail-nudge"
                                    disabled={!visible || displayedLayer <= 0}
                                    type="button"
                                    onClick={() => nudgeLayer(-1)}
                                >
                                    <ChevronDownIcon />
                                </button>
                            </div>
                        </div>
                        <button
                            aria-label={t("preview.reset")}
                            className="sc-clip-reset"
                            disabled={!visible || displayedLayer === 0}
                            type="button"
                            onClick={() => void onPreviewLayerChange(0, "committed")}
                        >
                            <RotateCcwIcon />
                        </button>
                    </div>
                </div>
                <div className="sc-slice-chrome">
                    <div aria-label={t("preview.toolbar")} className="sc-slice-toolbar" role="toolbar">
                        <ToolButton
                            disabled={!view.ready}
                            label={t("preview.fit")}
                            onClick={() => viewportRef.current?.fit()}
                        >
                            <MaximizeIcon />
                        </ToolButton>
                        <ToolButton
                            disabled={!view.ready}
                            label={t("preview.actualPixels")}
                            onClick={() => viewportRef.current?.setZoom(1)}
                        >
                            <ScanIcon />
                        </ToolButton>
                        <ToolButton
                            disabled={!view.ready}
                            label={t("preview.zoomOut")}
                            onClick={() => viewportRef.current?.zoomBy(0.5)}
                        >
                            <ZoomOutIcon />
                        </ToolButton>
                        <div className="sc-zoom-control">
                            <input
                                aria-label={t("preview.zoomPercent")}
                                className="sc-zoom-field"
                                disabled={!view.ready}
                                inputMode="decimal"
                                value={zoomDraft ?? String(percent)}
                                onBlur={(event) => commitZoom(event.currentTarget.value)}
                                onChange={(event) => setZoomDraft(event.currentTarget.value)}
                                onKeyDown={(event) => {
                                    if (event.key === "Enter") {
                                        event.currentTarget.blur();
                                    }
                                }}
                            />
                            <span className="sc-zoom-unit">%</span>
                        </div>
                        <ToolButton
                            disabled={!view.ready}
                            label={t("preview.zoomIn")}
                            onClick={() => viewportRef.current?.zoomBy(2)}
                        >
                            <ZoomInIcon />
                        </ToolButton>
                        <button
                            aria-label={t("preview.grid")}
                            aria-pressed={view.grid}
                            className={view.grid ? "sc-tool is-pressed" : "sc-tool"}
                            disabled={!view.ready}
                            title={t("preview.grid")}
                            type="button"
                            onClick={() => viewportRef.current?.setGrid(!view.grid)}
                        >
                            <GridIcon />
                        </button>
                    </div>
                    <div className="sc-pixel-status">
                        <span>{resolutionText(plate, view, t("preview.interactive"))}</span>
                        <span className="sc-pixel-readout">{pixelReadout(plate, view.pixel, t("preview.grey"))}</span>
                    </div>
                </div>
            </section>
        </div>
    );
}

function ToolButton({
    children,
    disabled,
    label,
    onClick
}: {
    children: ReactNode;
    disabled: boolean;
    label: string;
    onClick(): void;
}) {
    return (
        <button
            aria-label={label}
            className="sc-tool"
            disabled={disabled}
            title={label}
            type="button"
            onClick={onClick}
        >
            {children}
        </button>
    );
}

function plateFrom(header: ParsedHeader | null, preview: PreviewImageState | null): LayerPlate {
    return {
        nativeWidth: header?.metadata.resolution_x || preview?.width || 256,
        nativeHeight: header?.metadata.resolution_y || preview?.height || 256,
        platformWidth: header?.metadata.display_width_mm || 0,
        platformHeight: header?.metadata.display_height_mm || 0,
        mirrorX: Boolean(header?.metadata.mirror_x),
        mirrorY: Boolean(header?.metadata.mirror_y)
    };
}

function resolutionText(plate: LayerPlate, view: LayerViewportState, previewLabel: string): string {
    if (plate.nativeWidth < 1 || plate.nativeHeight < 1) {
        return "\u00a0";
    }
    const size =
        plate.platformWidth > 0 && plate.platformHeight > 0
            ? ` · ${formatMm(plate.platformWidth)} × ${formatMm(plate.platformHeight)} mm`
            : "";
    const preview = view.ready && !view.settled ? ` · ${previewLabel}` : "";
    return `${plate.nativeWidth} × ${plate.nativeHeight}${size}${preview}`;
}

function pixelReadout(plate: LayerPlate, pixel: LayerViewportState["pixel"], greyLabel: string): string {
    if (!pixel) {
        return `X — × Y — · —.00 × —.00 mm · ${greyLabel} —`;
    }
    const mm = layerPixelToMm(plate, pixel.x, pixel.y);
    const mmText = mm ? `${formatMm(mm.x)} × ${formatMm(mm.y)} mm` : "—.00 × —.00 mm";
    const grey = pixel.grey === undefined ? "—" : String(pixel.grey);
    return `X ${pixel.x} × Y ${pixel.y} · ${mmText} · ${greyLabel} ${grey}`;
}

function formatMm(value: number): string {
    return value.toLocaleString("en-US", {
        useGrouping: false,
        minimumFractionDigits: 2,
        maximumFractionDigits: 2
    });
}
