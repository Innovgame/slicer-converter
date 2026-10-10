import type { BrowserFormatBehavior } from "../behavior-contract.js";
import { anycubicZipBrowserBehavior } from "./anycubic-zip-behavior.js";
import { chituboxZipBrowserBehavior } from "./chitubox-zip-behavior.js";
import { gooBrowserBehavior } from "./goo-behavior.js";
import { sl1BrowserBehavior } from "./sl1-behavior.js";
import { vdtBrowserBehavior } from "./vdt-behavior.js";

/**
 * @zh package 内置 browser behavior adapter 闭合集合。
 * @en Closed set of package built-in browser behavior adapters.
 */
export const BUILTIN_BROWSER_FORMAT_BEHAVIORS: readonly BrowserFormatBehavior[] = [
    sl1BrowserBehavior,
    vdtBrowserBehavior,
    chituboxZipBrowserBehavior,
    anycubicZipBrowserBehavior,
    gooBrowserBehavior
];
