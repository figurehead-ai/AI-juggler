/**
 * E18 gallery: messages between agents (AJ-E1802..E1804).
 *
 * - A pool's Messages tab: the threads, the open conversation (an agent's
 *   handoff, a question to you, your reply, a delivered message, Deliver
 *   where it can go), and the shared documents with what was withheld.
 * - The same pool with its messages paused (two agents kept answering
 *   each other), the banner and Resume.
 * - A session's Messages tab: its own threads, written to it.
 * - Home's Needs you: an agent's held message and a chain at the hop
 *   limit as approval cards, a question to you and messages waiting.
 *
 * At 1280, 1680 and 2000 px wide, both themes; axe 0 critical/serious on
 * each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, openFilters } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/93-messages");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** The gate's one clock (axe-gallery.ts). */
const T0 = GATE_NOW;
const MIN = 60_000;

const POOLS = {
  pools: [{ id: "pool-checkout", name: "Checkout redesign", access: undefined }],
  members: [
    { session_id: "sess-live-1", pool_id: "pool-checkout", role: "lead" },
    { session_id: "sess-done-1", pool_id: "pool-checkout", role: "member" },
    { session_id: "sess-done-2", pool_id: "pool-checkout", role: "member" },
  ],
};

const planner = { id: "session:sess-live-1", name: "Planner", session_id: "sess-live-1" };
const builder = { id: "session:sess-done-1", name: "Builder", session_id: "sess-done-1" };
const reviewer = { id: "session:sess-done-2", name: "Reviewer", session_id: "sess-done-2" };
const to = (who: typeof planner, state: string, deliverable = false, how = "") => ({ ...who, state, via: state === "delivered" ? "turn" : null, deliverable, how });
const you = (state: string) => ({ id: "user", name: "You", session_id: null, state, via: null, deliverable: false, how: "To you" });
const msg = (over: Record<string, unknown>) => ({
  thread_id: "thr-api",
  from_kind: "agent",
  from_session_id: "sess-live-1",
  kind: "note",
  subject: "Start on the API",
  hop: 1,
  redacted: 0,
  to_all: false,
  ...over,
});

const THREADS = [
  ["thr-api", "Start on the API", ["Planner", "Builder", "You"], 6, 2, 8, "Builder", "Done: orders and cart endpoints are in, tests pass.", 0],
  ["thr-review", "Review the checkout steps", ["Reviewer", "You"], 2, 1, 26, "Reviewer", "Should the address step come before shipping?", 1],
  ["thr-schema", "Schema change: order totals", ["Planner", "Builder", "Reviewer"], 3, 2, 55, "Planner", "Totals move to integer cents.", 0],
  ["thr-flaky", "Flaky test in cart.spec", ["Builder", "Reviewer"], 4, 4, 140, "Reviewer", "Seen it twice on CI; retrying hides it.", 0],
  ["thr-release", "Release notes draft", ["Planner", "You"], 2, 0, 320, "You", "Keep it to user-facing changes.", 0],
  ["thr-deps", "Upgrade the payment SDK", ["Builder", "Planner"], 5, 3, 900, "Builder", "v9 drops the legacy client; two call sites change.", 0],
].map(([id, subject, people, count, hop, ago, last_from, last_line, unread]) => ({
  id,
  pool_id: "pool-checkout",
  pool_name: "Checkout redesign",
  subject,
  people,
  count,
  hop,
  hop_cap: 6,
  state: "open",
  state_reason: null,
  updated_at_ms: T0 - (ago as number) * MIN,
  last_from,
  last_line,
  unread,
}));

