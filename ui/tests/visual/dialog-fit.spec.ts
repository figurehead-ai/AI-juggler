/**
 * VALIDATION N1 (D-142, DESIGN-SYSTEM §4.2 DLG-1 to DLG-4, BTN-1): no
 * dialog's way out or first question is ever out of reach. The
 * maintainer, on the Move or Copy dialog, whose choices slid under its
 * top edge while its body scrolled: "The buttons should always be sticky
 * and can't be scrolled out of the way."
 *
 * Every dialog and sheet the gallery opens, each opened the way its
 * gallery spec opens it, at 1024×680 and 1280×800, dark and light. For
 * each (the frame, `[data-dialog-frame]`, else the dialog element):
 *  1. its box is inside the window;
 *  2. it has one header, one body and at most one footer
 *     (`data-dialog-part`, set by components/DialogFrame.tsx);
 *  3. the body scrolls (overflow-y auto or scroll); the frame and the
 *     overlay don't;
 *  4. with the body at its end, then its middle, the header's and the
 *     footer's boxes don't move (±1 px) and every footer button (and a
 *     flow's sticky actions, `data-dialog-actions`) is the topmost thing
 *     at its centre;
 *  5. a body that scrolls has a footer with a top border of 1 px or more,
 *     and carries `data-more` while it isn't at its end;
 *  6. the body never scrolls sideways;
 *  7. the first choice group (`[role=radiogroup]`, `[data-choices]`) is
 *     wholly inside the body's view on open (DLG-3);
 *  8. one main action where the footer offers a choice of buttons, never
 *     two: the filled primary (`data-primary`), or on a destructive
 *     confirm its danger button (`data-danger`, never the default).
 * And the reach the maintainer asked for: the title, the close and the
 * first field whole on open; the last field whole at the body's end (the
 * footer covers nothing).
 *
 * Writes docs/qa/evidence/design-system/dialog-fit.json (every dialog's
 * measurements). CI runs it in the accessibility gate's job
 * (scripts/a11y_gate.sh). It failed on 0.5.0's tree before DialogFrame:
 * "Copy settings into an app" at 1024×680, its Cancel and Copy cut by the
 * window's foot; "remove a worktree", its title cut by the session's side.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { open } from "./gallery-04";
import { allStatuses, DOOR_ONECLICK, DOOR_WIZARD, goTo, openRailList, openRailSession, openSwitch, openUserMenu, projectLens, settingsPane, switchRows } from "./fixtures";
import { EXAMPLE_MOVE_OFFER } from "../../src/preview/switch-move";
import { EXAMPLE_PROJECT_SYNC } from "../../src/preview/project-sync";
import { STATES } from "../../src/preview/ai-models";

const T0 = 1_728_800_000_000;

const SIZES = [
  { width: 1024, height: 680 },
  { width: 1280, height: 800 },
];

type Case = {
  name: string;
  seams?: Record<string, unknown>;
  storage?: Record<string, string>;
  /** The preview's own data, not allStatuses (the pools sheet's session). */
  previewData?: boolean;
  open: (page: Page) => Promise<void>;
};

/** A live session's switch list, then its first target. */
async function openSwitchFlow(page: Page): Promise<void> {
  await openRailSession(page, "sess-2");
  await openSwitch(page);
  await switchRows(page).first().click();
  await expect(page.getByTestId("switch-flow")).toBeVisible();
}

const GIT_FIXTURE = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) => ({ ...session, git_branch: "juggler/a1b2c3", git_dirty_count: 3, git_ahead: 1, git_worktree: true, cwd: "/Users/dev/ai-juggler" })),
};

