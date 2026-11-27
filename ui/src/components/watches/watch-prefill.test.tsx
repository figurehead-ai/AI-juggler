/**
 * AJ-E3830 (0.5.2) vitest — the Watches form as Bot pre-filled it: what
 * code verified is filled; what it left stays empty for the user (no
 * default chosen for them: no condition card, no lifetime), the note says
 * what is left and leaves each field once it is set, the nearest
 * condition is tagged but not chosen, quiet hours say to check them, and
 * Watch waits for how long.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { forgetPaged } from "../../lib/paged";
import { resetToasts } from "../Toast";
import { openWatches } from "../../lib/watches-open";
import { STATES, WATCHES_APPS, WATCHES_CATALOGUE, WATCHES_FIXTURE, WATCHES_NOW, draftWords, made } from "../../preview/watches";
import { WatchesPage } from "./WatchesPage";
import type { TargetSources } from "./watches-view";
import type { WatchPrefill } from "../../types/generated/WatchPrefill";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";

const SOURCES: TargetSources = { sessions: WATCHES_FIXTURE.sessions, apps: WATCHES_APPS, projects: WATCHES_FIXTURE.projects, pools: [], accounts: WATCHES_FIXTURE.accounts, now: WATCHES_NOW };

beforeEach(() => {
  vi.spyOn(Date, "now").mockReturnValue(WATCHES_NOW);
  window.__AIJ_WATCH_VIEW__ = { watches: structuredClone(STATES.list!.watches), catalogue: WATCHES_CATALOGUE, draft: draftWords, made: (draft, id) => made(draft, id, { createdAgo: 0 }), calls: [] };
});

afterEach(() => {
  cleanup();
  forgetPaged();
  resetToasts();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_WATCH_VIEW__");
  Reflect.deleteProperty(window, "__AIJ_NOTIFICATION_PREFS__");
});

const calls = () => window.__AIJ_WATCH_VIEW__?.calls ?? [];

function prefill(over: Partial<WatchPrefill>): WatchPrefill {
  return {
    draft: { targets: [{ kind: "session", id: "s-api" }], matching: "each", conditions: [{ code: "finishes" }], lifetime: { kind: "unsaid" }, evenInQuietHours: false, chatTurn: "t1" },
    left: [],
    why: "verified",
    asked: null,
    nearest: null,
    card: false,
    ...over,
  };
}

async function open(p: WatchPrefill): Promise<void> {
  render(<WatchesPage sources={SOURCES} onGoTo={() => undefined} onAskBot={() => undefined} onQuietHours={() => undefined} />);
  await waitFor(() => expect(screen.queryByTestId("watches-skeleton")).toBeNull());
  act(() => openWatches({ kind: "new", prefill: p }));
  await screen.findByTestId("watch-form");
}

describe("AJ-E3830 the Watches form pre-filled by Bot", () => {
  it("an unread lifetime: nothing chosen, the note names it, and Watch waits for a pick", async () => {
    await open(prefill({ left: ["lifetime"], why: "unread", asked: "bis ich es entferne" }));
    expect(screen.getByTestId("watch-bot-fill-line").textContent).toBe("Bot filled in what it could read. How long is left for you. It couldn't read “bis ich es entferne”.");
    const choice = screen.getByTestId("watch-lifetime-choice");
    expect(choice.querySelectorAll("[aria-checked=true]")).toHaveLength(0);
    expect(screen.getByTestId("watch-lifetime-line").textContent).toBe("Bot couldn't tell how long from your words. Pick one.");
    await waitFor(() => expect(screen.getByTestId("watch-condition-finishes").querySelector("input")?.checked).toBe(true));
    expect(screen.getByTestId("watch-form-consequence").textContent).toBe("Pick how long first.");
    expect(screen.getByTestId("watch-form-save").hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByTestId("watch-lifetime-choice-always"));
    expect(screen.getByTestId("watch-bot-fill").dataset.left).toBe("");
    expect(screen.getByTestId("watch-bot-fill-line").textContent).toContain("Bot read every part of your words.");
    await waitFor(() => expect(screen.getByTestId("watch-form-save").hasAttribute("disabled")).toBe(false));
    fireEvent.click(screen.getByTestId("watch-form-save"));
    await waitFor(() => expect(screen.queryByTestId("watch-form")).toBeNull());
    const created: WatchDraftInfo = JSON.parse(JSON.stringify(calls().find(([name]) => name === "watches_create")?.[1]));
    expect(created.lifetime).toEqual({ kind: "always" });
    expect(created.targets).toEqual([{ kind: "session", id: "s-api" }]);
  });

  it("an unread condition: no card chosen (no default either), the nearest tagged, not chosen", async () => {
    await open(prefill({ draft: { ...prefill({}).draft, conditions: [] }, left: ["condition"], why: "unread", asked: "wants something", nearest: "asks_you" }));
    await waitFor(() => expect(screen.getByTestId("watch-condition-asks_you")).toBeTruthy());
    const checked = [...screen.getByTestId("watch-conditions").querySelectorAll<HTMLInputElement>("input")].filter((i) => i.checked);
    expect(checked).toHaveLength(0);
    expect(screen.getByTestId("watch-condition-asks_you").querySelector("[data-testid=watch-condition-nearest]")?.textContent).toBe("Nearest");
    expect(screen.getAllByTestId("watch-condition-nearest")).toHaveLength(1);
    expect(screen.getByTestId("watch-form-save").hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByTestId("watch-condition-asks_you").querySelector("input")!);
    expect(screen.queryByTestId("watch-condition-nearest")).toBeNull();
  });

  it("an unread target: the picker is empty and asks; nothing is guessed", async () => {
    await open(prefill({ draft: { ...prefill({}).draft, targets: [] }, left: ["target"], why: "several" }));
    expect(screen.getByTestId("watch-bot-fill-line").textContent).toBe("More than one matches that name: pick which one.");
    expect(screen.getByTestId("watch-form-section-1").textContent).toContain("Bot couldn't tell which one");
  });

  it("unread quiet hours: the switch stays off and the form says to check it, until it is set", async () => {
    Reflect.set(window, "__AIJ_NOTIFICATION_PREFS__", { quiet_start: 22, quiet_end: 7 });
    await open(prefill({ left: ["quiet_hours"], why: "unread", asked: "yes, even overnight" }));
    const toggle = await screen.findByTestId("watch-quiet-toggle");
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(screen.getByTestId("watch-quiet-left").textContent).toBe("Bot couldn't tell whether this should wake you. Check it.");
    fireEvent.click(toggle);
    expect(screen.queryByTestId("watch-quiet-left")).toBeNull();
  });

  it("every field verified: the form is filled, nothing left, Watch ready", async () => {
    await open(prefill({}));
    expect(screen.getByTestId("watch-bot-fill-line").textContent).toBe("Bot read every part of your words. Check it, then click Watch.");
    await waitFor(() => expect(screen.getByTestId("watch-form-save").hasAttribute("disabled")).toBe(false));
  });
});
