import {
    AppShell,
    ConvertPanel,
    FileSummaryCard,
    PreviewPanel,
    StatusBanner,
    useConverterController
} from "../../features/converter";

/**
 * @zh 转换器页面。组合工作台槽位，不拥有转换或预览请求。
 * @en Converter page. Composes workbench slots and does not own conversion or preview requests.
 */
export function ConverterPage() {
    const controller = useConverterController();

    return (
        <AppShell
            title="Slicer File Converter"
            documentStatus={controller.sourceFile?.name ?? "No file open"}
            documentLoaded={controller.sourceFile !== null}
            source={
                <FileSummaryCard
                    sourceFile={controller.sourceFile}
                    parsedHeader={controller.parsedHeader}
                    onFileSelect={controller.onFileSelect}
                />
            }
            viewer={
                <PreviewPanel
                    parsedHeader={controller.parsedHeader}
                    selectedLayer={controller.selectedLayer}
                    previewImage={controller.previewImage}
                    onPreviewLayerChange={controller.onPreviewLayerChange}
                />
            }
            inspector={
                <ConvertPanel
                    wasmReady={controller.wasmReady}
                    parsedHeader={controller.parsedHeader}
                    status={controller.status}
                    outputFileName={controller.outputFileName}
                    slicerConverterFormatId={controller.slicerConverterFormatId}
                    slicerConverterSuffixId={controller.slicerConverterSuffixId}
                    slicerConverterVersionId={controller.slicerConverterVersionId}
                    converting={controller.converting}
                    exportWarning={controller.exportWarning}
                    progress={controller.progress}
                    onOutputFileNameChange={controller.onOutputFileNameChange}
                    onSelectSlicerConverterFormat={controller.onSelectSlicerConverterFormat}
                    onSelectSlicerConverterSuffix={controller.onSelectSlicerConverterSuffix}
                    onSelectSlicerConverterVersion={controller.onSelectSlicerConverterVersion}
                    onConvert={controller.onConvert}
                    onCancel={controller.onCancel}
                />
            }
            status={<StatusBanner id="statusMsg" status={controller.status} />}
        />
    );
}
