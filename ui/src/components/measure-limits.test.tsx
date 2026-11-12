/**
 * 0.4.4 (main's finding): "Measure its limits" only opened a page. It now
 * measures where it is, through D-39's ask (the tool's own usage check):
 * on, it asks and reads the limits again; turned off, it says so with the
 * way to turn it on; a tool with no such check gets no button, and says
 * how its limits will come.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as probe from "../lib/ipc/probe";
import { quotaDisplays } from "../lib/kept-reads";
import { OPEN_APP } from "../lib/apps";
import { rememberedTabId } from "../pages/registry";
import { MeasureLimits } from "./MeasureLimits";
import type { ProbeStatusInfo } from "../lib/ipc/probe";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const status = (over: Partial<ProbeStatusInfo> = {}): ProbeStatusInfo => ({ offered: true, state: "on", last_run_ms: null, last_outcome: null, ...over });
const statuses = (by: Record<string, ProbeStatusInfo>) => vi.spyOn(probe, "probeStatus").mockImplementation(async (id) => by[id] ?? status({ offered: false, state: "off" }));

describe("Measure its limits", () => {
  it("on: a click asks the tool here, says Measuring…, then reads the limits again", async () => {
    statuses({ "cc-side": status({ state: "off" }), "cc-work": status() });
    let answer: (value: ProbeStatusInfo) => void = () => {};
    const run = vi.spyOn(probe, "probeRun").mockImplementation(() => new Promise((done) => (answer = done)));
    const refresh = vi.spyOn(quotaDisplays, "refresh").mockResolvedValue([]);
    const measured = vi.fn();
    render(<MeasureLimits folders={["cc-side", "cc-work"]} tool="claude-code" name="Work" onMeasured={measured} readAgainMs={[0, 0]} />);
    const button = await screen.findByTestId("measure-limits-run");
    expect(button.textContent).toBe("Measure its limits › — Work");
    fireEvent.click(button);
    // The folder it's on for, of the Account's two; nothing else opens.
    expect(run).toHaveBeenCalledWith("cc-work");
    await waitFor(() => expect(screen.getByTestId("measure-limits-run").textContent).toBe("Measuring… — Work"));
    expect(screen.getByTestId("measure-limits-run").hasAttribute("disabled")).toBe(true);
    answer(status({ last_run_ms: 1, last_outcome: "ran" }));
    await waitFor(() => expect(measured).toHaveBeenCalledTimes(2));
    expect(refresh).toHaveBeenCalledTimes(2);
    // Still shown (the reading isn't in yet): it says so, calmly.
    expect((await screen.findByTestId("measure-limits-said")).textContent).toBe("Claude Code answered; its limits show here once they're in.");
    expect(screen.getByTestId("measure-limits-run").hasAttribute("disabled")).toBe(false);
  });

  it("on, as Home's Next steps' button", async () => {
    statuses({ "cc-work": status() });
    render(<MeasureLimits look="button" folders={["cc-work"]} tool="claude-code" name="Work" />);
    expect(await screen.findByRole("button", { name: "Measure its limits — Work" })).toBeTruthy();
  });

  it("a failed ask says why, in plain words", async () => {
    statuses({ "cc-work": status() });
    vi.spyOn(quotaDisplays, "refresh").mockResolvedValue([]);
    const said: string[] = [];
    for (const outcome of ["failed", "timed_out", "not_started"]) {
      vi.spyOn(probe, "probeRun").mockResolvedValueOnce(status({ last_run_ms: 1, last_outcome: outcome }));
      const view = render(<MeasureLimits folders={["cc-work"]} tool="claude-code" name="Work" readAgainMs={[0]} />);
      fireEvent.click(await view.findByTestId("measure-limits-run"));
      said.push((await view.findByTestId("measure-limits-said")).textContent ?? "");
      view.unmount();
    }
    expect(said).toEqual([
      "Couldn't measure: Claude Code couldn't check (signed out, or offline).",
      "Couldn't measure: it took too long and was stopped.",
      "Couldn't measure: Claude Code couldn't be started.",
    ]);
  });

  it("turned off for it: says so, and Turn on opens its App's settings", async () => {
    statuses({ "cc-work": status({ state: "off" }) });
    const run = vi.spyOn(probe, "probeRun");
    const opened: string[] = [];
    const listen = (event: Event) => opened.push(event instanceof CustomEvent ? String(event.detail.id) : "");
    window.addEventListener(OPEN_APP, listen);
    render(<MeasureLimits folders={["cc-work"]} tool="claude-code" name="Work" />);
    const off = await screen.findByTestId("measure-limits-off");
    expect(off.textContent).toBe("Asking Claude Code for its limits is off here · Turn on › — Work");
    expect(screen.queryByTestId("measure-limits-run")).toBeNull();
    fireEvent.click(screen.getByTestId("measure-limits-turn-on"));
    window.removeEventListener(OPEN_APP, listen);
    expect(opened).toEqual(["cli:cc-work"]);
    expect(rememberedTabId("app")).toBe("settings");
    expect(run).not.toHaveBeenCalled();
  });

  it("a tool with no such check: no button, and how its limits will come", async () => {
    statuses({});
    render(<MeasureLimits folders={["codex-work"]} tool="codex" name="Work" />);
    expect((await screen.findByTestId("measure-limits-how")).textContent).toBe("They show once a session on it reports them.");
    expect(screen.queryByRole("button")).toBeNull();
  });
});