const MESSAGES = {
  "thr-api": [
    msg({ id: "m1", from_name: "Planner", kind: "handoff", at_ms: T0 - 48 * MIN, body: "Start on the API: orders, then the cart. The schema is in the shared doc api-schema.md; keep totals in integer cents.", recipients: [to(builder, "delivered", false, "Started its turn")] }),
    msg({ id: "m2", from_name: "Builder", from_session_id: "sess-done-1", kind: "question", hop: 2, at_ms: T0 - 41 * MIN, body: "Should a cart with no items return 404 or an empty list?", recipients: [to(planner, "read"), you("read")] }),
    msg({ id: "m3", from_name: "You", from_kind: "user", from_session_id: null, hop: 0, at_ms: T0 - 38 * MIN, body: "An empty list. A 404 would break the mobile client.", recipients: [to(builder, "delivered", false, "Started its turn"), to(planner, "read")] }),
    msg({ id: "m4", from_name: "Planner", kind: "note", hop: 1, redacted: 1, at_ms: T0 - 20 * MIN, body: "The staging key in .env.example was real; I replaced it. STRIPE_KEY=•••", recipients: [to(builder, "acked")] }),
    msg({ id: "m5", from_name: "Builder", from_session_id: "sess-done-1", kind: "status", hop: 2, at_ms: T0 - 8 * MIN, body: "Done: orders and cart endpoints are in, tests pass. Next: the totals migration.", recipients: [to(planner, "unread", true, "Deliver puts it in as its next prompt, between turns"), to(reviewer, "unread", false, "Runs in a terminal: it reads its messages itself; AI Juggler never types into a terminal")] }),
  ],
  "thr-review": [
    msg({ id: "r1", thread_id: "thr-review", subject: "Review the checkout steps", from_name: "Reviewer", from_session_id: "sess-done-2", kind: "question", at_ms: T0 - 26 * MIN, body: "Should the address step come before shipping? The design has it after, the code before.", recipients: [you("unread")] }),
  ],
};

const DOCS = [
  { id: "doc-1", pool_id: "pool-checkout", name: "api-schema.md", mime: "text/markdown", size_bytes: 6_410, shared_by: "Planner", at_ms: T0 - 50 * MIN, policy: "strip", policy_words: "Secrets replaced with •••", withheld: [{ what: "API keys", count: 2 }] },
  { id: "doc-2", pool_id: "pool-checkout", name: "checkout-steps.md", mime: "text/markdown", size_bytes: 1_920, shared_by: "Reviewer", at_ms: T0 - 30 * MIN, policy: "strip", policy_words: "Secrets replaced with •••", withheld: [] },
  { id: "doc-3", pool_id: "pool-checkout", name: "migration-plan.txt", mime: "text/plain", size_bytes: 840, shared_by: "Builder", at_ms: T0 - 12 * MIN, policy: "strip", policy_words: "Secrets replaced with •••", withheld: [] },
];

const NEEDS = [
  { key: "ask:1", kind: "ask", pool_id: "pool-checkout", pool_name: "Checkout redesign", thread_id: "thr-schema", message_id: "x1", session_id: null, ask_id: "ask-1", offer_always: true, title: "«Reviewer» wants to message Builder", detail: "«Schema change: order totals» · above what the pool lets agents do", preview: "Before you migrate: the refunds table also stores totals as floats.", at_ms: T0 - 4 * MIN },
  { key: "ask:2", kind: "deliver", pool_id: "pool-checkout", pool_name: "Checkout redesign", thread_id: "thr-deps", message_id: "x2", session_id: null, ask_id: "ask-2", offer_always: false, title: "«Builder»'s message would start «Planner»'s next turn", detail: "«Upgrade the payment SDK» · 5 agent messages in a row without you", preview: "Planner: v9 drops the legacy client; can you take the two call sites?", at_ms: T0 - 3 * MIN },
  { key: "question:r1", kind: "question", pool_id: "pool-checkout", pool_name: "Checkout redesign", thread_id: "thr-review", message_id: "r1", session_id: null, ask_id: null, offer_always: false, title: "«Reviewer» asks you", detail: "«Review the checkout steps» · Checkout redesign", preview: null, at_ms: T0 - 26 * MIN },
  { key: "waiting:sess-live-1", kind: "waiting", pool_id: "pool-checkout", pool_name: "Checkout redesign", thread_id: "thr-api", message_id: "m5", session_id: "sess-live-1", ask_id: null, offer_always: false, title: "Planner has a message waiting", detail: "from Builder · Checkout redesign", preview: null, at_ms: T0 - 8 * MIN },
];

const SIZES = [
  [1280, 800],
  [1680, 1050],
  [2000, 1250],
] as const;

