import type { SupportedFormatCapabilities, SupportedFormatInfo } from "@slicer-converter/wasm";

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
    ...IMPORT_EXPORT_CAPABILITIES,
    can_export: false
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

type AnycubicSpecifics = Extract<SupportedFormatInfo["specifics"], { family: "anycubic" }>;
type AnycubicArchiveSpecifics = Extract<SupportedFormatInfo["specifics"], { family: "anycubic-archive" }>;
type ChituboxSpecifics = Extract<SupportedFormatInfo["specifics"], { family: "chitubox" }>;
type CrealitySpecifics = Extract<SupportedFormatInfo["specifics"], { family: "creality" }>;

type AnycubicFormatFactInput = Pick<
    AnycubicSpecifics,
    | "machine_id"
    | "display_name"
    | "extension"
    | "default_version"
    | "supported_versions"
    | "table_layout"
    | "resolution_x"
    | "resolution_y"
    | "display_width_mm"
    | "display_height_mm"
    | "machine_z_mm"
> &
    Partial<Pick<SupportedFormatInfo, "aliases" | "capabilities" | "profile_id" | "target_names">>;

/**
 * @zh 测试用 `anycubicFormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `anycubicFormatFact`.
 */
export function anycubicFormatFact(input: AnycubicFormatFactInput): SupportedFormatInfo {
    return formatFact({
        name: input.extension,
        display_name: input.display_name,
        family: "anycubic",
        profile_id: input.profile_id ?? `anycubic-${input.extension}`,
        aliases: input.aliases ?? [input.extension],
        extensions: [input.extension],
        target_names: input.target_names ?? [input.extension],
        is_zip: false,
        capabilities: input.capabilities,
        specifics: {
            family: "anycubic",
            machine_id: input.machine_id,
            display_name: input.display_name,
            extension: input.extension,
            default_version: input.default_version,
            supported_versions: input.supported_versions,
            table_layout: input.table_layout,
            resolution_x: input.resolution_x,
            resolution_y: input.resolution_y,
            display_width_mm: input.display_width_mm,
            display_height_mm: input.display_height_mm,
            machine_z_mm: input.machine_z_mm
        }
    });
}

/**
 * @zh 测试用 `anycubicArchiveFormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `anycubicArchiveFormatFact`.
 */
export function anycubicArchiveFormatFact(input: Partial<AnycubicArchiveSpecifics> = {}): SupportedFormatInfo {
    return formatFact({
        name: "anycubic-zip",
        display_name: "Anycubic Zip archive catalog",
        family: "anycubic",
        profile_id: "anycubic-zip-archive-catalog",
        aliases: ["anycubic-zip", "pm4u", "pm7"],
        extensions: ["pm4u", "pm7"],
        default_extension: "pm4u",
        target_names: ["anycubic-zip", "pm4u", "pm7"],
        is_zip: true,
        specifics: {
            family: "anycubic-archive",
            settings_manifest_entry: input.settings_manifest_entry ?? "anycubic_photon_resins.pwsp",
            layers_manifest_entry: input.layers_manifest_entry ?? "layers_controller.conf",
            layer_entry_prefix: input.layer_entry_prefix ?? "layer_images/layer_",
            default_layer_image_extension: input.default_layer_image_extension ?? "pw0Img",
            layer_image_extensions: input.layer_image_extensions ?? ["pw0Img", "pwszImg"],
            required_manifest_entries: input.required_manifest_entries ?? [
                "anycubic_photon_resins.pwsp",
                "layers_controller.conf"
            ]
        }
    });
}

type ChituboxFormatFactInput = Pick<SupportedFormatInfo, "name" | "display_name" | "aliases" | "extensions"> &
    Partial<
        Pick<SupportedFormatInfo, "capabilities" | "default_extension" | "profile_id" | "target_names"> &
            Pick<
                ChituboxSpecifics,
                | "kind"
                | "ctb_version"
                | "machine_profile"
                | "allowed_magics"
                | "allowed_versions"
                | "build_version"
                | "layer_table_entry_size"
            >
    >;

/**
 * @zh 测试用 `chituboxFormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `chituboxFormatFact`.
 */
export function chituboxFormatFact(input: ChituboxFormatFactInput): SupportedFormatInfo {
    return formatFact({
        name: input.name,
        display_name: input.display_name,
        family: "chitubox",
        profile_id: input.profile_id ?? `chitubox-${input.name}`,
        aliases: input.aliases,
        extensions: input.extensions,
        default_extension: input.default_extension,
        target_names: input.target_names ?? [input.name],
        is_zip: false,
        capabilities: input.capabilities,
        specifics: {
            family: "chitubox",
            kind: input.kind ?? "ctb",
            ctb_version: input.ctb_version ?? null,
            machine_profile: input.machine_profile ?? null,
            allowed_magics: input.allowed_magics ?? [],
            allowed_versions: input.allowed_versions ?? [],
            build_version: input.build_version ?? null,
            layer_table_entry_size: input.layer_table_entry_size ?? null
        }
    });
}

/**
 * @zh 测试用 `crealityCxdLpV1FormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `crealityCxdLpV1FormatFact`.
 */
