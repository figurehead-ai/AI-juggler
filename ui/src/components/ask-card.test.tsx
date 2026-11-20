/**
 * AJ-E2307 vitest — answering a driven session's asks in AI Juggler.
 *
 * - An approval shows the tool's own options in its own words, the first
 *   "Allow once" as the primary; a click carries that option, nothing else.
 * - The keyboard answers: `2` picks the second option, `Enter` the first.
 * - A question takes the tool's choices and a reply; Send stays off until
 *   every part has an answer.
 * - A terminal session's ask (from its hooks) has no options: answered in
 *   its terminal.
 * - The batch view lists every open ask; nothing is answered without a click.
 * - A driven session's page says how it runs, its limits by window length
 *   (no 5-hour window made up), and offers Open in terminal.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { AskBatch, AskCard } from "./AskCard";
import { DrivenPanel } from "../pages/session/DrivenPanel";
import { ActionHostProvider } from "./ItemMenu";
import { instance } from "../test/fixtures";
import type { AskInfo, RuntimeViewInfo } from "../lib/ipc/runtime";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { forgetStops } from "../lib/turn-words";

type Seams = { __AIJ_RUNTIME__?: Record<string, RuntimeViewInfo>; __AIJ_RUNTIME_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_RUNTIME_CALLS__ ?? [];

const approval = (over: Partial<AskInfo> = {}): AskInfo => ({
  id: "0",
  session_id: "sess-1",
  kind: "approval",
  approval_kind: "command",
  summary: "npm test",
  reason: "run the test suite",
  options: [
    { id: "accept", label: "Allow once", effect: "allow_once", rule: null },
    { id: "acceptWithExecpolicyAmendment", label: "Always allow `npm test`", effect: "allow_rule", rule: "commands starting with `npm test`" },
    { id: "decline", label: "Deny", effect: "deny", rule: null },
    { id: "cancel", label: "Deny and stop", effect: "deny_and_stop", rule: null },
  ],
  parts: [],
  asked_at_ms: 1,
  ...over,
});

const question: AskInfo = {
  id: "q",
  session_id: "sess-1",
  kind: "question",
  approval_kind: null,
  summary: "Which database?",
  reason: null,
  options: [],
  parts: [
    {
      id: "db",
      header: "Database",
      question: "Which database?",
      choices: [
        { label: "Postgres", description: "server" },
        { label: "SQLite", description: "file" },
      ],
      multi: false,
      free_text: true,
      secret: false,
    },
  ],
  asked_at_ms: 2,
};

const view = (asks: AskInfo[]): RuntimeViewInfo => ({
  session_id: "sess-1",
  runtime: {
    kind: "driven",
    label: "Codex's app-server",
    beta: false,
    state: asks.length ? "asks_you" : "idle",
    asks,
    can_steer: false,
    fell_back: null,
    error: null,
    limits: [
      { id: "seven_day", window_mins: 10080, used_pct: 41, resets_at_ms: 1_761_897_600_000, model: null },
      { id: "seven_day_opus", window_mins: 10080, used_pct: 64, resets_at_ms: null, model: "Opus" },
    ],
    cut_off: null,
  },
  items: [{ kind: "user_message", text: "Run the tests", at_ms: 1 }],
  streaming: null,
  model: "gpt-5.5-codex",
  // A conversation continued here: its tool has written it.
  written: true,
});

beforeEach(() => {
  seams().__AIJ_RUNTIME__ = {};
  delete seams().__AIJ_RUNTIME_CALLS__;
  forgetStops();
});
afterEach(() => cleanup());

describe("an approval", () => {
  it("offers the tool's own options and carries the one clicked", async () => {
    const { getByTestId, getByText } = render(<AskCard ask={approval()} />);
    expect(getByText("npm test")).toBeTruthy();
    expect(getByTestId("ask-option-accept").textContent).toContain("Allow once");
    expect(getByTestId("ask-option-acceptWithExecpolicyAmendment").getAttribute("title")).toContain("Take it back in the tool");
    expect(getByTestId("ask-option-accept").className).toContain("bg-accent-fill");
    expect(getByTestId("ask-option-cancel").className).toContain("text-danger");
    await act(async () => fireEvent.click(getByTestId("ask-option-decline")));
    expect(calls()).toEqual(["answer:sess-1:0:decline"]);
  });

  it("answers from the keyboard", async () => {
    const first = render(<AskCard ask={approval()} />);
    await act(async () => fireEvent.keyDown(first.getByTestId("ask-card"), { key: "2" }));
    expect(calls()).toEqual(["answer:sess-1:0:acceptWithExecpolicyAmendment"]);
    first.unmount();
    const again = render(<AskCard ask={approval({ id: "1" })} />);
    await act(async () => fireEvent.keyDown(again.getByTestId("ask-card"), { key: "Enter" }));
    expect(calls()).toEqual(["answer:sess-1:0:acceptWithExecpolicyAmendment", "answer:sess-1:1:accept"]);
  });

  it("sends nothing without a click", async () => {
    render(<AskCard ask={approval()} />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(calls()).toEqual([]);
  });

  it("from a terminal session's hooks, is answered in its terminal", () => {
    const { getByTestId, queryByTestId } = render(<AskCard ask={approval({ options: [] })} />);
    expect(getByTestId("ask-in-terminal").textContent).toContain("in its terminal");
    expect(queryByTestId("ask-option-accept")).toBeNull();
  });
});

describe("a question", () => {
  it("takes the tool's choice or a reply, once every part has one", async () => {
    const { getByTestId, getByText, getByLabelText } = render(<AskCard ask={question} />);
    expect((getByTestId("ask-reply") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(getByText("Postgres"));
    expect((getByTestId("ask-reply") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.change(getByLabelText("Or your own answer to: Which database?"), { target: { value: "  " } });
    await act(async () => fireEvent.click(getByTestId("ask-reply")));
    expect(calls()).toEqual(["answer:sess-1:q:reply"]);
  });
});

describe("the batch view", () => {
  it("lists every open ask of the driven sessions together", async () => {
    seams().__AIJ_RUNTIME__ = {
      "sess-1": view([approval()]),
      "sess-2": { ...view([{ ...question, session_id: "sess-2" }]), session_id: "sess-2" },
    };
    const { findByTestId, getAllByTestId } = render(<AskBatch titleOf={(id) => (id === "sess-1" ? "checkout" : "api")} />);
    const batch = await findByTestId("ask-batch");
    expect(batch.textContent).toContain("2 questions from your sessions");
    expect(getAllByTestId("ask-card")).toHaveLength(2);
    expect(getAllByTestId("ask-card")[0]!.getAttribute("aria-label")).toContain("checkout");
    expect(calls()).toEqual([]);
  });
});

describe("a driven session's page", () => {
  it("says how it runs, its limits honestly, and hands over to the terminal", async () => {
    seams().__AIJ_RUNTIME__ = { "sess-1": view([]) };
    const session = { id: "sess-1", runtime: view([]).runtime } as unknown as InstanceInfo;
    let opened: string | null = null;
    const { findByTestId, getByTestId } = render(<DrivenPanel session={session} onOpenSession={(id) => (opened = id)} />);
    await waitFor(async () => expect((await findByTestId("runtime-line")).textContent).toContain("Codex's app-server"));
    expect(getByTestId("runtime-limits").textContent).toContain("Weekly: 41% used");
    // AJ-E2311 item 8: a week on one model is named by its model.
    expect(getByTestId("runtime-limits").textContent).toContain("Weekly (Opus): 64% used");
    expect(getByTestId("runtime-limits").textContent).toContain("no 5-hour limit reported");
    expect(getByTestId("driven-items").textContent).toContain("Run the tests");
    await act(async () => fireEvent.click(getByTestId("open-in-terminal")));
    expect(calls()).toEqual(["handover:sess-1:terminal"]);
    expect(opened).toBe("sess-1");
  });

  // The RC walk: continued here (Run in AI Juggler), the page was blank;
  // with no turns held yet it points at the conversation's earlier ones.
  it("points at the earlier turns when none are held yet", async () => {
    seams().__AIJ_RUNTIME__ = { "sess-1": { ...view([]), items: [] } };
    const session = instance({ id: "sess-1", account_id: "acc-1", conversation_id: "conv-1", project_id: null, runtime: view([]).runtime });
    const opened: string[] = [];
    const { findByTestId } = render(
      <ActionHostProvider host={{ openConversationById: (account, id) => opened.push(`${account}/${id}`) }}>
        <DrivenPanel session={session} />
      </ActionHostProvider>,
    );
    const earlier = await findByTestId("driven-earlier");
    expect(earlier.textContent).toContain("Earlier turns are in its conversation.");
    fireEvent.click(earlier.querySelector("button")!);
    expect(opened).toEqual(["acc-1/conv-1"]);
  });

  // Lab 2: in a folder whose trust isn't answered yet, Open in terminal
  // asks the tool's own question first (its terminal would save the answer
  // to the tool's config).
  it("asks the folder's trust before Open in terminal, then hands over", async () => {
    seams().__AIJ_RUNTIME__ = { "sess-1": view([]) };
    (window as unknown as { __AIJ_TRUST__?: unknown }).__AIJ_TRUST__ = {
      folder: "/Users/me/lab", question: "Trust this folder?", trust: "Trust and continue", restrict: "Open restricted", note: "", choice: null,
    };
    try {
      const session = { id: "sess-1", launch_recipe_id: "p1", cwd: "/Users/me/lab", runtime: view([]).runtime } as unknown as InstanceInfo;
      const { findByTestId, getByTestId } = render(<DrivenPanel session={session} />);
      await act(async () => fireEvent.click(await findByTestId("open-in-terminal")));
      expect(getByTestId("folder-trust").textContent).toContain("Trust this folder?");
      expect(calls()).toEqual([]);
      await act(async () => fireEvent.click(getByTestId("folder-trust-restrict")));
      expect(calls()).toEqual(["trust:p1:untrusted", "handover:sess-1:terminal"]);
    } finally {
      delete (window as unknown as { __AIJ_TRUST__?: unknown }).__AIJ_TRUST__;
    }
  });

  // AJ-E2310: AI Juggler stopped during its turn.
  it("whose turn was cut off says so, sends nothing, and offers Resume and End", async () => {
    const runtime = { ...view([]).runtime, state: null, limits: [], cut_off: "Its turn was cut off when AI Juggler stopped" };
    const session = { id: "sess-1", runtime } as unknown as InstanceInfo;
    let opened: string | null = null;
    const { getByTestId, queryByTestId, queryByRole } = render(<DrivenPanel session={session} onOpenSession={(id) => (opened = id)} />);
    expect(getByTestId("runtime-cut-off").textContent).toContain("Its turn was cut off when AI Juggler stopped");
    expect(queryByTestId("open-in-terminal")).toBeNull();
    expect(queryByRole("textbox")).toBeNull();
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(calls()).toEqual([]);
    await act(async () => fireEvent.click(getByTestId("runtime-resume")));
    expect(calls()).toEqual(["resume:sess-1"]);
    expect(opened).toBe("sess-1");
    await act(async () => fireEvent.click(getByTestId("runtime-end")));
    expect(calls()).toEqual(["resume:sess-1", "end:sess-1"]);
  });
});

// F-rc3f-5 (0.4.6 rc3f): after "Deny and stop" on a session run without a
// terminal, the conversation said "NOTE error_during_execution" and the
// line under the message box kept saying "error_during_execution" after
// the next turn had gone well. A turn the user stopped says so in plain
// words, a tool's other bare codes too, and the line goes once a new
// turn runs or answers.
describe("a turn that didn't finish, in words", () => {
  // AJ-E2025: the tool's code stays the note's raw text; its adapter's
  // neutral kind says why, and only the kind is read.
  const STOPPED = "error_during_execution";
  const claudeAsk = approval({
    options: [
      { id: "allow", label: "Allow once", effect: "allow_once", rule: null },
      { id: "deny", label: "Deny", effect: "deny", rule: null },
      { id: "deny_stop", label: "Deny and stop", effect: "deny_and_stop", rule: null },
    ],
  });
  const after = (over: Partial<RuntimeViewInfo["runtime"]>, items: RuntimeViewInfo["items"]): RuntimeViewInfo => {
    const base = view([]);
    return { ...base, runtime: { ...base.runtime, limits: [], ...over }, items };
  };
  const panel = (id = "sess-1") => {
    const session = instance({ id, conversation_id: null, runtime: seams().__AIJ_RUNTIME__![id]!.runtime });
    return render(<DrivenPanel session={session} />);
  };

  it("after Deny and stop: \"Stopped: you chose Deny and stop\", never the tool's code; the line goes once the next turn answers", async () => {
    seams().__AIJ_RUNTIME__ = { "sess-1": view([claudeAsk]) };
    const asking = panel();
    await act(async () => fireEvent.click(await asking.findByTestId("ask-option-deny_stop")));
    expect(calls()).toEqual(["answer:sess-1:0:deny_stop"]);
    asking.unmount();
    // The tool ends the turn: its error, and a note in the conversation.
    const stoppedAt = Date.now() + 50;
    const note = { kind: "note", text: STOPPED, at_ms: stoppedAt, turn_end: "stopped" };
    seams().__AIJ_RUNTIME__ = { "sess-1": after({ state: "idle", error: STOPPED, error_end: "stopped" }, [{ kind: "user_message", text: "Run the tests", at_ms: 1 }, note]) };
    const stopped = panel();
    await waitFor(() => expect(stopped.getByTestId("driven-items").textContent).toContain("Stopped: you chose Deny and stop"));
    expect(stopped.getByTestId("runtime-error").textContent).toBe("Stopped: you chose Deny and stop");
    expect(stopped.getByTestId("driven-panel").textContent).not.toContain(STOPPED);
    stopped.unmount();
    // The next turn runs: the line goes (the tool's last error stays set).
    seams().__AIJ_RUNTIME__ = { "sess-1": after({ state: "working", error: STOPPED, error_end: "stopped" }, [note]) };
    const running = panel();
    await waitFor(() => expect(running.getByTestId("driven-items").textContent).toContain("Stopped: you chose Deny and stop"));
    expect(running.queryByTestId("runtime-error")).toBeNull();
    running.unmount();
    // ...and it answered: still gone, the conversation keeps the note in words.
    seams().__AIJ_RUNTIME__ = { "sess-1": after({ state: "idle", error: STOPPED, error_end: "stopped" }, [note, { kind: "agent_message", text: "All tests pass.", at_ms: stoppedAt + 5_000 }]) };
    const answered = panel();
    await waitFor(() => expect(answered.getByTestId("driven-items").textContent).toContain("All tests pass."));
    expect(answered.queryByTestId("runtime-error")).toBeNull();
    expect(answered.getByTestId("driven-panel").textContent).not.toContain(STOPPED);
  });

  it("an Interrupt says so; a stop it didn't see, and the tool's other codes, in words too", async () => {
    seams().__AIJ_RUNTIME__ = { "sess-1": after({ state: "working" }, []) };
    const working = panel();
    await act(async () => fireEvent.click(await working.findByRole("button", { name: "Interrupt" })));
    working.unmount();
    seams().__AIJ_RUNTIME__ = { "sess-1": after({ state: "idle", error: STOPPED, error_end: "stopped" }, [{ kind: "note", text: STOPPED, at_ms: Date.now() + 50, turn_end: "stopped" }]) };
    const interrupted = panel();
    await waitFor(() => expect(interrupted.getByTestId("runtime-error").textContent).toBe("Stopped: you interrupted it"));
    interrupted.unmount();
    const words: [string | undefined, string, string][] = [
      ["stopped", STOPPED, "Stopped before it finished"],
      ["turn_limit", "error_max_turns", "Stopped: it reached the most turns it may take"],
      ["spend_limit", "error_max_budget_usd", "Stopped: it reached its spending limit"],
      ["answer_form", "any_code", "Stopped: it couldn't give its answer in the form asked for"],
      ["failed", "error_some_new_code", "Stopped with an error"],
      // No kind: a bare code still never shows; a tool's sentence stays.
      [undefined, "error_some_new_code", "Stopped with an error"],
      [undefined, "Credit balance is too low", "Credit balance is too low"],
    ];
    for (const [end, code, said] of words) {
      seams().__AIJ_RUNTIME__ = { "sess-2": { ...after({ state: "idle", error: code, error_end: end }, [{ kind: "note", text: code, at_ms: 5, turn_end: end }]), session_id: "sess-2" } };
      const other = panel("sess-2");
      await waitFor(() => expect(other.getByTestId("runtime-error").textContent).toBe(said));
      expect(other.getByTestId("driven-items").textContent).toContain(said);
      other.unmount();
    }
    // Before its conversation is read, the error's own kind says it.
    seams().__AIJ_RUNTIME__ = { "sess-3": { ...after({ state: "idle", error: STOPPED, error_end: "stopped" }, []), session_id: "sess-3" } };
    const unread = panel("sess-3");
    await waitFor(() => expect(unread.getByTestId("runtime-error").textContent).toBe("Stopped before it finished"));
    unread.unmount();
  });

  // AJ-E2025: a tool's codes are its adapter's, never the UI's.
  it("turn-words.ts names no tool's code", async () => {
    const source = (await import("../lib/turn-words.ts?raw")).default;
    for (const code of [/error_during_execution/, /error_max_\w+/, /\berror_[a-z]+_[a-z_]+/]) expect(source).not.toMatch(code);
  });
});
