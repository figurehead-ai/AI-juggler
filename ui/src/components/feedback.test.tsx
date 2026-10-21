import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushHeld, resetToasts, showToast, ToastHost, undoLatest } from "./Toast";
import { QuitSheet, quitEffectWords, runningWords } from "./QuitSheet";
import { RowPlaceholders } from "./Placeholder";
import { WorkflowRail } from "./WorkflowRail";
import { HomeScreen } from "../screens/Home";
import { holdStop, STOP_HOLD_SECONDS } from "../lib/stop";
import type { InstanceInfo, QuitPlanInfo } from "../lib/bridge";

/**
 * AJ-E1503 — one feedback system:
 *  - a held Stop is sent only after its 5 s window, and never when
 *    Undo comes first (undo-before-send, send-after-window);
 *  - toasts are announced through a polite live region and never take
 *    focus from what has it;
 *  - the quit sheet's words for 1 and n live sessions (0 never asks:
 *    the backend quits at once, quit.rs tests);
 *  - loading lists show row-shaped placeholders, not a false "empty".
 */

type Seams = { __AIJ_TERMINAL_KILL_CALLS__?: string[]; __AIJ_QUIT_CALLS__?: string[] };
const seams = () => window as unknown as Seams;

beforeEach(() => {
  delete seams().__AIJ_TERMINAL_KILL_CALLS__;
  delete seams().__AIJ_QUIT_CALLS__;
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  resetToasts();
  vi.useRealTimers();
});

const NOW = Date.now();
function session(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-1",
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 },
    cpu_pct: 1,
    rss_bytes: 1,
    child_count: 0,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 1_000,
    machine_id: "this-mac",
    cwd: "/tmp/api",
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
    ...overrides,
  } as InstanceInfo;
}

describe("AJ-E1503 held stop", () => {
  it("sends the stop only once the 5 s window has passed", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    const after = vi.fn();
    act(() => holdStop({ id: "sess-1", label: "api · Work Max" }, after));
    expect(screen.getByTestId("stop-undo-toast").textContent).toContain("Stopping api · Work Max.");
    await act(async () => {
      vi.advanceTimersByTime(STOP_HOLD_SECONDS * 1000 - 1);
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toBeUndefined();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toEqual(["sess-1"]);
    expect(after).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("stop-undo-toast")).toBeNull();
  });

  it("never sends the stop when Undo comes first", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(() => holdStop({ id: "sess-1", label: "Work Max" }));
    await act(async () => {
      vi.advanceTimersByTime(4_000);
    });
    expect(screen.getByTestId("stop-undo").textContent).toContain("Undo (1s)");
    await act(async () => {
      fireEvent.click(screen.getByTestId("stop-undo"));
    });
    expect(screen.getByTestId("toast-undone").textContent).toBe("Work Max keeps running.");
    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toBeUndefined();
  });

  it("holding the same session twice sends one stop", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(() => {
      holdStop({ id: "sess-1", label: "Work Max" });
      holdStop({ id: "sess-1", label: "Work Max" });
    });
    await act(async () => {
      vi.advanceTimersByTime(5_000);
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toEqual(["sess-1"]);
  });

  it("closing the toast sends at once, and quitting flushes what is held", async () => {
    render(<ToastHost />);
    act(() => {
      holdStop({ id: "a", label: "A" });
      holdStop({ id: "b", label: "B" });
    });
    await act(async () => {
      fireEvent.click(screen.getAllByRole("button", { name: "Do it now" })[0]!);
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toEqual(["a"]);
    await act(async () => {
      await flushHeld();
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toEqual(["a", "b"]);
  });

  it("⌘Z undoes the newest toast that still can be", async () => {
    render(<ToastHost />);
    act(() => holdStop({ id: "sess-1", label: "Work Max" }));
    let undid = false;
    await act(async () => {
      undid = undoLatest();
    });
    expect(undid).toBe(true);
    await act(async () => {
      await flushHeld();
    });
    expect(seams().__AIJ_TERMINAL_KILL_CALLS__).toBeUndefined();
    expect(undoLatest()).toBe(false);
  });
});

describe("AJ-E1503 toasts and screen readers", () => {
  it("announces through the polite region that is always in the page", () => {
    render(<ToastHost />);
    const region = screen.getByTestId("toast-announcer");
    expect(region.getAttribute("role")).toBe("status");
    expect(region.getAttribute("aria-live")).toBe("polite");
    act(() => void showToast({ message: "Marked api reviewed.", undo: { onUndo: () => {} } }));
    expect(region.textContent).toBe("Marked api reviewed. Undo is available for 5 seconds.");
    // The countdown ticks in the toast, never in the live region.
    expect(region.contains(screen.getByRole("button", { name: /Undo/ }))).toBe(false);
  });

  it("never takes focus from what has it", () => {
    render(
      <>
        <button type="button">Elsewhere</button>
        <ToastHost />
      </>,
    );
    const elsewhere = screen.getByRole("button", { name: "Elsewhere" });
    elsewhere.focus();
    act(() => void showToast({ message: "Stopped.", focusWhenFree: true, undo: { onUndo: () => {} } }));
    expect(document.activeElement).toBe(elsewhere);
  });

  it("a failed undo says so and stays until closed", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(
      () =>
        void showToast({
          message: "Archived.",
          undo: {
            onUndo: () => Promise.reject("the session is gone"),
            failed: (cause) => `${cause} — nothing changed.`,
          },
        }),
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Undo/ }));
    });
    const toast = screen.getByTestId("toast");
    expect(toast.textContent).toContain("Undo didn’t complete");
    expect(toast.textContent).toContain("the session is gone — nothing changed.");
    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByTestId("toast").textContent).toContain("Undo didn’t complete");
  });
});

