import { initTheme } from "./theme";

initTheme();
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ScreenBoundary } from "./components/ScreenBoundary";
import "./styles.css";

import { loadTools } from "./lib/tools";
import { ownTerminalWindow } from "./lib/ipc/terminal";
import { lazyNamed } from "./lib/lazy";
import { guardContextMenu } from "./lib/context-menu";
import { guardCaretMark } from "./lib/caret-leaves";
import { fetchAppInfo } from "./lib/ipc/app-lifecycle";
import { loadPreview } from "./lib/preview-hooks";

// A terminal window loads its own code; the main window never does.
const TerminalWindow = lazyNamed(() => import("./screens/TerminalWindow"), "TerminalWindow");

// 0.3.2: no web-view menu (Reload…) where AI Juggler has none; in a Dev
// build ⌥-right-click keeps it, for inspecting the page.
let devBuild = import.meta.env.DEV;
guardContextMenu(() => devBuild);
// rc25 K1: a text field left for nothing editable takes macOS's ⊖ with it.
guardCaretMark();

// QA: a Dev window opened at ?open=<conversation id> opens that session.
if (import.meta.env.DEV) {
  const open = new URL(window.location.href).searchParams.get("open");
  if (open) void import("./lib/dev-open").then((dev) => dev.openWhenListed(open));
}

// AJ-E3301: a walk's steps timed, only when asked for.
void import("./lib/walk-timing").then((walk) => walk.walkTimingWanted() && walk.startWalkTiming());

// AJ-E2018: in a plain browser the design preview's data loads first (the
// app with a live backend never fetches it). AJ-E1525: then the tools
// catalog, so no screen shows a tool's id for a moment (it answers from
// the registry, without probing anything).
const ownTerminal = ownTerminalWindow();
void loadPreview().then(() => {
  void fetchAppInfo().then((info) => (devBuild = devBuild || info?.edition === "dev"), () => null);
  return loadTools();
}).finally(() => {
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
