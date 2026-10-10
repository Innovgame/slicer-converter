import type { SlicerFormatId, SupportedFormatInfo } from "../types.js";
import { syncFormatFacts } from "./facts.js";

export {
    getFormatFacts,
    listFormatFacts,
    listImportableFileExtensions,
    resolveConcreteFormatId,
    syncFormatFacts
} from "./facts.js";
export type { FormatCapabilities, FormatFacts } from "./facts.js";

const supportedFormatLookup = new Set<string>();

/**
 * @zh 同步 Rust 注册表导出的格式信息，用于后续格式 ID 校验。
 * @en Syncs Rust registry format information for later format-id validation.
 */
export function syncSupportedFormatInfosForValidation(formatInfos: readonly SupportedFormatInfo[]): void {
    supportedFormatLookup.clear();
    syncFormatFacts(formatInfos);

    for (const info of formatInfos) {
        supportedFormatLookup.add(info.name);
        for (const alias of info.aliases) {
            supportedFormatLookup.add(alias);
        }
        for (const extension of info.extensions) {
            supportedFormatLookup.add(extension);
        }
    }
}

/**
 * @zh 将任意字符串转换为 {@link SlicerFormatId}。若不在支持列表中则抛出异常。
 * @en Casts an arbitrary string to {@link SlicerFormatId}. Throws if the value is
 * not a supported format identifier.
 *
 * @param value - @zh 待验证的格式字符串 @en The format string to validate
 * @returns @zh 类型安全的 `SlicerFormatId` 值 @en A type-safe `SlicerFormatId` value
 * @throws @zh `value` 不属于支持格式时抛出 `Error` @en Throws `Error` when `value` is not a supported format
 */
export function asSlicerFormatId(value: string): SlicerFormatId {
    if (supportedFormatLookup.has(value)) {
        return value as SlicerFormatId;
    }

    throw new Error(`Unsupported format: ${value}`);
}
