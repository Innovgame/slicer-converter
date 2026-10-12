import { useEffect, useRef, type ReactNode } from "react";
import { FolderOpen, Info, Maximize, Minimize, SlidersHorizontal, X } from "lucide-react";

import { useTranslation } from "../../../shared/i18n/i18n";
import { PRODUCT_VERSION } from "../../../product-version";
import { LayersIcon } from "./icons";
import { SettingsPanel } from "./settings-panel";
import { useMobileWorkbench, type MobileWorkbenchSession } from "../model/use-mobile-workbench";

interface AppShellProps {
    title: string;
    documentStatus: string;
    documentLoaded: boolean;
    help: ReactNode | ((session: MobileWorkbenchSession) => ReactNode);
    source: ReactNode;
    viewer: ReactNode | ((session: MobileWorkbenchSession) => ReactNode);
    inspector: ReactNode | ((session: MobileWorkbenchSession) => ReactNode);
    status: ReactNode | ((session: MobileWorkbenchSession) => ReactNode);
    onBrowseFile?(): void;
    sourceSummary?: string;
    documentIdentity?: unknown;
    converting?: boolean;
}

/**
 * @zh 桌面与平板保留工作台布局；手机用独立设置页和覆盖画布的文件、导出抽屉。
 * @en Preserves desktop/tablet workbenches; phones use a separate settings page and source/export drawers over the canvas.
 */
