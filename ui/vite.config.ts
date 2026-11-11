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

// AJ-E2018: the lib/ipc modules the first window loads ship as one chunk
// (lib/bridge.ts was one module until it was split by feature): a chunk per
// module costs more in cross-chunk imports than it saves. The others stay
// with the screens that load them.
const STARTUP_IPC = [
  "account-import", "accountSettings", "accounts", "alerts", "app-lifecycle",
  "apps", "backup", "cleanup", "cloud-backup", "conversations", "costs",
  "crash", "desktop", "expert", "figurehead", "fleet", "fleet-launch",
  "fleet-remote", "fleet-switch", "instances", "kill", "launch", "link",
  "marks", "messages", "notices", "notifications", "park", "places", "pools",
  "processes", "project-folders", "providers", "quota", "redaction",
  "relations", "runtime", "services", "session-history", "sessions",
  "settings", "subagents", "subscriptions", "switch-preflight",
  "switch-targets", "switchSetup", "tasks", "tasks-across", "terminal-io",
  "tools", "trust", "updates", "usage-stats", "usageHistory", "validation",
  "vault",
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
