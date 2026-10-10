import { type ReactNode, createContext, useCallback, useLayoutEffect, useState } from "react";

/**
 * @zh `Theme` 类型契约。
 * @en Type contract for `Theme`.
 */
export type Theme = "light" | "dark";

interface ThemeContextValue {
    theme: Theme;
    toggleTheme: () => void;
    setTheme: (theme: Theme) => void;
}

/**
 * @zh `ThemeContext` 导出值。
 * @en Exported value `ThemeContext`.
 */
export const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

interface ThemeProviderProps {
    children: ReactNode;
}

/**
 * @zh `ThemeProvider` 导出函数。
 * @en Exported function `ThemeProvider`.
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
    const [theme, setThemeState] = useState<Theme>(() => {
        const stored = readStoredTheme();
        if (stored) {
            return stored;
        }
        if (window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches) {
            return "light";
        }
        return "dark";
    });

    useLayoutEffect(() => {
        writeStoredTheme(theme);
        document.documentElement.setAttribute("data-theme", theme);
    }, [theme]);

    const toggleTheme = useCallback(() => {
        setThemeState((prev) => (prev === "light" ? "dark" : "light"));
    }, []);

    const setTheme = useCallback((newTheme: Theme) => {
        setThemeState(newTheme);
    }, []);

    return <ThemeContext.Provider value={{ theme, toggleTheme, setTheme }}>{children}</ThemeContext.Provider>;
}

function themeStorage(): Storage | null {
    try {
        const storage = window.localStorage;
        if (!storage || typeof storage.getItem !== "function" || typeof storage.setItem !== "function") {
            return null;
        }
        return storage;
    } catch {
        return null;
    }
}

function readStoredTheme(): Theme | null {
    try {
        const stored = themeStorage()?.getItem("slicer-theme");
        return stored === "light" || stored === "dark" ? stored : null;
    } catch {
        return null;
    }
}

function writeStoredTheme(theme: Theme): void {
    try {
        themeStorage()?.setItem("slicer-theme", theme);
    } catch {
        // Preference storage can be unavailable in tests or locked browser profiles.
    }
}
