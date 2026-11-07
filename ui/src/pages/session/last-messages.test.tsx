/**
 * AJ-E2108 / 0.3.5 (ADR-0024) vitest — a session's conversation on its
 * page: followed only while its page is shown; your turns on the right,
 * the agent's on the left drawn as its own app draws them (the Claude
 * app's stretches, ChatGPT's Codex fold); changes land in place under
 * stable ids (rows and what the reader opened stay); a problem after
 * turns were shown is a line over them, never the empty card; earlier
 * turns on request; the question it asks at the bottom with Go to it;
 * off in Settings says so; a paired Mac's session is never read here.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/conversations";
import type { ConversationDeltaInfo, ConversationSnapshotInfo, TurnBlockInfo, TurnInfo } from "../../lib/ipc/conversations";
import { resetConversationStore } from "../../lib/conversation-store";
import { blockOf } from "../../preview/conversation-feed";
import { LastMessages, questionOf } from "./LastMessages";
import { foldOf, stepsSummary, workWords } from "./TurnWork";
import type { SessionModel } from "./model";

afterEach(() => {
  cleanup();
  resetConversationStore();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

const model = (overrides: Partial<SessionModel> = {}, state: Partial<SessionModel["state"]> = {}): SessionModel => ({
  tool: "claude-code",
  profileId: "work",
  profileLabel: "Work",
  conversationId: "c1",
  sessionId: null,
  switchSource: "conversation:work:c1",
  live: true,
  state: { words: "Asks you", tone: "warning", asks: true, working: false, since: 0, ...state },
  places: [{ app: "Claude", shown: "Claude app", app_id: "claude-app", profile: "Work", instance: "Work", pid: 42, kind: "desktop", app_pid: 41, label: "Claude app · Work" }],
  viewedMs: null,
  cwd: "/w/app",
  model: null,
  ...overrides,
});

const block = (kind: string, extra: Partial<TurnBlockInfo> = {}): TurnBlockInfo => blockOf({ kind, ...extra });
const words = (text: string) => block("text", { text });

let offset = 0;
const turn = (speaker: string, blocks: TurnBlockInfo[], extra: Partial<TurnInfo> = {}): TurnInfo => {
  offset += 100;
  return { id: `7:${offset}`, revision: 1, offset, open: false, speaker, blocks, blocks_hidden: 0, at_ms: null, model: null, redacted: 0, ...extra };
};
const text = (speaker: string, value: string, at_ms: number | null = null) => turn(speaker, [words(value)], { at_ms });

const snapshot = (turns: TurnInfo[], extra: Partial<ConversationSnapshotInfo> = {}): ConversationSnapshotInfo => ({
  state: "shown",
  subscription: 1,
  generation: 7,
  revision: 1,
  layout: "stretches",
  turns,
  earlier: null,
  status: null,
  skipped: 0,
  ...extra,
});

/** A follow that answers `first` and lets the test send changes. */
function follow(first: ConversationSnapshotInfo) {
  let send: (delta: ConversationDeltaInfo) => void = () => undefined;
  const spy = vi.spyOn(ipc, "conversationsFollow").mockImplementation(async (_a, _c, _m, onDelta) => {
    send = onDelta;
    return first;
  });
  vi.spyOn(ipc, "conversationsUnfollow").mockResolvedValue(undefined);
  const delta = (extra: Partial<ConversationDeltaInfo>) =>
    act(async () => {
      send({ subscription: 1, generation: 7, revision: 2, kind: "append", upserts: [], dropped_before: null, earlier: null, layout: first.layout, status: null, skipped: 0, ...extra });
    });
  return { spy, delta };
}

const ask = text("you", "Use the key •••", Date.now() - 360_000);
const reply = turn(
  "assistant",
  [
    words("Ran the **tests**:\n\n```ts\nconst a = 1;\n```"),
    block("steps", { steps: ['ran "pnpm test"', "Read a.ts", "Edit b.ts"], title: "Ran a command, read a.ts, edited b.ts", title_running: "Editing b.ts" }),
    words("All green. Keep the old layout?"),
  ],
  { at_ms: Date.now() - 120_000, model: "claude-opus-4-5", redacted: 1 },
);