async function open(page: Page, theme: "dark" | "light", width: number, height: number, paused = false) {
  // The gate's clock seam (axe-gallery.ts): Date.now() and new Date()
  // both read T0, in the gate and in the evidence gallery alike.
  await page.clock.setFixedTime(T0);
  await page.addInitScript(
    ({ theme, pools, seed }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_POOLS__ = pools;
      w.__AIJ_MESSAGES__ = seed;
      w.__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    {
      theme,
      pools: POOLS,
      seed: {
        threads: THREADS,
        messages: MESSAGES,
        docs: DOCS,
        docText: { "doc-1": "# Orders API\n\nPOST /orders\nGET /orders/:id\nSTRIPE_KEY=•••\n" },
        needs: NEEDS,
        paused: paused ? [{ pool_id: "pool-checkout", pool_name: "Checkout redesign", at_ms: T0 - 2 * MIN, reason: "«Builder» and «Planner» kept answering each other (8 messages without you)" }] : [],
      },
    },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function poolMessages(page: Page) {
  await openFilters(page);
  await page.getByTestId("rail-view").getByRole("radio", { name: /^Pool/ }).check();
  await page.keyboard.press("Escape");
  await page.getByTestId("place-header").getByRole("button", { name: /^Open Checkout redesign/ }).click();
  await expect(page.getByTestId("pool-page")).toBeVisible();
  await page.getByTestId("pool-panel-messages").click();
  await expect(page.getByTestId("thread-view")).toBeVisible();
  await expect(page.getByTestId("message-bubble")).toHaveCount(5);
}

for (const theme of ["dark", "light"] as const) {
  for (const [width, height] of SIZES) {
    const size = `${width}x${height}`;
    test(`a pool's messages — ${size} ${theme}`, width === 1280 ? gate("93-messages") : {}, async ({ page }) => {
      await open(page, theme, width, height);
      await poolMessages(page);
      await expect(page.getByTestId("message-deliver")).toHaveCount(1);
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `pool-messages-${size}-${theme}.png`) });
      await axe(page, `pool messages ${size} ${theme}`);
    });

    test(`the pool's shared docs — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await poolMessages(page);
      const docs = page.getByTestId("shared-docs");
      await docs.getByRole("button", { name: /Read api-schema\.md/ }).click();
      await expect(page.getByTestId("shared-doc-text")).toContainText("STRIPE_KEY=•••");
      await docs.scrollIntoViewIfNeeded();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `pool-docs-${size}-${theme}.png`) });
      await axe(page, `pool docs ${size} ${theme}`);
    });

    test(`Needs you with messages — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      const needs = page.getByTestId("message-needs");
      await expect(needs.getByTestId("ask-card")).toHaveCount(2);
      await expect(needs.getByTestId("ask-option-always")).toHaveCount(1);
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `needs-you-${size}-${theme}.png`) });
      await axe(page, `needs you ${size} ${theme}`);
    });
  }

  test(`paused messages — 1280x800 ${theme}`, async ({ page }) => {
    await open(page, theme, 1280, 800, true);
    await poolMessages(page);
    await expect(page.getByTestId("messages-paused")).toContainText("kept answering each other");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-paused-1280x800-${theme}.png`) });
    await axe(page, `paused ${theme}`);
  });

  test(`a session's messages — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050);
    await page.locator('[data-instance-id="sess-live-1"]').first().click();
    await page.getByRole("tab", { name: "Messages" }).click();
    await expect(page.getByTestId("thread-row")).toHaveCount(1);
    await expect(page.getByTestId("message-bubble")).toHaveCount(5);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `session-messages-1680x1050-${theme}.png`) });
    await axe(page, `session messages ${theme}`);
  });
}

test("keyboard: a held message answered with 1, Deliver reached by Tab", async ({ page }) => {
  await open(page, "dark", 1280, 800);
  const card = page.getByTestId("message-needs").getByTestId("ask-card").first();
  await card.focus();
  await page.keyboard.press("1");
  await expect(page.getByTestId("message-needs").getByTestId("ask-card")).toHaveCount(1);
  const calls = await page.evaluate(() => (window as unknown as { __AIJ_MESSAGES_CALLS__?: string[] }).__AIJ_MESSAGES_CALLS__ ?? []);
  expect(calls).toContain("answer:ask-1:once");
});
