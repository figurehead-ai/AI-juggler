/**
 * AJ-E1107 vitest — the crash-report setting and the next-launch
 * question, through the preview seam.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { CrashReportPrompt, CrashReportsSetting } from "./CrashReports";
import type { CrashInfo } from "../lib/bridge";

type Seams = { __AIJ_CRASH__?: CrashInfo; __AIJ_CRASH_ANSWERS__?: string[] };
const w = () => window as unknown as Seams;

const REPORT = {
  id: "a1b2c3d4e5f60718",
  at_ms: 1,
  signature: "9f2c1a0b7e3d4c5f",
  message: "called `Option::unwrap()` on a `None` value at ~/projects/demo",
  location: "crates/juggler-app/src/commands.rs:4211",
  frames: ["0: juggler_app::commands::switch_execute_core", "1: ~/Dev/AIJuggler/crates/juggler-app/src/commands.rs:4211"],
  app_version: "0.3.1",
  channel: "beta",
  os: "macos",
};

beforeEach(() => {
  delete w().__AIJ_CRASH__;
  delete w().__AIJ_CRASH_ANSWERS__;
});
afterEach(() => cleanup());

describe("AJ-E1107 crash reports", () => {
  it("Never by default; Ask keeps reports here while this build can't send", async () => {
    const { findByTestId, getByTestId, queryByTestId } = render(<CrashReportsSetting />);
    const select = (await findByTestId("crash-setting")) as HTMLSelectElement;
    expect(select.value).toBe("never");
    expect(queryByTestId("crash-cannot-send")).toBeNull();
    fireEvent.change(select, { target: { value: "ask" } });
    await waitFor(() => expect(getByTestId("crash-cannot-send").textContent).toContain("can't send crash reports yet"));
  });

  it("DO_NOT_TRACK forces Never and disables the choice", async () => {
    w().__AIJ_CRASH__ = {
      setting: "always",
      effective: "never",
      reason: "Never, because DO_NOT_TRACK is set",
      asking: [],
      queued: 0,
      can_send: false,
    };
    const { findByTestId, getByTestId } = render(<CrashReportsSetting />);
    const select = (await findByTestId("crash-setting")) as HTMLSelectElement;
    expect(select.disabled).toBe(true);
    expect(select.value).toBe("never");
    expect(getByTestId("privacy-crash-reports").textContent).toContain("DO_NOT_TRACK");
  });

  it("the next launch shows the whole report and asks: Send, Don't send, Always send", async () => {
    w().__AIJ_CRASH__ = { setting: "ask", effective: "ask", reason: null, asking: [REPORT], queued: 0, can_send: false };
    const { findByTestId, getByTestId, container } = render(<CrashReportPrompt />);
    const preview = await findByTestId("crash-preview");
    expect(preview.textContent).toContain("~/projects/demo");
    expect(preview.textContent).toContain('"signature"');
    expect(getByTestId("crash-prompt").textContent).toContain("AI Juggler quit unexpectedly. Send a crash report?");
    fireEvent.click(getByTestId("crash-always-send"));
    await waitFor(() => expect(container.querySelector("[data-testid='crash-prompt']")).toBeNull());
    expect(w().__AIJ_CRASH_ANSWERS__).toEqual([`${REPORT.id}:always_send`]);
    expect(w().__AIJ_CRASH__?.setting).toBe("always");
  });

  it("asks nothing under Never or Always", async () => {
    w().__AIJ_CRASH__ = { setting: "always", effective: "always", reason: null, asking: [REPORT], queued: 0, can_send: false };
    const { container } = render(<CrashReportPrompt />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(container.querySelector("[data-testid='crash-prompt']")).toBeNull();
  });
});
