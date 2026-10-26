/**
 * E24 gallery (AJ-E2404..E2407): the Processes page on the preview's
 * made-up Mac (AI Juggler's sessions, the ChatGPT app's and a terminal's,
 * every state in the one vocabulary) — each view at 1280, 1680 and 2000 px
 * in both themes; the filters, a row's menu, the verified stop sheet (ours
 * and another app's), "Clean up what this session left", the rules' dry
 * run, a session's Processes tab, "Include other apps" and the large Mac
 * (50 sessions, 2,000 processes). Each capture is scanned with axe (0
 * critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/90-processes");
const T0 = 1_728_800_000_000;
const THEMES = ["dark", "light"] as const;
const WIDTHS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1680x1000", width: 1680, height: 1000 },
  { name: "2000x1100", width: 2000, height: 1100 },
] as const;
const MID = WIDTHS[1];

/** The rail's sessions, two of them the made-up Mac's (a session's
 * Processes tab is that session's part of the one graph). */
const fixture: FixtureSet = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session, at) => (at === 1 ? { ...session, id: "s-web" } : session)),
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", vp: { width: number; height: number }, query = "") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      // The verify step reads each of the made-up Mac's leftovers as it
      // is now: the program the ledger recorded, still holding its port.
      const seen: Record<number, [string, string, number[]]> = {
        8123: ["/usr/local/bin/node", "node node_modules/.bin/vite dev --port 5173", [5173]],
        8131: ["/usr/local/bin/esbuild", "esbuild --service=0.21.5 --ping", []],
        8140: ["/usr/local/bin/node", "node node_modules/.bin/nodemon server.js", []],
        8155: ["/usr/bin/python3", "python3 -m http.server 8000", [8000]],
      };
      w.__AIJ_KILL_VERIFY__ = (session: string, pid: number) => {
        const [exe, cmdline, ports] = seen[pid] ?? ["/usr/local/bin/node", "node", []];
        return {
          pid, session, verified: true, refusal: null, exe_path: exe, cmdline, cwd: "/Users/you/Dev/payments", ports,
          pgid: null, escaped_group: false, tree_pids: [pid], platform: "TERM → grace → KILL", grace_secs: 5,
          members: [{ pid, pgid: null, exe: exe.split("/").pop(), cmdline, role_guess: "DevServer", unsaved_state_warning: null, from_ledger: true }],
          provider: "claude-code", account_id: "Work Max", project_id: "payments",
        };
      };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.removeItem("aij.processes.view");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto(`/${query}`);
}

async function openPage(page: Page) {
  await goTo(page, "Processes");
  await expect(page.getByTestId("processes-page")).toBeVisible();
  await expect(page.getByTestId("process-stop-8123")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function shot(page: Page, name: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}.png`) });
  await axeClean(page, name);
}

const VIEWS: { id: string; go: (page: Page) => Promise<void> }[] = [
  {
    id: "attention",
    go: async (page) => {
      await expect(page.getByTestId("tally-clean")).toContainText("7");
      await expect(page.getByTestId("process-row-s:ours:s-pay")).toContainText("Payments API");
    },
  },
  {
    id: "running",
    go: async (page) => {
      await page.getByTestId("process-view-running").click();
      await expect(page.getByTestId("process-row-s:ours:s-web")).toHaveAttribute("aria-expanded", "false");
    },
  },
  {
    id: "ended",
    go: async (page) => {
      await page.getByTestId("process-view-ended").click();
      await expect(page.getByRole("columnheader", { name: "Ran for" })).toBeVisible();
    },
  },
  {
    id: "ports",
    go: async (page) => {
      await page.getByTestId("process-view-ports").click();
      await expect(page.getByTestId("port-row-5173")).toContainText("Left running");
    },
  },
  {
    id: "history",
    go: async (page) => {
      await page.getByTestId("process-view-history").click();
      await expect(page.getByText("Checkout redesign").first()).toBeVisible();
    },
  },
];

for (const theme of THEMES) {
  for (const vp of WIDTHS) {
    for (const view of VIEWS) {
      const tag = view.id === "attention" && vp.name === "1280x800" && theme === "dark" ? gate("90-processes") : {};
      test(`processes — ${view.id} — ${vp.name} — ${theme}`, tag, async ({ page }) => {
        await seed(page, theme, vp);
        await openPage(page);
        await view.go(page);
        await shot(page, `${view.id}-${vp.name}-${theme}`);
      });
    }
  }

  test(`processes — include other apps — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.getByTestId("process-view-running").click();
    await page.getByTestId("process-filters").click();
    await page.getByTestId("filter-others").check();
    await shot(page, `filters-${MID.name}-${theme}`);
    await page.keyboard.press("Escape");
    const heading = page.getByTestId("process-row-a:other");
    await heading.scrollIntoViewIfNeeded();
    await expect(heading).toContainText("names and memory only");
    await expect(page.getByTestId("process-row-o:Safari")).not.toContainText("Stop…");
    await shot(page, `other-apps-${MID.name}-${theme}`);
  });

  test(`processes — a row's menu — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.locator('[data-testid^="process-menu-p:8123"]').first().click();
    await expect(page.getByTestId("menu-keep")).toBeVisible();
    await shot(page, `menu-${MID.name}-${theme}`);
  });

  test(`processes — Stop… opens the verified sheet — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.getByTestId("process-stop-8123").click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await shot(page, `stop-${MID.name}-${theme}`);
  });

  test(`processes — another app's leftover: the D-41b sheet — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.getByTestId("process-stop-7301").click();
    await expect(page.getByRole("dialog")).toContainText("node");
    await shot(page, `stop-theirs-${MID.name}-${theme}`);
  });

  test(`processes — clean up what a session left — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.getByTestId("session-clean-s:ours:s-pay").click();
    await expect(page.getByTestId("group-cleanup")).toBeVisible();
    await shot(page, `group-cleanup-${MID.name}-${theme}`);
  });

  test(`processes — automatic cleanup's dry run — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await openPage(page);
    await page.getByTestId("rules-preview-claude-work-1").click();
    const preview = page.getByTestId("rule-preview-claude-work-1");
    await expect(preview).toBeVisible();
    await preview.evaluate((element) => element.scrollIntoView({ block: "center" }));
    await shot(page, `rules-preview-${MID.name}-${theme}`);
  });

  test(`processes — a session's Processes tab — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID);
    await page.locator('[data-instance-id="s-web"]').first().click();
    await page.getByRole("tab", { name: "Processes" }).click();
    await expect(page.getByTestId("session-processes")).toContainText("node (vite)");
    await shot(page, `session-tab-${MID.name}-${theme}`);
  });

  test(`processes — the large Mac (2,000 processes) — ${theme}`, async ({ page }) => {
    await seed(page, theme, MID, "?aij_processes=large");
    await goTo(page, "Processes");
    await page.getByTestId("process-view-running").click();
    await expect(page.getByTestId("processes-page")).toBeVisible();
    // Windowed: at most 60 rows in the DOM, however many the view holds.
    const rows = await page.locator('[data-testid^="process-row-"]').count();
    expect(rows).toBeLessThanOrEqual(60);
    await shot(page, `large-${MID.name}-${theme}`);
  });
}
