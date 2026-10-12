import { useTranslation } from "../../../shared/i18n/i18n";

import { useEffect, useRef, useState, type RefObject } from "react";

import { listImportableFileExtensions, type ParsedHeader } from "@innovgame/slicer-converter";

import { formatBytes } from "../../../shared/lib/format";
import { renderThumbnail } from "../../preview";
import { buildFilePickerAccept } from "./file-picker-accept";
import { FileIcon, UploadIcon } from "./icons";

interface FileSummaryCardProps {
    sourceFile: File | null;
    parsedHeader: ParsedHeader | null;
    onFileSelect(file: File): Promise<void>;
    inputRef?: RefObject<HTMLInputElement | null>;
}

/**
 * @zh 源文件栏：拖放、格式摘要和缩略图。
 * @en Source column: drop target, format summary and thumbnail.
 */
export function FileSummaryCard({ sourceFile, parsedHeader, onFileSelect, inputRef }: FileSummaryCardProps) {
    const { t } = useTranslation();
    const [dragging, setDragging] = useState(false);
    const dragDepthRef = useRef(0);
    const localInputRef = useRef<HTMLInputElement | null>(null);
    const fileInputRef = inputRef ?? localInputRef;
    const thumbnailContainerRef = useRef<HTMLDivElement | null>(null);
    const thumbnailCanvasRef = useRef<HTMLCanvasElement | null>(null);

    useEffect(() => {
        if (parsedHeader) {
            renderThumbnail(parsedHeader.metadata, thumbnailContainerRef.current, thumbnailCanvasRef.current);
        }
    }, [parsedHeader]);

    const fileInfoVisible = sourceFile !== null && parsedHeader !== null;
    const metadata = parsedHeader?.metadata;
    const importableExtensions = listImportableFileExtensions();
    const accept = buildFilePickerAccept(importableExtensions);
    const supportText =
        importableExtensions.length > 0
            ? t("source.supports", { extensions: importableExtensions.join(", ") })
            : t("source.loadingFormats");
    const layerCount = parsedHeader ? parsedHeader.layers.length : null;

    return (
        <section className="sc-source-panel">
            <header className="sc-panel-header">
                <span aria-hidden="true" className="sc-panel-mark">
                    <FileIcon />
                </span>
                <h2>{t("source.title")}</h2>
                {layerCount !== null ? (
                    <span className="sc-panel-meta">{t("source.layerCount", { count: layerCount })}</span>
                ) : null}
            </header>
            <div className="sc-panel-body">
                <div
                    id="dropZone"
                    role="button"
                    aria-label={t("source.browse")}
                    tabIndex={0}
                    onClick={() => fileInputRef.current?.click()}
                    onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            fileInputRef.current?.click();
                        }
                    }}
                    onDragEnter={(event) => {
                        event.preventDefault();
                        dragDepthRef.current += 1;
                        setDragging(true);
                    }}
                    onDragOver={(event) => {
                        event.preventDefault();
                        setDragging(true);
                    }}
                    onDragLeave={(event) => {
                        event.preventDefault();
                        dragDepthRef.current = Math.max(0, dragDepthRef.current - 1);
                        if (dragDepthRef.current === 0) {
                            setDragging(false);
                        }
                    }}
                    onDrop={(event) => {
                        event.preventDefault();
                        dragDepthRef.current = 0;
                        setDragging(false);
                        const file = event.dataTransfer.files?.[0];
                        if (file) {
                            void onFileSelect(file);
                        }
                    }}
                    className={dropZoneClass(dragging, sourceFile !== null)}
                >
                    <span aria-hidden="true" className="sc-dropzone-mark">
                        <UploadIcon />
                    </span>
                    <p className="sc-dropzone-title">
                        {dragging ? t("source.release") : sourceFile ? t("source.replace") : t("source.drop")}
                    </p>
                    <p className="sc-dropzone-copy" title={supportText}>
                        {supportText}
                    </p>
                    <input
                        ref={fileInputRef}
                        id="fileInput"
                        type="file"
                        {...(accept ? { accept } : {})}
                        className="hidden"
                        onChange={(event) => {
                            const file = event.target.files?.[0];
                            if (file) {
                                void onFileSelect(file);
                            }
                            event.target.value = "";
                        }}
                    />
                </div>

                <div id="fileInfo" className={fileInfoVisible ? "sc-file-info" : "sc-file-info hidden"}>
                    <dl>
                        <dt>{t("source.file")}</dt>
                        <dd id="fileName" title={sourceFile?.name}>
                            {sourceFile?.name ?? "-"}
                        </dd>
                        <dt>{t("source.size")}</dt>
                        <dd id="fileSize">{sourceFile ? formatBytes(sourceFile.size) : "-"}</dd>
                        <dt>{t("common.format")}</dt>
                        <dd id="fileFormat">{parsedHeader?.format.toUpperCase() ?? "-"}</dd>
                        <dt>{t("source.resolution")}</dt>
                        <dd id="fileRes">
                            {metadata ? `${metadata.resolution_x || "?"} × ${metadata.resolution_y || "?"}` : "-"}
                        </dd>
                        <dt>{t("source.layers")}</dt>
                        <dd id="fileLayers">{parsedHeader?.layers.length ?? "-"}</dd>
                    </dl>
                    <div id="thumbnailContainer" ref={thumbnailContainerRef} className="sc-thumbnail hidden">
                        <canvas id="thumbnailCanvas" ref={thumbnailCanvasRef} />
                    </div>
                </div>
            </div>
        </section>
    );
}

/**
 * @zh 拖放区在拖入时高亮，在已有源文件时收成一行。
 * @en The drop zone highlights while dragging and collapses to one row once a source file is loaded.
 */
function dropZoneClass(dragging: boolean, hasSourceFile: boolean): string {
    const classes = ["sc-dropzone"];
    if (hasSourceFile) {
        classes.push("is-compact");
    }
    if (dragging) {
        classes.push("is-dragging");
    }
    return classes.join(" ");
}
