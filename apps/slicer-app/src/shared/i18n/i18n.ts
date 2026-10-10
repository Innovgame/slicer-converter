import { createInstance } from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import zh from "./locales/zh.json";

/**
 * @zh 界面语言，仅控制展示；与文档和转换任务无关。
 * @en UI language, independent of document and conversion state.
 */
export type Language = "en" | "zh";

/**
 * @zh 创建使用本地资源的 i18next 实例，无网络请求，缺失文案回退英文。
 * @en Creates i18next with bundled resources and English fallback, without network requests.
 */
export function createAppI18n(language: Language = "en") {
    const instance = createInstance();
    void instance.init({
        lng: language,
        fallbackLng: "en",
        supportedLngs: ["en", "zh"],
        resources: { en: { translation: en }, zh: { translation: zh } },
        initAsync: false,
        interpolation: { escapeValue: false },
        react: { useSuspense: false }
    });
    return instance;
}

const defaultI18n = createAppI18n();
initReactI18next.init(defaultI18n);

export { useTranslation } from "react-i18next";
