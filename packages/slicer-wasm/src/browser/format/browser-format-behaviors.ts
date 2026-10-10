import { BUILTIN_BROWSER_FORMAT_BEHAVIORS } from "./behaviors/registry.js";
import { resolveConcreteFormatId } from "./facts.js";
import type { BrowserFormatBehavior, DocumentOpener } from "./behavior-contract.js";
import type { SupportedFormatInfo } from "../types.js";

const behaviorsByFormat = new Map<string, BrowserFormatBehavior>();

for (const behavior of BUILTIN_BROWSER_FORMAT_BEHAVIORS) {
    behaviorsByFormat.set(behavior.formatId, behavior);
}

/**
 * @zh 根据格式 lookup key 查询内建 browser document opener。
 * @en Looks up the built-in browser document opener for a format lookup key.
 */
export function getDocumentOpenerForFormat(format: string): DocumentOpener | undefined {
    const canonicalFormat = resolveConcreteFormatId(format);
    return behaviorsByFormat.get(canonicalFormat)?.documentOpener;
}

/**
 * @zh 校验内建 browser behavior 都能在 WASM format facts 中找到对应格式。
 * @en Validates that every built-in browser behavior has matching WASM format facts.
 */
export function validateBrowserFormatBehaviors(formatInfos: readonly SupportedFormatInfo[]): void {
    const factsByFormat = new Map(formatInfos.map((info) => [info.name, info]));
    const missingWasmFacts = Array.from(behaviorsByFormat.keys()).filter((format) => !factsByFormat.has(format));
    if (missingWasmFacts.length > 0) {
        throw new Error(`WASM format facts missing browser behavior entries: ${missingWasmFacts.join(", ")}`);
    }
}
