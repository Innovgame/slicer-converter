import type { AnycubicVersion, SlicerFormatId, SupportedFormatInfo } from "../types.js";
import { getFormatFacts, listFormatFacts, resolveConcreteFormatId, type FormatFacts } from "../format/format.js";

/**
 * @zh 可直接导出的真实格式，或需要二级选择的虚拟目标。
 * @en A concrete exportable format or a virtual target that requires a secondary choice.
 */
export type ExportTargetFormat = SlicerFormatId | "anycubic-manual" | "chitubox-manual";

/**
 * @zh Browser API 可直接用于 UI 的导出目标选项。
 * @en Export target option that browser API consumers can use directly in UI.
 */
export interface ExportTargetOption {
    readonly value: ExportTargetFormat;
    readonly label: string;
    readonly disabled: boolean;
}

/**
 * @zh 解析后的导出目标；虚拟目标最终收敛到真实 `SlicerFormatId`。
 * @en Resolved export target; virtual targets are folded into a concrete `SlicerFormatId`.
 */
export interface ResolvedSlicerExportTarget {
    readonly requestedTargetFormat: ExportTargetFormat;
    readonly resolvedTargetFormat: SlicerFormatId;
    readonly resolvedExtension?: string;
    readonly resolvedAnycubicVersion?: AnycubicVersion;
    readonly statusMessage: string | null;
}

/**
 * @zh 二级手动目标选项。
 * @en Secondary manual target option.
 */
export interface ManualSlicerTargetOption {
    readonly value: string;
    readonly resolvedTargetFormat: SlicerFormatId;
    readonly label: string;
    readonly extension: string;
    readonly anycubicDefaultVersion?: AnycubicVersion;
    readonly anycubicSupportedVersions?: readonly AnycubicVersion[];
}

/**
 * @zh Anycubic 手动目标的默认值。
 * @en Default value for Anycubic manual target selection.
 */
export const DEFAULT_ANYCUBIC_MANUAL_TARGET = "";

/**
 * @zh Chitubox 手动目标的默认值。
 * @en Default value for Chitubox manual target selection.
 */
export const DEFAULT_CHITUBOX_MANUAL_TARGET = "";

const PRIMARY_TARGET_UI_ORDER: readonly string[] = ["ctb", "sl1", "goo", "prz", "vdt", "cxdlpv4"];

const ANYCUBIC_MANUAL_TARGET_OPTION = {
    value: "anycubic-manual",
    label: "Anycubic Manual",
    disabled: false
} satisfies ExportTargetOption;

const CHITUBOX_MANUAL_TARGET_OPTION = {
    value: "chitubox-manual",
    label: "Chitubox Manual",
    disabled: false
} satisfies ExportTargetOption;

/**
 * @zh 列出导出目标选项，并禁用与源格式相同的直接目标。
 * @en Lists export target options and disables the direct target matching the source format.
 */
export function listExportTargetOptions(sourceFormat: SlicerFormatId | null): ExportTargetOption[] {
    const manualTargetOverrideValues = new Set(listManualTargetOverrideValues());
    const baseOptions = listFormatFacts()
        .filter((facts) => facts.capabilities.canExport)
        .filter((facts) => !manualTargetOverrideValues.has(facts.format))
        .sort((left, right) => comparePrimaryTargetUiOrder(left.format, right.format))
        .map((facts) => ({
            value: facts.format as SlicerFormatId,
            label: facts.displayName,
            disabled: sourceFormat === facts.format
        }));

    return [...listManualPrimaryTargetOptions(), ...baseOptions, ...listTrailingManualPrimaryTargetOptions()];
}

/**
 * @zh 选择默认导出目标，优先返回第一个不同于源格式的可导出目标。
 * @en Selects a default export target, preferring the first export-capable target different from the source format.
 */
export function selectDefaultExportTarget(sourceFormat: SlicerFormatId): ExportTargetFormat {
    return listExportTargetOptions(sourceFormat).find((option) => option.value !== sourceFormat)?.value ?? "ctb";
}

/**
 * @zh 将用户选择的目标解析为真实可导出格式。
 * @en Resolves a user-selected target into a concrete exportable format.
 */
