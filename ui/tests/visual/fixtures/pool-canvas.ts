/**
 * 0.3 gallery fixture (redesign §5): a pool's canvas with something of
 * everything — a lead and five members on three tools, one asking to
 * message the lead (above the pool's level), one asking in its own tool,
 * one at its limit, one ended; messages between them, one waiting for a
 * Deliver and one chain paused at the hop limit; two open sessions not in
 * the pool for the palette; a project pool beside it. Invented names, no
 * real data. `canvasSeed(t0)` returns the window seams the preview reads.
 */

const MIN = 60_000;

type Over = Record<string, unknown>;

function session(t0: number, id: string, label: string, over: Over = {}) {
  return {
    id,
    adopted: false,
    launch_recipe_id: "p-1",
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: { input: 182_000, output: 41_000, cache_read: 900_000, cache_write: 12_000, reasoning: 0 },
    cpu_pct: 4.2,
    rss_bytes: 520_000_000,
    child_count: 2,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: t0 - 90 * MIN,
    last_activity_ms: t0 - 2 * MIN,
    machine_id: "this-mac",
    cwd: "/Users/dev/storefront",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-storefront",
    conversation_id: null,
    label,
    ...over,
  };
}

const PLAN = "s-plan";
const CART = "s-cart";
const COPY = "s-copy";
const REVIEW = "s-review";
const TAX = "s-tax";
const SPIKE = "s-spike";
const POOL = "pool-checkout";

/** Every member of the canvas pool, the lead first. */
export const CANVAS_MEMBERS = [PLAN, CART, COPY, REVIEW, TAX, SPIKE];

export function canvasSessions(t0: number) {
  return [
    session(t0, PLAN, "Planner", { last_activity_ms: t0 - 20_000 }),
    session(t0, CART, "Fix the cart totals"),
    session(t0, COPY, "Checkout copy pass", { provider: "codex", account_id: "codex-personal", account_label: "Personal", model: "gpt-5" }),
    session(t0, REVIEW, "Review the checkout steps", { status: "waiting_user", needs_attention: "waiting_user" }),
    session(t0, TAX, "Tax rules research", { status: "quota_blocked", parked: "measured", parked_resets_at_ms: t0 + 95 * MIN }),
    session(t0, SPIKE, "Spike: payment sandbox", { status: "finished", cpu_pct: null, rss_bytes: null, last_activity_ms: t0 - 300 * MIN }),
    // Open, not in the pool: the palette's.
    session(t0, "s-tests", "Price API tests", { last_activity_ms: t0 - 5 * MIN }),
    session(t0, "s-docs", "Rewrite install guide", { provider: "codex", account_id: "codex-personal", account_label: "Personal", project_id: "proj-docs", cwd: "/Users/dev/docs-site" }),
  ];
}

export function canvasProjects(t0: number) {
  const project = (id: string, name: string, letters: string, swatch: number) => ({
    id,
    root: `/Users/dev/${name}`,
    name,
    last_activity_ms: t0 - MIN,
    active_instance: null,
    accounts_used: ["Work Max"],
    instance_count: 1,
    history_sessions: 3,
    history_last_used_ms: null,
    repo: null,
    mark: { letters, swatch, shows: "letters" },
    missing: false,
  });
  return [project("proj-storefront", "storefront", "SF", 2), project("proj-docs", "docs-site", "DS", 4)];
}

const who = (id: string, name: string) => ({ id: `session:${id}`, name, session_id: id });
const to = (id: string, name: string, state: string, deliverable = false, how = "") => ({ ...who(id, name), state, via: state === "delivered" ? "turn" : null, deliverable, how });

