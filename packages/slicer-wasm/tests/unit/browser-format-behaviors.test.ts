import { beforeEach, describe, expect, it, vi } from "vitest";

import {
    getDocumentOpenerForFormat,
    validateBrowserFormatBehaviors
} from "../../src/browser/format/browser-format-behaviors.js";
import { syncFormatFacts } from "../../src/browser/format/format.js";
import { IMPORT_ONLY_CAPABILITIES, formatFact } from "./support/format-facts.js";

const KNOWN_BROWSER_BEHAVIOR_FACTS = [
    formatFact({ name: "sl1", aliases: ["sl1", "sl1s"], extensions: ["sl1", "sl1s"], is_zip: true }),
    formatFact({ name: "vdt", aliases: ["vdt"], extensions: ["vdt"], is_zip: true }),
    formatFact({ name: "chitubox-zip", aliases: ["chitubox-zip", "chituzip"], extensions: ["zip"], is_zip: true }),
    formatFact({
        name: "anycubic-zip",
        aliases: ["anycubic-zip", "pm7"],
        extensions: ["anycubic-zip", "pm7"],
        is_zip: true
    }),
    formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
];

describe("browser format behaviors / 浏览器格式行为", () => {
    beforeEach(() => {
        vi.resetModules();
    });

    it("resolves built-in document openers through canonical aliases and extensions / 通过 canonical aliases and extensions resolving built-in document openers", () => {
        syncFormatFacts(KNOWN_BROWSER_BEHAVIOR_FACTS);

        expect(getDocumentOpenerForFormat("goo")?.open).toBeTypeOf("function");
        expect(getDocumentOpenerForFormat("sl1s")?.open).toBeTypeOf("function");
        expect(getDocumentOpenerForFormat("vdt")?.open).toBeTypeOf("function");
        expect(getDocumentOpenerForFormat("chituzip")?.open).toBeTypeOf("function");
        expect(getDocumentOpenerForFormat("pm7")?.open).toBeTypeOf("function");
        expect(getDocumentOpenerForFormat("zip")?.open).toBeTypeOf("function");
    });

    it("returns undefined when synced facts have no built-in browser behavior / 返回 undefined 当 synced facts have no built-in browser behavior", () => {
        syncFormatFacts([formatFact({ name: "foozip", aliases: ["foozip"], extensions: ["fzp"], is_zip: true })]);

        expect(getDocumentOpenerForFormat("foozip")).toBeUndefined();
    });

    it("requires synced facts before resolving a built-in browser behavior / requires synced facts 在 resolving built-in browser behavior 之前", () => {
        syncFormatFacts([]);

        expect(() => getDocumentOpenerForFormat("goo")).toThrow(/Unsupported format facts entry: goo/);
    });

    it("fails fast when wasm facts omit a built-in browser behavior format / 失败 fast 当 wasm facts omit a built-in browser behavior format", () => {
        const formatFacts = [formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })];

        expect(() => validateBrowserFormatBehaviors(formatFacts)).toThrow(
            /WASM format facts missing browser behavior entries:/
        );
    });

    it("accepts Rust-only profiles without browser behaviors / 接受 Rust-only profiles without browser behaviors", () => {
        const formatFacts = [
            ...KNOWN_BROWSER_BEHAVIOR_FACTS,
            formatFact({
                name: "phz",
                aliases: ["phz"],
                extensions: ["phz"],
                is_zip: false,
                capabilities: IMPORT_ONLY_CAPABILITIES
            })
        ];

        syncFormatFacts(formatFacts);
        validateBrowserFormatBehaviors(formatFacts);

        expect(getDocumentOpenerForFormat("phz")).toBeUndefined();
    });
});
