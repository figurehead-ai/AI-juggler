/**
 * AJ-E2108 vitest — a session's conversation on its page: read only once
 * its page is shown, once per session; your turns on the right, the
 * agent's on the left with its name, markdown and folded tool calls;
 * earlier turns on request; followed along only while it runs and the
 * page is shown; the question it asks at the bottom with Go to it; off in
 * Settings says so; a paired Mac's session is never read here.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/conversations";
import type { LastMessagesInfo, LastTurnInfo } from "../../lib/ipc/conversations";
import { LastMessages, questionOf } from "./LastMessages";
import { foldOf, stepsSummary, workWords } from "./TurnWork";
import { FOLLOW_WORKING_MS } from "./useLastMessages";
import type { SessionModel } from "./model";

afterEach(() => {
  cleanup();
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

const text = (speaker: string, words: string, at_ms: number | null = null): LastTurnInfo => ({
  speaker,
  blocks: [{ kind: "text", text: words, steps: [], hidden: 0 }],
  at_ms,
  model: null,
});

const info = (turns: LastTurnInfo[], extra: Partial<LastMessagesInfo> = {}): LastMessagesInfo => ({ state: "shown", redacted: 0, turns, earlier: null, stamp: "s1", ...extra });

const shown = info(
  [
    text("you", "Use the key •••", Date.now() - 360_000),
    {
      speaker: "assistant",
      blocks: [
        { kind: "text", text: "Ran the **tests**:\n\n```ts\nconst a = 1;\n```", steps: [], hidden: 0 },
        { kind: "steps", text: "", steps: ['ran "pnpm test"', "Read a.ts", "Edit b.ts"], hidden: 0 },
        { kind: "text", text: "All green. Keep the old layout?", steps: [], hidden: 0 },
      ],
      at_ms: Date.now() - 120_000,
      model: "claude-opus-4-5",
    },
  ],
  { redacted: 1 },
);

describe("AJ-E2108 last messages", () => {
  it("are read only once the page is shown, and once", async () => {
    const read = vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    const ended = model({ live: false }, { asks: false });
    const view = render(<LastMessages model={ended} visible={false} />);
    expect(read).not.toHaveBeenCalled();
    view.rerender(<LastMessages model={ended} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    view.rerender(<LastMessages model={ended} visible={false} />);
    view.rerender(<LastMessages model={ended} visible />);
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith("work", "c1");
  });

  it("show your turns on the right and the agent's on the left, with its name, its work folded to one row (0.3.4: as the tools' own windows), then its reply", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    render(<LastMessages model={model({}, { asks: false })} visible />);
    const turns = await screen.findAllByTestId("last-turn");
    expect(turns.map((turn) => turn.dataset.speaker)).toEqual(["you", "assistant"]);
    expect(turns[0]!.className).toContain("items-end");
    expect(turns[1]!.textContent).toContain("Claude");
    expect(turns[1]!.querySelector("time")?.textContent).toBe("2 min ago");
    // The reply shows; the work before it is one row: how long, what it did.
    expect(turns[1]!.textContent).toContain("All green. Keep the old layout?");
    const work = screen.getByTestId("last-turn-work-toggle");
    expect(work.textContent).toBe("Worked for 4 min · Ran 1 command, edited 1 file, read 1 file · 1 note");
    expect(work.getAttribute("aria-expanded")).toBe("false");
    expect(turns[1]!.querySelector("strong")).toBeNull();
    fireEvent.click(work);
    // Open: the notes as written (markdown), and the steps' own row.
    expect(turns[1]!.querySelector("strong")?.textContent).toBe("tests");
    expect(turns[1]!.querySelector("[data-testid='md-code'] pre")?.textContent).toBe("const a = 1;");
    const steps = screen.getByTestId("last-turn-steps").querySelector("button")!;
    expect(steps.textContent).toBe("Ran 1 command, edited 1 file, read 1 file");
    expect(steps.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(steps);
    expect(screen.getByTestId("last-turn-step-lines").textContent).toContain('ran "pnpm test"');
    expect(screen.getByTestId("last-messages-note").textContent).toContain("nothing kept · 1 key taken out");
  });

  it("when it asks you, the question waits at the bottom with Go to it", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    const onGoTo = vi.fn();
    render(<LastMessages model={model()} visible onGoTo={onGoTo} />);
    expect((await screen.findByTestId("last-messages-question")).textContent).toBe("All green. Keep the old layout?");
    expect(screen.getAllByTestId("last-turn")[1]!.dataset.asks).toBe("true");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("Answer it in the Claude app");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("pick this one in its sidebar");
    fireEvent.click(screen.getByTestId("last-messages-go-to"));
    expect(onGoTo).toHaveBeenCalledTimes(1);
  });

  it("0.3.4: a turn ending on a step is all work; the question is the reply's, never a note's", () => {
    const step = { kind: "steps", text: "", steps: ['ran "pnpm test"'], hidden: 0 };
    const note = { kind: "text", text: "Should I also bump the version?", steps: [], hidden: 0 };
    const going = { speaker: "assistant", blocks: [note, step], at_ms: null, model: null } as LastTurnInfo;
    expect(foldOf(going)).toEqual({ work: [note, step], reply: null });
    const done = { ...going, blocks: [note, step, { kind: "text", text: "Done: 42 passed.", steps: [], hidden: 0 }] } as LastTurnInfo;
    expect(questionOf(done)).toBe("Done: 42 passed.");
    expect(workWords(foldOf(done).work, null)).toBe("Ran 1 command · 1 note");
  });

  it("a working session says so at the bottom, with no question card, and its latest turn shows open as it goes", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    render(<LastMessages model={model({}, { asks: false, working: true, words: "Working", tone: "live" })} visible />);
    expect((await screen.findByTestId("last-messages-working")).textContent).toContain("Claude is working");
    expect(screen.queryByTestId("last-turn-work")).toBeNull();
    expect(screen.getAllByTestId("last-turn")[1]!.querySelector("strong")?.textContent).toBe("tests");
    expect(screen.queryByTestId("last-messages-asks")).toBeNull();
    // Nothing to answer while it works: only where Go to it leads.
    expect(screen.getByTestId("last-messages-answer").textContent).not.toContain("Answer it");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("Go to it brings");
  });

  it("earlier turns load before the shown ones, from where they start", async () => {
    const read = vi
      .spyOn(ipc, "conversationsLastMessages")
      .mockResolvedValueOnce(info([text("you", "latest ask"), text("assistant", "latest answer")], { earlier: 900 }))
      .mockResolvedValueOnce(info([text("you", "older ask"), text("assistant", "older answer")], { earlier: null }));
    render(<LastMessages model={model({ live: false }, { asks: false })} visible />);
    fireEvent.click(await screen.findByTestId("last-messages-earlier"));
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(4));
    expect(read).toHaveBeenLastCalledWith("work", "c1", { before: 900 });
    expect(screen.getAllByTestId("last-turn").map((turn) => turn.textContent)).toEqual([
      expect.stringContaining("older ask"),
      expect.stringContaining("older answer"),
      expect.stringContaining("latest ask"),
      expect.stringContaining("latest answer"),
    ]);
    expect(screen.queryByTestId("last-messages-earlier")).toBeNull();
    expect(screen.getByTestId("last-messages-start")).toBeTruthy();
  });

  it("follow a working session only while its page is shown, asking with what it shows", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const read = vi
      .spyOn(ipc, "conversationsLastMessages")
      .mockResolvedValueOnce(info([text("you", "go")], { earlier: 40, stamp: "a" }))
      .mockResolvedValueOnce({ ...info([]), state: "unchanged", stamp: "a" })
      .mockResolvedValueOnce(info([text("you", "go"), text("assistant", "done")], { earlier: 40, stamp: "b" }))
      .mockResolvedValue({ ...info([]), state: "unchanged", stamp: "b" });
    const busy = model({}, { asks: false, working: true });
    const view = render(<LastMessages model={busy} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(1));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FOLLOW_WORKING_MS);
    });
    expect(read).toHaveBeenNthCalledWith(2, "work", "c1", { known: "a", from: 40 });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FOLLOW_WORKING_MS);
    });
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    expect(read).toHaveBeenNthCalledWith(3, "work", "c1", { known: "a", from: 40 });
    // Another tab in front: no asking at all.
    const before = read.mock.calls.length;
    view.rerender(<LastMessages model={busy} visible={false} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FOLLOW_WORKING_MS * 5);
    });
    expect(read.mock.calls.length).toBe(before);
    // Ended: not followed either.
    view.rerender(<LastMessages model={model({ live: false }, { asks: false })} visible />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(FOLLOW_WORKING_MS * 5);
    });
    expect(read.mock.calls.length).toBe(before);
  });

  it("off in Settings: says so, with the way to turn them on", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ ...info([]), state: "off" });
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-off")).textContent).toContain("Last messages are off");
    expect(screen.queryByTestId("last-turn")).toBeNull();
  });

  it("a chat with no transcript here shows its title only", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ ...info([]), state: "none" });
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-none")).textContent).toContain("its title is all");
  });

  it("a paired Mac's session is never read here", () => {
    const read = vi.spyOn(ipc, "conversationsLastMessages");
    render(<LastMessages model={model()} visible remoteMac="Mac mini" />);
    expect(screen.getByTestId("last-messages-remote").textContent).toBe("Open on Mac mini to read: its messages stay on that Mac.");
    expect(read).not.toHaveBeenCalled();
  });
});

describe("AJ-E2108 turns in words", () => {
  it("a run of calls reads as one line", () => {
    expect(stepsSummary(['ran "a"', 'ran "b"', "Read x.ts", "edited a.ts, b.ts", 'searched for "y"'])).toBe("Ran 2 commands, edited 2 files, read 1 file, searched once");
    expect(stepsSummary(["updated the task list"])).toBe("Updated the task list");
  });

  it("the question is the last paragraph that asks one, in plain words", () => {
    const turn: LastTurnInfo = text("assistant", "Pushed.\n\nShould I **rebase onto main first**, or open it as it is?\n\n```sh\ngit log\n```");
    expect(questionOf(turn)).toBe("Should I rebase onto main first, or open it as it is?");
    expect(questionOf(text("you", "hi?"))).toBeNull();
  });
});
