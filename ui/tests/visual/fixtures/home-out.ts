/**
 * 0.4.5 (the maintainer's 0.4.4 finding): Home's "Your apps" at its
 * narrowest cards (210 px, the grid's least), with an Account out of
 * room whose reset reads at its longest. Its mark, "Out · resets 6:00 AM
 * tomorrow", ran past the card's right edge. Work Max is out on an
 * estimate ("Out · resets about 10:00 AM tomorrow": the longest a
 * day's reset reads), Codex out on a measured reading ("Out · resets
 * 6:00 AM tomorrow", the words reported), Personal low ("Low · 7% left").
 * The gallery's capture and the a11y spec's check read this one screen.
 *
 * Times read in UTC and en-US (`HOME_OUT_USE`), so the words are the
 * same on any Mac and in the gate's container.
 */
import { expect, type Page } from "@playwright/test";
import { placesConversations } from "../../../src/preview/places";

/** The gate's clock (GATE_NOW): Sun, Oct 13 2024, 6:13 AM UTC. */
const T0 = 1_728_800_000_000;
/** The resets, tomorrow (Mon, Oct 14) in UTC. */
const AT_6 = Date.UTC(2024, 9, 14, 6, 0, 0);
const AT_10 = Date.UTC(2024, 9, 14, 10, 0, 0);

/** For `test.use`: the words' clock and language. */
export const HOME_OUT_USE = { locale: "en-US", timezoneId: "UTC" } as const;

/** The viewport whose Home has the grid's narrowest cards: its four
 * apps in one row of 210 px each beside the open rail (8 px narrower,
 * three to a row). */
export const HOME_OUT_VIEWPORT = { width: 1192, height: 800 } as const;

const account = (id: string, provider: string, label: string) => ({ id, provider, label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });

const limit = (used: number, resets: number, badge: "measured" | "estimated") => ({
  state: used >= 100 ? "exhausted" : "tracking",
  percentage: used,
  projected_pct: null,
  badge,
  stale_note_ms: null,
  resets_at_ms: resets,
  reset_basis: "measured",
  measured_at_ms: T0 - 5 * 60_000,
  burned_tokens: 0,
});

export const HOME_OUT_FIXTURE = {
  accounts: [account("claude-work", "claude-code", "Work Max"), account("claude-personal", "claude-code", "Personal"), account("codex", "codex", "Codex")],
  projects: [
    { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
    { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  ],
  sessions: [],
  conversations: placesConversations(T0),
};

export const HOME_OUT_QUOTA = [
  { account: "claude-work", five_hour: limit(100, AT_10, "estimated"), weekly: null },
  { account: "codex", five_hour: limit(100, AT_6, "measured"), weekly: null },
  { account: "claude-personal", five_hour: limit(93, AT_10, "measured"), weekly: null },
];

/** Home with the cards drawn in `theme`; again on the same page for the
 * other theme (the state is read on each load). */
export async function showHomeOut(page: Page, theme: "dark" | "light"): Promise<void> {
  if (!page.url().startsWith("http")) {
    await page.addInitScript(
      ({ fixture, quota, t0 }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_QUOTA_DISPLAY__ = quota;
        localStorage.setItem("aij.theme", sessionStorage.getItem("aij.gallery.theme") ?? "dark");
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => t0;
      },
      { fixture: HOME_OUT_FIXTURE, quota: HOME_OUT_QUOTA, t0: T0 },
    );
    await page.setViewportSize(HOME_OUT_VIEWPORT);
    await page.goto("/");
  }
  await page.evaluate((value) => sessionStorage.setItem("aij.gallery.theme", value), theme);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.reload();
  await expect(page.getByTestId("home-apps")).toBeVisible();
  await expect(page.getByTestId("home-app-room").first()).toBeVisible();
}
