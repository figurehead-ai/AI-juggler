import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AlertEventData, InstanceInfo, PreflightInfoData } from "./lib/bridge";
import { errorText } from "./lib/bridge";

/**
 * D-25 (AJ-E1025) guard, from the 2026-10-27 spec gap audit: internal
 * tokens never reach what people read. Each surface the audit caught is
 * rendered with data that carries the internal ids (a tool id, a window
 * id, a detector id, a tier tag), and its text must say none of them.
 * The source-level twin is `scripts/plain_words_lint.py` (just
 * terminology).
 */

const INTERNAL = [
  "five_hour",
  "seven_day",
  "claude-code",
  "sk-key",
  "env-value",
  "(Measured)",
  "(Computed)",
  "(estimate)",
  "Ready home",
  "waiting_user",
  "quota_blocked",
  "Error:",
];
const WORDS = [/\bpacks?\b/i, /redact/i];

function leaks(text: string): string[] {
  return [...INTERNAL.filter((token) => text.includes(token)), ...WORDS.filter((word) => word.test(text)).map(String)];
}

vi.mock("./lib/bridge", async (original) => {
  const real = await original<typeof import("./lib/bridge")>();
  return {
    ...real,
    overnightSummary: async () => ({
      since_ms: 0,
      generated_ms: 0,
      leftovers: 0,
      counts: { finished: 1 },
      rows: [
        { session_id: "s-1", account_label: "Mia", provider: "claude-code", project: "app", group: "finished", status: "finished", words: "Finished", transitions: [], tokens: 12, leftovers: 0 },
      ],
    }),
    overnightSummarySeen: async () => undefined,
  };
});

afterEach(cleanup);

const LIVE = { id: "s-1", account_id: "acc-mia", status: "running", started_at_ms: 1, last_activity_ms: 2 } as unknown as InstanceInfo;

describe("nothing internal on screen (D-25)", () => {
  it("a quota alert: the account, what's left, the window and until when", async () => {
    const { AlertLines } = await import("./components/AlertLines");
    const resets = Date.now() + 70 * 60_000;
    const event: AlertEventData = {
      kind: "actionable",
      account: "acc-mia",
      text: "Mia has 7% left in its 5-hour window, until it resets in 1 h 10 min",
      at_ms: 1,
      quota: { account_label: "Mia", window: "five_hour", left_pct: 7, minutes_left: 40, resets_at_ms: resets },
    };
    const view = render(<AlertLines events={[event]} sessions={[LIVE]} onHandOver={() => {}} />);
    const text = view.container.textContent ?? "";
    expect(leaks(text)).toEqual([]);
    expect(text).toContain("Mia has 7% left in its 5-hour window, until ");
    expect(view.getByTestId("alert-hand-over").textContent).toBe("Hand over");
    expect(view.getByTestId("alert-queue").textContent).toBe("Queue for the reset");
  });

  it("the overnight summary names the tool, never its id", async () => {
    const { MorningSummary } = await import("./components/MorningSummary");
    const view = render(<MorningSummary onOpenSession={() => {}} />);
    await waitFor(() => expect(view.getByTestId("summary-row")).toBeTruthy());
    expect(leaks(view.container.textContent ?? "")).toEqual([]);
  });

  it("the switch's keys question says what it found in words", async () => {
    const { SwitchFlow } = await import("./components/SwitchFlow");
    const preflight: Partial<PreflightInfoData> = {
      mechanism: "replay",
      door: "wizard",
      wizard_steps: ["Target", "What's carried", "Confirm"],
      wizard_reasons: [],
      redaction: { global: null, override: null, effective: null, source: "unset", line: "" },
      redaction_items: [
        { kind: "sk-key", preview: "s•••a91f", segment: 1 },
        { kind: "env-value", preview: "D•••word", segment: 2 },
      ],
      secrets: { asks: true, reader: "Codex (OpenAI)" },
    };
    (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__ = preflight;
    const view = render(<SwitchFlow sessionId="s-1" targetProfileId="p-1" targetLabel="Personal" onClose={() => {}} onSwitched={() => {}} />);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    expect(leaks(view.container.textContent ?? "")).toEqual([]);
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-keys")).toBeTruthy());
    const text = view.container.textContent ?? "";
    expect(leaks(text)).toEqual([]);
    expect(text).toContain("an API key (s•••a91f) and a secret in an environment value (D•••word)");
    delete (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__;
  });

  it("an error a page shows is words, never the program's", () => {
    expect(errorText(new Error("No handover chosen"))).toBe("No handover chosen");
    expect(errorText({ code: "not_found", retryable: false, user_message: "That account isn't here any more." })).toBe("That account isn't here any more.");
    expect(leaks(errorText(new Error("x")))).toEqual([]);
  });

  it("the guard itself catches each token", () => {
    expect(leaks("five_hour window (Measured) claude-code sk-key pack")).toHaveLength(5);
  });
});
