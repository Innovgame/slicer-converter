import {
    listAnycubicManualTargetOptions,
    listChituboxManualTargetGroups,
    listExportTargetOptions,
    resolveSlicerExportTarget,
    type ResolvedSlicerExportTarget
} from "./target.js";
import { getFormatFacts } from "../format/facts.js";
import type { SlicerFormatId } from "../types.js";

/**
 * @zh 导出目标矩阵中的格式选项。
 * @en Format option in the export target matrix.
 */
export interface SlicerExportTargetFormatOption {
    readonly id: string;
    readonly name: string;
    readonly suffixes: readonly SlicerExportTargetSuffixOption[];
}

/**
 * @zh 导出目标矩阵中的后缀选项。
 * @en Suffix option in the export target matrix.
 */
export interface SlicerExportTargetSuffixOption {
    readonly id: string;
    readonly name: string;
    readonly defaultVersionId?: string;
    readonly versions: readonly SlicerExportTargetVersionOption[];
}

/**
 * @zh 导出目标矩阵中的版本选项，携带已解析的导出目标。
 * @en Version option in the export target matrix, carrying the resolved export target.
 */
export interface SlicerExportTargetVersionOption {
    readonly id: string;
    readonly name: string;
    readonly target: ResolvedSlicerExportTarget;
}

/**
 * @zh 导出目标矩阵中的完整选择。
 * @en Complete selection inside the export target matrix.
 */
export interface SlicerExportTargetMatrixSelection {
    readonly format: SlicerExportTargetFormatOption;
    readonly suffix: SlicerExportTargetSuffixOption;
    readonly version: SlicerExportTargetVersionOption;
}

/**
 * @zh 外部浏览器 app 通过矩阵 id 准备导出目标时需要的派生信息。
 * @en Derived export target details needed by external browser apps preparing from matrix ids.
 */
export interface PreparedSlicerExportTarget {
    readonly selection: SlicerExportTargetMatrixSelection;
    readonly target: ResolvedSlicerExportTarget;
    readonly extension: string;
    readonly fileName: string;
}

/**
 * @zh 列出 UI 可消费的导出目标矩阵：格式 -> 后缀 -> 版本。
 * @en Lists the UI-consumable export target matrix: format -> suffix -> version.
 */
export function listSlicerExportTargetMatrix(sourceFormat: SlicerFormatId | null): SlicerExportTargetFormatOption[] {
    const options: SlicerExportTargetFormatOption[] = [];

    for (const option of listExportTargetOptions(sourceFormat)) {
        if (option.disabled) {
            continue;
        }

        if (option.value === "chitubox-manual") {
            options.push(buildChituboxManualMatrixOption(sourceFormat));
            continue;
        }

        if (option.value === "anycubic-manual") {
            options.push(buildAnycubicManualMatrixOption(sourceFormat));
            continue;
        }

        options.push(buildDirectMatrixOption(sourceFormat, option.value));
    }

    return options.filter((option) => option.suffixes.length > 0);
}

/**
 * @zh 按给定 id 查找导出目标矩阵选择，缺失时回退到第一个可用项。
 * @en Finds an export target matrix selection by id, falling back to the first available option.
 */
export function findSlicerExportTargetMatrixSelection(input: {
    readonly sourceFormat: SlicerFormatId | null;
    readonly formatId?: string | null;
    readonly suffixId?: string | null;
    readonly versionId?: string | null;
}): SlicerExportTargetMatrixSelection | null {
    const formats = listSlicerExportTargetMatrix(input.sourceFormat);
    const format = findByIdOrFirst(formats, input.formatId);
    const suffix = format ? findByIdOrFirst(format.suffixes, input.suffixId) : undefined;
    const version = suffix ? findVersionByIdOrDefault(suffix, input.versionId) : undefined;

    if (!format || !suffix || !version) {
        return null;
    }

    return { format, suffix, version };
}

/**
 * @zh 从 UI 矩阵选择 id 准备外部 app 常用的 target、扩展名和下载文件名。
 * @en Prepares the target, extension, and download filename commonly needed by external apps from matrix selection ids.
 */
export function prepareSlicerExportTargetFromMatrix(input: {
    readonly sourceFormat: SlicerFormatId | null;
    readonly fileNameStem: string;
    readonly formatId?: string | null;
    readonly suffixId?: string | null;
    readonly versionId?: string | null;
}): PreparedSlicerExportTarget | null {
    const selection = findSlicerExportTargetMatrixSelection(input);
    if (!selection) {
        return null;
    }

    const target = selection.version.target;
    const extension = target.resolvedExtension ?? selection.suffix.id;
    return {
        selection,
        target,
        extension,
        fileName: `${input.fileNameStem}.${extension}`
    };
}