const CASES: Case[] = [
  {
    name: "Move or Copy to account",
    seams: { __AIJ_SWITCH_PREFLIGHT__: DOOR_ONECLICK, __AIJ_MOVE_OFFER__: EXAMPLE_MOVE_OFFER },
    open: async (page) => {
      await openSwitchFlow(page);
      await expect(page.getByTestId("switch-how")).toBeVisible();
      await expect(page.getByTestId("switch-move")).toBeVisible();
    },
  },
  {
    name: "the switch wizard",
    seams: { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD },
    open: async (page) => {
      await openSwitchFlow(page);
      await expect(page.getByTestId("switch-wizard")).toBeVisible();
    },
  },
  {
    name: "the switch wizard, What's carried",
    seams: { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD },
    open: async (page) => {
      await openSwitchFlow(page);
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-step-carried")).toBeVisible();
    },
  },
  {
    name: "the switch wizard, Confirm",
    seams: { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD },
    open: async (page) => {
      await openSwitchFlow(page);
      await page.getByTestId("switch-continue").click();
      await page.getByTestId("switch-continue").click();
      await expect(page.getByTestId("switch-step-confirm")).toBeVisible();
    },
  },
  {
    name: "the switch, done",
    seams: { __AIJ_SWITCH_PREFLIGHT__: DOOR_WIZARD },
    open: async (page) => {
      await openSwitchFlow(page);
      await page.getByTestId("switch-continue").click();
      await page.getByTestId("switch-continue").click();
      await page.getByTestId("switch-confirm").click();
      await expect(page.getByTestId("switch-done")).toBeVisible();
    },
  },
  {
    name: "Continue on another account",
    open: async (page) => {
      await openRailSession(page, "sess-2");
      await openSwitch(page);
    },
  },
  {
    name: "New session",
    open: async (page) => {
      await page.getByRole("banner").getByRole("button", { name: "New session", exact: true }).click();
      await expect(page.getByTestId("new-session-sheet")).toBeVisible();
    },
  },
  {
    name: "Add an app",
    seams: { __AIJ_SCAN_REPORT__: { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } },
    open: async (page) => {
      await goTo(page, "Accounts");
      await page.getByTestId("add-account").click();
      await expect(page.getByTestId("provider-cards")).toBeVisible();
    },
  },
  {
    name: "New project",
    open: async (page) => {
      await expect(page.getByRole("button", { name: "New project" }).first()).toBeVisible();
      await page.keyboard.press("Meta+Shift+N");
      await expect(page.getByRole("dialog", { name: "New project" })).toBeVisible();
    },
  },
  {
    name: "Copy settings into an app",
    open: async (page) => {
      await goTo(page, "Accounts");
      await page.getByRole("button", { name: "App actions for Claude Code CLI · Personal", exact: true }).click();
      await page.getByRole("menuitem", { name: "Copy settings from…" }).click();
      await expect(page.getByRole("dialog", { name: /^Copy settings into/ })).toBeVisible();
    },
  },
  {
    name: "Quit AI Juggler",
    open: async (page) => {
      await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: { live: 3, end_on_quit: 2, keep_running: 1 } })));
      await expect(page.getByTestId("quit-sheet")).toBeVisible();
    },
  },
  {
    name: "the limit",
    open: async (page) => {
      await page.getByTestId("home-need-limit").first().getByRole("button", { name: /^Continue in another app/ }).click();
      await expect(page.getByTestId("limit-dialog")).toBeVisible();
    },
  },
  {
    name: "Report a bug",
    open: async (page) => {
      await openUserMenu(page);
      await page.getByRole("menu", { name: "Your menu" }).getByRole("menuitem", { name: /Report a bug/ }).click();
      await expect(page.getByTestId("report-bug")).toBeVisible();
    },
  },
  {
    name: "Re-create the office",
    seams: { __AIJ_FLOOR_SETTINGS__: { open_at_start: false, on_top: false, apps_per_floor: 8, motion: "system", decay: true, hidden: ["cli:old"] } },
    open: async (page) => {
      await goTo(page, "Settings");
      await settingsPane(page, "The Floor");
      await page.getByTestId("floor-recreate-open").click();
      await expect(page.getByTestId("floor-recreate-sheet")).toBeVisible();
    },
  },
  {
    name: "Prices and details",
    open: async (page) => {
      await goTo(page, "Usage");
      await page.getByTestId("usage-prices-open").click();
      await expect(page.getByTestId("usage-prices")).toBeVisible();
    },
  },
  {
    name: "the restore plan",
    seams: {
      __AIJ_BACKUP_FILE__: "/Users/dev/b.ajbackup",
      __AIJ_RESTORE_PLAN__: {
        path: "",
        other_machine: true,
        token: "t",
        notes: ["Sign in to each Account again afterwards."],
        manifest: { format: "ai-juggler-backup", format_version: 1, schema_version: 27, app_version: "0.0.1", created_at_ms: T0, origin: { fingerprint: "x", label: "Studio Mac" }, counts: { instances: 3 }, files: [], withheld: [], excluded: [] },
      },
    },
    open: async (page) => {
      await goTo(page, "Settings");
      await settingsPane(page, "Advanced");
      await page.getByTestId("backup-restore").click();
      await expect(page.getByTestId("restore-plan")).toBeVisible();
    },
  },
  {
    name: "the recovery code",
    seams: {
      __AIJ_FIGUREHEAD__: {
        state: "linked",
        email: "you@example.com",
        plan: "Free",
        device_name: "Studio Mac",
        features: [
          { key: "cloudBackup", access: "included", tier: "free", on: true },
          { key: "fleetDiscovery", access: "included", tier: "free", on: true },
          { key: "usageSync", access: "included", tier: "free", on: false },
        ],
        link: null,
        refreshed_at_ms: T0 - 3_600_000,
        offline: false,
        notice: null,
        consent_version: 1,
        consent_current: 1,
      },
      __AIJ_VAULT_STATUS__: { has_key: false, code_saved: false },
      __AIJ_VAULT_CODE__: "MZXW-6YTB-OI3D-EZLM-NFXG-K4TU-OJSW-Y5DF-MFZG-K3TF-ON2G-K3DF-NZTW-Q2AB",
    },
    open: async (page) => {
      await goTo(page, "Settings");
      await settingsPane(page, "Advanced");
      await expect(page.getByTestId("recovery-sheet")).toBeVisible();
    },
  },
  {
    name: "the wrong account",
    seams: {
      __AIJ_IDENTITY_CHECK__: {
        "p-1": {
          state: "mismatch",
          provider: "claude-code",
          account_id: "claude-work",
          account_label: "Work Max",
          model: "opus",
          expected: "o…@work.example · Acme",
          detected: "o…@home.example",
          reason: null,
          alternatives: [{ recipe_id: "p-2", recipe_name: "Main", account_label: "Personal" }],
        },
      },
    },
    open: async (page) => {
      await openEnded(page);
      await page.getByTestId("header-relaunch").click();
      await expect(page.getByTestId("identity-mismatch")).toBeVisible();
    },
  },
  {
    name: "a folder of its own",
    seams: {
      __AIJ_ISOLATION_OFFER__: {
        project_name: "ai-juggler",
        in_use: [{ id: "sess-1", label: "Work Max", git_branch: "main", cwd: "/Users/dev/ai-juggler", account_label: "Personal", kind: "terminal", provider: "claude-code", started_at_ms: T0 - 3_600_000 }],
        available: true,
        unavailable_reason: null,
        proposed_path: "/Users/dev/ai-juggler.juggler/a1b2c3",
        proposed_branch: "juggler/a1b2c3",
        base_branch: "main",
      },
    },
    open: async (page) => {
      await openEnded(page);
      await page.getByTestId("header-relaunch").click();
      await expect(page.getByTestId("isolation-offer")).toBeVisible();
    },
  },
  {
    name: "remove a worktree",
    seams: {
      __AIJ_FIXTURE__: GIT_FIXTURE,
      __AIJ_GIT_STATUS__: Object.fromEntries(
        allStatuses.sessions.map((session) => [session.id, { branch: "juggler/a1b2c3", changed: 2, untracked: 1, ahead: 1, behind: 0, worktree: true, created_by_juggler: true, path: "/Users/dev/ai-juggler.juggler/a1b2c3" }]),
      ),
      __AIJ_WORKTREE_REMOVAL__: { path: "/Users/dev/ai-juggler.juggler/a1b2c3", folder: "a1b2c3", branch: "juggler/a1b2c3", changed: 2, untracked: 1, needs_confirmation: true, in_use: false, summary: "Discards 2 changed and 1 untracked files." },
    },
    open: async (page) => {
      await openEnded(page);
      await page.getByTestId("worktree-remove").click();
      await expect(page.getByTestId("worktree-remove-dialog")).toBeVisible();
    },
  },
  {
    name: "clean up what a session left",
    open: async (page) => {
      await goTo(page, "Processes");
      await page.getByTestId("session-clean-s:ours:s-pay").click();
      await expect(page.getByTestId("group-cleanup")).toBeVisible();
    },
  },
  {
    name: "keep a project apart",
    seams: { __AIJ_EDITORS__: { installed: ["Visual Studio Code", "Zed"], chosen: null } },
    open: async (page) => {
      await projectLens(page);
      await openRailList(page);
      await page.getByTestId("rail").getByRole("button", { name: /^ai-juggler/ }).first().click();
      await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: "Overview" }).click();
      await page.getByTestId("project-agents-keep-apart").click();
      await expect(page.getByTestId("keep-apart")).toBeVisible();
    },
  },
  {
    name: "Rules and MCP servers",
    seams: { __AIJ_PROJECT_SYNC__: EXAMPLE_PROJECT_SYNC },
    open: async (page) => {
      await projectLens(page);
      await openRailList(page);
      await page.getByTestId("rail").getByTestId("rail-project").first().click();
      await page.getByTestId("project-rules").click();
      await expect(page.getByTestId("project-sync")).toBeVisible();
    },
  },
  {
    name: "Move to pool",
    previewData: true,
    seams: {
      __AIJ_POOLS__: {
        pools: [{ id: "pool-checkout", name: "Checkout redesign", rules: [] }, { id: "pool-docs", name: "Docs" }],
        members: [{ session_id: "sess-live-1", pool_id: "pool-checkout", role: "lead" }],
      },
      __AIJ_AGENT_BRIDGE__: { asked: true, on: true },
    },
    open: async (page) => {
      await openRailList(page);
      await page.getByTestId("rail").locator('[data-instance-id="sess-live-1"]').first().click({ button: "right" });
      await page.getByRole("menuitem", { name: "Move to pool…" }).click();
      await expect(page.getByTestId("pool-move-sheet")).toBeVisible();
    },
  },
  {
    name: "usage statistics, asked again",
    seams: {
      __AIJ_USAGE_STATS__: {
        info: { included: true, chosen: "share", effective: "share", can_upload: false, reason: null, decided_at_ms: T0 - 90 * 86_400_000, consent_version: 1, current_consent_version: 2 },
        week: "2026-W42",
        rows: [],
        reports: [],
        next_upload_ms: null,
        last_error: null,
        docs_url: "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md",
        sync: { offered: false, on: false, email: null, notice: null },
      },
    },
    open: async (page) => {
      await expect(page.getByTestId("usage-stats-reconsent")).toBeVisible();
    },
  },
  {
    name: "Add a machine",
    seams: { __AIJ_FLEET_PEERS__: [], __AIJ_LINK__: { enabled: true, port: 47821, error: null } },
    open: async (page) => {
      await goTo(page, "Fleet");
      await page.getByTestId("fleet-add").click();
      await expect(page.getByTestId("pairing-wizard")).toBeVisible();
    },
  },
  {
    name: "a model's download",
    seams: { __AIJ_AI_CATALOG__: STATES.approval.catalog, __AIJ_AI_MODELS__: STATES.approval.models, __AIJ_AI_JOBS__: STATES.approval.jobs, __AIJ_AI_START__: STATES.approval.start },
    storage: { "aij.settings.pane": "ai-models" },
    open: async (page) => {
      await goTo(page, "Settings");
      await expect(page.getByTestId("models-skeleton")).toHaveCount(0);
      await page.locator("[data-model='granite-4-1-3b-q4km']").getByTestId("model-download").click();
      await expect(page.getByTestId("models-approval")).toBeVisible();
    },
  },
];

