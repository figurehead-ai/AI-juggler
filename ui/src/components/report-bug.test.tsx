/**
 * AJ-E2911 vitest — Report a bug: it opens from anywhere with what the
 * failure showed; every part is shown and can be left out; the form opens
 * with exactly what's kept; Copy and the debug section work.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ReportBugHost } from "./ReportBugSheet";
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
