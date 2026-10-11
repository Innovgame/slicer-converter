import type { ReactNode } from "react";

import { useTranslation } from "../../../shared/i18n/i18n";
import { PRODUCT_VERSION } from "../../../product-version";
import { LayersIcon } from "./icons";
import { ProjectLinks } from "./project-links";
import { ThemeToggle } from "./theme-toggle";
import { LanguageToggle } from "./language-toggle";

interface AppShellProps {
    title: string;
    documentStatus: string;
    documentLoaded: boolean;
    help: ReactNode;
    source: ReactNode;
    viewer: ReactNode;
    inspector: ReactNode;
    status: ReactNode;
}

/**
 * @zh 桌面保持三栏。平板是上预览、下并排。手机按预览、源文件、导出向下滚动。
 * @en Desktop keeps three columns. Tablets put the preview above two columns. Phones scroll preview, source, then export.
 */
export function AppShell({
    title,
    documentStatus,
    documentLoaded,
    help,
    source,
    viewer,
    inspector,
    status
}: AppShellProps) {
    const { t } = useTranslation();
    const versionLabel = t("app.version", { version: PRODUCT_VERSION });

    return (
        <main className="sc-workbench">
            <header className="sc-header">
                <div className="sc-brand">
                    <div className="sc-brand-mark">
                        <LayersIcon />
                    </div>
                    <div className="sc-brand-copy">
                        <div className="sc-product-title">
                            <h1 className="sc-product-name">{title}</h1>
                            <span className="sc-app-version" aria-label={versionLabel} title={versionLabel}>
                                v{PRODUCT_VERSION}
                            </span>
                        </div>
                        <p className={documentLoaded ? "sc-document-status is-loaded" : "sc-document-status"}>
                            {documentStatus}
                        </p>
                    </div>
                </div>
                <div className="sc-header-actions">
                    <ProjectLinks />
                    {help}
                    <div className="sc-preferences" data-tour="preferences">
                        <ThemeToggle />
                        <LanguageToggle />
                    </div>
                </div>
            </header>
            <div className="sc-body">
                <div className="sc-viewer-column">
                    <section className="sc-viewport" data-tour="preview">
                        {viewer}
                    </section>
                </div>
                <aside className="sc-source">{source}</aside>
                <aside className="sc-inspector" data-tour="export">
                    {inspector}
                </aside>
            </div>
            <footer className="sc-status-bar">{status}</footer>
        </main>
    );
}
