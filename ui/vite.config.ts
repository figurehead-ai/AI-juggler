import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri dev-server contract (tauri.conf.json build.devUrl).
export default defineConfig({
  plugins: [react(), tailwindcss()],
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
