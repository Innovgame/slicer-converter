import { useTranslation } from "../../shared/i18n/i18n";
import { useRef } from "react";
import { Onboarding } from "../../features/onboarding";

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
    const { t } = useTranslation();
    const controller = useConverterController();
    const sourceInputRef = useRef<HTMLInputElement | null>(null);
    const browseFile = () => sourceInputRef.current?.click();
    const extension = controller.outputExtension ?? "";
    const exportSummary = controller.exportSummary ?? t("export.target");

    return (
        <AppShell
            title={t("app.title")}
            documentStatus={controller.sourceFile?.name ?? t("app.noFile")}
            documentLoaded={controller.sourceFile !== null}
            documentIdentity={controller.sourceFile}
            onBrowseFile={browseFile}
            converting={controller.converting}
            sourceSummary={
                controller.parsedHeader
                    ? `${controller.parsedHeader.format.toUpperCase()} · ${t("source.layerCount", { count: controller.parsedHeader.layers.length })}`
                    : undefined
            }
            help={(mobile) => (
                <Onboarding
                    documentLoaded={controller.parsedHeader !== null}
                    converting={controller.converting}
                    onNavigate={
                        mobile.isMobile
                            ? (step) => {
                                  mobile.setImmersive(false);
                                  if (step === "preferences" || step === "feedback") {
                                      mobile.selectTab("settings");
                                      return;
                                  }
                                  mobile.selectTab("preview");
                                  if (step === "source") mobile.openDrawer("source");
                                  else if (step === "export" || step === "convert") mobile.openDrawer("export");
                                  else mobile.closeDrawer();
                              }
                            : undefined
                    }
                />
            )}
            source={
                <FileSummaryCard
                    sourceFile={controller.sourceFile}
                    parsedHeader={controller.parsedHeader}
                    onFileSelect={controller.onFileSelect}
                    inputRef={sourceInputRef}
                />
            }
            viewer={(mobile) => (
                <PreviewPanel
                    parsedHeader={controller.parsedHeader}
                    selectedLayer={controller.selectedLayer}
                    previewImage={controller.previewImage}
                    onPreviewLayerChange={controller.onPreviewLayerChange}
                    mobile={mobile}
                    onBrowseFile={browseFile}
                    exportSummary={exportSummary}
                    outputName={`${controller.outputFileName}${extension ? `.${extension}` : ""}`}
                    converting={controller.converting}
                />
            )}
            inspector={(mobile) => (
                <ConvertPanel
                    mobile={mobile.isMobile}
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
            )}
            status={(mobile) => (
                <StatusBanner
                    id="statusMsg"
                    status={controller.status}
                    mobile={mobile.isMobile}
                    converting={controller.converting}
                    progress={controller.progress}
                    documentLoaded={controller.parsedHeader !== null}
                />
            )}
        />
    );
}
