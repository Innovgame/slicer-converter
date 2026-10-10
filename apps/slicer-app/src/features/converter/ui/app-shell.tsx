import type { ReactNode } from "react";

import { LayersIcon } from "./icons";
import { ProjectLinks } from "./project-links";
import { ThemeToggle } from "./theme-toggle";

interface AppShellProps {
    title: string;
    documentStatus: string;
    source: ReactNode;
    viewer: ReactNode;
    inspector: ReactNode;
    status: ReactNode;
}

/**
 * @zh 桌面保持三栏。平板是上预览、下并排。手机按预览、源文件、导出向下滚动。
 * @en Desktop keeps three columns. Tablets put the preview above two columns. Phones scroll preview, source, then export.
 */
export function AppShell({ title, documentStatus, source, viewer, inspector, status }: AppShellProps) {
    return (
        <main className="sc-workbench">
            <header className="sc-header">
                <div className="sc-brand">
                    <div className="sc-brand-mark">
                        <LayersIcon />
                    </div>
                    <div className="sc-brand-copy">
                        <h1 className="sc-product-name">{title}</h1>
                        <p className="sc-document-status">{documentStatus}</p>
                    </div>
                </div>
                <div className="sc-header-actions">
                    <ProjectLinks />
                    <ThemeToggle />
                </div>
            </header>
            <div className="sc-body">
                <div className="sc-viewer-column">
                    <section className="sc-viewport">{viewer}</section>
                </div>
                <aside className="sc-source">{source}</aside>
                <aside className="sc-inspector">{inspector}</aside>
            </div>
            <footer className="sc-status-bar">{status}</footer>
        </main>
    );
}
