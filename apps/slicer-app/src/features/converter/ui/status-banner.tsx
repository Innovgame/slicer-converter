import { useEffect, useRef, useState } from "react";
import { Check, CircleAlert, Info, LoaderCircle, X } from "lucide-react";
import type { TFunction } from "i18next";

import { translateRuntimeMessage } from "../../../shared/i18n/runtime-messages";

import { useTranslation } from "../../../shared/i18n/i18n";

import type { ProgressState, StatusState } from "../model/converter-view-state";

interface StatusBannerProps {
    id: string;
    status: StatusState | null;
    mobile?: boolean;
    converting?: boolean;
    progress?: ProgressState | null;
}

/**
 * @zh 桌面完整状态行与移动端行内状态；保留完整消息和空状态节点供辅助技术与测试定位。
 * @en Full desktop status and inline mobile status; retains complete messages and empty nodes for accessibility and tests.
 */
export function StatusBanner({ id, status, mobile = false, converting = false, progress }: StatusBannerProps) {
    const { t } = useTranslation();
    if (mobile) {
        return (
            <MobileStatus
                key={`${status?.type}:${status?.message}`}
                id={id}
                status={status}
                converting={converting}
                progress={progress}
            />
        );
    }
    const tone =
        status?.type === "error"
            ? "status-error"
            : status?.type === "success"
              ? "status-success"
              : status?.type === "warning"
                ? "status-warning"
                : "status-info";

    return (
        <div id={id} className={`sc-status-message ${tone}${status ? "" : " hidden"}`}>
            {translateRuntimeMessage(t, status?.message ?? "")}
        </div>
    );
}
function MobileStatus({ id, status, converting = false, progress }: StatusBannerProps) {
    const { t } = useTranslation();
    const [expanded, setExpanded] = useState(false);
    const rootRef = useRef<HTMLDivElement>(null);
    const triggerRef = useRef<HTMLButtonElement>(null);

    useEffect(() => {
        if (!expanded) return;
        const outside = (event: PointerEvent) => {
            if (event.target instanceof Node && !rootRef.current?.contains(event.target)) setExpanded(false);
        };
        const escape = (event: KeyboardEvent) => {
            if (event.key !== "Escape") return;
            setExpanded(false);
            triggerRef.current?.focus();
            event.stopPropagation();
        };
        document.addEventListener("pointerdown", outside);
        document.addEventListener("keydown", escape);
        return () => {
            document.removeEventListener("pointerdown", outside);
            document.removeEventListener("keydown", escape);
        };
    }, [expanded]);

    const summary = mobileSummary(t, status, converting, progress);
    const detail = converting
        ? `${translateRuntimeMessage(t, progress?.label ?? "Converting...")}${progress ? ` · ${progress.percent}%` : ""}`
        : translateRuntimeMessage(t, status?.message ?? "");
    const tone = converting ? "progress" : (status?.type ?? "info");
    const Icon = converting
        ? LoaderCircle
        : tone === "success"
          ? Check
          : tone === "error" || tone === "warning"
            ? CircleAlert
            : Info;

    return (
        <div
            ref={rootRef}
            id={id}
            className={`sc-status-message sc-status-inline${status || converting ? "" : " hidden"}`}
            data-tone={tone}
        >
            <span className="sc-sr" role={tone === "error" ? "alert" : "status"} aria-atomic="true">
                {detail}
            </span>
            <button
                ref={triggerRef}
                type="button"
                className="sc-status-trigger"
                aria-label={t("mobile.status.showDetails", { status: summary })}
                aria-expanded={expanded}
                aria-controls={expanded ? `${id}-details` : undefined}
                onClick={() => setExpanded((value) => !value)}
            >
                <Icon className="sc-status-icon" aria-hidden="true" />
                <span>{summary}</span>
            </button>
            {expanded && (
                <section
                    id={`${id}-details`}
                    className="sc-status-details"
                    role="region"
                    aria-label={t("mobile.status.details")}
                >
                    <header>
                        <strong>{t("mobile.status.details")}</strong>
                        <button
                            type="button"
                            aria-label={t("mobile.status.closeDetails")}
                            onClick={() => {
                                setExpanded(false);
                                triggerRef.current?.focus();
                            }}
                        >
                            <X aria-hidden="true" />
                        </button>
                    </header>
                    <p>{detail}</p>
                </section>
            )}
        </div>
    );
}

function mobileSummary(t: TFunction, status: StatusState | null, converting: boolean, progress?: ProgressState | null) {
    if (converting)
        return progress
            ? t("mobile.status.convertingProgress", { percent: progress.percent })
            : t("mobile.status.converting");
    const message = status?.message ?? "";
    if (status?.type === "error") {
        if (message.startsWith("Parse error:")) return t("mobile.status.openError");
        if (message.startsWith("Convert error:")) return t("mobile.status.convertError");
        return t("mobile.status.error");
    }
    if (status?.type === "warning") return t("mobile.status.warning");
    if (message.startsWith("Ready -")) return t("mobile.status.ready");
    if (message.startsWith("Parsed ")) return t("mobile.status.opened");
    if (message.startsWith("Converted ")) return t("mobile.status.converted");
    if (message.includes("WASM")) return t("mobile.status.starting");
    if (message.startsWith("Using manual ")) return t("mobile.status.targetUpdated");
    return translateRuntimeMessage(t, message);
}