/** sess-8, stopped with an error: Relaunch, a worktree's removal. Below
 * 1100 px the rail is its icon column, which shows only what is open,
 * so its list first. */
async function openEnded(page: Page): Promise<void> {
  await openRailList(page);
  await openRailSession(page, "sess-8");
}

/** The top dialog: the last modal one open. */
function topDialog(page: Page): Locator {
  return page.locator("[role='dialog'][aria-modal='true']:visible, [role='alertdialog']:visible, dialog[open]:visible").last();
}

type Measure = {
  problems: string[];
  facts: Record<string, unknown>;
};

/** N1's eight checks and the reach checks, at one scroll position. */
function measure(dialog: Locator, where: "open" | "end" | "middle"): Promise<Measure> {
  return dialog.evaluate((root, at) => {
    const problems: string[] = [];
    const describe = (el: Element): string =>
      el.getAttribute("data-testid") ?? el.getAttribute("aria-label") ?? `${el.tagName.toLowerCase()} “${(el.textContent ?? "").trim().slice(0, 32)}”`;
    const sized = (el: Element) => {
      const box = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      return box.width > 0 && box.height > 0 && style.visibility !== "hidden" && style.display !== "none";
    };
    /** Why `el` isn't whole and reachable, or null. */
    const whole = (el: Element | undefined | null): string | null => {
      if (!el) return null;
      const box = el.getBoundingClientRect();
      if (box.width < 1 || box.height < 1) return "has no size";
      if (box.top < -0.5 || box.bottom > innerHeight + 0.5) return `outside the window (${Math.round(box.top)}…${Math.round(box.bottom)} of ${innerHeight})`;
      for (let up = el.parentElement; up && up !== document.body; up = up.parentElement) {
        const style = getComputedStyle(up);
        if (style.overflowY === "visible" && style.overflowX === "visible") continue;
        const edge = up.getBoundingClientRect();
        const top = edge.top + up.clientTop;
        const bottom = top + up.clientHeight;
        if (box.top < top - 0.5 || box.bottom > bottom + 0.5) return `cut by ${describe(up)} (${Math.round(box.top)}…${Math.round(box.bottom)} in ${Math.round(top)}…${Math.round(bottom)})`;
      }
      const inset = Math.max(0.5, Math.min(3, box.width / 2 - 0.5, box.height / 2 - 0.5));
      for (const [x, y] of [
        [box.left + inset, box.top + inset],
        [box.right - inset, box.top + inset],
        [box.left + inset, box.bottom - inset],
        [box.right - inset, box.bottom - inset],
        [(box.left + box.right) / 2, (box.top + box.bottom) / 2],
      ] as const) {
        const hit = document.elementFromPoint(x, y);
        if (!hit) return "nothing drawn at it";
        if (hit === el || el.contains(hit) || hit.contains(el) || el.closest("label")?.contains(hit)) continue;
        return `covered by ${describe(hit)}`;
      }
      return null;
    };
    const frame = root.querySelector<HTMLElement>("[data-dialog-frame]") ?? (root as HTMLElement);
    const parts = (name: string) => [...frame.querySelectorAll<HTMLElement>(`[data-dialog-part='${name}']`)].filter((part) => part.closest("[data-dialog-frame]") === (frame.matches("[data-dialog-frame]") ? frame : part.closest("[data-dialog-frame]")));
    const [header, body, footer] = [parts("header"), parts("body"), parts("footer")];
    const box = frame.getBoundingClientRect();
    // 1. Inside the window.
    if (box.top < -0.5 || box.left < -0.5 || box.bottom > innerHeight + 0.5 || box.right > innerWidth + 0.5)
      problems.push(`1: the dialog's box ${Math.round(box.left)},${Math.round(box.top)}–${Math.round(box.right)},${Math.round(box.bottom)} isn't inside ${innerWidth}×${innerHeight}`);
    // 2. One header, one body, at most one footer.
    if (header.length !== 1 || body.length !== 1 || footer.length > 1) problems.push(`2: parts header ${header.length}, body ${body.length}, footer ${footer.length}`);
    const scroller = body[0];
    const overlay = frame.closest("[data-dialog-overlay]") ?? frame.parentElement;
    // 3. Only the body scrolls.
    if (scroller && !/(auto|scroll)/.test(getComputedStyle(scroller).overflowY)) problems.push("3: the body doesn't scroll");
    for (const [name, el] of [["frame", frame], ["overlay", overlay]] as const)
      if (el && /(auto|scroll)/.test(getComputedStyle(el).overflowY) && el.scrollHeight > el.clientHeight + 1) problems.push(`3: the ${name} scrolls`);
    const scrolls = !!scroller && scroller.scrollHeight > scroller.clientHeight + 1;
    const atEnd = !scroller || scroller.scrollHeight - scroller.clientHeight - scroller.scrollTop <= 1;
    // 5. A divider and data-more while it scrolls.
    if (scrolls && footer[0] && parseFloat(getComputedStyle(footer[0]).borderTopWidth) < 1) problems.push("5: the footer has no top border while the body scrolls");
    if (scrolls && !atEnd && !scroller.hasAttribute("data-more")) problems.push("5: the body isn't marked data-more while more is below");
    // 6. Never sideways.
    if (scroller && scroller.scrollWidth > scroller.clientWidth + 1) problems.push(`6: the body scrolls sideways (${scroller.scrollWidth} in ${scroller.clientWidth})`);
    // 4. Every footer button (and a flow's sticky actions) reachable.
    const buttons = [...(footer[0]?.querySelectorAll("button") ?? []), ...frame.querySelectorAll("[data-dialog-actions] button")].filter(sized);
    for (const button of buttons) {
      const why = whole(button);
      if (why) problems.push(`4: ${at}: ${describe(button)} ${why}`);
    }
    // 7. The first choice group whole on open.
    if (at === "open" && scroller) {
      const group = scroller.querySelector("[role='radiogroup'], [data-choices]");
      const why = group && sized(group) ? whole(group) : null;
      if (why) problems.push(`7: the first choice group ${why}`);
    }
    // 8. One primary where the footer offers a choice of buttons.
    const primaries = [...frame.querySelectorAll("[data-primary], [data-danger]")].filter(sized).length;
    const choiceOfButtons = (footer[0]?.querySelectorAll("button").length ?? frame.querySelectorAll("[data-dialog-actions] button").length) >= 2;
    if (primaries > 1 || (choiceOfButtons && primaries !== 1)) problems.push(`8: ${primaries} main actions (filled primary or danger)`);
    // Reach: the title and close, the first field on open, the last at the end.
    const FIELD = "input:not([type='hidden']), select, textarea, [role='radio'], [role='checkbox'], [role='switch'], [role='combobox']";
    const fields = [...(scroller ?? frame).querySelectorAll(FIELD)].filter(sized);
    const labelled = root.getAttribute("aria-labelledby");
    const title = (labelled && document.getElementById(labelled)) || frame.querySelector("h1, h2, h3");
    for (const [name, el] of [
      ["title", title],
      ["close", frame.querySelector("[aria-label='Close']")],
      [at === "open" ? "first field" : "last field", at === "open" ? fields[0] : at === "end" ? fields.at(-1) : null],
    ] as const) {
      const why = whole(el);
      if (why) problems.push(`reach: ${at}: ${name} ${why}`);
    }
    const rect = (el: Element | undefined) => (el ? (({ top, bottom, left, right }) => ({ top: Math.round(top), bottom: Math.round(bottom), left: Math.round(left), right: Math.round(right) }))(el.getBoundingClientRect()) : null);
    return {
      problems,
      facts: {
        dialog: rect(frame),
        header: rect(header[0]),
        body: scroller ? { ...rect(scroller), scrollHeight: scroller.scrollHeight, clientHeight: scroller.clientHeight, scrollTop: Math.round(scroller.scrollTop) } : null,
        footer: rect(footer[0]),
        primaries,
      },
    };
  }, where);
}