function buildDirectMatrixOption(
    sourceFormat: SlicerFormatId | null,
    formatId: SlicerFormatId
): SlicerExportTargetFormatOption {
    const facts = getFormatFacts(formatId);
    return {
        id: facts.format,
        name: facts.displayName,
        suffixes: [
            {
                id: facts.defaultExtension,
                name: facts.defaultExtension,
                versions: [
                    {
                        id: "default",
                        name: "默认",
                        target: resolveSlicerExportTarget({
                            sourceFormat: toResolverSourceFormat(sourceFormat, formatId),
                            selectedTarget: formatId
                        })
                    }
                ]
            }
        ]
    };
}

function buildChituboxManualMatrixOption(sourceFormat: SlicerFormatId | null): SlicerExportTargetFormatOption {
    return {
        id: "chitubox-manual",
        name: "Chitubox",
        suffixes: listChituboxManualTargetGroups().map((group) => ({
            id: group.value,
            name: group.label,
            versions: group.versions.map((version) => ({
                id: version.value,
                name: version.label,
                target: resolveSlicerExportTarget({
                    sourceFormat: toResolverSourceFormat(sourceFormat, version.value),
                    selectedTarget: "chitubox-manual",
                    chituboxManualTarget: version.value
                })
            }))
        }))
    };
}

function buildAnycubicManualMatrixOption(sourceFormat: SlicerFormatId | null): SlicerExportTargetFormatOption {
    return {
        id: "anycubic-manual",
        name: "Anycubic",
        suffixes: Array.from(groupAnycubicManualTargetSuffixes(sourceFormat).values())
    };
}

function groupAnycubicManualTargetSuffixes(
    sourceFormat: SlicerFormatId | null
): Map<string, SlicerExportTargetSuffixOption> {
    const suffixes = new Map<string, SlicerExportTargetSuffixOption>();
    const versionIdsBySuffix = new Map<string, Set<string>>();

    for (const option of listAnycubicManualTargetOptions()) {
        const existing = suffixes.get(option.extension);
        const versionIds = versionIdsBySuffix.get(option.extension) ?? new Set<string>();
        const versions = expandAnycubicManualTargetVersions(sourceFormat, option).filter((version) => {
            if (versionIds.has(version.id)) {
                return false;
            }
            versionIds.add(version.id);
            return true;
        });
        versionIdsBySuffix.set(option.extension, versionIds);

        if (versions.length === 0) {
            continue;
        }

        const defaultVersionId = existing?.defaultVersionId ?? anycubicDefaultVersionId(option);
        suffixes.set(option.extension, {
            id: option.extension,
            name: option.extension,
            ...(defaultVersionId ? { defaultVersionId } : {}),
            versions: [...(existing?.versions ?? []), ...versions]
        });
    }

    return suffixes;
}

function expandAnycubicManualTargetVersions(
    sourceFormat: SlicerFormatId | null,
    option: ReturnType<typeof listAnycubicManualTargetOptions>[number]
): SlicerExportTargetVersionOption[] {
    if (!option.anycubicSupportedVersions || option.anycubicSupportedVersions.length === 0) {
        return [
            {
                id: option.resolvedTargetFormat,
                name: "默认",
                target: resolveSlicerExportTarget({
                    sourceFormat: toResolverSourceFormat(sourceFormat, option.resolvedTargetFormat),
                    selectedTarget: "anycubic-manual",
                    anycubicManualTarget: option.value
                })
            }
        ];
    }

    return option.anycubicSupportedVersions.map((version) => ({
        id: String(version),
        name: String(version),
        target: resolveSlicerExportTarget({
            sourceFormat: toResolverSourceFormat(sourceFormat, option.resolvedTargetFormat),
            selectedTarget: "anycubic-manual",
            anycubicManualTarget: option.value,
            anycubicManualVersion: version
        })
    }));
}

function findByIdOrFirst<T extends { readonly id: string }>(options: readonly T[], id?: string | null): T | undefined {
    return (id ? options.find((option) => option.id === id) : undefined) ?? options[0];
}

function findVersionByIdOrDefault(
    suffix: SlicerExportTargetSuffixOption,
    id?: string | null
): SlicerExportTargetVersionOption | undefined {
    return (
        (id ? suffix.versions.find((version) => version.id === id) : undefined) ??
        (suffix.defaultVersionId
            ? suffix.versions.find((version) => version.id === suffix.defaultVersionId)
            : undefined) ??
        suffix.versions[0]
    );
}

function anycubicDefaultVersionId(
    option: ReturnType<typeof listAnycubicManualTargetOptions>[number]
): string | undefined {
    return option.anycubicDefaultVersion === undefined ? undefined : String(option.anycubicDefaultVersion);
}

function toResolverSourceFormat(sourceFormat: SlicerFormatId | null, fallbackFormat: SlicerFormatId): SlicerFormatId {
    return sourceFormat ?? fallbackFormat;
}
