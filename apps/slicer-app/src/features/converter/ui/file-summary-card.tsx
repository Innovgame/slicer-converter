import { useEffect, useRef, useState } from "react";

import { listImportableFileExtensions, type ParsedHeader } from "@innovgame/slicer-converter";

import { formatBytes } from "../../../shared/lib/format";
import { renderThumbnail } from "../../preview";

interface FileSummaryCardProps {
    sourceFile: File | null;
    parsedHeader: ParsedHeader | null;
    onFileSelect(file: File): Promise<void>;
}

/**
 * @zh 源文件栏：拖放、格式摘要和缩略图。
 * @en Source column: drop target, format summary and thumbnail.
 */
export function FileSummaryCard({ sourceFile, parsedHeader, onFileSelect }: FileSummaryCardProps) {
    const [dragging, setDragging] = useState(false);
    const dragDepthRef = useRef(0);
    const fileInputRef = useRef<HTMLInputElement | null>(null);
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
    const accept = importableExtensions.join(",");
    const supportText =
        importableExtensions.length > 0
            ? `Supports ${importableExtensions.join(", ")} or click to browse`
            : "Load WASM to discover supported slicer files";

    return (
        <section className="sc-source-panel">
            <header className="sc-panel-header">
                <h2>Source</h2>
            </header>
            <div className="sc-panel-body">
                <div
                    id="dropZone"
                    role="button"
                    aria-label="Drop slicer file or browse"
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
                    className={dragging ? "sc-dropzone is-dragging" : "sc-dropzone"}
                >
                    <p className="sc-dropzone-title">
                        {dragging ? "Release to upload your slicer file" : "Drag and drop a slicer file here"}
                    </p>
                    <p className="sc-dropzone-copy" title={supportText}>
                        {supportText}
                    </p>
                    <input
                        ref={fileInputRef}
                        id="fileInput"
                        type="file"
                        accept={accept}
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
                        <dt>File</dt>
                        <dd id="fileName">{sourceFile?.name ?? "-"}</dd>
                        <dt>Size</dt>
                        <dd id="fileSize">{sourceFile ? formatBytes(sourceFile.size) : "-"}</dd>
                        <dt>Format</dt>
                        <dd id="fileFormat">{parsedHeader?.format.toUpperCase() ?? "-"}</dd>
                        <dt>Resolution</dt>
                        <dd id="fileRes">
                            {metadata ? `${metadata.resolution_x || "?"} × ${metadata.resolution_y || "?"}` : "-"}
                        </dd>
                        <dt>Layers</dt>
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
