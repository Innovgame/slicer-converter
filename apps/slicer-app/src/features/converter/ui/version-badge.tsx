import { useState } from "react";

import { PRODUCT_BUILD_COMMIT, PRODUCT_VERSION } from "../../../product-version";
import { useTranslation } from "../../../shared/i18n/i18n";

/**
 * @zh 默认展示版本号，点击切换构建提交的显示状态。
 * @en Shows the version by default; clicking toggles the build commit.
 */
export function VersionBadge({ className = "" }: { className?: string }) {
    const { t } = useTranslation();
    const [showCommit, setShowCommit] = useState(false);
    const version = `${PRODUCT_VERSION}${showCommit && PRODUCT_BUILD_COMMIT ? ` · ${PRODUCT_BUILD_COMMIT}` : ""}`;
    const label = t("app.version", { version });

    return (
        <button
            type="button"
            className={`sc-version-toggle ${className}`}
            aria-label={label}
            aria-pressed={showCommit}
            title={label}
            disabled={!PRODUCT_BUILD_COMMIT}
            onClick={() => setShowCommit((value) => !value)}
        >
            v{version}
        </button>
    );
}
