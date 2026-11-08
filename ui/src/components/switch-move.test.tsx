/**
 * ADR-0031 (D-93): Move to account on the switch screen. Move is the
 * default when it can happen, Copy stays one click away, Move says what
 * happens to the original and that both profiles restart, nothing runs
 * without the click, the done screen keeps the four outcomes apart (a
 * model reply is always the user's to check), and Undo is there.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "./SwitchFlow";
import type { MoveDoneInfo, MoveOfferInfo } from "../lib/ipc/switch-move";
import { switchPreflight } from "../lib/ipc/switch-preflight";

const executed = vi.hoisted(() => [] as unknown[][]);
vi.mock("../lib/ipc/switch-move", async (original) => {
  const real = await original<typeof import("../lib/ipc/switch-move")>();
  return {
    ...real,
    switchMoveExecute: (sessionId: string, targetProfileId: string, takeNotes: boolean) => {
      executed.push([sessionId, targetProfileId, takeNotes]);
      return real.switchMoveExecute(sessionId, targetProfileId, takeNotes);
    },
  };
});

const OFFER: MoveOfferInfo = {
  applies: true,
  offered: true,
  why_not: [],
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
  delete window.__AIJ_MOVE_OFFER__;
  delete window.__AIJ_MOVE_DONE__;
  delete window.__AIJ_MOVE_ERROR__;
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

function mount() {
  return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={vi.fn()} onSwitched={vi.fn()} />);
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
    expect(view.getByTestId("switch-how-move")).toHaveProperty("checked", true);
    expect(view.getByTestId("switch-move-what").textContent).toContain("the conversation and its 1 session (1.2 GB)");
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
    fireEvent.click(view.getByTestId("switch-how-copy"));
    await waitFor(() => expect(view.getByTestId("switch-confirm")).toBeTruthy());
    expect(view.queryByTestId("switch-move")).toBeNull();
    expect(view.getByTestId("switch-how")).toBeTruthy();
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

  it("can't be chosen while something works, says why, and checks again", async () => {
    window.__AIJ_MOVE_OFFER__ = { ...OFFER, offered: false, why_not: ["Claude app · Work has 2 sessions working: a move restarts it, which would stop them. Let them finish first"] };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-how")).toBeTruthy());
    expect(view.getByTestId("switch-how-move")).toHaveProperty("disabled", true);
    expect(view.getByTestId("switch-how").textContent).toContain("2 sessions working");
    expect(view.getByTestId("switch-confirm")).toBeTruthy();
    window.__AIJ_MOVE_OFFER__ = OFFER;
    fireEvent.click(view.getByTestId("switch-how-again"));
    await waitFor(() => expect(view.getByTestId("switch-move")).toBeTruthy());
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
});
