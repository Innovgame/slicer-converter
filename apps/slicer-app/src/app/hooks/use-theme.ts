import { useContext } from "react";

import { ThemeContext, type Theme } from "../providers/theme-provider";

/**
 * @zh `useTheme` 导出函数。
 * @en Exported function `useTheme`.
 */
export function useTheme() {
    const context = useContext(ThemeContext);
    if (!context) {
        throw new Error("useTheme must be used within ThemeProvider");
    }
    return context;
}

export type { Theme };
