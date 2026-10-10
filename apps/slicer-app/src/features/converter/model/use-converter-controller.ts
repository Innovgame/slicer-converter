import { useEffect, useState } from "react";

import { findSlicerExportTargetMatrixSelection, type ParsedHeader } from "@slicer-converter/wasm";

import type { PreviewImageState, ProgressState, StatusState } from "./converter-view-state";
import { useRuntimeController } from "../../../app/runtime/use-runtime-controller";
import { useConversionController } from "../../conversion";
import { useDocumentController } from "../../document-opening";
import { type PreviewLayerInteraction, usePreviewController } from "../../preview";

export type { PreviewImageState, ProgressState, StatusState } from "./converter-view-state";

/**
 * @zh Converter 页面暴露给 React 组件树的聚合控制器。
 * @en Aggregated controller exposed by the converter page to the React component tree.
 */
export interface ConverterController {
    wasmReady: boolean;
    status: StatusState | null;
    sourceFile: File | null;
    parsedHeader: ParsedHeader | null;
    selectedLayer: number;
    previewImage: PreviewImageState | null;
    progress: ProgressState | null;
    outputFileName: string;
    slicerConverterFormatId: string;
    slicerConverterSuffixId: string;
    slicerConverterVersionId: string;
    converting: boolean;
    exportWarning: string | null;
    onFileSelect(file: File): Promise<void>;
    onOutputFileNameChange(value: string): void;
    onSelectSlicerConverterFormat(value: string): void;
    onSelectSlicerConverterSuffix(value: string): void;
    onSelectSlicerConverterVersion(value: string): void;
    onConvert(): Promise<void>;
    onCancel(): void;
    onPreviewLayerChange(layerIndex: number, interaction?: PreviewLayerInteraction): Promise<void>;
}

/**
 * @zh 组合 runtime、document、preview 和 conversion 子控制器。
 * @en Composes the runtime, document, preview, and conversion sub-controllers.
 */
export function useConverterController(): ConverterController {
    const runtime = useRuntimeController();
    const [outputFileName, setOutputFileName] = useState("");
    const [slicerConverterFormatId, setSlicerConverterFormatId] = useState("");
    const [slicerConverterSuffixId, setSlicerConverterSuffixId] = useState("");
    const [slicerConverterVersionId, setSlicerConverterVersionId] = useState("");

    const document = useDocumentController({
        wasmReady: runtime.wasmReady,
        setStatus: runtime.setStatus
    });
    const preview = usePreviewController({
        document: document.document,
        workerPool: runtime.workerPoolRef.current,
        setStatus: runtime.setStatus
    });
    const sourceFormat = document.document?.parsedHeader.format ?? null;
    const slicerConverterSelection = findSlicerExportTargetMatrixSelection({
        sourceFormat,
        formatId: slicerConverterFormatId,
        suffixId: slicerConverterSuffixId,
        versionId: slicerConverterVersionId
    });
    const conversion = useConversionController({
        document: document.document,
        workerPool: runtime.workerPoolRef.current,
        wasmReady: runtime.wasmReady,
        outputFileName,
        conversionTarget: slicerConverterSelection?.version.target ?? null,
        setStatus: runtime.setStatus
    });

    useEffect(() => {
        if (!slicerConverterSelection) {
            return;
        }
        setSlicerConverterFormatId(slicerConverterSelection.format.id);
        setSlicerConverterSuffixId(slicerConverterSelection.suffix.id);
        setSlicerConverterVersionId(slicerConverterSelection.version.id);
    }, [
        slicerConverterSelection?.format.id,
        slicerConverterSelection?.suffix.id,
        slicerConverterSelection?.version.id
    ]);

    function onSelectSlicerConverterFormat(value: string): void {
        const selection = findSlicerExportTargetMatrixSelection({
            sourceFormat,
            formatId: value
        });
        setSlicerConverterFormatId(selection?.format.id ?? value);
        setSlicerConverterSuffixId(selection?.suffix.id ?? "");
        setSlicerConverterVersionId(selection?.version.id ?? "");
    }

    function onSelectSlicerConverterSuffix(value: string): void {
        const selection = findSlicerExportTargetMatrixSelection({
            sourceFormat,
            formatId: slicerConverterSelection?.format.id ?? slicerConverterFormatId,
            suffixId: value
        });
        setSlicerConverterFormatId(
            selection?.format.id ?? slicerConverterSelection?.format.id ?? slicerConverterFormatId
        );
        setSlicerConverterSuffixId(selection?.suffix.id ?? value);
        setSlicerConverterVersionId(selection?.version.id ?? "");
    }

    function onSelectSlicerConverterVersion(value: string): void {
        const selection = findSlicerExportTargetMatrixSelection({
            sourceFormat,
            formatId: slicerConverterSelection?.format.id ?? slicerConverterFormatId,
            suffixId: slicerConverterSelection?.suffix.id ?? slicerConverterSuffixId,
            versionId: value
        });
        setSlicerConverterFormatId(
            selection?.format.id ?? slicerConverterSelection?.format.id ?? slicerConverterFormatId
        );
        setSlicerConverterSuffixId(
            selection?.suffix.id ?? slicerConverterSelection?.suffix.id ?? slicerConverterSuffixId
        );
        setSlicerConverterVersionId(selection?.version.id ?? value);
    }

    async function onConvert(): Promise<void> {
        await conversion.onConvert();
    }

    async function onFileSelect(file: File): Promise<void> {
        setOutputFileName(fileNameStem(file.name));
        await document.onFileSelect(file);
    }

    return {
        wasmReady: runtime.wasmReady,
        status: runtime.status,
        sourceFile: document.sourceFile,
        parsedHeader: document.document?.parsedHeader ?? null,
        selectedLayer: preview.selectedLayer,
        previewImage: preview.previewImage,
        progress: conversion.progress,
        outputFileName,
        slicerConverterFormatId: slicerConverterSelection?.format.id ?? slicerConverterFormatId,
        slicerConverterSuffixId: slicerConverterSelection?.suffix.id ?? slicerConverterSuffixId,
        slicerConverterVersionId: slicerConverterSelection?.version.id ?? slicerConverterVersionId,
        converting: conversion.converting,
        exportWarning: conversion.exportWarning,
        onFileSelect,
        onOutputFileNameChange: setOutputFileName,
        onSelectSlicerConverterFormat,
        onSelectSlicerConverterSuffix,
        onSelectSlicerConverterVersion,
        onConvert,
        onCancel: conversion.onCancel,
        onPreviewLayerChange: preview.onPreviewLayerChange
    };
}

function fileNameStem(fileName: string): string {
    return fileName.replace(/\.[^.]+$/, "");
}