describe("AJ-E1503 quit warning", () => {
  const plan = (live: number, end_on_quit: number, keep_running: number): QuitPlanInfo => ({ live, end_on_quit, keep_running });

  it("says how many are running and what quitting does", () => {
    expect(runningWords(plan(1, 1, 0))).toBe("1 session is running.");
    expect(quitEffectWords(plan(1, 1, 0))).toBe("It ends when AI Juggler quits.");
    expect(runningWords(plan(3, 3, 0))).toBe("3 sessions are running.");
    expect(quitEffectWords(plan(3, 3, 0))).toBe("They end when AI Juggler quits.");
    expect(quitEffectWords(plan(1, 0, 1))).toBe("It keeps running after AI Juggler quits.");
    expect(quitEffectWords(plan(3, 2, 1))).toBe("2 end when AI Juggler quits; 1 keeps running.");
  });

  // Pinned to the backend's kinds (quit.rs `each_kind_is_pinned_…`): a
  // terminal session lands in end_on_quit, a desktop or re-adopted one in
  // keep_running.
  it("pins each wording to its session kind", () => {
    const terminal = plan(1, 1, 0);
    const desktopOrAdopted = plan(1, 0, 1);
    expect(quitEffectWords(terminal)).toMatch(/ends when AI Juggler quits/);
    expect(quitEffectWords(terminal)).not.toMatch(/keeps running/);
    expect(quitEffectWords(desktopOrAdopted)).toMatch(/keeps running/);
    expect(quitEffectWords(desktopOrAdopted)).not.toMatch(/ends/);
    expect(quitEffectWords(plan(2, 0, 2))).toBe("They keep running after AI Juggler quits.");
    expect(quitEffectWords(plan(2, 1, 1))).toBe("1 ends when AI Juggler quits; 1 keeps running.");
  });

  it("shows only when asked, and quits only from a click", async () => {
    render(<QuitSheet />);
    expect(screen.queryByTestId("quit-sheet")).toBeNull();
    act(() => {
      window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: plan(3, 3, 0) }));
    });
    expect(screen.getByTestId("quit-running").textContent).toBe("3 sessions are running.");
    expect(screen.getByRole("dialog", { name: "Quit AI Juggler?" })).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByTestId("quit-anyway"));
    expect(seams().__AIJ_QUIT_CALLS__).toBeUndefined();
    await act(async () => {
      fireEvent.click(screen.getByTestId("quit-stop-first"));
    });
    expect(seams().__AIJ_QUIT_CALLS__).toEqual(["stop-first"]);
  });

  it("Cancel and Escape leave everything running", () => {
    render(<QuitSheet />);
    act(() => {
      window.dispatchEvent(new CustomEvent("aij:quit-requested", { detail: plan(1, 1, 0) }));
    });
    expect(screen.getByTestId("quit-stop-first").textContent).toBe("Stop it first");
    fireEvent.keyDown(screen.getByTestId("quit-anyway"), { key: "Escape" });
    expect(screen.queryByTestId("quit-sheet")).toBeNull();
    expect(seams().__AIJ_QUIT_CALLS__).toBeUndefined();
  });
});

