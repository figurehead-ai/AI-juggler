/**
 * 0.4.4 (main's finding): "Measure its limits" only opened a page. It now
 * measures where it is, through D-39's ask (the tool's own usage check):
 * on, it asks and reads the limits again; turned off, it says so with the
 * way to turn it on; a tool with no such check never offers to measure:
 * "How it's measured ›" opens where its limits come from (D-103.4).
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
    // Still shown after the re-reads: it answered with no limits; it says
    // so and how they'll come, never an open-ended wait (0.4.4 walk M6).
    expect((await screen.findByTestId("measure-limits-said")).textContent).toBe("Claude Code answered without limits for this account; it may not be signed in with a plan. They also show once a session on it reports them.");
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

  it("D-103.4: a tool with no such check never says Measure: How it's measured › opens where its limits come from", async () => {
    statuses({});
    const run = vi.spyOn(probe, "probeRun");
    render(<MeasureLimits folders={["codex-work"]} tool="codex" name="Work" />);
    const how = await screen.findByTestId("measure-limits-how-open");
    expect(how.textContent).toBe("How it's measured › — Work");
    expect(screen.queryByText(/Measure its limits/)).toBeNull();
    expect(how.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(how);
    expect(how.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByTestId("measure-limits-how-words").textContent).toBe("No current reading. Limits come from Codex's own records when it reports them.");
    fireEvent.click(how);
    expect(screen.queryByTestId("measure-limits-how-words")).toBeNull();
    expect(run).not.toHaveBeenCalled();
  });

  it("as Home's Next steps' button, and in a caller's own fold (how): it only toggles it", async () => {
    statuses({});
    const toggle = vi.fn();
    render(<MeasureLimits look="button" folders={["codex-work"]} tool="codex" name="Work" how={{ open: true, onToggle: toggle }} />);
    const how = await screen.findByRole("button", { name: "How it's measured — Work" });
    expect(how.getAttribute("aria-expanded")).toBe("true");
    // The caller shows the words.
    expect(screen.queryByTestId("measure-limits-how-words")).toBeNull();
    fireEvent.click(how);
    expect(toggle).toHaveBeenCalledOnce();
  });
});
