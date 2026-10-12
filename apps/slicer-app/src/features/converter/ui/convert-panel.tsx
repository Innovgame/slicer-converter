import { translateRuntimeMessage } from "../../../shared/i18n/runtime-messages";

import { useTranslation } from "../../../shared/i18n/i18n";

import {
    findSlicerExportTargetMatrixSelection,
    listSlicerExportTargetMatrix,
    type ParsedHeader
} from "@innovgame/slicer-converter";

import type { ProgressState, StatusState } from "../model/use-converter-controller";
import { ConvertIcon, ExportIcon, FileIcon, SlidersIcon } from "./icons";

interface ConvertPanelProps {
    wasmReady: boolean;
    mobile?: boolean;
    parsedHeader: ParsedHeader | null;
    status: StatusState | null;
    outputFileName: string;
    slicerConverterFormatId: string;
    slicerConverterSuffixId: string;
    slicerConverterVersionId: string;
    converting: boolean;
    exportWarning?: string | null;
    progress: ProgressState | null;
    onOutputFileNameChange(value: string): void;
    onSelectSlicerConverterFormat(value: string): void;
    onSelectSlicerConverterSuffix(value: string): void;
    onSelectSlicerConverterVersion(value: string): void;
    onConvert(): Promise<void>;
    onCancel(): void;
}

/**
 * @zh 导出栏：目标文件名、格式矩阵、转换和进度。
 * @en Export column: target filename, format matrix, convert action and progress.
 */