describe("AJ-E1503 loading placeholders", () => {
  it("are announced once as loading, with rows hidden from screen readers", () => {
    render(<RowPlaceholders shape="rail" rows={4} label="Loading sessions" />);
    const status = screen.getByRole("status", { name: "Loading sessions" });
    expect(status.getAttribute("aria-busy")).toBe("true");
    expect(status.querySelectorAll("[aria-hidden='true']").length).toBe(4);
  });

  it("the rail shows placeholders while loading, not \"No sessions yet\"", () => {
    const props = {
      density: "comfortable" as const,
      onCycleDensity: () => {},
      projects: [],
      sessions: [],
      loginTerminals: [],
      selectedId: null,
      onSelectSession: () => {},
      onSelectProject: () => {},
      onNewSession: () => {},
      onNewProject: () => {},
      header: () => null,
      footer: () => null,
    };
    const { rerender } = render(<WorkflowRail {...props} loading />);
    expect(screen.getByTestId("placeholder-rail")).toBeTruthy();
    expect(screen.queryByTestId("rail-empty")).toBeNull();
    rerender(<WorkflowRail {...props} loading={false} />);
    expect(screen.queryByTestId("placeholder-rail")).toBeNull();
    expect(screen.getByTestId("rail-empty")).toBeTruthy();
  });

  it("Home shows placeholders while loading, not \"Bring in your accounts\"", () => {
    const props = {
      sessions: [],
      accounts: [],
      projects: [],
      now: NOW,
      onOpenSession: () => {},
      onNewSession: () => {},
      onAddAccounts: () => {},
    };
    const { rerender } = render(<HomeScreen {...props} loading />);
    expect(screen.getByTestId("home-loading")).toBeTruthy();
    expect(screen.queryByTestId("home-empty-accounts")).toBeNull();
    rerender(<HomeScreen {...props} loading={false} />);
    expect(screen.getByTestId("home-empty-accounts")).toBeTruthy();
  });
});

describe("AJ-E1503 snooze with Undo", () => {
  it("snoozing says so, and Undo brings the row's earlier state back", async () => {
    render(
      <>
        <WorkflowRail
          density="comfortable"
          onCycleDensity={() => {}}
          projects={[]}
          sessions={[session()]}
          loginTerminals={[]}
          selectedId={null}
          onSelectSession={() => {}}
          onSelectProject={() => {}}
          onNewSession={() => {}}
          onNewProject={() => {}}
          header={() => null}
          footer={() => null}
        />
        <ToastHost />
      </>,
    );
    fireEvent.click(screen.getAllByRole("button", { name: "Row actions for Work Max" })[0]!);
    fireEvent.click(screen.getByRole("menuitem", { name: "Snooze 1h" }));
    expect(screen.getByTestId("toast").textContent).toContain("Snoozed Work Max for 1 h.");
    expect(JSON.parse(localStorage.getItem("aij.snooze") ?? "{}")["sess-1"]).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Undo/ }));
    });
    expect(JSON.parse(localStorage.getItem("aij.snooze") ?? "{}")["sess-1"]).toBeUndefined();
  });
});
