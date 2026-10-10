import type { BrowserDocumentSource } from "../document/source.js";
import type { SlicerFormatId } from "../types.js";

/**
 * @zh 格式专属 opener，可在浏览器侧用 streaming parser 或 ZIP directory 解析来创建文档 source。
 * @en Format-specific opener that can create a document source using a browser-side streaming parser or ZIP directory parser.
 */
export interface DocumentOpener {
    open(file: File): Promise<BrowserDocumentSource>;
}

/**
 * @zh 浏览器运行期的格式行为注册项；只承载 package 侧 document-opening 所需的轻量 behavior。
 * @en Browser-runtime format behavior registration; only carries lightweight behavior needed by package document opening.
 */
export interface BrowserFormatBehavior {
    readonly formatId: SlicerFormatId | string;
    readonly documentOpener?: DocumentOpener;
}
