import { driver, type Config, type DriveStep, type Driver } from "driver.js";
import { useEffect, useRef, useState } from "react";
import type { TFunction } from "i18next";
import { flushSync } from "react-dom";

import { useTranslation } from "../../../shared/i18n/i18n";

const STORAGE_KEY = "slicer-onboarding-v1";

function hasSeenInvitation(): boolean {
    try {
        return window.localStorage.getItem(STORAGE_KEY) !== null;
    } catch {
        return false;
    }
}

function rememberTour(outcome: "completed" | "dismissed"): void {
    try {
        window.localStorage.setItem(STORAGE_KEY, outcome);
    } catch {
        // The tour remains usable when preference storage is unavailable.
    }
}

function buildSteps(t: TFunction, documentLoaded: boolean, navigate: (id: string) => void): DriveStep[] {
    const step = (id: string, element: string, description = id): DriveStep => ({
        element,
        data: { id },
        onHighlightStarted: () => navigate(id),
        popover: {
            title: t(`onboarding.steps.${id}.title`),
            description: t(`onboarding.steps.${description}.description`),
            side: "left",
            align: "center"
        }
    });
    return [
        step("source", "#dropZone"),
        step("preview", '[data-tour="preview"]', documentLoaded ? "preview" : "previewEmpty"),
        ...(documentLoaded ? [step("layer", ".sc-rail-copy, .sc-mobile-layer-readout")] : []),
        step(
            "export",
            documentLoaded ? '[data-tour="export-target"]' : '[data-tour="export"]',
            documentLoaded ? "export" : "exportEmpty"
        ),
        ...(documentLoaded ? [step("convert", "#convertBtn")] : []),
        step("feedback", '[data-tour="feedback"]'),
        step("preferences", '[data-tour="preferences"]')
    ];
}

/**
 * @zh Driver.js 引导的展示态与首次邀请；不加载文件、不改图层、不触发转换。
 * @en Driver.js tour and first-visit invitation state; never loads files, changes layers or starts conversion.
 */
export function useOnboardingTour({
    documentLoaded,
    converting,
    onNavigate
}: {
    documentLoaded: boolean;
    converting: boolean;
    onNavigate?(step: string): void;
}) {
    const { t } = useTranslation();
    const [invitationVisible, setInvitationVisible] = useState(() => !hasSeenInvitation());
    const tourRef = useRef<Driver | null>(null);
    const triggerRef = useRef<HTMLButtonElement | null>(null);
    const navigateRef = useRef(onNavigate);
    navigateRef.current = onNavigate;

    const createConfig = (): Config => ({
        steps: buildSteps(t, documentLoaded, (id) => {
            if (navigateRef.current) flushSync(() => navigateRef.current?.(id));
        }),
        animate: false,
        smoothScroll: false,
        allowClose: true,
        overlayClickBehavior: "none",
        overlayOpacity: 0.5,
        disableActiveInteraction: true,
        showProgress: true,
        popoverClass: "sc-tour-popover",
        stagePadding: 6,
        stageRadius: 8,
        nextBtnText: t("onboarding.next"),
        prevBtnText: t("onboarding.back"),
        doneBtnText: t("onboarding.finish"),
        closeBtnLabel: t("onboarding.skip"),
        // Driver.js replaces these tokens after it determines the current step.
        progressText: t("onboarding.progress", { current: "{{current}}", total: "{{total}}" }),
        onPopoverRender(popover) {
            popover.wrapper.setAttribute("aria-modal", "true");
            popover.closeButton.textContent = t("onboarding.skip");
        },
        onNextClick(_element, _step, { driver: tour }) {
            if (tour.isLastStep()) {
                rememberTour("completed");
                tour.destroy();
            } else {
                tour.moveNext();
            }
        },
        onDestroyStarted(_element, _step, { driver: tour }) {
            rememberTour("dismissed");
            tour.destroy();
        },
        onDestroyed(_element, _step, { driver: tour }) {
            if (tourRef.current !== tour) return;
            tourRef.current = null;
            queueMicrotask(() => {
                if (navigateRef.current) flushSync(() => navigateRef.current?.("preferences"));
                triggerRef.current?.focus();
            });
        }
    });

    useEffect(() => {
        const tour = tourRef.current;
        if (!tour?.isActive()) return;
        if (converting) {
            tour.destroy();
            return;
        }
        const id = tour.getActiveStep()?.data?.id;
        const config = createConfig();
        const nextIndex = config.steps?.findIndex((step) => step.data?.id === id) ?? 0;
        tour.setConfig(config);
        tour.drive(Math.max(0, nextIndex));
    }, [t, documentLoaded, converting]);

    useEffect(
        () => () => {
            const tour = tourRef.current;
            tourRef.current = null;
            tour?.destroy();
        },
        []
    );

    return {
        triggerRef,
        invitationVisible: invitationVisible && !documentLoaded && !converting,
        dismissInvitation(): void {
            setInvitationVisible(false);
            rememberTour("dismissed");
        },
        startTour(): void {
            if (converting || tourRef.current?.isActive()) return;
            setInvitationVisible(false);
            const tour = driver(createConfig());
            tourRef.current = tour;
            tour.drive();
        }
    };
}