export function canvasSeed(t0: number) {
  const msg = (id: string, thread: string, from: [string, string], subject: string, body: string, ago: number, recipients: unknown[], over: Over = {}) => ({
    id,
    thread_id: thread,
    from_name: from[1],
    from_kind: "agent",
    from_session_id: from[0],
    kind: "note",
    subject,
    body,
    hop: 1,
    redacted: 0,
    to_all: false,
    at_ms: t0 - ago * MIN,
    recipients,
    ...over,
  });
  const thread = (id: string, subject: string, people: string[], count: number, ago: number, last: [string, string], hop = 1) => ({
    id,
    pool_id: POOL,
    pool_name: "Checkout team",
    subject,
    people,
    count,
    hop,
    hop_cap: 6,
    state: "open",
    state_reason: null,
    updated_at_ms: t0 - ago * MIN,
    last_from: last[0],
    last_line: last[1],
    unread: 0,
  });
  const messages = {
    "thr-totals": [
      msg("m-1", "thr-totals", [PLAN, "Planner"], "Round tax per line", "Totals are off by a cent on three-item carts. Round tax per line, not on the total.", 42, [to(CART, "Fix the cart totals", "delivered")]),
      msg("m-2", "thr-totals", [CART, "Fix the cart totals"], "Round tax per line", "Done: tax rounds per line; the three-item test passes.", 18, [to(PLAN, "Planner", "read")]),
      msg("m-3", "thr-totals", [PLAN, "Planner"], "Round tax per line", "Good. Do the shipping line the same way.", 3, [to(CART, "Fix the cart totals", "unread", true, "Goes in at its next turn")]),
    ],
    "thr-copy": [msg("m-copy", "thr-copy", [COPY, "Checkout copy pass"], "Button copy", "Can I rename “Place order” to “Pay now”? It tested better.", 6, [to(PLAN, "Planner", "held")])],
    "thr-review": [
      msg("m-r1", "thr-review", [REVIEW, "Review the checkout steps"], "Steps share one test", "The address and shipping steps share one test; split it?", 30, [to(CART, "Fix the cart totals", "read")]),
      msg("m-hop", "thr-review", [CART, "Fix the cart totals"], "Steps share one test", "Split. Can you take the shipping half?", 9, [to(REVIEW, "Review the checkout steps", "unread")], { hop: 5 }),
    ],
    "thr-tax": [msg("m-t1", "thr-tax", [PLAN, "Planner"], "EU VAT rules", "Find the VAT rules for digital goods in the EU.", 70, [to(TAX, "Tax rules research", "delivered")])],
  };
  return {
    __AIJ_FIXTURE__: { sessions: canvasSessions(t0), projects: canvasProjects(t0) },
    __AIJ_POOLS__: {
      pools: [{ id: POOL, name: "Checkout team", rules: [{ id: 1, pool_id: POOL, action: "claim_task", created_at: t0 - 26 * 60 * MIN, made_by: "user" }] }],
      members: [
        { session_id: PLAN, pool_id: POOL, role: "lead" },
        ...CANVAS_MEMBERS.slice(1).map((id) => ({ session_id: id, pool_id: POOL, role: "member" })),
      ],
    },
    __AIJ_POOL_ACTIVITY__: {
      [PLAN]: { said: "Splitting checkout into totals, copy and review", files: ["checkout/plan.md"] },
      [CART]: { said: "Rounding tax per line", files: ["cart/totals.ts", "cart/totals.test.ts"] },
    },
    __AIJ_AGENT_BRIDGE__: { asked: true, on: true },
    __AIJ_MESSAGES__: {
      threads: [
        thread("thr-totals", "Round tax per line", ["Planner", "Fix the cart totals"], 3, 3, ["Planner", "Good. Do the shipping line the same way."]),
        thread("thr-copy", "Button copy", ["Checkout copy pass", "Planner"], 1, 6, ["Checkout copy pass", "Can I rename “Place order” to “Pay now”?"]),
        thread("thr-review", "Steps share one test", ["Review the checkout steps", "Fix the cart totals"], 2, 9, ["Fix the cart totals", "Split. Can you take the shipping half?"], 5),
        thread("thr-tax", "EU VAT rules", ["Planner", "Tax rules research"], 1, 70, ["Planner", "Find the VAT rules for digital goods in the EU."]),
      ],
      messages,
      needs: [
        { key: "ask:ask-copy", kind: "ask", pool_id: POOL, pool_name: "Checkout team", thread_id: "thr-copy", message_id: "m-copy", session_id: null, ask_id: "ask-copy", offer_always: false, title: "Checkout copy pass wants to message Planner", detail: "«Button copy» · above what the pool lets agents do", preview: "Can I rename “Place order” to “Pay now”?", at_ms: t0 - 6 * MIN },
        { key: "ask:ask-hop", kind: "deliver", pool_id: POOL, pool_name: "Checkout team", thread_id: "thr-review", message_id: "m-hop", session_id: null, ask_id: "ask-hop", offer_always: false, title: "A reply would wake Review the checkout steps", detail: "«Steps share one test» · 5 agent messages in a row without you", preview: "Split. Can you take the shipping half?", at_ms: t0 - 9 * MIN },
      ],
    },
    __AIJ_POOL_CANVAS__: {
      [POOL]: {
        moving: 2,
        edges: [{ from_session_id: PLAN, to_session_id: COPY, messages: 0, tasks: 1, last_at_ms: t0 - 50 * MIN, thread_id: null, pending: [] }],
        stream: [
          { at_ms: t0 - 3 * MIN, words: "Planner wrote to Fix the cart totals: «Round tax per line»", source: "message", session_id: PLAN, other_session_id: CART },
          { at_ms: t0 - 6 * MIN, words: "Checkout copy pass asked to message Planner: it waits on you", source: "message", session_id: COPY, other_session_id: null },
          { at_ms: t0 - 9 * MIN, words: "Fix the cart totals wrote to Review the checkout steps: «Steps share one test»", source: "message", session_id: CART, other_session_id: REVIEW },
          { at_ms: t0 - 18 * MIN, words: "Fix the cart totals moved «Round tax per line»: In progress → In review", source: "task", session_id: CART, other_session_id: null },
          { at_ms: t0 - 50 * MIN, words: "Planner gave «Checkout copy» to Checkout copy pass", source: "task", session_id: PLAN, other_session_id: COPY },
        ],
      },
    },
  };
}
