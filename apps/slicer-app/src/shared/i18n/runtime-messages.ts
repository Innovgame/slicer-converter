import type { TFunction } from "i18next";

const messageKeys: Record<string, string> = {
    "Loading WASM...": "status.loading",
    "WASM loaded. Initializing worker pool...": "status.initializing",
    "Compiling WASM module...": "status.compiling",
    "WASM not loaded - cannot parse file.": "status.notReady",
    "Conversion cancelled.": "status.cancelled",
    "Converting...": "progress.converting",
    "Opening file...": "progress.opening",
    "Converting and preparing ZIP layers...": "progress.zipPreparing",
    "Finalizing archive...": "progress.finalizing",
    "Assembling file...": "progress.assembling",
    "Conversion complete.": "progress.complete"
};

/**
 * @zh 将底层英文消息适配为资源键；在展示时翻译，使现有状态随语言切换更新。
 * @en Adapts core English messages to resource keys at render time so existing status follows language changes.
 */
export function translateRuntimeMessage(t: TFunction, message: string): string {
    const warningStart = message.indexOf(" Warning:");
    if (message.startsWith("Converted ") && warningStart > 0) {
        return `${translateRuntimeMessage(t, message.slice(0, warningStart))} ${translateRuntimeMessage(t, message.slice(warningStart + 1))}`;
    }
    if (messageKeys[message]) return t(messageKeys[message]);
    let match = message.match(/^Parsed (.+): (\d+) layers, (.+)$/);
    if (match) return t("status.parsed", { format: match[1], count: Number(match[2]), resolution: match[3] });
    match = message.match(/^Ready - prewarmed (\d+) worker\(s\), scales up to (\d+) on demand\.$/);
    if (match) return t("status.ready", { workers: match[1], max: match[2] });
    match = message.match(/^Loading WASM\.\.\. (.+)$/);
    if (match) return t("status.loadingProgress", { progress: match[1] });
    match = message.match(/^Converting( and preparing ZIP)? layers? (\d+)\/(\d+)\.\.\.$/);
    if (match) return t(match[1] ? "progress.zipLayer" : "progress.layer", { current: match[2], total: match[3] });
    match = message.match(/^Using manual (.+) target: (.+)$/);
    if (match) return t("status.manualTarget", { format: match[1], target: match[2] });
    match = message.match(
        /^Warning: Source machine (.+) geometry differs from target machine (.+) \((.+)\)\. Export continues without layer resampling\. Source is (.+) at (.+) mm; target profile is (.+) at (.+) mm\. UVTools may report non-square pixels or an incorrect image ratio, and the model may print with stretched proportions\.$/
    );
    if (match)
        return t("status.geometryWarning", {
            source: match[1],
            target: match[2],
            extension: match[3],
            sourceResolution: match[4],
            sourceSize: match[5],
            targetResolution: match[6],
            targetSize: match[7]
        });
    match = message.match(
        /^Warning: PhotonS export keeps the source layer pixels but rewrites the bed size from (.+) mm to (.+) mm\. UVTools may report a changed image ratio\.$/
    );
    if (match) return t("status.photonsKeepWarning", { sourceSize: match[1], targetSize: match[2] });
    match = message.match(
        /^Warning: PhotonS export resamples layers from (.+) to (.+) and rewrites the bed size to (.+) mm\. Source bed is (.+) mm\. Nearest-neighbor resampling can change edges and printed proportions\.$/
    );
    if (match)
        return t("status.photonsResampleWarning", {
            sourceResolution: match[1],
            targetResolution: match[2],
            targetSize: match[3],
            sourceSize: match[4]
        });
    for (const [prefix, key] of [
        ["Failed to load WASM: ", "status.loadError"],
        ["Parse error: ", "status.parseError"],
        ["Convert error: ", "status.convertError"],
        ["Layer decode error: ", "status.decodeError"]
    ]) {
        if (message.startsWith(prefix)) return t(key, { detail: message.slice(prefix.length) });
    }
    match = message.match(/^Converted (\d+) layers to (.+?) in (.+?)\. Output (.+?)\. (.+)$/);
    if (match) {
        let diagnostics = match[5];
        for (const [text, key] of [
            ["workers (streamed)", "status.streamedWorkers"],
            ["workers", "status.workers"],
            ["setup", "status.setup"],
            ["layers", "status.layers"],
            ["worker sum", "status.workerSum"],
            ["assemble", "status.assemble"],
            ["finalize-only", "status.finalize"]
        ])
            diagnostics = diagnostics.replace(text, t(key));
        return t("status.converted", {
            count: Number(match[1]),
            format: match[2],
            duration: match[3],
            output: match[4] === "streamed to disk" ? t("status.disk") : match[4],
            diagnostics
        });
    }
    return message;
}
