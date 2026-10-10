import type { SupportedFormatCapabilities, SupportedFormatInfo } from "../../../src/browser/types.js";

/**
 * @zh 测试用 `IMPORT_EXPORT_CAPABILITIES` capabilities fixture。
 * @en Test fixture capabilities for `IMPORT_EXPORT_CAPABILITIES`.
 */
export const IMPORT_EXPORT_CAPABILITIES: SupportedFormatCapabilities = {
    can_import: true,
    can_export: true,
    is_archive: false,
    supports_preview: true,
    supports_anti_alias: false,
    supports_per_layer_settings: false,
    requires_full_document_parse: false,
    supports_encryption: false
};

/**
 * @zh 测试用 `IMPORT_ONLY_CAPABILITIES` capabilities fixture。
 * @en Test fixture capabilities for `IMPORT_ONLY_CAPABILITIES`.
 */
export const IMPORT_ONLY_CAPABILITIES: SupportedFormatCapabilities = {
    can_import: true,
    can_export: false,
    is_archive: false,
    supports_preview: false,
    supports_anti_alias: false,
    supports_per_layer_settings: false,
    requires_full_document_parse: false,
    supports_encryption: false
};

type FormatFactInput = Omit<
    SupportedFormatInfo,
    "capabilities" | "default_extension" | "display_name" | "family" | "profile_id" | "specifics" | "target_names"
> &
    Partial<
        Pick<
            SupportedFormatInfo,
            | "capabilities"
            | "default_extension"
            | "display_name"
            | "family"
            | "profile_id"
            | "specifics"
            | "target_names"
        >
    >;

/**
 * @zh 测试用 `formatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `formatFact`.
 */
export function formatFact(input: FormatFactInput): SupportedFormatInfo {
    return {
        ...input,
        display_name: input.display_name ?? input.name.toUpperCase(),
        default_extension: input.default_extension ?? input.extensions[0] ?? input.name,
        family: input.family ?? input.name,
        profile_id: input.profile_id ?? `${input.family ?? input.name}-${input.name}`,
        target_names: input.target_names ?? [input.name],
        capabilities: input.capabilities ?? {
            ...IMPORT_EXPORT_CAPABILITIES,
            is_archive: input.is_zip,
            requires_full_document_parse: input.is_zip
        },
        specifics: input.specifics ?? { family: "none" }
    };
}
