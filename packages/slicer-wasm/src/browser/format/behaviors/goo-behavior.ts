import { parseGooHeaderStreaming } from "../goo-header-parser.js";
import type { BrowserFormatBehavior } from "../behavior-contract.js";

/**
 * @zh GOO 的浏览器文档 opener adapter。
 * @en Browser document opener adapter for GOO.
 */
export const gooBrowserBehavior: BrowserFormatBehavior = {
    formatId: "goo",
    documentOpener: {
        async open(file) {
            return {
                parsedHeader: await parseGooHeaderStreaming(file),
                layerSource: { kind: "raw-file" }
            };
        }
    }
};
