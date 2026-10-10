import type { SlicerFormatId, SupportedFormatInfo } from "../types.js";

/**
 * @zh 格式能力事实，来自 WASM 运行期格式目录。
 * @en Format capability facts sourced from the WASM runtime format catalog.
 */
export interface FormatCapabilities {
    readonly canImport: boolean;
    readonly canExport: boolean;
    readonly isArchive: boolean;
    readonly supportsPreview: boolean;
    readonly supportsAntiAlias: boolean;
    readonly supportsPerLayerSettings: boolean;
    readonly supportsEncryption: boolean;
    readonly requiresFullDocumentParse: boolean;
}

/**
 * @zh package 级格式事实快照，支持 document opener 通过 canonical name、alias 或 extension 查询。
 * @en Package-level format facts snapshot for document openers to query by canonical name, alias, or extension.
 */
export interface FormatFacts {
    readonly format: string;
    readonly displayName: string;
    readonly family: string;
    readonly profileId: string;
    readonly aliases: readonly string[];
    readonly extensions: readonly string[];
    readonly defaultExtension: string;
    readonly targetNames: readonly string[];
    readonly isZip: boolean;
    readonly capabilities: FormatCapabilities;
    readonly specifics: SupportedFormatInfo["specifics"];
}

const factsByFormat = new Map<string, FormatFacts>();
const factsByLookupKey = new Map<string, FormatFacts>();

/**
 * @zh 将 WASM 支持格式信息同步为 package 可查询的格式事实。
 * @en Synchronizes WASM supported format info into package-queryable format facts.
 */
export function syncFormatFacts(formatInfos: readonly SupportedFormatInfo[]): void {
    factsByFormat.clear();
    factsByLookupKey.clear();
    for (const info of formatInfos) {
        storeFormatFacts(info);
    }
}

/**
 * @zh 查询单个格式事实，支持 canonical name、alias 和 extension。
 * @en Looks up format facts by canonical name, alias, or extension.
 */
export function getFormatFacts(format: string): FormatFacts {
    const facts = factsByLookupKey.get(format);
    if (!facts) {
        throw new Error(`Unsupported format facts entry: ${format}`);
    }
    return facts;
}

/**
 * @zh 列出当前已同步的 canonical 格式事实。
 * @en Lists the currently synchronized canonical format facts.
 */
export function listFormatFacts(): FormatFacts[] {
    return Array.from(factsByFormat.values());
}

/**
 * @zh 列出当前格式目录中可导入文件扩展名，返回带 `.` 前缀的排序列表。
 * @en Lists importable file extensions from the current format catalog, sorted and prefixed with `.`.
 */
export function listImportableFileExtensions(): string[] {
    const extensions = new Set<string>();
    for (const facts of listFormatFacts()) {
        if (!facts.capabilities.canImport) {
            continue;
        }

        for (const extension of facts.extensions) {
            extensions.add(`.${extension}`);
        }
    }

    return Array.from(extensions).sort();
}

/**
 * @zh 将任意格式 lookup key 解析为 canonical slicer format id。
 * @en Resolves an arbitrary format lookup key to a canonical slicer format id.
 */
export function resolveConcreteFormatId(value: string): SlicerFormatId {
    return getFormatFacts(value).format as SlicerFormatId;
}

function normalizeCapabilities(info: SupportedFormatInfo): FormatCapabilities {
    const { capabilities } = info;
    if (!capabilities) {
        throw new Error(`WASM format facts for ${info.name} missing capabilities`);
    }
    if (capabilities.is_archive !== info.is_zip) {
        throw new Error(`WASM format facts for ${info.name} archive flag mismatch`);
    }

    return {
        canImport: capabilities.can_import,
        canExport: capabilities.can_export,
        isArchive: capabilities.is_archive,
        supportsPreview: capabilities.supports_preview,
        supportsAntiAlias: capabilities.supports_anti_alias,
        supportsPerLayerSettings: capabilities.supports_per_layer_settings,
        supportsEncryption: capabilities.supports_encryption,
        requiresFullDocumentParse: capabilities.requires_full_document_parse
    };
}

function storeFormatFacts(info: SupportedFormatInfo): void {
    const facts: FormatFacts = {
        format: info.name,
        displayName: info.display_name,
        family: info.family,
        profileId: info.profile_id,
        aliases: [...info.aliases],
        extensions: [...info.extensions],
        defaultExtension: info.default_extension,
        targetNames: [...info.target_names],
        isZip: info.is_zip,
        capabilities: normalizeCapabilities(info),
        specifics: info.specifics
    };

    factsByFormat.set(facts.format, facts);
    factsByLookupKey.set(facts.format, facts);

    for (const alias of facts.aliases) {
        addFirstLookupKey(alias, facts);
    }

    for (const extension of facts.extensions) {
        addFirstLookupKey(extension, facts);
    }
}

function addFirstLookupKey(key: string, facts: FormatFacts): void {
    if (!factsByLookupKey.has(key)) {
        factsByLookupKey.set(key, facts);
    }
}
