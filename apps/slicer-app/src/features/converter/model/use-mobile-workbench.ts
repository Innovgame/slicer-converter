import { useCallback, useEffect, useState, useSyncExternalStore } from "react";

const MOBILE_QUERY = "(max-width: 767px)";

function subscribeViewport(callback: () => void) {
    const query = window.matchMedia?.(MOBILE_QUERY);
    query?.addEventListener("change", callback);
    return () => query?.removeEventListener("change", callback);
}

function mobileSnapshot() {
    return window.matchMedia?.(MOBILE_QUERY).matches ?? false;
}

/**
 * @internal
 * @zh 移动工作台的页面、抽屉与纯画布展示态；不持有文档或转换任务。
 * @en Mobile page, drawer and immersive presentation state; owns no document or conversion task.
 */
export interface MobileWorkbenchSession {
    isMobile: boolean;
    tab: "preview" | "settings";
    drawer: "source" | "export" | null;
    immersive: boolean;
    selectTab(tab: "preview" | "settings"): void;
    openDrawer(drawer: "source" | "export"): void;
    closeDrawer(): void;
    setImmersive(value: boolean): void;
}

/**
 * @internal
 * @zh 用媒体查询同步移动展示态；更换文档退出纯画布，切换页面不重建文档。
 * @en Synchronizes mobile presentation with a media query; a new document exits immersive mode without recreating documents on tab changes.
 */
export function useMobileWorkbench(documentIdentity: unknown): MobileWorkbenchSession {
    const isMobile = useSyncExternalStore(subscribeViewport, mobileSnapshot, () => false);
    const [tab, selectTab] = useState<"preview" | "settings">("preview");
    const [drawer, setDrawer] = useState<"source" | "export" | null>(null);
    const [immersive, setImmersive] = useState(false);
    const closeDrawer = useCallback(() => setDrawer(null), []);

    useEffect(() => {
        setImmersive(false);
        setDrawer(null);
        selectTab("preview");
    }, [documentIdentity]);

    useEffect(() => {
        if (!isMobile) setImmersive(false);
    }, [isMobile]);

    useEffect(() => {
        if (!isMobile) return;
        const escape = (event: KeyboardEvent) => {
            if (event.key !== "Escape") return;
            setImmersive(false);
            setDrawer(null);
        };
        document.addEventListener("keydown", escape, true);
        return () => document.removeEventListener("keydown", escape, true);
    }, [isMobile]);

    return {
        isMobile,
        tab,
        drawer,
        immersive,
        selectTab,
        openDrawer: setDrawer,
        closeDrawer,
        setImmersive
    };
}