export function AppShell({
    title,
    documentStatus,
    documentLoaded,
    help,
    source,
    viewer,
    inspector,
    status,
    onBrowseFile,
    sourceSummary,
    documentIdentity,
    converting = false
}: AppShellProps) {
    const { t } = useTranslation();
    const versionLabel = t("app.version", { version: PRODUCT_VERSION });
    const session = useMobileWorkbench(documentIdentity);
    const returnFocus = useRef<HTMLElement | null>(null);
    const drawerRef = useRef<HTMLElement | null>(null);
    const canvasButtonRef = useRef<HTMLButtonElement | null>(null);
    const restoreRef = useRef<HTMLButtonElement | null>(null);
    const wasImmersive = useRef(false);
    const openDrawer = (drawer: "source" | "export", trigger?: HTMLElement) => {
        returnFocus.current =
            trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
        session.openDrawer(drawer);
    };
    const closeDrawer = () => {
        session.closeDrawer();
    };
    const previewHidden = session.isMobile && session.tab !== "preview";
    const sourceOpen = session.isMobile && session.tab === "preview" && session.drawer === "source";
    const exportOpen = session.isMobile && session.tab === "preview" && session.drawer === "export";
    const chromeHidden = session.isMobile && (previewHidden || session.immersive || session.drawer !== null);

    useEffect(() => {
        if (!session.isMobile || session.tab !== "preview") return;
        if (!session.drawer) {
            returnFocus.current?.focus();
            return;
        }
        drawerRef.current?.querySelector<HTMLButtonElement>(".sc-drawer-close")?.focus();
    }, [session.isMobile, session.drawer, session.tab]);

    useEffect(() => {
        if (session.immersive) restoreRef.current?.focus();
        else if (wasImmersive.current) canvasButtonRef.current?.focus();
        wasImmersive.current = session.immersive;
    }, [session.immersive]);

    const mobileSession = { ...session, openDrawer };

    return (
        <main
            className="sc-workbench"
            data-mobile-tab={session.tab}
            data-mobile-drawer={session.drawer ?? "closed"}
            data-immersive={session.immersive}
        >
            <header className="sc-header">
                <div className="sc-brand">
                    <div className="sc-brand-mark">
                        <LayersIcon />
                    </div>
                    <div className="sc-brand-copy">
                        <div className="sc-product-title">
                            <h1 className="sc-product-name sc-desktop-only">{title}</h1>
                            <span
                                className="sc-app-version sc-desktop-only"
                                aria-label={versionLabel}
                                title={versionLabel}
                            >
                                v{PRODUCT_VERSION}
                            </span>
                        </div>
                        <p
                            className={`${documentLoaded ? "sc-document-status is-loaded" : "sc-document-status"} sc-desktop-only`}
                        >
                            {documentStatus}
                        </p>
                        {session.isMobile && (
                            <div className="sc-mobile-file-navigation">
                                <button
                                    type="button"
                                    className="sc-mobile-file-open"
                                    aria-label={t("mobile.openFile")}
                                    disabled={chromeHidden}
                                    onClick={(event) => {
                                        if (onBrowseFile) onBrowseFile();
                                        else openDrawer("source", event.currentTarget);
                                    }}
                                >
                                    <FolderOpen aria-hidden="true" />
                                </button>
                                <div className="sc-mobile-file-copy">
                                    <strong title={documentLoaded ? documentStatus : undefined}>
                                        {documentLoaded ? documentStatus : t("mobile.openFile")}
                                    </strong>
                                    <small>
                                        {documentLoaded
                                            ? (sourceSummary ?? t("mobile.fileDetails"))
                                            : t("mobile.chooseFile")}
                                    </small>
                                </div>
                                {documentLoaded && (
                                    <button
                                        type="button"
                                        className="sc-mobile-file-info"
                                        aria-label={t("mobile.fileDetails")}
                                        aria-expanded={sourceOpen}
                                        aria-controls="sc-source-panel"
                                        aria-haspopup="dialog"
                                        disabled={chromeHidden}
                                        onClick={(event) => openDrawer("source", event.currentTarget)}
                                    >
                                        <Info aria-hidden="true" />
                                    </button>
                                )}
                            </div>
                        )}
                    </div>
                </div>
                <div
                    className="sc-header-actions"
                    role={session.isMobile ? "tabpanel" : undefined}
                    aria-label={session.isMobile ? t("mobile.settings") : undefined}
                    aria-hidden={session.isMobile && session.tab !== "settings" ? true : undefined}
                    inert={session.isMobile && session.tab !== "settings"}
                    id="sc-settings-page"
                >
                    <SettingsPanel help={typeof help === "function" ? help(mobileSession) : help} />
                </div>
            </header>
            {session.isMobile && !previewHidden && (
                <>
                    <button
                        ref={canvasButtonRef}
                        className="sc-mobile-immersive"
                        aria-label={t("mobile.enterCanvas")}
                        onClick={() => session.setImmersive(true)}
                        hidden={chromeHidden}
                    >
                        <Maximize aria-hidden="true" />
                    </button>
                    <button
                        ref={restoreRef}
                        className="sc-mobile-restore"
                        aria-label={t("mobile.exitCanvas")}
                        hidden={!session.immersive}
                        onClick={() => {
                            session.setImmersive(false);
                            canvasButtonRef.current?.focus();
                        }}
                    >
                        <Minimize aria-hidden="true" />
                    </button>
                    <button
                        className="sc-mobile-export-fallback"
                        aria-label={t("mobile.openExport")}
                        onClick={() => openDrawer("export")}
                        hidden={chromeHidden || typeof viewer === "function" || !documentLoaded}
                    >
                        {t("export.title")}
                    </button>
                </>
            )}
            <div className="sc-body">
                <div
                    id="sc-preview-page"
                    role={session.isMobile ? "tabpanel" : undefined}
                    aria-label={session.isMobile ? t("mobile.preview") : undefined}
                    className="sc-viewer-column"
                    aria-hidden={previewHidden || (session.isMobile && session.drawer !== null) || undefined}
                    inert={previewHidden || (session.isMobile && session.drawer !== null)}
                >
                    <section className="sc-viewport" data-tour="preview">
                        {typeof viewer === "function" ? viewer(mobileSession) : viewer}
                    </section>
                </div>
                {session.isMobile && session.drawer !== null && !previewHidden && (
                    <button
                        className="sc-drawer-scrim"
                        aria-label={t("mobile.closePanel")}
                        onClick={closeDrawer}
                        tabIndex={-1}
                    />
                )}
                <aside
                    id="sc-source-panel"
                    ref={sourceOpen ? drawerRef : undefined}
                    className="sc-source"
                    role={sourceOpen ? "dialog" : undefined}
                    aria-label={sourceOpen ? t("mobile.fileDetails") : undefined}
                    aria-hidden={(session.isMobile && !sourceOpen) || undefined}
                    inert={session.isMobile && !sourceOpen}
                >
                    {sourceOpen && (
                        <button className="sc-drawer-close" aria-label={t("mobile.closePanel")} onClick={closeDrawer}>
                            <X aria-hidden="true" />
                        </button>
                    )}
                    {source}
                </aside>
                <aside
                    ref={exportOpen ? drawerRef : undefined}
                    className="sc-inspector"
                    data-tour="export"
                    role={exportOpen ? "dialog" : undefined}
                    aria-label={exportOpen ? t("export.title") : undefined}
                    aria-hidden={(session.isMobile && !exportOpen) || undefined}
                    inert={session.isMobile && !exportOpen}
                >
                    {exportOpen && (
                        <button className="sc-drawer-close" aria-label={t("mobile.closePanel")} onClick={closeDrawer}>
                            <X aria-hidden="true" />
                        </button>
                    )}
                    {typeof inspector === "function" ? inspector(mobileSession) : inspector}
                </aside>
            </div>
            <footer className="sc-status-bar" hidden={session.isMobile && session.immersive}>
                {typeof status === "function" ? status(mobileSession) : status}
            </footer>
            {session.isMobile && !session.immersive && (
                <nav className="sc-mobile-tabs" role="tablist" aria-label={t("mobile.navigation")}>
                    <button
                        role="tab"
                        aria-selected={session.tab === "preview"}
                        aria-controls="sc-preview-page"
                        onClick={() => session.selectTab("preview")}
                    >
                        <LayersIcon />
                        <span>{t("mobile.preview")}</span>
                        {converting && <span className="sc-converting-dot" aria-label={t("progress.converting")} />}
                    </button>
                    <button
                        role="tab"
                        aria-selected={session.tab === "settings"}
                        aria-controls="sc-settings-page"
                        onClick={() => session.selectTab("settings")}
                    >
                        <SlidersHorizontal aria-hidden="true" />
                        <span>{t("mobile.settings")}</span>
                    </button>
                </nav>
            )}
        </main>
    );
}
