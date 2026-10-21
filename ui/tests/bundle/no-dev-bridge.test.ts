// @vitest-environment node
/**
 * The read-only test bridge never ships: a production build of the UI
 * carries no trace of it, while a development build does (so the check
 * can see it when it is there).
 */
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { env } from "node:process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import { describe, expect, it } from "vitest";

const ui = fileURLToPath(new URL("../..", import.meta.url));
const MARKERS = ["devBridge", "Read-only test bridge", "/v1/invoke/", "127.0.0.1:*"];

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name: string) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

async function bundle(mode: "production" | "development"): Promise<string> {
  const outDir = mkdtempSync(join(tmpdir(), `aij-ui-${mode}-`));
  // Vite takes DEV/PROD from NODE_ENV when it is set, and vitest sets it
  // to "test": build as `pnpm build` and `vite build --mode development` do.
  const nodeEnv = env.NODE_ENV;
  env.NODE_ENV = mode;
  try {
    await build({
      root: ui,
      mode,
      logLevel: "silent",
      build: { outDir, emptyOutDir: true, minify: mode === "production" },
    });
    return files(outDir)
      .map((path) => readFileSync(path, "utf8"))
      .join("\n");
  } finally {
    env.NODE_ENV = nodeEnv;
    rmSync(outDir, { recursive: true, force: true });
  }
}

describe("read-only test bridge — never in a production bundle", () => {
  it("the production bundle contains none of it", async () => {
    const code = await bundle("production");
    expect(code.length).toBeGreaterThan(10_000);
    for (const marker of MARKERS) expect(code).not.toContain(marker);
    // The shipped page's CSP lets it connect to itself only.
    expect(code).toContain(`content="default-src 'self'; img-src`);
  }, 120_000);

  it("a development build does (the check can see it)", async () => {
    const code = await bundle("development");
    expect(code).toContain("devBridge");
  }, 120_000);
});
