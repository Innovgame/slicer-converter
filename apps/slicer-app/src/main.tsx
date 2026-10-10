import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { ThemeProvider } from "./app/providers";
import { LanguageProvider } from "./shared/i18n/language-provider";
import { ConverterPage } from "./app/routes/converter-page";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
    throw new Error("Missing #root");
}

createRoot(root).render(
    <StrictMode>
        <ThemeProvider>
            <LanguageProvider>
                <ConverterPage />
            </LanguageProvider>
        </ThemeProvider>
    </StrictMode>
);
