/**
 * AJ-E1515 gallery: New session on a paired Mac — the "Runs on" picker,
 * that Mac's projects and launch settings, a Mac that can't start one
 * saying why, Start opening the session as seen from this Mac — and the
 * last day's usage by machine on Usage and Fleet. Both themes; axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/88-fleet-launch");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    const now = Date.now();
    const w = window as unknown as Record<string, unknown>;
    localStorage.setItem("aij.theme", theme);
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
    w.__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
    w.__AIJ_FLEET_PEERS__ = [
      { machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA", permission: "control",
        permission_label: "View + control", ask_each_time: false, paired_via: "code", paired_at_ms: now, last_seen_ms: now,
        connection: "connected", protocol: "v1.0", connection_detail: null },
    ];
    w.__AIJ_FLEET_REMOTE__ = [
      { machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live", last_heard_ms: now, permission: "control",
        resources: { cpu_percent: 12, memory_used_bytes: 1e9, memory_total_bytes: 8e9, measured_at_ms: now },
        instances: [{ id: "remote-1", provider: "claude-code", account_label: "Mini Max", status: "running", model: "claude-opus-4",
          model_source: "launch", project_name: "storefront", started_at_ms: now - 5_000, last_activity_ms: now, cpu_percent: 3,
          rss_bytes: 3e8, process_count: 2, measured_at_ms: now }] },
    ];
    w.__AIJ_LAUNCH_MACHINES__ = [
      { machine_id: "machine-mini", label: "Mac mini", why_not: null,
        recipes: [{ id: "r-1", name: "Main", tool: "Claude Code", account: "Mini Max" }, { id: "r-2", name: "Deep work", tool: "Codex", account: "Codex Pro" }],
        projects: [{ id: "p-1", name: "storefront" }, { id: "p-2", name: "payments-api" }] },
      { machine_id: "machine-air", label: "MacBook Air", why_not: "MacBook Air isn't connected right now", recipes: [], projects: [] },
    ];
    w.__AIJ_MACHINE_USAGE__ = [
      { machine_id: null, label: "This Mac", used_units: 1_840_000, usd_est: 6.42, sessions: 4, as_of_ms: null, reported: true },
      { machine_id: "machine-mini", label: "Mac mini", used_units: 920_000, usd_est: 3.1, sessions: 2, as_of_ms: now - 60_000, reported: true },
      { machine_id: "machine-air", label: "MacBook Air", used_units: 0, usd_est: null, sessions: 0, as_of_ms: null, reported: false },
    ];
  }, theme);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`a session starts on a paired Mac from New session — ${theme}`, gate("88-fleet-launch"), async ({ page }) => {
    await open(page, theme);
    await page.getByRole("button", { name: "New session", exact: true }).first().click();
    await page.getByTestId("machine-picker").getByRole("radio", { name: "Mac mini" }).click();
    const sheet = page.getByTestId("remote-session-sheet");
    await expect(sheet.getByRole("heading")).toHaveText("New session on Mac mini");
    await expect(sheet.getByTestId("remote-recipes")).toContainText("Mini Max");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `remote-sheet-1280x800-${theme}.png`) });
    await axe(page, `remote sheet ${theme}`);

    await sheet.getByRole("radio", { name: "MacBook Air" }).click();
    await expect(sheet.getByTestId("remote-why-not")).toHaveText("MacBook Air isn't connected right now.");
    await expect(sheet.getByTestId("remote-start")).toBeDisabled();
    await sheet.getByRole("radio", { name: "Mac mini" }).click();
    await sheet.getByTestId("remote-start").click();
    await expect(sheet).toHaveCount(0);
    expect(await page.evaluate(() => (window as unknown as { __AIJ_REMOTE_STARTED__?: unknown }).__AIJ_REMOTE_STARTED__)).toEqual({
      machineId: "machine-mini", recipeId: "r-1", projectId: "p-1",
    });
    await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toContainText("Mac mini");
  });

  test(`usage by machine on Usage and Fleet — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Usage");
    const card = page.getByTestId("machine-usage");
    await expect(card.getByTestId("machine-usage-row")).toHaveCount(3);
    await expect(card).toContainText("1.8M tokens · ≈ $6.42 · 4 sessions");
    await expect(card).toContainText("Hasn't reported yet");
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `usage-by-machine-1280x800-${theme}.png`) });
    await axe(page, `usage by machine ${theme}`);
    await goTo(page, "Fleet");
    await expect(page.getByTestId("machine-usage")).toContainText("Mac mini");
  });
}
