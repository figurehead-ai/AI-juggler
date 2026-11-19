/**
 * AJ-E3816 vitest — the Watches view in each state of the approved
 * mock-up, on the preview's data with the backend's stand-in: the list
 * (echo, chips, state, one primary), the menu, pause and resume by id,
 * remove with Undo, the quiet-hours line, Ended (folded, each reason with
 * its way forward), empty with its starters, many (toolbar, paging, the
 * limit only once New watch is pressed), and the form (the picker's
 * order, conditions hidden for what an App can't report, the live echo
 * the backend's own, the cost by the button). Plain words throughout.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { forgetPaged } from "../../lib/paged";
import { primariesIn } from "../primitives";
import { resetToasts, toastEntries, undoToast } from "../Toast";
import { ToastHost } from "../ToastStack";
import { openWatches } from "../../lib/watches-open";
import { STATES, WATCHES_CATALOGUE, WATCHES_FIXTURE, WATCHES_NOW, draftWords, made, type WatchesPreviewState } from "../../preview/watches";
import { WatchesPage } from "./WatchesPage";
import type { TargetSources } from "./watches-view";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";

const INTERNAL = [/\bundefined\b/, /\bnull\b/, /NaN/, /\bw-[0-9a-f]+\b/, /first_fire|targets_ended|time_reached|target_gone|no_longer_offered/, /asks_you|hits_limit|runway_below|leaves_processes|idle_for/, /\[object/];

const SOURCES: TargetSources = {
  sessions: WATCHES_FIXTURE.sessions,
  apps: WATCHES_FIXTURE.profiles,
  projects: WATCHES_FIXTURE.projects,
  pools: [{ id: "pool-rel", name: "Release pool", kind: "user", project_id: "proj-shop", members: 4 }],
  accounts: WATCHES_FIXTURE.accounts,
  now: WATCHES_NOW,
};

beforeEach(() => {
  vi.spyOn(Date, "now").mockReturnValue(WATCHES_NOW);
});

afterEach(() => {
  cleanup();
  forgetPaged();
  resetToasts();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_WATCH_VIEW__");
  Reflect.deleteProperty(window, "__AIJ_NOTIFICATION_PREFS__");
});

function seed(state: WatchesPreviewState): void {
  window.__AIJ_WATCH_VIEW__ = { watches: structuredClone(state.watches), catalogue: WATCHES_CATALOGUE, draft: draftWords, made: (draft, id) => made(draft, id, { createdAgo: 0 }), calls: [] };
}

const calls = () => window.__AIJ_WATCH_VIEW__?.calls ?? [];
const rowOf = (id: string): HTMLElement => screen.getAllByTestId("watch-row").find((row) => row.dataset.watchId === id)!;

async function show(state: WatchesPreviewState, props: Partial<Parameters<typeof WatchesPage>[0]> = {}): Promise<HTMLElement> {
  seed(state);
  render(
    <>
      <WatchesPage sources={SOURCES} onGoTo={() => undefined} onAskBot={() => undefined} onQuietHours={() => undefined} {...props} />
      <ToastHost />
    </>,
  );
  await waitFor(() => expect(screen.queryByTestId("watches-skeleton")).toBeNull());
  return screen.getByTestId("watches-page");
}

function plain(text: string): void {
  for (const word of INTERNAL) expect(text, `"${word}" on screen`).not.toMatch(word);
}

describe("AJ-E3816 the Watches list", () => {
  it("lists each watch by its echo, names bold, with its target, chips and state; one primary", async () => {
    const page = await show(STATES.list!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    expect(screen.getByTestId("watches-count").textContent).toBe("6 of 50");
    const [first] = screen.getAllByTestId("watch-row");
    expect(within(first!).getByTestId("watch-echo").textContent).toBe("Tell you when “api” finishes, or looks stuck for 15 minutes.");
    expect(within(first!).getByTestId("watch-echo").querySelector("b")?.textContent).toBe("“api”");
    expect(within(first!).getByTestId("watch-target-line").textContent).toBe("Session · Agent CLI · shop");
    expect(within(first!).getByTestId("watch-lifetime").textContent).toBe("Until it ends");
    expect(within(first!).getByTestId("watch-state").textContent).toContain("Told you 1 time");
    expect(within(first!).getByTestId("watch-state").textContent).toContain("finished");
    const quiet = screen.getAllByTestId("watch-quiet");
    expect(quiet).toHaveLength(1);
    expect(quiet[0]!.textContent).toBe("Even during quiet hours");
    // The paused one sits last, with Resume in place of Edit.
    const last = screen.getAllByTestId("watch-row").at(-1)!;
    expect(last.dataset.state).toBe("paused");
    expect(within(last).getByTestId("watch-resume")).toBeTruthy();
    expect(within(last).queryByTestId("watch-edit")).toBeNull();
    expect(screen.getByTestId("watches-quiet-line").textContent).toContain("except 1 watch set to tell you anyway");
    expect(screen.getByTestId("watches-ask-bot").textContent).toBe("Ask Bot");
    expect(page.textContent).not.toMatch(/Ask Chat/);
    expect(primariesIn(page)).toHaveLength(1);
    plain(page.textContent ?? "");
  });

  it("the full echo is on screen: the head, the lifetime chip and the quiet chip join to the backend's words", async () => {
    await show(STATES.list!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    for (const info of STATES.list!.watches.filter((w) => w.state !== "ended")) {
      const row = document.querySelector(`[data-watch-id='${info.watch.id}']`)!;
      const quiet = row.querySelector("[data-testid=watch-quiet]")?.textContent;
      const words = [row.querySelector("[data-testid=watch-echo]")!.textContent, `${row.querySelector("[data-testid=watch-lifetime]")!.textContent}.`, quiet ? `${quiet}.` : null].filter(Boolean).join(" ");
      expect(words).toBe(info.words);
    }
  });

  it("without quiet hours the line and the menu's quiet switch are hidden", async () => {
    Reflect.set(window, "__AIJ_NOTIFICATION_PREFS__", { quiet_start: 0, quiet_end: 0 });
    await show(STATES.list!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    expect(screen.queryByTestId("watches-quiet-line")).toBeNull();
    fireEvent.click(screen.getAllByTestId("watch-more")[0]!);
    expect(screen.getByTestId("watch-menu")).toBeTruthy();
    expect(screen.queryByTestId("watch-menu-quiet")).toBeNull();
  });

  it("pauses and resumes by the watch's id", async () => {
    await show(STATES.list!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    fireEvent.click(within(rowOf("w-a1")).getByTestId("watch-pause"));
    await waitFor(() => expect(document.querySelector("[data-watch-id='w-a1']")?.getAttribute("data-state")).toBe("paused"));
    expect(calls()).toContainEqual(["watches_pause", { id: "w-a1", paused: true }]);
    fireEvent.click(within(rowOf("w-a1")).getByTestId("watch-resume"));
    await waitFor(() => expect(document.querySelector("[data-watch-id='w-a1']")?.getAttribute("data-state")).toBe("active"));
    expect(calls()).toContainEqual(["watches_pause", { id: "w-a1", paused: false }]);
  });

  it("the menu: Edit, Pause, the quiet switch, Go to, Watch something similar, Remove", async () => {
    const goTo = vi.fn();
    await show(STATES.list!, { onGoTo: goTo });
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    fireEvent.click(screen.getAllByTestId("watch-more")[0]!);
    const menu = screen.getByTestId("watch-menu");
    expect([...menu.querySelectorAll("[data-label]")].map((l) => l.textContent)).toEqual(["Edit", "Pause", "Even during quiet hours", "Go to “api”", "Watch something similar…", "Remove"]);
    expect(menu.textContent).toContain("Stops at once. Undo for a few seconds.");
    fireEvent.click(screen.getByTestId("watch-menu-goto"));
    expect(goTo).toHaveBeenCalledWith({ kind: "session", id: "s-api" });
    fireEvent.click(screen.getAllByTestId("watch-more")[0]!);
    fireEvent.click(screen.getByTestId("watch-menu-quiet"));
    await waitFor(() => expect(calls().some(([name, args]) => name === "watches_update" && Reflect.get(Object(Reflect.get(Object(args), "draft")), "evenInQuietHours") === true)).toBe(true));
  });

  it("removes at once and Undo brings it back, by id", async () => {
    await show(STATES.list!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    fireEvent.click(screen.getAllByTestId("watch-more")[1]!);
    fireEvent.click(screen.getByTestId("watch-menu-remove"));
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(5));
    expect(calls()).toContainEqual(["watches_remove", { id: "w-a2" }]);
    const toast = toastEntries().find((t) => t.id === "watch-removed-w-a2")!;
    expect(toast.options.message).toBe("Watch removed: all of “api” and “web” finish");
    await act(() => undoToast(toast.id));
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    expect(calls()).toContainEqual(["watches_restore", { id: "w-a2" }]);
  });
});

describe("AJ-E3816 Ended", () => {
  it("is folded with its count, then each reason in the code's words with its way forward", async () => {
    await show(STATES.ended!);
    const fold = await screen.findByTestId("watches-ended-fold");
    expect(fold.textContent).toContain("5 · kept for 30 days");
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByTestId("watch-ended-row")).toBeNull();
    fireEvent.click(fold);
    await waitFor(() => expect(screen.getAllByTestId("watch-ended-row")).toHaveLength(5));
    const way = (reason: string) => document.querySelector(`[data-reason='${reason}'] [data-testid=watch-ended-forward]`)?.textContent ?? null;
    const note = (reason: string) => document.querySelector(`[data-reason='${reason}'] [data-testid=watch-ended-note]`)!.textContent;
    expect(note("targets_ended")).toContain("Ended: what it watched has ended.");
    expect(note("targets_ended")).toContain("“api” closed at");
    expect(way("targets_ended")).toBe("Watch Agent CLI instead…");
    expect(way("first_fire")).toBe("Renew");
    expect(way("time_reached")).toBe("Renew for today");
    expect(note("target_gone")).toContain("Ended: what it watched was removed.");
    expect(way("target_gone")).toBe("Pick a new target…");
    // An ended target's note, and the ghost mark.
    expect(document.querySelector("[data-reason='target_gone'] [data-testid=watch-mark-ghost]")).toBeTruthy();
    expect(note("no_longer_offered")).toContain("Agent app two” no longer reports it");
    expect(way("no_longer_offered")).toBeNull();
    plain(screen.getByTestId("watches-page").textContent ?? "");
  });

  it("Renew brings an ended watch back by id", async () => {
    await show(STATES.ended!);
    fireEvent.click(await screen.findByTestId("watches-ended-fold"));
    await waitFor(() => expect(screen.getAllByTestId("watch-ended-row")).toHaveLength(5));
    fireEvent.click(document.querySelector("[data-reason='first_fire'] [data-testid=watch-ended-forward]")!);
    await waitFor(() => expect(calls().some(([name, args]) => name === "watches_update" && Reflect.get(Object(args), "id") === "w-e2")).toBe(true));
  });
});

describe("AJ-E3816 empty and many", () => {
  it("empty: what a watch is, one primary, Ask Bot, and starters that open the form filled in", async () => {
    const page = await show(STATES.empty!);
    expect(await screen.findByTestId("watches-empty")).toBeTruthy();
    expect(page.textContent).toContain("Get told when an agent finishes, asks you or gets stuck");
    expect(screen.getByTestId("watches-empty-bot").textContent).toContain("or ask Bot");
    await waitFor(() => expect(screen.getAllByTestId("watch-starter")).toHaveLength(3));
    expect(primariesIn(page)).toHaveLength(1);
    fireEvent.click(screen.getAllByTestId("watch-starter")[0]!);
    expect(await screen.findByTestId("watch-form")).toBeTruthy();
    await waitFor(() => expect(screen.getByTestId("watch-preview-echo").textContent).toContain("Tell you when “api” finishes. Once."));
    // Nothing was made by the starter.
    expect(calls().filter(([name]) => name === "watches_create")).toHaveLength(0);
  });

  it("without a Bot route there is no Ask Bot", async () => {
    await show(STATES.empty!, { onAskBot: null });
    expect(await screen.findByTestId("watches-empty")).toBeTruthy();
    expect(screen.queryByTestId("watches-empty-bot")).toBeNull();
  });

  it("many: the toolbar, compact rows ten at a time, and the limit only once New watch is pressed", async () => {
    await show(STATES.many!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(10));
    expect(screen.getByTestId("watches-count").textContent).toBe("50 of 50");
    expect(screen.getByTestId("watches-toolbar")).toBeTruthy();
    expect(screen.getByTestId("watches-pager").textContent).toContain("10 of 50 shown");
    expect(screen.queryByTestId("watches-limit")).toBeNull();
    fireEvent.click(screen.getByTestId("watches-more"));
    expect(screen.getAllByTestId("watch-row")).toHaveLength(30);
    fireEvent.click(screen.getByTestId("watches-new"));
    const limit = screen.getByTestId("watches-limit");
    expect(limit.textContent).toContain("50 watches is the most AI Juggler keeps at once.");
    expect(screen.queryByTestId("watch-form")).toBeNull();
    fireEvent.click(screen.getByTestId("watches-limit-show"));
    await waitFor(() => expect(screen.getAllByTestId("watch-row").every((row) => within(row).getByTestId("watch-state").textContent?.includes("never told you"))).toBe(true));
  });
});

describe("AJ-E3816 the form", () => {
  it("New watch: the picker orders Open now, then Recent, then All with counts", async () => {
    await show(STATES.list!);
    fireEvent.click(await screen.findByTestId("watches-new"));
    fireEvent.click(await screen.findByTestId("watch-target-add"));
    const picker = screen.getByTestId("watch-picker");
    const keys = [...picker.querySelectorAll<HTMLElement>("[data-testid=watch-pick-option]")].map((o) => o.dataset.key);
    expect(keys).toEqual(["session:s-api", "session:s-web", "session:s-login", "session:s-draft", "project:proj-shop", "account:acct-work"]);
    expect(within(picker).getByTestId("watch-picker-kind-session").textContent).toContain("4");
    expect(within(picker).getByTestId("watch-picker-kind-app").textContent).toContain("3");
    expect(within(picker).getByTestId("watch-picker-count").textContent).toBe("0 picked · up to 10");
  });

  it("only what every chosen target's App can report is offered (D-127), and the cost sits by the button", async () => {
    await show(STATES.list!);
    act(() => openWatches({ kind: "new", target: { kind: "session", id: "s-draft" } }));
    await waitFor(() => expect(screen.getByTestId("watch-condition-finishes")).toBeTruthy());
    fireEvent.click(screen.getByTestId("watch-conditions-more"));
    const codes = [...screen.getByTestId("watch-conditions").querySelectorAll<HTMLElement>("[data-testid^=watch-condition-]")].map((c) => c.dataset.testid);
    expect(codes).not.toContain("watch-condition-stuck");
    expect(codes).not.toContain("watch-condition-idle_for");
    expect(codes).toContain("watch-condition-asks_you");
    expect(screen.getByTestId("watch-form").textContent).not.toMatch(/unsupported|can't report/i);
    expect(screen.getByTestId("watch-form-consequence").textContent).toBe("Starts watching now · 7 of 50.");
    expect(screen.getByTestId("watch-preview-footer").textContent).toBe("A watch only tells you. It never sends, stops or approves.");
    expect(primariesIn(screen.getByTestId("watch-form"))).toHaveLength(1);
  });

  it("the live echo is the backend's words for the draft, and Watch makes it", async () => {
    await show(STATES.list!);
    act(() => openWatches({ kind: "new", target: { kind: "session", id: "s-api" } }));
    await waitFor(() => expect(screen.getByTestId("watch-condition-stuck")).toBeTruthy());
    fireEvent.click(screen.getByTestId("watch-condition-stuck").querySelector("input")!);
    // Finishes was picked already; with Looks stuck the suggestion is Today.
    const expected = "Tell you when “api” finishes, or looks stuck for 15 minutes. Until midnight today.";
    await waitFor(() => expect(screen.getByTestId("watch-preview-echo").textContent).toBe(expected));
    expect(screen.getByTestId("watch-preview-ends").textContent).toBe("Ends at midnight tonight.");
    fireEvent.click(screen.getByTestId("watch-form-save"));
    await waitFor(() => expect(screen.queryByTestId("watch-form")).toBeNull());
    const created: WatchDraftInfo = JSON.parse(JSON.stringify(calls().find(([name]) => name === "watches_create")?.[1]));
    expect(created.targets).toEqual([{ kind: "session", id: "s-api" }]);
    expect(created.conditions).toEqual([{ code: "finishes" }, { code: "stuck", param: { kind: "minutes", value: 15 } }]);
    // The backend's own words for what was made: the shared cases hold the stand-in to them.
    expect(draftWords(created).words).toBe(expected);
  });

  it("Edit: a second session shows For each / When all, the Was line, and saves by id", async () => {
    await show(STATES.edit!);
    await waitFor(() => expect(screen.getAllByTestId("watch-row")).toHaveLength(6));
    fireEvent.click(screen.getAllByTestId("watch-edit")[0]!);
    expect(await screen.findByTestId("watch-form")).toBeTruthy();
    expect(screen.queryByTestId("watch-matching")).toBeNull();
    fireEvent.click(screen.getByTestId("watch-target-add"));
    fireEvent.click(document.querySelector("[data-key='session:s-web']")!);
    expect(screen.getByTestId("watch-matching")).toBeTruthy();
    await waitFor(() => expect(screen.getByTestId("watch-preview-was").textContent).toContain("Tell you when “api” finishes, or looks stuck for 15 minutes. Until it ends."));
    await waitFor(() => expect(screen.getByTestId("watch-preview-echo").textContent).toBe("Tell you when “api” or “web” finishes, or looks stuck for 15 minutes. Until they end."));
    expect(screen.getByTestId("watch-form-consequence").textContent).toBe("Changes it at once. Its history stays.");
    expect(screen.getByTestId("watch-form-remove")).toBeTruthy();
    fireEvent.click(screen.getByTestId("watch-form-save"));
    await waitFor(() => expect(calls().some(([name, args]) => name === "watches_update" && Reflect.get(Object(args), "id") === "w-a1")).toBe(true));
  });

  it("Until it ends is offered only when every target is a session", async () => {
    await show(STATES.list!);
    act(() => openWatches({ kind: "new", target: { kind: "app", id: "p-cli" } }));
    await waitFor(() => expect(screen.getByTestId("watch-lifetime-choice")).toBeTruthy());
    expect(screen.queryByTestId("watch-lifetime-choice-until_ends")).toBeNull();
    expect(screen.getByTestId("watch-lifetime-choice-today").getAttribute("aria-checked")).toBe("true");
  });
});
