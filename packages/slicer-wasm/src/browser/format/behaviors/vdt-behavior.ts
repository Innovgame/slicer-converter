import { parseVdtFile } from "../../archive/zip/header-parser.js";
import type { BrowserFormatBehavior } from "../behavior-contract.js";

/**
 * @zh VDT 的浏览器文档 opener adapter。
 * @en Browser document opener adapter for VDT.
 */
export const vdtBrowserBehavior: BrowserFormatBehavior = {
    formatId: "vdt",
    documentOpener: {
        open: parseVdtFile
    }
};
