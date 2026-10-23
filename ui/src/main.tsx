import { initTheme } from "./theme";

initTheme();
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ScreenBoundary } from "./components/ScreenBoundary";
import "./styles.css";

import { loadTools } from "./lib/tools";
import { ownTerminalWindow } from "./lib/ipc/terminal";
import { TerminalWindow } from "./screens/TerminalWindow";

// AJ-E1525: the tools catalog first, so no screen shows a tool's id for
// a moment (it answers from the registry, without probing anything).
const ownTerminal = ownTerminalWindow();
void loadTools().finally(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      {/* The last resort: a screen's own boundary normally catches first. */}
      <ScreenBoundary screen="app" label="AI Juggler" root>
        {/* AJ-E1506: a terminal in its own window shows only that terminal. */}
        {ownTerminal ? <TerminalWindow id={ownTerminal} /> : <App />}
      </ScreenBoundary>
    </React.StrictMode>,
  );
});
