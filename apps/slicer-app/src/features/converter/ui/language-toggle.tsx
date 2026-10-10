import { useTranslation } from "../../../shared/i18n/i18n";

/**
 * @zh 顶栏中英文切换按钮；仅修改界面语言偏好。
 * @en Header Chinese/English toggle; it only changes the presentation language preference.
 */
export function LanguageToggle() {
    const { t, i18n } = useTranslation();
    const nextLanguage = i18n.resolvedLanguage === "zh" ? "en" : "zh";
    const label = t(nextLanguage === "zh" ? "language.toChinese" : "language.toEnglish");
    return (
        <button
            type="button"
            className="sc-icon-btn sc-language-toggle"
            aria-label={label}
            title={label}
            onClick={() => void i18n.changeLanguage(nextLanguage)}
        >
            {nextLanguage === "zh" ? "中" : "EN"}
        </button>
    );
}
