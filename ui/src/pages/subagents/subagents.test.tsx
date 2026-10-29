/**
 * AJ-E1520 (0.3) vitest — the Subagents views' words, the Usage tab's
 * line to them, a Processes row's "started by subagent", a project's
 * view, and the preview's rows (the backend's order and tree lines).
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/subagents";
import { forgetPaged } from "../../lib/paged";
import { previewList, type Seeded } from "../../lib/ipc/subagents-preview";
import { takeSubagentsAsk } from "../../lib/subagent-links";
import { NameCell } from "../../screens/processes/cells";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";
import { useHasSubagents } from "../session/useSubagents";
import { ProjectSubagents } from "./ProjectSubagents";
import { SubagentsShareLine } from "./ShareLine";
import { kindWords, modelWords, shareWords, tokenWords, treeLines } from "./words";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  forgetPaged();
  delete (window as unknown as { __AIJ_FIXTURE__?: unknown }).__AIJ_FIXTURE__;
});

const T0 = 1_792_600_000_000;
const MIN = 60_000;

const SEEDED: Seeded = {
  live: true,
  session_tokens: 1_000,
  items: [
    { id: "a1", task: "Audit the cart", status: "running", started_ms: T0 - 4 * MIN, own_tokens: 400, asked: "fix the totals", asked_ms: T0 - 5 * MIN, kind: "general-purpose" },
    { id: "n1", task: "Read the tax tables", parent: "a1", depth: 2, status: "done", started_ms: T0 - 3 * MIN, own_tokens: 100, kind: "Explore", files: ["/p/tax.ts"] },
    { id: "a2", task: "Write tests", status: "failed", started_ms: T0 - 2 * MIN, own_tokens: 300, asked: "fix the totals", asked_ms: T0 - 5 * MIN },
    { id: "a0", task: "Plan the epic", status: "done", started_ms: T0 - 60 * MIN, own_tokens: 50, asked: "plan it", asked_ms: T0 - 61 * MIN },
  ],
};

describe("AJ-E1520 (0.3) the Subagents words", () => {
  it("says types, models, counts, shares and tree lines plainly", () => {
    expect(kindWords("general-purpose")).toBe("General purpose");
    expect(kindWords("Explore")).toBe("Explore");
    expect(modelWords("claude-opus-4-1-20250805")).toBe("Opus 4.1");
    expect(modelWords("claude-haiku-4-5")).toBe("Haiku 4.5");
    expect(modelWords("gpt-5-codex")).toBe("GPT-5 Codex");
    expect(modelWords("kimi-k2")).toBe("kimi-k2");
    expect(tokenWords(412_000)).toBe("412k");
    expect(tokenWords(null)).toBe("—");
    expect(shareWords(41.2)).toBe("41%");
    expect(shareWords(0.62)).toBe("0.6%");
    expect(shareWords(0.01)).toBe("<0.1%");
    expect(treeLines([true, false], true)).toBe("│   └ ");
  });

  it("the preview answers as the backend: newest group open, a tree with its lines, a filter keeps the parent", () => {
    const all = previewList("c", { view: null, search: null, kind: null, sort: null, open: [], closed: [] }, { cursor: null, limit: 50 });
    (window as unknown as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = { subagents: { c: SEEDED } };
    const rows = previewList("c", { view: null, search: null, kind: null, sort: null, open: [], closed: [] }, { cursor: null, limit: 50 });
    expect(all.page.items).toEqual([]);
    expect(rows.page.items.map((r) => r.key)).toEqual(["g:p:fix the totals", "a:a2", "a:a1", "a:n1", "g:p:plan it"]);
    expect(rows.summary.share_pct).toBeCloseTo(45.95, 1);
    const a1 = rows.page.items[2]!;
    expect(a1.agent?.tokens).toBe(500);
    expect(rows.page.items[3]!.guides).toEqual([false]);
    const found = previewList("c", { view: null, search: "file:tax", kind: null, sort: null, open: [], closed: [] }, { cursor: null, limit: 50 });
    expect(found.page.items.map((r) => [r.key, r.context])).toEqual([
      ["g:p:fix the totals", false],
      ["a:a1", true],
      ["a:n1", false],
    ]);
  });
});

describe("AJ-E1520 (0.3) the links to the Subagents tab", () => {
  it("the Usage tab says the subagents' share and opens them sorted by most tokens", async () => {
    vi.spyOn(ipc, "subagentsList").mockResolvedValue({
      state: "listed",
      reason: null,
      page: { items: [], next_cursor: null, total: 0 },
      summary: { total: 437, running: 3, failed: 2, attention: 2, tokens: 9_000_000, share_pct: 41, counted: "in_session" },
      kinds: [],
      matched: 0,
      running: 3,
      reading: false,
      live: true,
      conversations_note: null,
      tool: "Claude Code",
    });
    const Page = () => {
      useHasSubagents("work", "u-share", false);
      return <SubagentsShareLine profileId="work" conversationId="u-share" />;
    };
    render(<Page />);
    const line = await screen.findByTestId("usage-subagents-line");
    expect(line.textContent).toBe("Subagents used 41% of these tokens·see which ›");
    const shown: string[] = [];
    const hear = (event: Event) => shown.push(String((event as CustomEvent<string>).detail));
    window.addEventListener("aij:page-tab", hear);
    fireEvent.click(screen.getByTestId("usage-subagents-open"));
    window.removeEventListener("aij:page-tab", hear);
    expect(shown).toEqual(["subagents"]);
    expect(takeSubagentsAsk()).toEqual({ sort: "tokens" });
  });

  it("a process a subagent probably started says so, and in a session's tab opens it", () => {
    const row = {
      key: "p:9:1",
      kind: "process",
      depth: 2,
      parent_key: "s:1",
      has_children: false,
      state: "idle",
      origin: "theirs",
      owner_words: "",
      cpu_pct: 1,
      rss_bytes: 1,
      ports: [],
      app: null,
      session: null,
      rollup: null,
      session_key: "s:1",
      process: { pid: 9, start_time: 1, name: "node (vite)", role: "dev_server", role_words: "dev server", role_sure: false, label: "Started by Bash: npm run dev", subagent_id: "a1", subagent_title: "Audit the cart", spark: [], peak_rss: 1, age_ms: 1, cpu_time_ms: 1, waits_for_pid: null, waits_for_name: null, stop: "observed", stop_reason: null, mark: null, mark_until_ms: null, heavy: false, failed: null, internal: false },
    } as unknown as ProcessGraphRowInfo;
    const open = vi.fn();
    const view = render(<NameCell row={row} open onToggle={() => undefined} onSubagent={open} />);
    const link = screen.getByTestId("process-subagent");
    expect(link.textContent).toBe("probably started by subagent ‘Audit the cart’ ›");
    fireEvent.click(link);
    expect(open).toHaveBeenCalledWith("a1");
    view.rerender(<NameCell row={row} open onToggle={() => undefined} />);
    expect(screen.getByTestId("process-subagent").tagName).toBe("SPAN");
  });
});

describe("AJ-E1520 (0.3) a project's subagents", () => {
  it("is the same view over its sessions, each group saying its session", async () => {
    (window as unknown as { __AIJ_FIXTURE__: unknown }).__AIJ_FIXTURE__ = {
      subagents: { c: { ...SEEDED, items: SEEDED.items.map((item) => ({ ...item, session: { account_id: "work", conversation_id: "c", title: "Fix the cart totals" } })) } },
      projectSubagents: { p1: ["c"] },
    };
    const project = vi.spyOn(ipc, "subagentsProject");
    render(<ProjectSubagents projectId="p1" now={T0} />);
    expect((await screen.findAllByTestId("subagent-group"))[0]!.textContent).toContain("in Fix the cart totals");
    expect(project).toHaveBeenCalled();
    expect(screen.getByTestId("subagents-summary").textContent).toContain("of these sessions' tokens");
  });
});
