import { CircleHelp, Languages, Sun } from "lucide-react";
import type { ReactNode } from "react";

import { useTheme } from "../../../app/providers";
import { VersionBadge } from "./version-badge";
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
                <div className="sc-settings-list sc-mobile-only">
                    <div className="sc-settings-row sc-mobile-only">
                        <Sun aria-hidden="true" />
                        <span>{t("mobile.appearance")}</span>
                        <div
                            role="group"
                            className="sc-ios-segmented"
                            aria-label={t("mobile.appearance")}
                            data-selected={theme === "dark" ? "1" : "0"}
                        >
                            <button type="button" aria-pressed={theme === "light"} onClick={() => setTheme("light")}>
                                {t("mobile.light")}
                            </button>
                            <button type="button" aria-pressed={theme === "dark"} onClick={() => setTheme("dark")}>
                                {t("mobile.dark")}
                            </button>
                        </div>
                    </div>
                    <div className="sc-settings-row sc-mobile-only">
                        <Languages aria-hidden="true" />
                        <span>{t("mobile.language")}</span>
                        <div
                            role="group"
                            className="sc-ios-segmented"
                            aria-label={t("mobile.language")}
                            data-selected={i18n.resolvedLanguage === "zh" ? "0" : "1"}
                        >
                            <button
                                type="button"
                                aria-pressed={i18n.resolvedLanguage === "zh"}
                                onClick={() => void i18n.changeLanguage("zh")}
                            >
                                中文
                            </button>
                            <button
                                type="button"
                                aria-pressed={i18n.resolvedLanguage !== "zh"}
                                onClick={() => void i18n.changeLanguage("en")}
                            >
                                EN
                            </button>
                        </div>
                    </div>
                </div>
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
                {t("app.title")} · <VersionBadge />
            </p>
        </div>
    );
}
