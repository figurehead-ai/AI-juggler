/**
 * AJ-E2911 vitest — Report a bug: it opens from anywhere with what the
 * failure showed; every part is shown and can be left out; the form opens
 * with exactly what's kept; Copy and the debug section work.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { goesWithWords, inSentence, ReportBugHost } from "./ReportBugSheet";
import { closeReportBug, openReportBug } from "../lib/report-bug";
import { DebugModeSection, debugWords } from "../screens/settings/DebugModeSection";
import type { BugComposedInfo, DebugModeInfo } from "../lib/ipc/report";

const w = () =>
  window as unknown as { __AIJ_REPORT_OPENED__?: BugComposedInfo; __AIJ_DEBUG__?: DebugModeInfo; __AIJ_DEBUG_LOG__?: string };

afterEach(() => {
  cleanup();
  closeReportBug();
});
beforeEach(() => {
  delete w().__AIJ_REPORT_OPENED__;
  delete w().__AIJ_DEBUG__;
  delete w().__AIJ_DEBUG_LOG__;
});

describe("AJ-E2911 — Report a bug", () => {
  it("is closed until something asks", () => {
    const { queryByTestId } = render(<ReportBugHost />);
    expect(queryByTestId("report-bug")).toBeNull();
  });

  it("shows every part, and the form gets only what's kept", async () => {
    const { findAllByTestId, getByTestId } = render(<ReportBugHost />);
    act(() => openReportBug({ from: "switch", words: "The switch stopped" }));
    const parts = await findAllByTestId("report-part");
    expect(parts.map((p) => p.textContent)).toEqual([
      expect.stringContaining("What AI Juggler showed"),
      expect.stringContaining("AI Juggler and this computer"),
      expect.stringContaining("The last switch"),
      expect.stringContaining("Recent errors"),
    ]);
    fireEvent.change(getByTestId("report-words"), { target: { value: "I switched to Codex" } });
    fireEvent.click(getByTestId("report-keep-errors"));
    await waitFor(() => expect((getByTestId("report-open") as HTMLButtonElement).disabled).toBe(false));
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
      fireEvent.click(getByTestId("report-open"));
    });
    const sent = w().__AIJ_REPORT_OPENED__!;
    expect(sent.title).toBe("A switch failed");
    const what = sent.fields.find((f) => f.id === "what-happened")!.value;
    expect(what.startsWith("I switched to Codex")).toBe(true);
    const logs = sent.fields.find((f) => f.id === "logs")!.value;
    expect(logs).toContain("The last switch");
    expect(logs).not.toContain("Recent errors");
    expect(getByTestId("report-status").textContent).toContain("open in your browser");
  });
});

describe("design review 28/33/38/56 — the parts fold to one line", () => {
  it("one closed line names what goes with it; unticking a part drops it from the line", async () => {
    const { findByTestId, getByTestId, getAllByTestId } = render(<ReportBugHost />);
    act(() => openReportBug({ from: "switch", words: "The switch stopped" }));
    const fold = (await findByTestId("report-parts")) as HTMLDetailsElement;
    expect(fold.open).toBe(false);
    const line = fold.querySelector("summary")!;
    expect(line.textContent).toBe("Goes with it: what AI Juggler showed, AI Juggler and this computer, the last switch, recent errors · Change");
    // Plain rows: no bordered card and no "Show" per part; the row is the fold.
    for (const part of getAllByTestId("report-part")) {
      expect(part.className).not.toContain("border");
      const row = part.querySelector("details summary")!;
      expect(row.textContent).not.toMatch(/\bShow\b/);
      expect(row.textContent).toContain(" · ");
    }
    fireEvent.click(getByTestId("report-keep-errors"));
    expect(line.textContent).not.toContain("recent errors");
  });

  it("the fields use the standard field surface, not the page's", async () => {
    const { findByTestId, getByTestId } = render(<ReportBugHost />);
    act(() => openReportBug({ from: "switch", words: "x" }));
    await findByTestId("report-parts");
    for (const id of ["report-title", "report-words"]) {
      expect(getByTestId(id).className).toContain("bg-surface-2");
      expect(getByTestId(id).className).not.toContain("bg-canvas");
    }
  });

  it("words for the line", () => {
    const parts = [
      { key: "a", title: "The last switch", about: "", text: "" },
      { key: "b", title: "AI Juggler and this computer", about: "", text: "" },
    ];
    expect(inSentence("AI Juggler and this computer")).toBe("AI Juggler and this computer");
    expect(goesWithWords(parts, ["a", "b"])).toBe("Goes with it: the last switch, AI Juggler and this computer");
    expect(goesWithWords(parts, [])).toBe("Nothing else goes with it");
  });
});

describe("AJ-E2912 — Debug mode", () => {
  it("says when it turns itself off, and clears in one click", async () => {
    w().__AIJ_DEBUG__ = { on: false, until_ms: null, bytes: 2048, lines: 12 };
    w().__AIJ_DEBUG_LOG__ = "1793831000000 switch start pair=claude>codex";
    const { findByTestId, getByTestId } = render(<DebugModeSection />);
    const row = await findByTestId("debug-mode");
    expect(row.textContent).toContain("Never what's in a conversation");
    await act(async () => {
      fireEvent.click(getByTestId("debug-mode-switch"));
    });
    expect(w().__AIJ_DEBUG__?.on).toBe(true);
    expect(getByTestId("debug-mode").textContent).toContain("then off by itself");
    // rc26 X2b: said once, not again in a note ("On for seven days.").
    expect(getByTestId("debug-mode").textContent).not.toMatch(/seven days/i);
    await act(async () => {
      fireEvent.click(getByTestId("debug-log-show"));
    });
    expect(getByTestId("debug-log-text").textContent).toContain("pair=claude>codex");
    await act(async () => {
      fireEvent.click(getByTestId("debug-log-clear"));
    });
    expect(getByTestId("debug-log-row").textContent).toContain("Empty.");
  });

  it("words for off and on", () => {
    expect(debugWords({ on: false, until_ms: null, bytes: 0, lines: 0 })).toContain("each step");
    expect(debugWords({ on: true, until_ms: Date.UTC(2026, 10, 11), bytes: 0, lines: 0 })).toMatch(/^On until/);
  });
});

describe("AJ-E2911 — from an error", () => {
  it("an error toast with no action of its own offers Report a bug with its words", async () => {
    const { ToastHost, showToast, resetToasts } = await import("./Toast");
    const { useReportBug } = await import("../lib/report-bug");
    const { renderHook } = await import("@testing-library/react");
    resetToasts();
    const asked = renderHook(() => useReportBug());
    const view = render(<ToastHost />);
    act(() => void showToast({ tone: "error", message: "The session couldn't start.", detail: "claude exited with code 1" }));
    act(() => fireEvent.click(view.getByRole("button", { name: "Report a bug…" })));
    expect(asked.result.current).toMatchObject({ from: "error", words: "The session couldn't start.", detail: "claude exited with code 1" });
    resetToasts();
  });
});

describe("AJ-E2911 R1 — the other doors", () => {
  it("a launch that fails in New session offers Report a bug… with that failure", async () => {
    const { NewSessionModal } = await import("../screens/NewSession");
    const launch = await import("../lib/ipc/launch");
    const { useReportBug } = await import("../lib/report-bug");
    const { renderHook } = await import("@testing-library/react");
    const { vi } = await import("vitest");
    const { account, clearSeams, recipe, setSeam } = await import("../test/fixtures");
    const recipes = [recipe({ id: "p-1", account_id: "claude-work", account_label: "Work Max", default_cwd: "/Users/dev/api" })];
    setSeam("__AIJ_RECIPES__", recipes);
    const failing = vi.spyOn(launch, "launchSession").mockRejectedValue({ code: "adapter", retryable: true, user_message: "Claude Code didn't start: it exited with code 1.", detail: "stage: launch; claude exited with code 1" });
    try {
      const asked = renderHook(() => useReportBug());
      const accounts = [account({ id: "claude-work", label: "Work Max" })];
      const view = render(<NewSessionModal projects={[]} accounts={accounts} recipes={recipes} initialDirectory="/Users/dev/api" initialAccount="claude-work" onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);
      await act(async () => {
        fireEvent.click(view.getByTestId("launch-now"));
      });
      const alert = await view.findByTestId("launch-error");
      expect(alert.getAttribute("role")).toBe("alert");
      expect(alert.textContent).toContain("Claude Code didn't start: it exited with code 1.");
      expect(asked.result.current).toBeNull();
      act(() => {
        fireEvent.click(view.getByTestId("launch-report-bug"));
      });
      expect(view.getByTestId("launch-report-bug").textContent).toBe("Report a bug…");
      expect(asked.result.current).toMatchObject({ from: "launch", words: "Claude Code didn't start: it exited with code 1.", detail: "stage: launch; claude exited with code 1", code: "adapter" });
      expect(typeof asked.result.current?.at_ms).toBe("number");
    } finally {
      failing.mockRestore();
      clearSeams("__AIJ_RECIPES__");
    }
  });

  it("Help › Report a bug… in the menu bar opens it, with no failure", async () => {
    const report = await import("../lib/ipc/report");
    const { vi } = await import("vitest");
    let fromMenu: (() => void) | null = null;
    const listening = vi.spyOn(report, "onReportBugAsked").mockImplementation((on) => {
      fromMenu = on;
      return () => {
        fromMenu = null;
      };
    });
    const { queryByTestId, findByTestId } = render(<ReportBugHost />);
    expect(listening).toHaveBeenCalledTimes(1);
    expect(queryByTestId("report-bug")).toBeNull();
    act(() => {
      fromMenu?.();
    });
    expect(await findByTestId("report-bug")).toBeTruthy();
    expect(report.REPORT_BUG_ASKED).toBe("app://report-bug");
    listening.mockRestore();
  });
});

describe("AJ-E2911 R4 — Copy, and logs cut to fit", () => {
  it("Copy puts the whole report on the clipboard", async () => {
    const { vi } = await import("vitest");
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    const { getByTestId, findAllByTestId } = render(<ReportBugHost />);
    act(() => openReportBug({ from: "switch", words: "The switch stopped" }));
    await findAllByTestId("report-part");
    fireEvent.change(getByTestId("report-words"), { target: { value: "I switched to Codex" } });
    const { asButton } = await import("../test/fixtures");
    await waitFor(() => expect(asButton(getByTestId("report-copy")).disabled).toBe(false));
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
      fireEvent.click(getByTestId("report-copy"));
    });
    expect(writeText).toHaveBeenCalledTimes(1);
    const text = String(writeText.mock.calls.at(0)?.at(0));
    expect(text.startsWith("A switch failed\n\n### What happened?")).toBe(true);
    expect(text).toContain("I switched to Codex");
    for (const part of ["The switch stopped before the new session started.", "# The last switch", "# Recent errors", "### AI Juggler version"]) expect(text).toContain(part);
    expect(getByTestId("report-status").textContent).toContain("Copied.");
  });

  it("says when the form's logs were cut to fit its link, and that Copy has them all", async () => {
    const report = await import("../lib/ipc/report");
    const { vi } = await import("vitest");
    const real = report.bugReportCompose;
    const cut = vi.spyOn(report, "bugReportCompose").mockImplementation(async (...args) => ({ ...(await real(...args)), cut: true }));
    const { findByTestId } = render(<ReportBugHost />);
    act(() => openReportBug({ from: "switch", words: "The switch stopped" }));
    expect((await findByTestId("report-cut")).textContent).toBe("The logs are cut to fit a link, oldest lines first. Copy has all of them.");
    cut.mockRestore();
  });
});

describe("AJ-E2912 R1 — Clear says so in place", () => {
  it("Clear the log empties it and says “Cleared.” where it was", async () => {
    w().__AIJ_DEBUG__ = { on: true, until_ms: Date.now() + 86_400_000, bytes: 4096, lines: 30 };
    const { findByTestId, getByTestId } = render(<DebugModeSection />);
    await findByTestId("debug-log-row");
    expect(getByTestId("debug-log-row").textContent).not.toContain("Cleared.");
    await act(async () => {
      fireEvent.click(getByTestId("debug-log-clear"));
    });
    await waitFor(() => expect(getByTestId("debug-mode").textContent).toContain("Cleared."));
    expect(getByTestId("debug-log-row").textContent).toContain("Empty.");
    const { asButton } = await import("../test/fixtures");
    expect(asButton(getByTestId("debug-log-clear")).disabled).toBe(true);
  });
});
