import { initTheme } from "./theme";

initTheme();
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

import { loadTools } from "./lib/tools";

// AJ-E1525: the tools catalog first, so no screen shows a tool's id for
// a moment (it answers from the registry, without probing anything).
void loadTools().finally(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
