import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Component-test harness (AJ-E402): jsdom environment, same plugins as
// the app build so token-driven classes resolve. Run via `pnpm test`.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.tsx", "src/**/*.test.ts"],
  },
});
