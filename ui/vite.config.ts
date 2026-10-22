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
          return id.includes("/node_modules/@xterm/") ? "xterm" : undefined;
        },
      },
    },
  },
});
