import { useTheme } from "../../../app/providers";
import { MoonIcon, SunIcon } from "./icons";

/**
 * @zh 顶栏主题切换。只改展示偏好，不进入文档状态。
 * @en Header theme toggle. It changes presentation preference only and does not enter document state.
 */
export function ThemeToggle() {
    const { theme, toggleTheme } = useTheme();
    const nextTheme = theme === "light" ? "dark" : "light";

    return (
        <button
            onClick={toggleTheme}
            className="sc-icon-btn"
            aria-label={`Switch to ${nextTheme} theme`}
            title={`Current theme: ${theme}`}
            type="button"
        >
            {theme === "light" ? <MoonIcon /> : <SunIcon />}
        </button>
    );
}
