import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { I18nProvider } from "./i18n";
import { ThemeProvider, applyTheme, readStoredTheme } from "./theme";
import "@fontsource-variable/noto-serif-sc";
import "@fontsource/dm-serif-display";
import "./index.css";

applyTheme(readStoredTheme());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <ThemeProvider>
        <App />
      </ThemeProvider>
    </I18nProvider>
  </React.StrictMode>,
);