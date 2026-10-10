import {
    createBlobOutputSink as createBlobSink,
    createBufferedOutputSink,
    getFormatFacts,
    openFileSystemSink,
    type FileSystemOutputSinkDiagnostic,
    type PatchableSlicerOutputSink as PatchableOutputSink,
    type ResolvedSlicerExportTarget
} from "@innovgame/slicer-converter";

import { buildDownloadFilename } from "../../../shared/lib/output";

/**
 * @zh `ConversionOutputTarget` 接口契约。
 * @en Interface contract for `ConversionOutputTarget`.
 */
export interface ConversionOutputTarget {
    downloadName: string;
    jobName: string;
    isZipTarget: boolean;
    mode: "streaming" | "blob";
    sink: PatchableOutputSink;
}

/**
 * @zh 打开输出 sink 后的内部结果；让 adapter 显式返回输出模式。
 * @en Internal result after opening an output sink; keeps the adapter output mode explicit.
 */
interface OpenConversionOutputSinkResult {
    sink: PatchableOutputSink;
    mode: ConversionOutputTarget["mode"];
}

/**
 * @zh `openConversionOutputTarget` 导出函数。
 * @en Exported function `openConversionOutputTarget`.
 */
export async function openConversionOutputTarget(input: {
    sourceFileName: string | null;
    target: ResolvedSlicerExportTarget;
}): Promise<ConversionOutputTarget> {
    const { requestedTargetFormat, resolvedTargetFormat, resolvedExtension } = input.target;
    const resolvedFormatFacts = getFormatFacts(resolvedTargetFormat);
    const concreteExtension = resolvedExtension ?? resolvedFormatFacts.defaultExtension;
    const downloadName = buildDownloadFilename(input.sourceFileName, requestedTargetFormat, concreteExtension);
    const isZipTarget = resolvedFormatFacts.isZip;
    const output = await openConversionOutputSink({
        downloadName,
        resolvedTargetFormat,
        resolvedExtension: concreteExtension,
        isZipTarget
    });

    return {
        downloadName,
        jobName: downloadName.replace(/\.[^.]+$/, ""),
        isZipTarget,
        mode: output.mode,
        sink: output.sink
    };
}

async function openConversionOutputSink(input: {
    downloadName: string;
    resolvedTargetFormat: string;
    resolvedExtension?: string;
    isZipTarget: boolean;
}): Promise<OpenConversionOutputSinkResult> {
    const mimeType = input.isZipTarget ? "application/zip" : "application/octet-stream";
    try {
        const fileSystemSink = await openFileSystemSink(
            input.downloadName,
            mimeType,
            [`.${input.resolvedExtension ?? input.resolvedTargetFormat}`],
            { onDiagnostic: logFileSystemOutputDiagnostic }
        );
        return fileSystemSink
            ? { sink: createBufferedOutputSink(fileSystemSink), mode: "streaming" }
            : { sink: createBlobSink(), mode: "blob" };
    } catch {
        return { sink: createBlobSink(), mode: "blob" };
    }
}

function logFileSystemOutputDiagnostic(diagnostic: FileSystemOutputSinkDiagnostic): void {
    const { type, ...payload } = diagnostic;
    console.info(`[slicer-output] ${type} ${JSON.stringify(payload)}`);
}
