import { initTheme } from "./theme";

initTheme();
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ScreenBoundary } from "./components/ScreenBoundary";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {/* The last resort: a screen's own boundary normally catches first. */}
    <ScreenBoundary screen="app" label="AI Juggler" root>
      <App />
    </ScreenBoundary>
  </React.StrictMode>,
);