/** The body to its end (`1`) or its middle (`0.5`), then a frame. */
async function scrollBody(dialog: Locator, to: number): Promise<void> {
  await dialog.evaluate((root, share) => {
    const body = root.querySelector<HTMLElement>("[data-dialog-part='body']");
    if (body) body.scrollTop = Math.round((body.scrollHeight - body.clientHeight) * share);
  }, to);
  await dialog.page().evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
}

const OUT = resolve(process.cwd(), "../docs/qa/evidence/design-system");
const results: Record<string, unknown>[] = [];

for (const theme of ["dark", "light"] as const) {
  for (const size of SIZES) {
    for (const sheet of CASES) {
      const name = `${sheet.name} — ${size.width}x${size.height} — ${theme}`;
      test(`${name}: fixed header and footer, the body alone scrolls`, async ({ page }) => {
        const seams = sheet.previewData ? { ...sheet.seams } : { __AIJ_FIXTURE__: allStatuses, ...sheet.seams };
        await open(page, theme, size, seams, sheet.storage);
        await sheet.open(page);
        const dialog = topDialog(page);
        await expect(dialog).toBeVisible();
        await dialog.evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
        const opened = await measure(dialog, "open");
        await scrollBody(dialog, 1);
        const end = await measure(dialog, "end");
        await scrollBody(dialog, 0.5);
        const middle = await measure(dialog, "middle");
        const moved: string[] = [];
        for (const part of ["header", "footer"] as const) {
          const a = opened.facts[part] as Record<string, number> | null;
          for (const [at, other] of [["end", end], ["middle", middle]] as const) {
            const b = other.facts[part] as Record<string, number> | null;
            if (a && b && (Math.abs(a.top! - b.top!) > 1 || Math.abs(a.bottom! - b.bottom!) > 1)) moved.push(`4: the ${part} moved at the ${at} (${a.top}…${a.bottom} → ${b.top}…${b.bottom})`);
          }
        }
        const problems = [...opened.problems, ...end.problems.filter((p) => !p.startsWith("reach: end: first")), ...middle.problems.filter((p) => !p.startsWith("reach:")), ...moved];
        results.push({ dialog: sheet.name, size: `${size.width}x${size.height}`, theme, pass: problems.length === 0, problems, open: opened.facts, end: end.facts });
        expect(problems).toEqual([]);
      });
    }
  }
}

// Each worker (the gate runs it fully parallel, in shards) merges its
// rows into the one table, by dialog, size and theme.
test.afterAll(() => {
  if (results.length === 0) return;
  mkdirSync(OUT, { recursive: true });
  const file = resolve(OUT, "dialog-fit.json");
  const key = (r: Record<string, unknown>) => `${r.dialog}|${r.size}|${r.theme}`;
  let earlier: Record<string, unknown>[] = [];
  try {
    earlier = (JSON.parse(readFileSync(file, "utf8")) as { dialogs: Record<string, unknown>[] }).dialogs;
  } catch {
    earlier = [];
  }
  const mine = new Set(results.map(key));
  const known = new Set(CASES.map((sheet) => sheet.name));
  const rows = [...earlier.filter((r) => !mine.has(key(r)) && known.has(r.dialog as string)), ...results].sort((a, b) => key(a).localeCompare(key(b)));
  writeFileSync(file, JSON.stringify({ check: "VALIDATION N1 (ui/tests/visual/dialog-fit.spec.ts)", dialogs: rows }, null, 2) + "\n");
});
