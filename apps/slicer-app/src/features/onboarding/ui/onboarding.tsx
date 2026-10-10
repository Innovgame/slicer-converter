import { CircleHelp } from "lucide-react";
import { createPortal } from "react-dom";

import { useTranslation } from "../../../shared/i18n/i18n";
import { useOnboardingTour } from "../model/use-onboarding-tour";

/**
 * @zh 顶栏可重开的新手引导入口和首次访问邀请；邀请不阻挡正常操作。
 * @en Replayable header tour entry and a first-visit invitation that leaves normal interaction available.
 */
export function Onboarding({ documentLoaded, converting }: { documentLoaded: boolean; converting: boolean }) {
    const { t } = useTranslation();
    const tour = useOnboardingTour({ documentLoaded, converting });
    return (
        <>
            <button
                ref={tour.triggerRef}
                type="button"
                className="sc-icon-btn"
                aria-label={t("onboarding.entry")}
                title={t("onboarding.entry")}
                disabled={converting}
                onClick={tour.startTour}
            >
                <CircleHelp aria-hidden="true" />
            </button>
            {tour.invitationVisible &&
                createPortal(
                    <section className="sc-tour-invitation" aria-labelledby="sc-tour-welcome-title">
                        <h2 id="sc-tour-welcome-title">{t("onboarding.welcome")}</h2>
                        <p>{t("onboarding.invitation")}</p>
                        <div className="sc-tour-invitation-actions">
                            <button type="button" className="sc-tour-later" onClick={tour.dismissInvitation}>
                                {t("onboarding.later")}
                            </button>
                            <button type="button" className="sc-tour-start" onClick={tour.startTour}>
                                {t("onboarding.start")}
                            </button>
                        </div>
                    </section>,
                    document.body
                )}
        </>
    );
}
