import { CircleHelp, Languages, Sun } from "lucide-react";
import type { ReactNode } from "react";

import { useTheme } from "../../../app/providers";
import { PRODUCT_VERSION } from "../../../product-version";
import { useTranslation } from "../../../shared/i18n/i18n";
import { LanguageToggle } from "./language-toggle";
import { ProjectLinks } from "./project-links";
import { ThemeToggle } from "./theme-toggle";

/**
 * @internal
 * @zh 移动端独立设置页，桌面端保留顶栏偏好、帮助与项目链接。
 * @en Standalone mobile settings page; retains header preferences, help and project links on desktop.
 */
export function SettingsPanel({ help }: { help: ReactNode }) {
    const { t, i18n } = useTranslation();
    const { theme, setTheme } = useTheme();

    return (
        <div className="sc-settings-content">
            <header className="sc-settings-heading sc-mobile-only">
                <h2>{t("mobile.settings")}</h2>
                <p>{t("mobile.settingsDescription")}</p>
            </header>
            <section className="sc-settings-preferences" data-tour="preferences">
                <h3 className="sc-mobile-only">{t("mobile.interface")}</h3>
                <label className="sc-settings-row sc-mobile-only">
                    <Sun aria-hidden="true" />
                    <span>{t("mobile.appearance")}</span>
                    <select value={theme} onChange={(event) => setTheme(event.target.value as "light" | "dark")}>
                        <option value="light">{t("mobile.light")}</option>
                        <option value="dark">{t("mobile.dark")}</option>
                    </select>
                </label>
                <label className="sc-settings-row sc-mobile-only">
                    <Languages aria-hidden="true" />
                    <span>{t("mobile.language")}</span>
                    <select
                        value={i18n.resolvedLanguage === "zh" ? "zh" : "en"}
                        onChange={(event) => void i18n.changeLanguage(event.target.value)}
                    >
                        <option value="zh">简体中文</option>
                        <option value="en">English</option>
                    </select>
                </label>
                <div className="sc-preferences sc-desktop-only">
                    <ThemeToggle />
                    <LanguageToggle />
                </div>
            </section>
            <section className="sc-settings-help">
                <h3 className="sc-mobile-only">{t("mobile.help")}</h3>
                <div className="sc-settings-tour">
                    <CircleHelp className="sc-mobile-only" aria-hidden="true" />
                    {help}
                </div>
            </section>
            <section className="sc-settings-project">
                <h3 className="sc-mobile-only">{t("mobile.project")}</h3>
                <ProjectLinks />
            </section>
            <p className="sc-settings-version sc-mobile-only">
                {t("app.title")} · v{PRODUCT_VERSION}
            </p>
        </div>
    );
}
