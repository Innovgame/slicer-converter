import type { StatusState } from "../model/use-converter-controller";

interface StatusBannerProps {
    id: string;
    status: StatusState | null;
}

/**
 * @zh 底栏状态行。空状态仍保留节点，供页面测试和 e2e 定位。
 * @en Status-bar line. The node stays mounted when empty so page tests and e2e can find it.
 */
export function StatusBanner({ id, status }: StatusBannerProps) {
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
            {status?.message ?? ""}
        </div>
    );
}
