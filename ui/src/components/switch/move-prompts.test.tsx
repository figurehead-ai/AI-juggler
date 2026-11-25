/**
 * 0.5.1 F93 (PAT-OSP): a move's system prompts are asked on the Move
 * screen, their sheet over the switch dialog, after the Move click and
 * before the move starts (nothing quits until it's answered). Not now
 * still moves, and the done screen says how to open the conversation.
 * Undo asks the same way first. The fake gate here is the backend's: the
 * request shows the sheet (`os-prompt://ask`) and returns once answered.
 */
import { act, cleanup, fireEvent, render, waitFor, type RenderResult } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow } from "../SwitchFlow";
import { OsPromptHost } from "../OsPromptSheet";
import { MoveRows } from "./MovesDoor";
import type { OsPromptInfo, OsPromptOutcomeInfo } from "../../lib/ipc/os-prompts";
import type { MoveDoneInfo, MoveOfferInfo, MoveRecordInfo } from "../../lib/ipc/switch-move";
import { switchPreflight } from "../../lib/ipc/switch-preflight";

const LINKS = "automation-app-links";

/** What happened, in order: each sheet asked and answered, each call. */
const log = vi.hoisted(() => [] as string[]);
const gate = vi.hoisted(() => ({
  ask: undefined as ((prompt: OsPromptInfo) => void) | undefined,
  told: undefined as ((outcome: OsPromptOutcomeInfo) => void) | undefined,
  waiting: new Map<string, (outcome: OsPromptOutcomeInfo) => void>(),
  info: undefined as OsPromptInfo | undefined,
}));

vi.mock("../../lib/ipc/os-prompts", async (original) => {
  const real = await original<typeof import("../../lib/ipc/os-prompts")>();
  return {
    ...real,
    onOsPromptAsk: async (handler: (prompt: OsPromptInfo) => void) => {
      gate.ask = handler;
      return () => undefined;
    },
    onOsPromptOutcome: async (handler: (outcome: OsPromptOutcomeInfo) => void) => {
      gate.told = handler;
      return () => undefined;
    },
    osPromptRequest: (id: string) => {
      log.push(`asked ${id}`);
      return new Promise<OsPromptOutcomeInfo>((resolve) => {
        gate.waiting.set(id, resolve);
        gate.ask?.(gate.info!);
      });
    },
    osPromptAnswer: async (id: string, go: boolean) => {
      log.push(`${go ? "continue" : "not now"} ${id}`);
      const outcome: OsPromptOutcomeInfo = { id, result: go ? "called" : "not_now", status: "asked", words: "Asked." };
      gate.waiting.get(id)?.(outcome);
      gate.waiting.delete(id);
      if (go) gate.told?.(outcome);
    },
  };
});

vi.mock("../../lib/ipc/switch-move", async (original) => {
  const real = await original<typeof import("../../lib/ipc/switch-move")>();
  return {
    ...real,
    switchMoveExecute: (...args: Parameters<typeof real.switchMoveExecute>) => {
      log.push("move");
      return real.switchMoveExecute(...args);
    },
    switchMoveUndo: (moveId: string) => {
      log.push("undo");
      return real.switchMoveUndo(moveId);
    },
  };
});

const SHEET: OsPromptInfo = {
  id: LINKS,
  title: "Open a session in its desktop app",
  why: "Sends the app that keeps a session its own link, so the session itself opens, in the right profile.",
  system_says: "“AI Juggler” wants access to control “Claude”.",
  click: "Click Allow.",
  if_declined: "The app comes forward; you pick the session in its sidebar.",
  offered_again: "It asks again the next time you open a session in its app.",
  needed: true,
  status: "not_asked",
  primed: false,
  change_later: null,
  background: "Never asks: links sent without you are sent quietly or not at all.",
  never_shown: false,
  dialog: null,
};

const OFFER: MoveOfferInfo = {
  applies: true,
  blocks: [],
  holders: [],
  members: [{ title: "Refunds API", archived: false, side: "moves", folder_missing: false }],
  restarts: ["Claude app · Work", "Claude app · Personal"],
  notes: [],
  notes_conflict: null,
  kept_outputs: 0,
  bytes: 1_000_000,
  app: true,
  prompts: [LINKS],
  carried: [],
};

/** Moved after Not now: no link sent, how to open it said, Open offered. */
const DECLINED: MoveDoneInfo = {
  move_id: "1762600000000-0a1b2c3d",
  payload: { state: "verified", detail: "every byte of 1 conversation is in Personal, checked before and after it opened" },
  catalog: { state: "verified", detail: "Claude app · Personal lists the conversation and its family as they were nested" },
  opened: { state: "pending", detail: "not opened for you: AI Juggler may not send links to apps yet. Pick it in the App's sidebar, or click Open below" },
  model: { state: "pending", detail: "send a message to check it answers on Personal; AI Juggler never sends one for you" },
  notes: [],
  open_place_id: "place-b",
  conversation: "main",
};

