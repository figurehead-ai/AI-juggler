import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The dev server only: the page may reach the read-only test bridge of a
// running AI Juggler Dev build on 127.0.0.1 (docs/qa/DEV-BRIDGE.md).
// Builds keep the strict CSP; tests/bundle checks it.
const BRIDGE_CSP = "connect-src 'self' http://127.0.0.1:*;";
const readOnlyBridgeCsp: Plugin = {
  name: "aij-read-only-bridge-csp",
  apply: "serve",
  transformIndexHtml(html) {
    return html.replace("default-src 'self';", `default-src 'self'; ${BRIDGE_CSP}`);
  },
};

// The backend calls the first window makes (lib/bridge.ts until AJ-E2018
// split it into lib/ipc/<feature>.ts) ship as one chunk, as they did: one
// chunk per module cost ~17 KB of cross-chunk imports. Other lib/ipc
// modules stay with the screens that load them.
const STARTUP_IPC = [
  "account-import", "accounts", "alerts", "app-lifecycle", "backup", "cleanup",
  "cloud-backup", "crash", "desktop", "expert", "figurehead", "fleet",
  "fleet-remote", "fleet-switch", "instances", "kill", "launch", "link",
  "park", "preview-support", "project-folders", "providers", "redaction",
  "session-history", "settings", "switch-preflight", "switch-targets",
  "terminal-io", "updates", "usage-stats", "validation", "vault",
];
const STARTUP_CALLS = new RegExp(`/src/lib/(bridge|ipc/(${STARTUP_IPC.join("|")}))\\.ts$`);

// Tauri dev-server contract (tauri.conf.json build.devUrl).
export default defineConfig({
  plugins: [react(), tailwindcss(), readOnlyBridgeCsp],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "es2022",
    outDir: "dist",
    sourcemap: false,
    rollupOptions: {
      output: {
        // xterm is heavy and terminal-first paint doesn't need it. (Vite 8
        // bundles with Rolldown, which takes only the function form.)
        manualChunks(id: string) {
          if (id.includes("/node_modules/@xterm/")) return "xterm";
          return STARTUP_CALLS.test(id) ? "bridge" : undefined;
        },
      },
    },
  },
});
