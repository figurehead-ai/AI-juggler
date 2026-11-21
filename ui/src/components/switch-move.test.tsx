/**
 * ADR-0031 (D-93): Move to account on the switch screen. Move is the
 * default when it can happen, Copy stays one click away, Move says what
 * happens to the original and that both profiles restart, nothing runs
 * without the click, the done screen keeps the four outcomes apart (a
 * model reply is always the user's to check), and Undo is there.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { MoveDoneInfo, MoveOfferInfo, MoveProgressInfo } from "../lib/ipc/switch-move";
import { switchPreflight } from "../lib/ipc/switch-preflight";

const executed = vi.hoisted(() => [] as unknown[][]);
// The move's progress listeners, as the backend would tell them.
const listeners = vi.hoisted(() => [] as Array<(progress: MoveProgressInfo) => void>);
vi.mock("../lib/ipc/switch-move", async (original) => {
  const real = await original<typeof import("../lib/ipc/switch-move")>();
  return {
    ...real,
    switchMoveExecute: (sessionId: string, targetProfileId: string, takeNotes: boolean) => {
      executed.push([sessionId, targetProfileId, takeNotes]);
      return real.switchMoveExecute(sessionId, targetProfileId, takeNotes);
    },
    onMoveProgress: (handler: (progress: MoveProgressInfo) => void) => {
      listeners.push(handler);
      const stop = real.onMoveProgress(handler);
      return () => {
        listeners.splice(listeners.indexOf(handler), 1);
        stop();
      };
    },
  };
});

const OFFER: MoveOfferInfo = {
  applies: true,
  blocks: [],
  members: [
    { title: "Refunds API", archived: false, side: "moves", folder_missing: false },
    { title: "Webhook tests", archived: false, side: "moves", folder_missing: false },
    { title: "Old spike", archived: true, side: "relinked", folder_missing: true },
  ],
  restarts: ["Claude app · Work", "Claude app · Personal"],
  notes: ["the target had its own different version of “Refunds API”; it is kept aside, never joined to this one, and Undo brings it back"],
  notes_conflict: null,
  kept_outputs: 2,
  bytes: 1_200_000_000,
  app: true,
};

const DONE: MoveDoneInfo = {
  move_id: "1762600000000-0a1b2c3d",
  payload: { state: "verified", detail: "every byte of 2 conversation(s) is in Personal, checked before and after it opened" },
  catalog: { state: "verified", detail: "Claude app · Personal lists the conversation and its family as they were nested" },
  opened: { state: "verified", detail: "opened in Claude app · Personal (process 4242), signed in as that account" },
  model: { state: "pending", detail: "send a message to check it answers on Personal; AI Juggler never sends one for you" },
  notes: [],
};

afterEach(() => {
  cleanup();
  executed.length = 0;
  listeners.length = 0;
  delete window.__AIJ_MOVE_OFFER__;
  delete window.__AIJ_MOVE_DONE__;
  delete window.__AIJ_MOVE_ERROR__;
  delete window.__AIJ_MOVE_PROGRESS__;
  delete window.__AIJ_MOVE_HOLD__;
  delete window.__AIJ_MOVE_UNDONE__;
  delete window.__AIJ_MOVES__;
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
});

/** The one-click door (the preview's own sample, made one-click). */
async function oneClick(): Promise<void> {
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
  const sample = await switchPreflight("sess-1", "p-1");
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: { ...sample, door: "one-click", wizard_steps: [], wizard_reasons: [], secrets: { ...sample.secrets, asks: false } } });
}

beforeEach(oneClick);

function mount(onClose: () => void = vi.fn()) {
  return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={onClose} onSwitched={vi.fn()} />);
}

/** 0.4.9: the maintainer's refusal, as the backend sends it now: plain
 * words, the path only in its detail. */
const STILL_WRITING = {
  code: "move_refused",
  retryable: true,
  user_message: "Claude app · Work was still writing this conversation as it closed, so nothing was moved. Try again in a moment.",
  detail: "still changing after 9 s: ~/example/projects/-w-proj/main.jsonl",
};

