import { parseSl1File } from "../../archive/zip/header-parser.js";
import type { BrowserFormatBehavior } from "../behavior-contract.js";

/**
 * @zh SL1 的浏览器文档 opener adapter。
 * @en Browser document opener adapter for SL1.
 */
export const sl1BrowserBehavior: BrowserFormatBehavior = {
    formatId: "sl1",
    documentOpener: {
        open: parseSl1File
    }
};
