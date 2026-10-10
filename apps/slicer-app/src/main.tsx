import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { ThemeProvider } from "./app/providers";
import { ConverterPage } from "./app/routes/converter-page";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
    throw new Error("Missing #root");
}

createRoot(root).render(
    <StrictMode>
        <ThemeProvider>
            <ConverterPage />
        </ThemeProvider>
    </StrictMode>
);