/** Mounted, Move clicked, and refused with `STILL_WRITING`. */
async function refused(onClose: () => void) {
  window.__AIJ_MOVE_OFFER__ = OFFER;
  window.__AIJ_MOVE_ERROR__ = STILL_WRITING;
  const view = mount(onClose);
  await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
  fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
  await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
  return view;
}

describe("Move to account", () => {
  it("isn't there where moving doesn't apply: the switch is the copy it always was", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-how")).toBeNull();
  });

  it("is the default when it can happen, and says what happens to the original and the restarts", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
    expect(view.getByRole("radio", { name: /^Move/ })).toHaveProperty("checked", true);
    expect(view.getByTestId("switch-move-what").textContent).toContain("The conversation and its 1 session · 1.2 GB");
    expect(view.getByTestId("switch-move-original").textContent).toContain("no longer lists it");
    expect(view.getByTestId("switch-move-restart").textContent).toContain("Claude app · Work and Claude app · Personal quit and start again. Anything running in them stops");
    expect(view.getByTestId("switch-move-model").textContent).toContain("AI Juggler sends no message");
    expect(view.getByTestId("switch-move-outputs").textContent).toContain("2 folders of tool output stay");
    expect(view.getByTestId("switch-move-confirm").textContent).toBe("Move to Personal");
    expect(view.queryByTestId("switch-confirm")).toBeNull();
    expect(executed).toHaveLength(0);
  });

  it("keeps Copy one click away", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
    fireEvent.click(view.getByRole("radio", { name: /^Copy/ }));
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-move")).toBeNull();
    expect(view.getByTestId("switch-how")).toBeTruthy();
  });

  it("0.4.1 (rc6): with Copy picked, the original is said once, not on the card too", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
    fireEvent.click(view.getByRole("radio", { name: /^Copy/ }));
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.getByTestId("switch-how-copy").closest("label")?.textContent).toContain("A copy continues on Personal.");
    expect(view.getByTestId("switch-how").textContent).not.toMatch(/original/i);
    expect(view.getByTestId("switch-original").textContent).toMatch(/^The original /);
  });

  it("moves on the click and keeps the four outcomes apart, a reply the user's to check", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_DONE__ = DONE;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    expect(executed).toEqual([["sess-1", "p-1", false]]);
    for (const key of ["payload", "catalog", "opened"]) expect(view.getByTestId(`switch-moved-${key}`).getAttribute("data-state")).toBe("verified");
    expect(view.getByTestId("switch-moved-model").getAttribute("data-state")).toBe("pending");
    fireEvent.click(view.getByTestId("switch-moved-undo"));
    await waitFor(() => expect(view.getByTestId("switch-moved-undo-said").textContent).toContain("Undone"));
  });

  it("can't be chosen while something works, says how it clears, and checks again", async () => {
    window.__AIJ_MOVE_OFFER__ = {
      ...OFFER,
      blocks: [{ kind: "user_can_clear", words: "2 sessions are working in Claude app · Work; Move becomes available once they finish" }],
    };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-how")).toBeTruthy());
    expect(view.getByRole("radio", { name: /^Move/ })).toHaveProperty("disabled", true);
    // Faded, how it clears said on the card, and no "Recommended" (D-127).
    expect(view.getByTestId("switch-how-move").textContent).not.toContain("Recommended");
    expect(view.getByTestId("switch-how").textContent).toContain(
      "2 sessions are working in Claude app · Work; Move becomes available once they finish.",
    );
    expect(view.getByTestId("switch-confirm")).toBeTruthy();
    window.__AIJ_MOVE_OFFER__ = OFFER;
    fireEvent.click(view.getByTestId("switch-how-again"));
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
  });

  // D-127 (the maintainer, 2026-11-18: Move greyed out with "proven for
  // version 4.x; this one is 5.11492.5"): what the user can't change hides
  // Move. Copy is the switch then, alone, with no word about it.
  it("isn't shown at all when the user can't clear what stands in its way", async () => {
    window.__AIJ_MOVE_OFFER__ = {
      ...OFFER,
      blocks: [
        { kind: "setup", words: "this version of the Claude app keeps its sessions in a way AI Juggler doesn't know yet" },
        { kind: "user_can_clear", words: "1 session is working in Claude app · Work; Move becomes available once it finishes" },
      ],
    };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-how")).toBeNull();
    expect(view.queryByRole("radio", { name: /Move to account/ })).toBeNull();
    expect(view.queryByTestId("switch-move")).toBeNull();
    expect(view.getByTestId("switch-flow").textContent).not.toMatch(/Move to account|doesn't know|version 5/i);
  });

  it("asks about the project's notes only when the two accounts' differ", async () => {
    window.__AIJ_MOVE_OFFER__ = { ...OFFER, notes_conflict: "this project's notes differ in the two accounts" };
    window.__AIJ_MOVE_DONE__ = DONE;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-take-notes")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-take-notes"));
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    expect(executed).toEqual([["sess-1", "p-1", true]]);
  });

  it("says why when the move stops, nothing half moved", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_ERROR__ = "Claude app · Personal didn't quit within 30 seconds, so nothing was moved";
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-flow").textContent).toContain("didn't quit within 30 seconds"));
    expect(view.getByTestId("switch-cancel")).toBeTruthy();
    expect(view.queryByTestId("switch-moved-undo")).toBeNull();
  });

  it("offers Undo when a move stopped part way", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_ERROR__ = "a check before reopening failed. Undoing it didn't finish";
    window.__AIJ_MOVES__ = [{ move_id: "1-aa", conversation: "c", state: "interrupted", at_ms: 1, detail: null, kept: [] }];
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved-undo")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-moved-undo"));
    await waitFor(() => expect(view.getByTestId("switch-moved-undo-said").textContent).toContain("Undone"));
  });

  // D-142 (DESIGN-SYSTEM §8): the header names the App, from → to; the
  // choice first, Move recommended once; what happens as four fact rows;
  // the restart as a needs-you Alert; the family behind a disclosure with
  // its tags; the serious choice explained, with the guide.
  it("is built to the design system's worked example", async () => {
    window.__AIJ_MOVE_OFFER__ = { ...OFFER, notes_conflict: "notes differ" };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
    const header = view.getByTestId("switch-header");
    expect(view.getByTestId("switch-move-title").textContent).toBe("Continue on Claude app · Personal");
    expect(header.contains(view.getByTestId("switch-move-title"))).toBe(true);
    expect(view.getByTestId("switch-move-strip").textContent).toMatch(/Claude app · Work.*to.*Claude app · Personal/);
    // The choice is the body's first part, never in the header.
    const body = view.getByTestId("switch-body");
    expect(body.contains(view.getByRole("radiogroup", { name: "How it continues" }))).toBe(true);
    expect(view.getAllByText("Recommended")).toHaveLength(1);
    expect(view.getByTestId("switch-how-move").textContent).toContain("No extra space");
    expect(view.getByTestId("switch-how-copy").textContent).toContain("Uses 1.2 GB");
    // What happens: Moves, Original, Restarts, After.
    for (const id of ["switch-move-what", "switch-move-original", "switch-move-restarts", "switch-move-model"]) expect(view.getByTestId(id)).toBeTruthy();
    expect(view.getByTestId("switch-move-restart").getAttribute("data-level")).toBe("needs-you");
    // The family, each with its tags; the tool output left behind with it.
    const members = view.getAllByTestId("switch-move-member");
    expect(members).toHaveLength(3);
    expect(members[2]!.textContent).toMatch(/Already on Personal.*Archived.*Folder gone/);
    expect(view.getByTestId("switch-move-family").contains(view.getByTestId("switch-move-outputs"))).toBe(true);
    expect(view.getByRole("checkbox", { name: /Use this conversation's project notes on Personal/ })).toBeTruthy();
    expect(view.getByTestId("switch-move-explain")).toBeTruthy();
    // One filled primary, last, naming its outcome.
    const footer = view.getByTestId("switch-footer");
    const buttons = [...footer.querySelectorAll("button")];
    expect(buttons.at(-1)!.textContent).toBe("Move to Personal");
    expect(buttons.at(-1)!.hasAttribute("data-primary")).toBe(true);
    expect(view.getByTestId("switch-flow").querySelectorAll("[data-primary]")).toHaveLength(1);
  });

  it("keeps its way out at the foot: the actions in the fixed footer, outside the body that scrolls", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    const body = view.getByTestId("switch-body");
    expect(body.getAttribute("data-dialog-part")).toBe("body");
    expect(view.getByTestId("switch-footer").getAttribute("data-dialog-part")).toBe("footer");
    expect(body.contains(view.getByTestId("switch-move-confirm"))).toBe(false);
    expect(body.contains(view.getByTestId("switch-cancel"))).toBe(false);
  });

  // 0.4.9 (the maintainer's 406 MB move): the refusal's dialog couldn't
  // be closed: Esc, × and Cancel all did nothing (the dialog stayed
  // marked busy after the move ended), and AI Juggler had to be quit.
  it.each([
    ["Esc", (view: ReturnType<typeof mount>) => fireEvent.keyDown(view.getByTestId("switch-flow"), { key: "Escape" })],
    ["×", (view: ReturnType<typeof mount>) => fireEvent.click(view.getByTestId("switch-close"))],
    ["Cancel", (view: ReturnType<typeof mount>) => fireEvent.click(view.getByTestId("switch-cancel"))],
  ])("closes after a refusal with %s", async (_key, close) => {
    const onClose = vi.fn();
    const view = await refused(onClose);
    close(view);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("says a refusal that changed nothing calmly, in plain words, the path only in Copy details", async () => {
    const view = await refused(vi.fn());
    expect(view.getByTestId("switch-failed").getAttribute("data-calm")).toBe("true");
    expect(view.getByTestId("switch-failed").textContent).toContain("Nothing was moved");
    expect(view.getByTestId("switch-failed").textContent).not.toContain("didn't complete");
    const words = view.getByTestId("switch-failed-words");
    expect(words.getAttribute("data-level")).toBe("info");
    expect(words.textContent).toBe(STILL_WRITING.user_message);
    expect(words.textContent).not.toContain("/");
    expect(view.getByTestId("switch-details").textContent).toContain("main.jsonl");
    expect(view.queryByTestId("switch-moved-undo")).toBeNull();
  });

  it("tries again from Move after a refusal, its offer read again", async () => {
    const view = await refused(vi.fn());
    expect(view.getByTestId("switch-move-again").textContent).toBe("Retry");
    fireEvent.click(view.getByTestId("switch-move-again"));
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-failed")).toBeNull();
  });

  it("closes with Done after a move (the same busy mark held it)", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_DONE__ = DONE;
    const onClose = vi.fn();
    const view = mount(onClose);
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-moved-close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("never looks frozen: each step in one line, and a bar that rises while it backs up", async () => {
    window.__AIJ_MOVE_OFFER__ = OFFER;
    window.__AIJ_MOVE_HOLD__ = true;
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(listeners).toHaveLength(1));
    const tell = (progress: MoveProgressInfo) => act(() => listeners[0]!(progress));
    tell({ stage: "closing", words: "Waiting for Claude app · Work to finish closing…", done: 0, total: 0 });
    expect(view.getByTestId("switch-moving-now").textContent).toBe("Waiting for Claude app · Work to finish closing…");
    expect(view.queryByTestId("switch-moving-bar")).toBeNull();
    const total = 812_000_000;
    const seen: number[] = [];
    for (const done of [8_120_000, 203_000_000, 609_000_000, total]) {
      tell({ stage: "backing_up", words: "Backing up the conversation (406 MB)…", done, total });
      seen.push(Number(view.getByTestId("switch-moving-bar").getAttribute("aria-valuenow")));
    }
    expect(seen).toEqual([1, 25, 75, 100]);
    expect(view.getByTestId("switch-moving-now").textContent).toBe("Backing up the conversation (406 MB)…");
    expect(view.getByTestId("switch-moving-percent").textContent).toBe("100%");
    tell({ stage: "moving", words: "Moving the conversation…", done: 0, total: 0 });
    expect(view.queryByTestId("switch-moving-bar")).toBeNull();
    expect(view.getByTestId("switch-moving").getAttribute("data-stage")).toBe("moving");
  });
});
