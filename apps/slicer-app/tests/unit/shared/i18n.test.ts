import { describe, expect, it } from "vitest";

import { createAppI18n } from "../../../src/shared/i18n/i18n";
import en from "../../../src/shared/i18n/locales/en.json";
import zh from "../../../src/shared/i18n/locales/zh.json";
import { translateRuntimeMessage } from "../../../src/shared/i18n/runtime-messages";

describe("UI localization / 界面国际化", () => {
    it("keeps resource keys aligned and supports English plurals / 保持资源键一致并支持英文复数", () => {
        const keys = (object: object, prefix = ""): string[] =>
            Object.entries(object).flatMap(([key, value]) => {
                const name = prefix + key;
                return typeof value === "string" ? [name.replace(/_(one|other)$/, "")] : keys(value, name + ".");
            });
        expect([...new Set(keys(en))].sort()).toEqual([...new Set(keys(zh))].sort());
        const i18n = createAppI18n();
        expect(i18n.t("source.layerCount", { count: 1 })).toBe("1 layer");
        expect(i18n.t("source.layerCount", { count: 3 })).toBe("3 layers");
        void i18n.changeLanguage("zh");
        expect(i18n.t("source.layerCount", { count: 3 })).toBe("3 层");
        expect(i18n.t("source.supports", { extensions: ".goo, .ctb" })).toContain(".goo, .ctb");
    });

    it("translates live statuses while retaining diagnostics and unknown errors / 翻译状态并保留诊断与未知错误详情", () => {
        const i18n = createAppI18n("zh");
        const t = i18n.t.bind(i18n);
        expect(translateRuntimeMessage(t, "Parsed GOO: 914 layers, 8520x4320")).toBe("已解析 GOO：914 层，8520x4320");
        expect(translateRuntimeMessage(t, "Converting and preparing ZIP layers 5/10...")).toContain("5/10");
        expect(translateRuntimeMessage(t, "Convert error: unknown codec")).toBe("转换失败：unknown codec");
        expect(translateRuntimeMessage(t, "unrecognized low-level error")).toBe("unrecognized low-level error");
        const warning =
            "Warning: Source machine Mars geometry differs from target machine Photon (PW0). Export continues without layer resampling. Source is 100x200 at 10x20 mm; target profile is 200x400 at 20x40 mm. UVTools may report non-square pixels or an incorrect image ratio, and the model may print with stretched proportions.";
        expect(translateRuntimeMessage(t, warning)).toContain("源设备 Mars");
        expect(translateRuntimeMessage(t, warning)).toContain("100x200");
        expect(
            translateRuntimeMessage(
                t,
                "Converted 10 layers to CTB in 1.00s. Output 1 MB. workers; layers 0.5s; worker sum 0.4s; assemble 0.1s; finalize-only 0.1s."
            )
        ).toContain("已将 10 层转换为 CTB");
        void i18n.changeLanguage("en");
        expect(translateRuntimeMessage(t, "Parsed GOO: 914 layers, 8520x4320")).toBe(
            "Parsed GOO: 914 layers, 8520x4320"
        );
    });
});