export function resolveSlicerExportTarget(input: {
    readonly sourceFormat: SlicerFormatId;
    readonly selectedTarget: string;
    readonly anycubicManualTarget?: string;
    readonly anycubicManualVersion?: AnycubicVersion;
    readonly chituboxManualTarget?: string;
}): ResolvedSlicerExportTarget {
    const targetFormat = asExportTargetFormat(input.selectedTarget);

    if (targetFormat === "chitubox-manual") {
        const target = resolveManualExportTarget({
            targetFormat,
            manualTarget: input.chituboxManualTarget ?? DEFAULT_CHITUBOX_MANUAL_TARGET,
            virtualTarget: "chitubox-manual",
            options: listChituboxManualTargetOptions(),
            noAvailableTargetsMessage: "No exportable Chitubox target is available.",
            statusLabel: "Chitubox"
        });
        return { requestedTargetFormat: targetFormat, ...target };
    }

    if (targetFormat === "anycubic-manual") {
        const target = resolveManualExportTarget({
            targetFormat,
            manualTarget: input.anycubicManualTarget ?? DEFAULT_ANYCUBIC_MANUAL_TARGET,
            virtualTarget: "anycubic-manual",
            options: listAnycubicManualTargetOptions(),
            noAvailableTargetsMessage: "No exportable Anycubic target is available.",
            statusLabel: "Anycubic",
            anycubicManualVersion: input.anycubicManualVersion
        });
        return { requestedTargetFormat: targetFormat, ...target };
    }

    return {
        requestedTargetFormat: targetFormat,
        resolvedTargetFormat: targetFormat,
        resolvedExtension: undefined,
        resolvedAnycubicVersion: undefined,
        statusMessage: null
    };
}

/**
 * @zh 列出 Anycubic 二级手动目标。
 * @en Lists Anycubic secondary manual targets.
 */
export function listAnycubicManualTargetOptions(): ManualSlicerTargetOption[] {
    return listFormatFacts()
        .filter((facts) => facts.capabilities.canExport)
        .filter((facts) => facts.specifics.family === "anycubic" || facts.specifics.family === "anycubic-archive")
        .flatMap((facts) => {
            const specifics = facts.specifics;
            if (specifics.family === "anycubic") {
                return [
                    {
                        value: specifics.extension,
                        resolvedTargetFormat: specifics.extension,
                        label: `${specifics.display_name} (${specifics.extension.toUpperCase()})`,
                        extension: specifics.extension,
                        anycubicDefaultVersion: specifics.default_version,
                        anycubicSupportedVersions: specifics.supported_versions
                    }
                ];
            }

            if (specifics.family === "anycubic-archive") {
                return facts.targetNames
                    .filter((targetName) => targetName !== facts.format)
                    .map((targetName) => ({
                        value: targetName,
                        resolvedTargetFormat: facts.format as SlicerFormatId,
                        label: `${facts.displayName} (${targetName.toUpperCase()})`,
                        extension: targetName
                    }));
            }

            throw new Error("unreachable non-anycubic specifics");
        });
}

/**
 * @zh Chitubox 二级目标中的版本候选。
 * @en Version candidate inside a secondary Chitubox target.
 */
export interface ChituboxManualTargetVersionOption {
    readonly value: SlicerFormatId;
    readonly label: string;
    readonly version: number;
}

/**
 * @zh Chitubox 二级目标分组，用于 UI 按需显示版本。
 * @en Chitubox secondary target group for UIs that expose version selection.
 */
export interface ChituboxManualTargetGroup {
    readonly value: string;
    readonly label: string;
    readonly defaultTarget: SlicerFormatId;
    readonly versions: readonly ChituboxManualTargetVersionOption[];
}

/**
 * @zh 列出 Chitubox 二级手动目标。
 * @en Lists Chitubox secondary manual targets.
 */
export function listChituboxManualTargetOptions(): ManualSlicerTargetOption[] {
    return listExportableChituboxFacts().map((facts) => ({
        value: facts.format as SlicerFormatId,
        resolvedTargetFormat: facts.format as SlicerFormatId,
        label: `${facts.displayName} (${facts.defaultExtension})`,
        extension: facts.defaultExtension
    }));
}

/**
 * @zh 列出 Chitubox 二级目标分组。
 * @en Lists grouped Chitubox secondary targets.
 */
export function listChituboxManualTargetGroups(): ChituboxManualTargetGroup[] {
    const groups = new Map<string, ChituboxManualTargetGroup>();

    for (const facts of listExportableChituboxFacts()) {
        const groupValue = chituboxTargetGroupValue(facts);
        const version = facts.specifics.build_version ?? facts.specifics.ctb_version;
        const versionOption = {
            value: facts.format as SlicerFormatId,
            label: version === null ? facts.defaultExtension : String(version),
            version: version ?? 0
        };
        const existing = groups.get(groupValue);
        if (existing) {
            groups.set(groupValue, {
                ...existing,
                versions: [...existing.versions, versionOption].sort(compareChituboxVersionOptions)
            });
            continue;
        }

        groups.set(groupValue, {
            value: groupValue,
            label: chituboxTargetGroupLabel(facts),
            defaultTarget: facts.format as SlicerFormatId,
            versions: [versionOption]
        });
    }

    return Array.from(groups.values());
}

function asExportTargetFormat(value: string): ExportTargetFormat {
    if (isVirtualExportTargetFormat(value)) {
        return value;
    }

    const format = resolveConcreteFormatId(value);
    const facts = getFormatFacts(format);
    if (!facts.capabilities.canExport) {
        throw new Error(`Format ${value} is not export-capable`);
    }

    return format;
}

function isVirtualExportTargetFormat(value: string): value is Exclude<ExportTargetFormat, SlicerFormatId> {
    return value === "anycubic-manual" || value === "chitubox-manual";
}