export function ConvertPanel({
    wasmReady,
    mobile = false,
    parsedHeader,
    status,
    outputFileName,
    slicerConverterFormatId,
    slicerConverterSuffixId,
    slicerConverterVersionId,
    converting,
    exportWarning = null,
    progress,
    onOutputFileNameChange,
    onSelectSlicerConverterFormat,
    onSelectSlicerConverterSuffix,
    onSelectSlicerConverterVersion,
    onConvert,
    onCancel
}: ConvertPanelProps) {
    const { t } = useTranslation();
    const visible = parsedHeader !== null;
    const actionsDisabled = !wasmReady || !parsedHeader || converting;
    const showRetry = status?.type === "error" && !actionsDisabled;
    const sourceFormat = parsedHeader?.format ?? null;
    const slicerConverterMatrix = listSlicerExportTargetMatrix(sourceFormat);
    const slicerConverterSelection = findSlicerExportTargetMatrixSelection({
        sourceFormat,
        formatId: slicerConverterFormatId,
        suffixId: slicerConverterSuffixId,
        versionId: slicerConverterVersionId
    });
    const outputExtension =
        slicerConverterSelection?.version.target.resolvedExtension ?? slicerConverterSelection?.suffix.id ?? "";
    const visibleSlicerConverterVersions =
        slicerConverterSelection?.suffix.versions.filter(isVisibleSlicerConverterVersionOption) ?? [];
    const hasSlicerConverterVersionOptions = visibleSlicerConverterVersions.length > 0;

    const outputControls = (
        <div className="sc-group">
            <p className="sc-group-title">
                <FileIcon />
                {t("export.output")}
            </p>
            <label htmlFor="slicerConverterFileName" className="sc-label">
                {t("export.fileName")}
            </label>
            <div className="sc-filename">
                <input
                    id="slicerConverterFileName"
                    value={outputFileName}
                    onChange={(event) => onOutputFileNameChange(event.target.value)}
                />
                <span>{outputExtension ? `.${outputExtension}` : ""}</span>
            </div>
        </div>
    );
    const targetControls = (
        <div className="sc-group" data-tour="export-target">
            <p className="sc-group-title">
                <SlidersIcon />
                {t("export.target")}
            </p>
            <label htmlFor="slicerConverterFormat" className="sc-label">
                {t("common.format")}
            </label>
            <select
                id="slicerConverterFormat"
                value={slicerConverterSelection?.format.id ?? ""}
                onChange={(event) => onSelectSlicerConverterFormat(event.target.value)}
                className="sc-select"
            >
                {slicerConverterMatrix.map((format) => (
                    <option key={format.id} value={format.id}>
                        {format.name}
                    </option>
                ))}
            </select>

            <label htmlFor="slicerConverterSuffix" className="sc-label">
                {t("export.suffix")}
            </label>
            <select
                id="slicerConverterSuffix"
                value={slicerConverterSelection?.suffix.id ?? ""}
                onChange={(event) => onSelectSlicerConverterSuffix(event.target.value)}
                className="sc-select"
            >
                {(slicerConverterSelection?.format.suffixes ?? []).map((suffix) => (
                    <option key={suffix.id} value={suffix.id}>
                        {suffix.name}
                    </option>
                ))}
            </select>

            {hasSlicerConverterVersionOptions ? (
                <>
                    <label htmlFor="slicerConverterVersion" className="sc-label">
                        {t("export.version")}
                    </label>
                    <select
                        id="slicerConverterVersion"
                        value={slicerConverterSelection?.version.id ?? ""}
                        onChange={(event) => onSelectSlicerConverterVersion(event.target.value)}
                        className="sc-select"
                    >
                        {visibleSlicerConverterVersions.map((version) => (
                            <option key={version.id} value={version.id}>
                                {version.name}
                            </option>
                        ))}
                    </select>
                </>
            ) : null}
        </div>
    );

    return (
        <section id="convertCard" className="sc-inspector-panel">
            <div className={visible ? "sc-inspector-empty hidden" : "sc-inspector-empty"}>
                <div className="sc-empty-mark">
                    <ExportIcon />
                </div>
                <p className="sc-empty-title">{t("export.title")}</p>
                <p className="sc-empty-copy">{t("export.empty")}</p>
            </div>
            <header className={visible ? "sc-panel-header" : "sc-panel-header hidden"}>
                <span aria-hidden="true" className="sc-panel-mark">
                    <ExportIcon />
                </span>
                <h2>{t("export.title")}</h2>
                {sourceFormat ? (
                    <span className="sc-panel-meta">{t("export.from", { format: sourceFormat.toUpperCase() })}</span>
                ) : null}
            </header>
            <div className={visible ? "sc-inspector-body" : "sc-inspector-body hidden"}>
                {mobile ? (
                    <>
                        {targetControls}
                        {outputControls}
                    </>
                ) : (
                    <>
                        {outputControls}
                        {targetControls}
                    </>
                )}

                {exportWarning ? (
                    <p id="exportGeometryWarning" className="sc-warning">
                        {translateRuntimeMessage(t, exportWarning)}
                    </p>
                ) : null}
            </div>
            <footer className={visible ? "sc-inspector-actions" : "sc-inspector-actions hidden"}>
                <button
                    type="button"
                    id="convertBtn"
                    disabled={actionsDisabled}
                    onClick={() => void onConvert()}
                    className="sc-primary-btn"
                >
                    <ConvertIcon />
                    {t("export.convert")}
                </button>

                {showRetry ? (
                    <button type="button" id="retryBtn" onClick={() => void onConvert()} className="sc-secondary-btn">
                        {t("export.retry")}
                    </button>
                ) : null}

                <div id="progressContainer" className={progress ? "sc-progress" : "sc-progress hidden"}>
                    <div className="sc-progress-meta">
                        <span id="progressLabel">{translateRuntimeMessage(t, progress?.label ?? "Converting...")}</span>
                        <span id="progressPct">{progress ? `${progress.percent}%` : "0%"}</span>
                    </div>
                    <div className="sc-progress-track">
                        <div id="progressBar" style={{ width: `${progress?.percent ?? 0}%` }} />
                    </div>
                    <button type="button" id="cancelBtn" onClick={onCancel} className="sc-danger-btn">
                        {t("export.cancel")}
                    </button>
                </div>
            </footer>
        </section>
    );
}

function isVisibleSlicerConverterVersionOption(option: { id: string; name: string }) {
    return option.id !== "default" && option.name !== "默认";
}