export function crealityCxdLpV1FormatFact(input: Partial<CrealitySpecifics> = {}): SupportedFormatInfo {
    return formatFact({
        name: "cxdlp-v1",
        display_name: "Creality CXDLP v1 read-only profile",
        family: "creality",
        profile_id: "creality-cxdlp-v1-read-only",
        aliases: ["cxdlp-v1", "creality-cxdlp-v1", "v1.cxdlp"],
        extensions: ["v1.cxdlp"],
        target_names: ["cxdlp-v1", "creality-cxdlp-v1", "v1.cxdlp"],
        is_zip: false,
        capabilities: {
            ...IMPORT_ONLY_CAPABILITIES,
            supports_anti_alias: true,
            supports_per_layer_settings: false,
            supports_encryption: false
        },
        specifics: {
            family: "creality",
            variant: input.variant ?? "cxdlp-v1",
            version: input.version ?? 1,
            extension: input.extension ?? "v1.cxdlp",
            magic: input.magic ?? "CXSW3DV2",
            checksum: input.checksum ?? "none",
            layer_definition_size: input.layer_definition_size ?? 8,
            layer_parameter_size: input.layer_parameter_size ?? 8,
            preview_count: input.preview_count ?? 3,
            supports_layer_encryption: input.supports_layer_encryption ?? false
        }
    });
}

/**
 * @zh 测试用 `crealityCxdLpV2FormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `crealityCxdLpV2FormatFact`.
 */
export function crealityCxdLpV2FormatFact(input: Partial<CrealitySpecifics> = {}): SupportedFormatInfo {
    return formatFact({
        name: "cxdlp-v2",
        display_name: "Creality CXDLP v2 read-only profile",
        family: "creality",
        profile_id: "creality-cxdlp-v2-read-only",
        aliases: ["cxdlp-v2", "creality-cxdlp-v2"],
        extensions: ["cxdlp"],
        target_names: ["cxdlp-v2", "creality-cxdlp-v2"],
        is_zip: false,
        capabilities: {
            ...IMPORT_ONLY_CAPABILITIES,
            supports_anti_alias: true,
            supports_per_layer_settings: false,
            supports_encryption: false
        },
        specifics: {
            family: "creality",
            variant: input.variant ?? "cxdlp-v2",
            version: input.version ?? 2,
            extension: input.extension ?? "cxdlp",
            magic: input.magic ?? "CXSW3DV2",
            checksum: input.checksum ?? "xor8",
            layer_definition_size: input.layer_definition_size ?? 8,
            layer_parameter_size: input.layer_parameter_size ?? 8,
            preview_count: input.preview_count ?? 3,
            supports_layer_encryption: input.supports_layer_encryption ?? false
        }
    });
}

/**
 * @zh 测试用 `crealityCxdLpV3FormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `crealityCxdLpV3FormatFact`.
 */
export function crealityCxdLpV3FormatFact(input: Partial<CrealitySpecifics> = {}): SupportedFormatInfo {
    return formatFact({
        name: "cxdlp",
        display_name: "Creality CXDLP v3 read-only profile",
        family: "creality",
        profile_id: "creality-cxdlp-v3-read-only",
        aliases: ["cxdlp", "creality-cxdlp"],
        extensions: ["cxdlp"],
        target_names: ["cxdlp", "creality-cxdlp"],
        is_zip: false,
        capabilities: {
            ...IMPORT_ONLY_CAPABILITIES,
            supports_anti_alias: true,
            supports_per_layer_settings: false,
            supports_encryption: false
        },
        specifics: {
            family: "creality",
            variant: input.variant ?? "cxdlp",
            version: input.version ?? 3,
            extension: input.extension ?? "cxdlp",
            magic: input.magic ?? "CXSW3DV2",
            checksum: input.checksum ?? "crc32",
            layer_definition_size: input.layer_definition_size ?? 8,
            layer_parameter_size: input.layer_parameter_size ?? 8,
            preview_count: input.preview_count ?? 3,
            supports_layer_encryption: input.supports_layer_encryption ?? false
        }
    });
}

/**
 * @zh 测试用 `crealityCxdLpV4FormatFact` format fact fixture builder。
 * @en Test fixture format-fact builder `crealityCxdLpV4FormatFact`.
 */
export function crealityCxdLpV4FormatFact(input: Partial<CrealitySpecifics> = {}): SupportedFormatInfo {
    return formatFact({
        name: "cxdlpv4",
        display_name: "Creality CXDLPv4 profile",
        family: "creality",
        profile_id: "creality-cxdlpv4",
        aliases: ["cxdlpv4", "creality-cxdlpv4"],
        extensions: ["cxdlpv4"],
        target_names: ["cxdlpv4", "creality-cxdlpv4"],
        is_zip: false,
        capabilities: {
            ...IMPORT_EXPORT_CAPABILITIES,
            supports_anti_alias: true,
            supports_per_layer_settings: true,
            supports_encryption: true
        },
        specifics: {
            family: "creality",
            variant: input.variant ?? "cxdlpv4",
            version: input.version ?? 4,
            extension: input.extension ?? "cxdlpv4",
            magic: input.magic ?? "CXSW3DV2",
            checksum: input.checksum ?? "crc32",
            layer_definition_size: input.layer_definition_size ?? 40,
            layer_parameter_size: input.layer_parameter_size ?? 44,
            preview_count: input.preview_count ?? 2,
            supports_layer_encryption: input.supports_layer_encryption ?? true
        }
    });
}
