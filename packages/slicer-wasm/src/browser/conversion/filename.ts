import type { ExportTargetFormat } from "./target.js";

/**
 * @zh 根据源文件名和目标格式构建转换输出文件名。
 * @en Builds a converted output filename from the source filename and target format.
 */
export function buildSlicerOutputFilename(input: {
    readonly sourceFileName: string | null | undefined;
    readonly requestedTargetFormat: ExportTargetFormat;
    readonly resolvedExtension?: string;
}): string {
    const baseName = input.sourceFileName ? input.sourceFileName.replace(/\.[^.]+$/, "") : "converted";
    const extension = input.resolvedExtension ?? input.requestedTargetFormat;
    return `${baseName}.${extension}`;
}

/**
 * @zh 从输出文件名派生 archive job name。
 * @en Derives an archive job name from an output filename.
 */
export function buildSlicerOutputJobName(fileName: string): string {
    return fileName.replace(/\.[^.]+$/, "");
}
