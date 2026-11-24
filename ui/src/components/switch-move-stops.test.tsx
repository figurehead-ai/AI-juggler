/**
 * AJ-E2319: a move's refusals are typed, and the Move window says each
 * class's own words and offers only its own buttons: Retry for one that
 * can clear by itself, Check again and the holder's door for one only the
 * person can clear, Copy instead for a setup that can't move, Copy details
 * and Report for a bug. Never Retry where trying again can't help (each
 * try quits and reopens both apps).
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { MoveOfferInfo } from "../lib/ipc/switch-move";
import { switchPreflight } from "../lib/ipc/switch-preflight";

const executed = vi.hoisted(() => [] as unknown[][]);
const opened = vi.hoisted(() => [] as string[]);
const went = vi.hoisted(() => [] as string[][]);
vi.mock("../lib/ipc/conversations", async (original) => {
  const real = await original<typeof import("../lib/ipc/conversations")>();
  return {
    ...real,
    conversationsGoTo: (accountId: string, conversationId: string) => {
      went.push([accountId, conversationId]);
      return Promise.resolve({ outcome: "Brought its Terminal window forward", reached: "thread" });
    },
  };
});
const undid = vi.hoisted(() => [] as string[]);
vi.mock("../lib/ipc/switch-move", async (original) => {
  const real = await original<typeof import("../lib/ipc/switch-move")>();
  return {
    ...real,
    switchMoveExecute: (sessionId: string, targetProfileId: string, takeNotes: boolean, targetPlaceId: string | null = null) => {
      executed.push([sessionId, targetProfileId, takeNotes, targetPlaceId]);
      return real.switchMoveExecute(sessionId, targetProfileId, takeNotes, targetPlaceId);
    },
    switchMoveUndo: (moveId: string) => {
      undid.push(moveId);
      return real.switchMoveUndo(moveId);
    },
  };
});
vi.mock("../lib/ipc/places", async (original) => {
  const real = await original<typeof import("../lib/ipc/places")>();
  return {
    ...real,
    placesOpen: (id: string) => {
      opened.push(id);
      return Promise.resolve({
        outcome: "Brought Claude app · Work forward",
        reached: "app",
      });
    },
  };
});

const OFFER: MoveOfferInfo = {
  applies: true,
  blocks: [],
  holders: [],
  members: [
    {
      title: "Refunds API",
      archived: false,
      side: "moves",
      folder_missing: false,
    },
    {
      title: "Webhook tests",
      archived: false,
      side: "moves",
      folder_missing: false,
    },
    {
      title: "Old spike",
      archived: true,
      side: "relinked",
      folder_missing: true,
    },
  ],
  restarts: ["Claude app · Work", "Claude app · Personal"],
  notes: ["the target had its own different version of “Refunds API”; it is kept aside, never joined to this one, and Undo brings it back"],
  notes_conflict: null,
  kept_outputs: 2,
  bytes: 1_200_000_000,
  app: true,
  carried: [],
};

beforeEach(async () => {
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
  const sample = await switchPreflight("sess-1", "p-1");
  Object.assign(window, {
    __AIJ_SWITCH_PREFLIGHT__: {
      ...sample,
      door: "one-click",
      wizard_steps: [],
      wizard_reasons: [],
      secrets: { ...sample.secrets, asks: false },
    },
  });
});

afterEach(() => {
  cleanup();
  executed.length = 0;
  opened.length = 0;
  went.length = 0;
  undid.length = 0;
  delete window.__AIJ_MOVE_OFFER__;
  delete window.__AIJ_MOVE_ERROR__;
  delete window.__AIJ_MOVE_UNDONE__;
  delete window.__AIJ_MOVES__;
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
});

function mount() {
  return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" targetPlaceId={null} onClose={vi.fn()} onSwitched={vi.fn()} />);
}

/** Each refusal class's words and buttons (R2, R4's UI table). */
describe("Move's refusals, by class", () => {
  const WORK = { name: "Claude app · Work", place_id: "place-work", account_id: null, conversation: null };
  const stopped = async (cls: "transient" | "needs_you" | "setup" | "bug", words: string, holders = [WORK]) => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_ERROR__ = {
      code: "move_refused",
      retryable: cls === "transient",
      user_message: words,
      detail: "detail",
      stop: { class: cls, holders, move_id: "3-cc", unchanged: true },
    };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    return view;
  };
  const buttons = (view: ReturnType<typeof mount>) => ({
    retry: view.queryByTestId("switch-move-again") !== null,
    check: view.queryByTestId("switch-move-check") !== null,
    copy: view.queryByTestId("switch-move-copy") !== null,
    door: view.queryByTestId("switch-move-holder-door") !== null,
    undo: view.queryByTestId("switch-moved-undo") !== null,
  });

  it.each([
    [
      "transient",
      "Claude app · Work was still writing this conversation as it closed, so nothing was moved. Try again in a moment.",
      { retry: true, check: false, copy: false, door: false, undo: false },
    ],
    [
      "needs_you",
      "Claude app · Work didn't quit within 30 seconds, so nothing was moved. It may be asking something: look at its window",
      { retry: false, check: true, copy: false, door: true, undo: false },
    ],
    ["setup", "this tool's conversations travel as a copy", { retry: false, check: false, copy: true, door: false, undo: false }],
    ["bug", "couldn't quit Claude app · Work: not permitted", { retry: false, check: false, copy: false, door: false, undo: false }],
  ] as const)("%s: its words, what holds the files, and only its buttons", async (cls, words, shown) => {
    const view = await stopped(cls, words);
    expect(buttons(view)).toEqual(shown);
    expect(view.getByTestId("switch-failed-words").textContent?.toLowerCase()).toContain(words.split(",")[0]!.toLowerCase());
    expect(view.getByTestId("switch-move-holders").querySelector("p")?.textContent).toBe("Holding the files: Claude app · Work");
    // Copy details and Report are there for every class (a bug's way out).
    expect(view.getByTestId("switch-copy-details")).toBeTruthy();
    expect(view.getByTestId("switch-report-bug")).toBeTruthy();
  });

  it("needs you: Check again reads the offer again and quits nothing; the door goes to the app", async () => {
    const view = await stopped("needs_you", "Claude app · Work didn't quit within 30 seconds, so nothing was moved. It may be asking something: look at its window");
    expect(view.getByTestId("switch-move-holder-door").textContent).toBe("Go to Claude app · Work");
    fireEvent.click(view.getByTestId("switch-move-holder-door"));
    await waitFor(() => expect(opened).toEqual(["place-work"]));
    expect(await view.findByText("Brought Claude app · Work forward")).toBeTruthy();
    fireEvent.click(view.getByTestId("switch-move-check"));
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    expect(executed).toHaveLength(1);
  });

  it("setup: Copy instead hides Move and the switch is the copy", async () => {
    const view = await stopped("setup", "this tool's conversations travel as a copy", []);
    expect(view.queryByTestId("switch-move-holders")).toBeNull();
    fireEvent.click(view.getByTestId("switch-move-copy"));
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-how")).toBeNull();
    expect(view.queryByTestId("switch-move-confirm")).toBeNull();
  });

  // AJ-E2319 R3: Undo targets the move the error names, never the newest
  // interrupted one (here another conversation's), and offers no Retry.
  it("offers Undo of the very move that stopped part way, by its id", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_ERROR__ = {
      code: "move_stopped",
      retryable: false,
      user_message: "a check before reopening failed. Undoing it didn't finish",
      stop: { class: "bug", holders: [], move_id: "2-bb", unchanged: false },
    };
    window.__AIJ_MOVES__ = [
      {
        move_id: "1-aa",
        conversation: "other",
        title: null,
        from: "A",
        to: "B",
        can_undo: true,
        undo_why: null,
        can_finish: false,
        has_backup: false,
        kept_until_ms: null,
        backup_bytes: null,
        backup_exact: false,
        recovered: null,
        repair: false,
        timeline: [],
        state: "interrupted",
        at_ms: 1,
        detail: null,
        kept: [],
      },
    ];
    window.__AIJ_MOVE_UNDONE__ = {
      words: "Undone: the conversation is back where it was.",
      kept: "",
      notes: ["node that Claude app · Work started is still running; the move doesn't need it closed"],
    };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved-undo")).toBeTruthy());
    expect(view.queryByTestId("switch-move-again")).toBeNull();
    fireEvent.click(view.getByTestId("switch-moved-undo"));
    await waitFor(() => expect(view.getByTestId("switch-moved-undo-said").textContent).toContain("Undone"));
    expect(undid).toEqual(["2-bb"]);
    // R5: what kept running, said after Undo as the move says it.
    expect(view.getByTestId("switch-undone-notes").textContent).toContain("Node that Claude app · Work started is still running");
  });
});

