/**
 * 0.4.9 (AJ-E2312 items 1 and 2, the PM5 walk): one state per session,
 * read the same in its page's header, the rail and Home.
 *
 * - Past its reset, a session that stopped at its limit said "Limit
 *   reached · 4 min" in its header; the backend now says its limit has
 *   reset (`limit_reset`), and every view reads that one word: "Room
 *   again", with its way on (Resume the same conversation).
 * - SK-20: a parked one read "Asks you" in the rail by App (Needs you
 *   left it out, its state stayed) while its header said "Limit reached":
 *   now "Parked …" in both, as its rail row always said.
 * - Item 2: the Parked card's words broke one per line beside its button.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ParkCard, PARK_WORDS } from "./ParkCard";
import { itemsOf, sessionState } from "../lib/rail-items";
import { rowState } from "./row/SessionRow";
import { SessionChips } from "../pages/session/SessionHead";
import { fromInstance } from "../pages/session/model";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const NOW = 1_800_000_000_000;
const MIN = 60_000;

afterEach(cleanup);
beforeEach(() => {
  Reflect.deleteProperty(window, "__AIJ_SESSION_RESUME__");
});

function own(over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "writer", launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "cc-work", account_label: "Work", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "quota_blocked", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: "quota_blocked",
    attention: { rule: "quota_exit", inputs: [], since_ms: NOW - 10 * MIN }, started_at_ms: NOW - 40 * MIN,
    last_activity_ms: NOW - 10 * MIN, machine_id: "this-mac", cwd: "/Users/you/lab/web", kind: "terminal", isolation_label: null, project_id: null,
    conversation_id: "c0c0c0c0-0000-4000-8000-000000000001", ...over,
  };
}

/** Its limit reset four minutes ago (the backend's word, `limits::RESET_RULE`). */
const reset = own({ needs_attention: "limit_reset", attention: { rule: "limit_reset", inputs: ["its limit has reset"], since_ms: NOW - 4 * MIN } });
/** Parked until Work's reset, which passed a minute ago. */
const parked = own({ parked: "measured", parked_resets_at_ms: NOW - MIN });

/** The header's state chip, as its page draws it. */
function header(row: InstanceInfo): string {
  const view = render(<SessionChips model={fromInstance(row)} account={undefined} now={NOW} />);
  const words = view.getByTestId("session-state-chip").textContent ?? "";
  view.unmount();
  return words;
}

/** The rail by App's row word: Needs you is the backend's (D-49), which
 * leaves a parked one out. */
function railWord(row: InstanceInfo, need: "limit" | "resume" | null): string {
  const needOf = new Map(need ? [[`s:${row.id}`, need] as const] : []);
  const [item] = itemsOf({ sessions: [row], conversations: [], projects: [], seen: {}, needOf });
  return rowState(item!, NOW).word;
}

describe("past its reset, nothing says it is at its limit", () => {
  it("one that stopped at its limit reads Room again in its header, the rail and Needs you", () => {
    expect(header(own())).toBe("Limit reached · 10 min");
    expect(header(reset)).toBe("Room again · 4 min");
    expect(sessionState(reset).word).toBe("Room again");
    expect(railWord(reset, "resume")).toBe("Room again");
    for (const said of [header(reset), sessionState(reset).word, railWord(reset, "resume")]) expect(said).not.toMatch(/limit reached/i);
  });

  it("its page offers the way on a parked one has: Resume the same conversation", async () => {
    const view = render(<ParkCard session={reset} now={NOW} />);
    const card = view.getByTestId("park-card");
    expect(card.textContent).toContain("Work has reset. Resume its work — the same conversation, on the same Account.");
    expect(card.textContent).not.toMatch(/Blocked by its quota|Park until reset/);
    await act(async () => {
      fireEvent.click(view.getByTestId("park-resume"));
    });
    expect(Reflect.get(window, "__AIJ_SESSION_RESUME__")).toEqual(["writer"]);
  });

  it("SK-20: a parked one reads as parked in its header and the rail, never Asks you or Limit reached", () => {
    expect(header(parked)).toBe("Parked · ready");
    expect(railWord(parked, null)).toBe("Parked · ready");
    expect(railWord(own({ parked: "measured", parked_resets_at_ms: NOW + 3 * MIN }), null)).toBe("Parked 3m");
  });
});

describe("item 2: the Parked card reads as a sentence beside its button", () => {
  it("its words keep a sentence's width, so a narrow card puts the button under them", () => {
    // jsdom draws nothing: the contract is the words' flex basis. A basis
    // of 0 (`flex-1` alone) let a wrapping row squeeze them to a word a
    // line beside "Resume the same conversation" (1024 × 680, the PM5 walk).
    const basisRem = (classes: string) => {
      const basis = /(?:^|\s)basis-(\d+)(?:\s|$)/.exec(classes);
      return basis ? Number(basis[1]) / 4 : 0;
    };
    for (const row of [parked, reset, own()]) {
      const view = render(<ParkCard session={row} now={NOW} />);
      const card = view.getByTestId("park-card");
      expect(card.className).toContain("flex-wrap");
      const words = card.querySelector("span")!;
      expect(words.className).toBe(PARK_WORDS);
      expect(basisRem(words.className)).toBeGreaterThanOrEqual(10);
      view.unmount();
    }
  });
});
