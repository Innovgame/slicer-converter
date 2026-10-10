import { useLayoutEffect, useState, type ReactNode } from "react";
import { I18nextProvider } from "react-i18next";

import { createAppI18n, type Language } from "./i18n";

/**
 * @zh 将 i18next 接入页面并保存语言偏好；切换语言不重建转换器状态。
 * @en Connects i18next and persists language preferences without rebuilding converter state.
 */
export function LanguageProvider({ children }: { children: ReactNode }) {
    const [i18n] = useState(() => {
        let language: Language = "en";
        try {
            if (window.localStorage.getItem("slicer-language") === "zh") language = "zh";
        } catch {
            // Storage may be unavailable in locked browser profiles.
        }
        return createAppI18n(language);
    });

    useLayoutEffect(() => {
        const persist = (language: string): void => {
            document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
            try {
                window.localStorage.setItem("slicer-language", language);
            } catch {
                // Storage may be unavailable in locked browser profiles.
            }
        };
        persist(i18n.language);
        i18n.on("languageChanged", persist);
        return () => {
            i18n.off("languageChanged", persist);
        };
    }, [i18n]);

    return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}
