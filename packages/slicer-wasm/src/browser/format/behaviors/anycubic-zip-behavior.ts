import { parseAnycubicZipFile } from "../../archive/zip/header-parser.js";
import type { BrowserFormatBehavior } from "../behavior-contract.js";

/**
 * @zh Anycubic ZIP 的浏览器文档 opener adapter。
 * @en Browser document opener adapter for Anycubic ZIP.
 */
export const anycubicZipBrowserBehavior: BrowserFormatBehavior = {
    formatId: "anycubic-zip",
    documentOpener: {
        open: parseAnycubicZipFile
    }
};