/** 0.5.1 F72 (PL1 S-I4-refused): Move faded for something the user can
 * clear names what holds the files and offers its door, as a stop does
 * (AJ-E2319), never only "1 session … is open outside the app". */
describe("Move waiting on what holds the files", () => {
  const TERMINAL = { name: "“Fix the login test” in Terminal", place_id: null, account_id: "acct-work", conversation: "term-1" };
  const blocked = (holders: MoveOfferInfo["holders"]): MoveOfferInfo => ({
    ...OFFER,
    blocks: [{ kind: "user_can_clear", words: "“Fix the login test” in Terminal is open outside the app; Move becomes available once you end it" }],
    holders,
  });

  it("names the session and where it is open, with a Go to that goes to it", async () => {
    window.__AIJ_MOVE_OFFER__ = blocked([TERMINAL]);
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-how-move")).toBeTruthy());
    expect(view.getByTestId("switch-how-move").textContent).toContain("“Fix the login test” in Terminal is open outside the app; Move becomes available once you end it.");
    const door = view.getByTestId("switch-how-holders").querySelector("button")!;
    expect(door.textContent).toBe("Go to “Fix the login test” in Terminal");
    fireEvent.click(door);
    await waitFor(() => expect(went).toEqual([["acct-work", "term-1"]]));
    expect(await view.findByText("Brought its Terminal window forward")).toBeTruthy();
  });

  it("an app a turn runs in has its door too; a program known only by name has none", async () => {
    window.__AIJ_MOVE_OFFER__ = blocked([{ name: "Claude app · Work", place_id: "place-work", account_id: null, conversation: null }, { name: "claude (process 730)", place_id: null, account_id: null, conversation: null }]);
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-how-holders")).toBeTruthy());
    const doors = [...view.getByTestId("switch-how-holders").querySelectorAll("button")].map((b) => b.textContent);
    expect(doors).toEqual(["Go to Claude app · Work"]);
  });
});
