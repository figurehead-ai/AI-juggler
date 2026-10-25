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
import type { AskInfo, RuntimeViewInfo } from "../lib/ipc/runtime";
import type { InstanceInfo } from "../lib/bridge";

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
    limits: [{ id: "seven_day", window_mins: 10080, used_pct: 41, resets_at_ms: 1_761_897_600_000 }],
    cut_off: null,
  },
  items: [{ kind: "user_message", text: "Run the tests", at_ms: 1 }],
  streaming: null,
  model: "gpt-5.5-codex",
});

beforeEach(() => {
  seams().__AIJ_RUNTIME__ = {};
  delete seams().__AIJ_RUNTIME_CALLS__;
});
afterEach(() => cleanup());

describe("an approval", () => {
  it("offers the tool's own options and carries the one clicked", async () => {
    const { getByTestId, getByText } = render(<AskCard ask={approval()} />);
    expect(getByText("npm test")).toBeTruthy();
    expect(getByTestId("ask-option-accept").textContent).toContain("Allow once");
    expect(getByTestId("ask-option-acceptWithExecpolicyAmendment").getAttribute("title")).toContain("Take it back in the tool");
    expect(getByTestId("ask-option-accept").className).toContain("from-accent-hi");
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
    expect(getAllByTestId("ask-card")[0].getAttribute("aria-label")).toContain("checkout");
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
    expect(getByTestId("runtime-limits").textContent).toContain("no 5-hour limit reported");
    expect(getByTestId("driven-items").textContent).toContain("Run the tests");
    await act(async () => fireEvent.click(getByTestId("open-in-terminal")));
    expect(calls()).toEqual(["handover:sess-1:terminal"]);
    expect(opened).toBe("sess-1");
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