function listManualTargetOverrideValues(): string[] {
    return [
        ...listManualTargetValues(listChituboxManualTargetOptions()),
        ...listManualTargetValues(listAnycubicManualTargetOptions())
    ];
}

function listManualPrimaryTargetOptions(): ExportTargetOption[] {
    return [...(listChituboxManualTargetOptions().length > 0 ? [CHITUBOX_MANUAL_TARGET_OPTION] : [])];
}

function listTrailingManualPrimaryTargetOptions(): ExportTargetOption[] {
    return [...(listAnycubicManualTargetOptions().length > 0 ? [ANYCUBIC_MANUAL_TARGET_OPTION] : [])];
}

function listManualTargetValues(options: readonly ManualSlicerTargetOption[]): string[] {
    return options.map((option) => option.value);
}

function resolveManualExportTarget(input: {
    readonly targetFormat: ExportTargetFormat;
    readonly manualTarget: string;
    readonly virtualTarget: ExportTargetFormat;
    readonly options: readonly ManualSlicerTargetOption[];
    readonly noAvailableTargetsMessage: string;
    readonly statusLabel: string;
    readonly anycubicManualVersion?: AnycubicVersion;
}): Omit<ResolvedSlicerExportTarget, "requestedTargetFormat"> {
    if (input.targetFormat !== input.virtualTarget) {
        return {
            resolvedTargetFormat: input.targetFormat,
            resolvedExtension: undefined,
            resolvedAnycubicVersion: undefined,
            statusMessage: null
        };
    }

    const manualTarget = normalizeManualTargetAlias(input.manualTarget, input.options);
    const option = manualTarget
        ? input.options.find((candidate) => candidate.value === manualTarget)
        : input.options[0];
    if (!option) {
        throw new Error(input.noAvailableTargetsMessage);
    }

    return {
        resolvedTargetFormat: option.resolvedTargetFormat,
        resolvedExtension: option.extension,
        resolvedAnycubicVersion: resolveAnycubicManualVersion({
            option,
            requestedVersion: input.anycubicManualVersion
        }),
        statusMessage: `Using manual ${input.statusLabel} target: ${option.label}`
    };
}

function normalizeManualTargetAlias(manualTarget: string, options: readonly ManualSlicerTargetOption[]): string {
    const direct = options.find((candidate) => candidate.value === manualTarget);
    if (direct) {
        return direct.value;
    }

    const identityAlias = options.find((candidate) => candidate.resolvedTargetFormat === manualTarget);
    return identityAlias?.extension ?? manualTarget;
}

function resolveAnycubicManualVersion(input: {
    readonly option: ManualSlicerTargetOption;
    readonly requestedVersion?: AnycubicVersion;
}): AnycubicVersion | undefined {
    const supportedVersions = input.option.anycubicSupportedVersions;
    if (!supportedVersions || supportedVersions.length === 0) {
        return undefined;
    }

    const version = input.requestedVersion ?? input.option.anycubicDefaultVersion ?? supportedVersions[0];
    if (supportedVersions.includes(version)) {
        return version;
    }

    throw new Error(
        `Unsupported Anycubic version ${version} for ${input.option.label}. Supported versions: ${supportedVersions.join(", ")}`
    );
}

type ChituboxSpecifics = Extract<SupportedFormatInfo["specifics"], { family: "chitubox" }>;
type ChituboxFormatFacts = FormatFacts & { readonly specifics: ChituboxSpecifics };

function listExportableChituboxFacts(): ChituboxFormatFacts[] {
    return listFormatFacts()
        .filter((facts) => facts.capabilities.canExport)
        .filter(isChituboxFacts);
}

function isChituboxFacts(facts: FormatFacts): facts is ChituboxFormatFacts {
    return facts.specifics.family === "chitubox";
}

function chituboxTargetGroupValue(facts: ChituboxFormatFacts): string {
    return facts.specifics.kind === "ctb" ? "ctb" : facts.specifics.kind;
}

function chituboxTargetGroupLabel(facts: ChituboxFormatFacts): string {
    if (facts.specifics.kind === "ctb") {
        return "Chitubox CTB";
    }
    return facts.displayName.replace(/\s+v\d+$/i, "");
}

function compareChituboxVersionOptions(
    left: ChituboxManualTargetVersionOption,
    right: ChituboxManualTargetVersionOption
): number {
    return left.version - right.version;
}

function comparePrimaryTargetUiOrder(left: string, right: string): number {
    const leftIndex = PRIMARY_TARGET_UI_ORDER.indexOf(left);
    const rightIndex = PRIMARY_TARGET_UI_ORDER.indexOf(right);
    if (leftIndex !== -1 || rightIndex !== -1) {
        return (
            (leftIndex === -1 ? Number.MAX_SAFE_INTEGER : leftIndex) -
            (rightIndex === -1 ? Number.MAX_SAFE_INTEGER : rightIndex)
        );
    }
    return left.localeCompare(right);
}