describe("AJ-E2108 / 0.3.5 the conversation", () => {
  it("is followed only while its page is shown: once, and let go when it isn't", async () => {
    const { spy } = follow(snapshot([ask, reply]));
    const unfollow = vi.mocked(ipc.conversationsUnfollow);
    const ended = model({ live: false }, { asks: false });
    const view = render(<LastMessages model={ended} visible={false} />);
    expect(spy).not.toHaveBeenCalled();
    view.rerender(<LastMessages model={ended} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0]!.slice(0, 3)).toEqual(["work", "c1", 30]);
    view.rerender(<LastMessages model={ended} visible={false} />);
    expect(unfollow).toHaveBeenCalledWith(1);
    // Shown again: at once from memory, and followed again behind.
    view.rerender(<LastMessages model={ended} visible />);
    expect(screen.getAllByTestId("last-turn")).toHaveLength(2);
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(2));
  });

  it("draws a Claude turn as the Claude app does: every text, each run of work one titled line", async () => {
    follow(snapshot([ask, reply]));
    render(<LastMessages model={model({}, { asks: false })} visible />);
    const turns = await screen.findAllByTestId("last-turn");
    expect(turns.map((one) => one.dataset.speaker)).toEqual(["you", "assistant"]);
    expect(turns[0]!.className).toContain("items-end");
    expect(turns[1]!.textContent).toContain("Claude");
    expect(turns[1]!.querySelector("time")?.textContent).toBe("2 min ago");
    // Both texts in full, in order; no whole-turn "Worked for" row.
    expect(turns[1]!.querySelector("strong")?.textContent).toBe("tests");
    expect(turns[1]!.textContent).toContain("All green. Keep the old layout?");
    expect(screen.queryByTestId("last-turn-work")).toBeNull();
    const fold = screen.getByTestId("last-turn-fold-toggle");
    expect(fold.textContent).toBe("Ran a command, read a.ts, edited b.ts");
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(fold);
    expect(screen.getByTestId("last-turn-step-lines").textContent).toContain('ran "pnpm test"');
    expect(screen.getByTestId("last-messages-note").textContent).toContain("not saved · 1 key taken out");
  });

  it("while a Claude turn works, its current run says what it's doing; narration and a message sent mid-turn show", async () => {
    const working = turn(
      "assistant",
      [block("narration", { text: "Checking the header first." }), words("On it."), block("steps", { steps: ['ran "make"'], title: "Ran the build", title_running: "Running the build" })],
      { open: true },
    );
    follow(snapshot([ask, text("assistant", "Started."), text("you", "also the footer"), working]));
    render(<LastMessages model={model({}, { asks: false, working: true })} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(4));
    expect(screen.getAllByTestId("last-turn")[2]!.dataset.speaker).toBe("you");
    expect(screen.getAllByTestId("last-turn")[3]!.textContent).toContain("Checking the header first.");
    expect(screen.getByTestId("last-turn-fold-toggle").textContent).toBe("Running the build");
  });

  it("draws a Codex turn as ChatGPT's Codex does: its work in one row, then its reply", async () => {
    follow(snapshot([ask, reply], { layout: "folded" }));
    render(<LastMessages model={model({ tool: "codex" }, { asks: false })} visible />);
    const turns = await screen.findAllByTestId("last-turn");
    expect(turns[1]!.textContent).toContain("All green. Keep the old layout?");
    const work = screen.getByTestId("last-turn-work-toggle");
    expect(work.textContent).toBe("Worked for 4 min · Ran 1 command, edited 1 file, read 1 file · 1 note");
    fireEvent.click(work);
    expect(turns[1]!.querySelector("[data-testid='md-code'] pre")?.textContent).toBe("const a = 1;");
  });

  it("a change lands in place: the rows it doesn't name stay, and what the reader opened stays open", async () => {
    const { delta } = follow(snapshot([ask, reply, text("you", "and the docs?")], { layout: "folded" }));
    render(<LastMessages model={model({ tool: "codex" }, { asks: false })} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(3));
    const rows = screen.getAllByTestId("last-turn");
    fireEvent.click(screen.getByTestId("last-turn-work-toggle"));
    const answer = turn("assistant", [words("Docs updated.")], { open: true });
    await delta({ upserts: [answer] });
    const after = screen.getAllByTestId("last-turn");
    expect(after).toHaveLength(4);
    expect(after.slice(0, 3)).toEqual(rows);
    expect(screen.getByTestId("last-turn-work-toggle").getAttribute("aria-expanded")).toBe("true");
    // The open turn grows under its id: the same row.
    await delta({ revision: 3, upserts: [{ ...answer, revision: 2, blocks: [words("Docs updated and linked.")] }] });
    expect(screen.getAllByTestId("last-turn")[3]).toBe(after[3]);
    expect(after[3]!.textContent).toContain("Docs updated and linked.");
  });

  it("the last turn keeps its row when it stops working", async () => {
    follow(snapshot([ask, reply], { layout: "folded" }));
    const view = render(<LastMessages model={model({ tool: "codex" }, { asks: false, working: true })} visible />);
    await waitFor(() => expect(screen.getByTestId("last-turn-live")).toBeTruthy());
    const row = screen.getAllByTestId("last-turn")[1];
    view.rerender(<LastMessages model={model({ tool: "codex" }, { asks: false, working: false })} visible />);
    expect(screen.getAllByTestId("last-turn")[1]).toBe(row);
    expect(screen.getByTestId("last-turn-work-toggle")).toBeTruthy();
  });

  it("a problem after turns were shown is a line over them, never the empty card", async () => {
    const { delta } = follow(snapshot([ask, reply]));
    render(<LastMessages model={model({}, { asks: false, working: true })} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    await delta({ kind: "state", status: "unreadable" });
    expect(screen.getByTestId("last-messages-banner").textContent).toBe("Couldn't read the latest changes. Trying again.");
    expect(screen.getAllByTestId("last-turn")).toHaveLength(2);
    expect(screen.queryByTestId("last-messages-none")).toBeNull();
    await delta({ revision: 3, kind: "state", status: null });
    expect(screen.queryByTestId("last-messages-banner")).toBeNull();
  });

  it("when it asks you, the question waits at the bottom with Go to it", async () => {
    follow(snapshot([ask, reply]));
    const onGoTo = vi.fn();
    render(<LastMessages model={model()} visible onGoTo={onGoTo} />);
    expect((await screen.findByTestId("last-messages-question")).textContent).toBe("All green. Keep the old layout?");
    expect(screen.getAllByTestId("last-turn")[1]!.dataset.asks).toBe("true");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("Answer it in the Claude app");
    fireEvent.click(screen.getByTestId("last-messages-go-to"));
    expect(onGoTo).toHaveBeenCalledTimes(1);
  });

  it("earlier turns load before the shown ones, from where they start", async () => {
    const latest = [text("you", "latest ask"), text("assistant", "latest answer")];
    follow(snapshot(latest, { earlier: latest[0]!.offset }));
    const older = [text("you", "older ask"), text("assistant", "older answer")].map((one, index) => ({ ...one, offset: index + 1, id: `7:${index + 1}` }));
    const read = vi.spyOn(ipc, "conversationsEarlier").mockResolvedValue({ state: "shown", generation: 7, turns: older, earlier: null });
    render(<LastMessages model={model({ live: false }, { asks: false })} visible />);
    fireEvent.click(await screen.findByTestId("last-messages-earlier"));
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(4));
    expect(read).toHaveBeenCalledWith(1, latest[0]!.offset, 30);
    expect(screen.getAllByTestId("last-turn").map((one) => one.textContent)).toEqual([
      expect.stringContaining("older ask"),
      expect.stringContaining("older answer"),
      expect.stringContaining("latest ask"),
      expect.stringContaining("latest answer"),
    ]);
    expect(screen.getByTestId("last-messages-start")).toBeTruthy();
  });

  it("reads on past a stretch with no turn instead of saying there is none", async () => {
    follow(snapshot([], { earlier: 5_000 }));
    const read = vi
      .spyOn(ipc, "conversationsEarlier")
      .mockResolvedValueOnce({ state: "shown", generation: 7, turns: [], earlier: 2_000 })
      .mockResolvedValueOnce({ state: "shown", generation: 7, turns: [text("you", "found it")], earlier: null });
    render(<LastMessages model={model()} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(1));
    expect(read.mock.calls.map((call) => call[1])).toEqual([5_000, 2_000]);
    expect(screen.queryByTestId("last-messages-none")).toBeNull();
  });

  it("off in Settings: says so, with the way to turn them on", async () => {
    follow(snapshot([], { state: "off", subscription: null }));
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-off")).textContent).toContain("Last messages are off");
    expect(screen.queryByTestId("last-turn")).toBeNull();
  });

  it("a chat with no transcript here shows its title only", async () => {
    follow(snapshot([], { state: "none", subscription: null }));
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-none")).textContent).toContain("its title is all");
  });

  it("a paired Mac's session is never read here", () => {
    const { spy } = follow(snapshot([]));
    render(<LastMessages model={model()} visible remoteMac="Mac mini" />);
    expect(screen.getByTestId("last-messages-remote").textContent).toBe("Open on Mac mini to read: its messages stay on that Mac.");
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("AJ-E2108 turns in words", () => {
  it("a run of calls reads as one line", () => {
    expect(stepsSummary(['ran "a"', 'ran "b"', "Read x.ts", "edited a.ts, b.ts", 'searched for "y"'])).toBe("Ran 2 commands, edited 2 files, read 1 file, searched once");
    expect(stepsSummary(["updated the task list"])).toBe("Updated the task list");
  });

  it("0.3.4: a turn ending on a step is all work; the question is the reply's, never a note's", () => {
    const step = block("steps", { steps: ['ran "pnpm test"'] });
    const note = words("Should I also bump the version?");
    const going = turn("assistant", [note, step]);
    expect(foldOf(going)).toEqual({ work: [note, step], reply: null });
    const done = { ...going, blocks: [note, step, words("Done: 42 passed.")] };
    expect(questionOf(done)).toBe("Done: 42 passed.");
    expect(workWords(foldOf(done).work, null)).toBe("Ran 1 command · 1 note");
  });

  it("the question is the last paragraph that asks one, in plain words", () => {
    const one = text("assistant", "Pushed.\n\nShould I **rebase onto main first**, or open it as it is?\n\n```sh\ngit log\n```");
    expect(questionOf(one)).toBe("Should I rebase onto main first, or open it as it is?");
    expect(questionOf(text("you", "hi?"))).toBeNull();
  });
});