beforeEach(async () => {
  log.length = 0;
  gate.waiting.clear();
  gate.info = SHEET;
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
  const sample = await switchPreflight("sess-1", "p-1");
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: { ...sample, door: "one-click", wizard_steps: [], wizard_reasons: [], secrets: { ...sample.secrets, asks: false } } });
  window.__AIJ_MOVE_OFFER__ = OFFER;
  window.__AIJ_MOVE_DONE__ = DECLINED;
});

afterEach(() => {
  cleanup();
  delete window.__AIJ_MOVE_OFFER__;
  delete window.__AIJ_MOVE_DONE__;
  delete window.__AIJ_MOVE_UNDONE__;
  delete window.__AIJ_MOVES__;
  Object.assign(window, { __AIJ_SWITCH_PREFLIGHT__: undefined });
});

function mount(): RenderResult {
  return render(
    <>
      <SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" targetPlaceId="place-b" onClose={vi.fn()} onSwitched={vi.fn()} />
      <OsPromptHost />
    </>,
  );
}

/** Mounted, Move clicked: the sheet is up. */
async function moveClicked(): Promise<RenderResult> {
  const view = mount();
  await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
  fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
  await waitFor(() => expect(view.getByTestId("os-prompt-sheet")).toBeTruthy());
  return view;
}

describe("Move's system prompts (0.5.1 F93)", () => {
  it("shows the link's sheet on the Move screen before the move starts; nothing runs until it's answered", async () => {
    const view = await moveClicked();
    expect(log).toEqual([`asked ${LINKS}`]);
    expect(view.getByTestId("os-prompt-sheet").textContent).toContain("Open a session in its desktop app");
    // Still the Move screen under it: nothing quit, the move not started.
    expect(view.getByTestId("switch-move")).toBeTruthy();
    expect(view.queryByTestId("switch-moving")).toBeNull();
    expect(view.getByTestId("switch-move-confirm")).toHaveProperty("disabled", true);
    fireEvent.click(view.getByTestId("os-prompt-continue"));
    await waitFor(() => expect(log).toEqual([`asked ${LINKS}`, `continue ${LINKS}`, "move"]));
  });

  it("Not now still moves, and the done screen says how to open the conversation, with Open", async () => {
    const view = await moveClicked();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    expect(log).toEqual([`asked ${LINKS}`, `not now ${LINKS}`, "move"]);
    expect(view.getByTestId("switch-moved-opened").textContent).toContain("Pick it in the App's sidebar, or click Open below");
    expect(view.getByTestId("switch-moved-open").textContent).toContain("Open in Personal");
  });

  it("asks nothing once it's primed: Move starts at once", async () => {
    window.__AIJ_MOVE_OFFER__ = { ...OFFER, prompts: [] };
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-move-confirm")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-move-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    expect(log).toEqual(["move"]);
    expect(view.queryByTestId("os-prompt-sheet")).toBeNull();
  });

  it("Undo after the move asks again first (Not now was no answer for good), before anything quits", async () => {
    const view = await moveClicked();
    fireEvent.click(view.getByTestId("os-prompt-not-now"));
    await waitFor(() => expect(view.getByTestId("switch-moved")).toBeTruthy());
    fireEvent.click(view.getByTestId("os-prompt-done"));
    log.length = 0;
    fireEvent.click(view.getByTestId("switch-moved-undo"));
    await waitFor(() => expect(view.getByTestId("os-prompt-sheet")).toBeTruthy());
    expect(log).toEqual([`asked ${LINKS}`]);
    fireEvent.click(view.getByTestId("os-prompt-not-now"));
    await waitFor(() => expect(view.getByTestId("switch-undone")).toBeTruthy());
    expect(log).toEqual([`asked ${LINKS}`, `not now ${LINKS}`, "undo"]);
  });
});

describe("the Moves door's Undo (0.5.1 F93)", () => {
  const RECORD: MoveRecordInfo = {
    move_id: "m-1",
    conversation: "main",
    title: "Refunds API",
    repair: false,
    from: "Claude app · Work",
    to: "Claude app · Personal",
    state: "committed",
    at_ms: 1_762_600_000_000,
    detail: null,
    can_undo: true,
    undo_why: null,
    can_finish: false,
    has_backup: true,
    kept_until_ms: null,
    backup_bytes: null,
    backup_exact: true,
    recovered: null,
    prompts: [LINKS],
    timeline: [],
    kept: [],
  };

  it("asks the record's prompts in place of its Undo sheet, then undoes", async () => {
    const view = render(
      <>
        <MoveRows moves={[RECORD]} onChanged={vi.fn()} />
        <OsPromptHost />
      </>,
    );
    fireEvent.click(view.getByTestId("move-undo"));
    const confirm = await view.findByTestId("move-undo-confirm");
    await act(async () => fireEvent.click(confirm));
    await waitFor(() => expect(view.getByTestId("os-prompt-sheet")).toBeTruthy());
    expect(log).toEqual([`asked ${LINKS}`]);
    expect(view.queryByTestId("move-undo-confirm")).toBeNull();
    fireEvent.click(view.getByTestId("os-prompt-continue"));
    await waitFor(() => expect(log).toEqual([`asked ${LINKS}`, `continue ${LINKS}`, "undo"]));
  });
});
