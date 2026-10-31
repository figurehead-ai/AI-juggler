import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstanceInfo, PreflightInfoData } from "./lib/bridge";
import { errorText } from "./lib/bridge";
import switchFailureWords from "./test/switch-failure-words.json";
import crossFailureWords from "./test/cross-switch-failure-words.json";

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
  "spawn",
  "S-F",
  "preflight",
  "harness",
  "os error",
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
  it("a quota banner: the account, what's left, the window and when it resets", async () => {
    // The alert pass feeds the notification model (D-49); its banner is
    // what shows, in the model's words and this Mac's local time.
    const { NotificationRegion } = await import("./components/NotificationRegion");
    const { resetNoticeStandIn } = await import("./lib/ipc/notices");
    const seams = window as never as { __AIJ_NOTICES__?: unknown };
    const target = { session_id: null, conversation_id: null, account_id: "acc-mia", project_id: null, page: null, task_id: null, pool_id: null };
    const facts = { window: "5-hour", resets_at_ms: Date.now() + 70 * 60_000, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Mia" };
    seams.__AIJ_NOTICES__ = {
      app: [{ id: 7, kind: "quota", needs: true, title: "Mia is almost out of room", detail: "7% of its 5-hour window left", project_id: null, project: null, target, facts, action: "hand-over", meter: 7, created_ms: Date.now(), unread: true, subject: "n:7", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false }],
    };
    resetNoticeStandIn();
    const view = render(<NotificationRegion sessions={[LIVE]} />);
    const banner = await view.findByTestId("quota-banner");
    const text = banner.textContent ?? "";
    expect(leaks(text)).toEqual([]);
    expect(text).toContain("Mia is almost out of room");
    expect(text).toMatch(/7% of its 5-hour window left · resets at .+ · 1 session uses it/);
    // F16: its session is open, so the button continues it elsewhere.
    expect(view.getByTestId("quota-banner-hand-over").textContent).toBe("Continue on another account…");
    // 0.3.2: the reminder names its time ("Queue for the reset" said nothing).
    expect(view.getByTestId("quota-banner-remind").textContent).toMatch(/^Remind me at .+$/);
    delete seams.__AIJ_NOTICES__;
    resetNoticeStandIn();
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

  // Every switch failure, with the words the backend sends
  // (`AppError::switch`, committed by error_words.rs's golden test): the
  // dialog names the account and never says the program's detail.
  it.each(switchFailureWords)("a failed switch ($kind) says it in plain words", async ({ words }) => {
    const { SwitchFlow } = await import("./components/SwitchFlow");
    const seams = window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown; __AIJ_SWITCH_EXECUTE_FAIL__?: unknown };
    seams.__AIJ_SWITCH_PREFLIGHT__ = {
      mechanism: "transplant",
      door: "one-click",
      wizard_steps: [],
      wizard_reasons: [],
      redaction_items: [],
      carry_items: [],
      secrets: { asks: false, reader: "Claude Code (Anthropic)" },
      target_tool: "claude-code",
    };
    seams.__AIJ_SWITCH_EXECUTE_FAIL__ = { code: "switch_failed", retryable: true, user_message: words };
    const view = render(<SwitchFlow sessionId="s-1" targetProfileId="p-1" targetLabel="Personal" onClose={() => {}} onSwitched={() => {}} />);
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    expect(view.getByTestId("switch-failed-words").textContent).toBe(words);
    // Said once: the dialog adds a title, never a second telling.
    expect((view.getByTestId("switch-failed").textContent ?? "").split(words).length - 1).toBe(1);
    expect(leaks(view.container.textContent ?? "")).toEqual([]);
    expect(words).toMatch(/Work|Personal/);
    delete seams.__AIJ_SWITCH_PREFLIGHT__;
    delete seams.__AIJ_SWITCH_EXECUTE_FAIL__;
  });

  // Every way a switch to another computer stops, with the words the
  // backend sends (crossmachine/failure.rs, committed by its golden
  // test): said once, under the title only, with exactly one next step
  // and the other computer named, never "this Mac".
  it.each(crossFailureWords)("a failed switch to another computer ($kind) says it once, with one next step", async ({ words }) => {
    const { CrossMachineSwitchFlow } = await import("./components/CrossMachineSwitchFlow");
    const seams = window as never as { __AIJ_FLEET_SWITCH_OUTCOME__?: unknown };
    seams.__AIJ_FLEET_SWITCH_OUTCOME__ = { accepted: false, reason_code: "made-up", message: words, instance_id: null, edge_id: null, divergence: null };
    const target = {
      machine_id: "m-1", machine_label: "Studio", recipe_id: "r-1", recipe_name: "Main", account_label: "Desktop Max", provider: "claude-code",
      mechanism: "transplant", readiness: "t2_ready", blockers: [], fidelity: "F3", badge: "Same conversation, new account · Not proven yet", validated: false,
      carried: [], regenerated: [], dropped: [], same_path_project: "p-1", projects: [{ project_id: "p-1", name: "shop", same_path: true, same_name: true }],
      replay_available: true, version_here: "2.1.0", version_there: "2.1.0",
    };
    const view = render(<CrossMachineSwitchFlow sessionId="s-1" target={target} onClose={() => {}} />);
    fireEvent.click(view.getByTestId("cross-switch-next"));
    fireEvent.click(view.getByTestId("cross-switch-next"));
    fireEvent.click(view.getByTestId("cross-switch-go"));
    await waitFor(() => expect(view.getByTestId("cross-switch-failed")).toBeTruthy());
    expect(view.getByTestId("cross-switch-failed-words").textContent).toBe(words);
    // Said once: the dialog adds its title and nothing else.
    expect(view.getByTestId("cross-switch-failed").textContent).toBe(`The switch didn't complete${words}`);
    // What happened, then exactly one next step.
    const sentences = words.split(/(?<=\.) /);
    expect(sentences).toHaveLength(2);
    expect(sentences[0]).toMatch(/, so /);
    expect(view.container.textContent ?? "").not.toMatch(/this Mac|either Mac/);
    expect(leaks(view.container.textContent ?? "")).toEqual([]);
    delete seams.__AIJ_FLEET_SWITCH_OUTCOME__;
  });

  it("a server that needs its token says what to do, never a bare status", async () => {
    const { SwitchSetupList } = await import("./components/SwitchSetupList");
    const { EXAMPLE_SETUP } = await import("./lib/ipc/switchSetup");
    const view = render(<SwitchSetupList check={EXAMPLE_SETUP} picked={new Set()} onToggle={() => {}} onCheckAgain={async () => ""} />);
    const text = view.container.textContent ?? "";
    expect(leaks(text)).toEqual([]);
    expect(text).not.toContain("in your environment: secrets are never copied");
    expect(text).toContain("You can switch now, then finish it: it needs your GITHUB_TOKEN");
    expect(view.getByTestId("switch-setup-secret-steps").textContent).toContain("without the value");
  });

  it("an error a page shows is words, never the program's", () => {
    expect(errorText(new Error("No handover chosen"))).toBe("No handover chosen");
    expect(errorText({ code: "not_found", retryable: false, user_message: "That account isn't here any more." })).toBe("That account isn't here any more.");
    expect(leaks(errorText(new Error("x")))).toEqual([]);
  });

  it("the guard itself catches each token", () => {
    expect(leaks("five_hour window (Measured) claude-code sk-key pack")).toHaveLength(5);
    expect(leaks("didn't start (terminal spawn failed)")).toEqual(["spawn"]);
  });
});
