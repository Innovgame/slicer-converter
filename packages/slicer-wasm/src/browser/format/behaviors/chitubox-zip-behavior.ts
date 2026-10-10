import { parseChituboxZipFile } from "../../archive/zip/header-parser.js";
import type { BrowserFormatBehavior } from "../behavior-contract.js";

/**
 * @zh Chitubox ZIP 的浏览器文档 opener adapter。
 * @en Browser document opener adapter for Chitubox ZIP.
 */
export const chituboxZipBrowserBehavior: BrowserFormatBehavior = {
    formatId: "chitubox-zip",
    documentOpener: {
        open: parseChituboxZipFile
    }
};
