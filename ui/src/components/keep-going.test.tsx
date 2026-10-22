/**
 * AJ-E1509 vitest — keeping work going: Settings › Sessions' switches
 * (restart off, awake on power on, on battery off and only with the main
 * switch, reopen after a crash on), This Mac's line saying what keeps the
 * Mac awake and why not, with its setting one click away, and a launch
 * setting's own restart choice.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionsSettingsCard } from "./SessionsSettings";
import { AwakeLine } from "./this-mac-parts";
import { RecipeOptions } from "./RecipeOptions";
import type { SessionPrefsInfo } from "../lib/ipc/session-prefs";
import { mockMachineHealth, type ThisMacInfo } from "../lib/machine";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";

const restartChoice = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock("../lib/ipc/sessions", () => ({ sessionsRestartChoiceSet: restartChoice }));

type Seams = { __AIJ_SESSION_PREFS__?: SessionPrefsInfo };
afterEach(() => {
  cleanup();
  delete (window as unknown as Seams).__AIJ_SESSION_PREFS__;
  restartChoice.mockClear();
});
const stored = () => (window as unknown as Seams).__AIJ_SESSION_PREFS__;

describe("Settings › Sessions: Keep work going", () => {
  it("starts from the defaults, each with its why", async () => {
    render(<SessionsSettingsCard />);
    const restart = (await screen.findByLabelText("Restart sessions that crash")) as HTMLInputElement;
    expect(restart.checked).toBe(false);
    expect(document.getElementById(restart.getAttribute("aria-describedby")!)!.textContent).toContain("Never after a usage limit, a stop you made or a failed sign-in");
    expect((screen.getByLabelText("Keep the Mac awake while sessions work") as HTMLInputElement).checked).toBe(true);
    const battery = screen.getByLabelText("On battery too") as HTMLInputElement;
    expect(battery.checked).toBe(false);
    expect(battery.disabled).toBe(false);
    expect((screen.getByLabelText("Reopen AI Juggler if it quits unexpectedly") as HTMLInputElement).checked).toBe(true);
  });

  it("saves each switch at once, and On battery too needs the main switch", async () => {
    render(<SessionsSettingsCard />);
    const restart = await screen.findByLabelText("Restart sessions that crash");
    await act(async () => {
      fireEvent.click(restart);
    });
    await waitFor(() => expect(stored()?.restart_on_crash).toBe(true));
    expect(screen.getByRole("status").textContent).toBe("Sessions that crash are started again.");
    await act(async () => {
      fireEvent.click(screen.getByLabelText("Keep the Mac awake while sessions work"));
    });
    await waitFor(() => expect(stored()?.keep_awake_on_power).toBe(false));
    expect((screen.getByLabelText("On battery too") as HTMLInputElement).disabled).toBe(true);
    // The other settings were kept as they were.
    expect(stored()?.restart_on_crash).toBe(true);
    expect(stored()?.stuck_after_minutes).toBe(5);
  });
});

const info = (awake: ThisMacInfo["awake"]): ThisMacInfo =>
  ({ machine: mockMachineHealth("plenty", 1_728_800_000_000), awake }) as unknown as ThisMacInfo;

describe("This Mac: what keeps it awake", () => {
  it("says nothing while no session works", () => {
    render(<AwakeLine info={info({ working: 0, kept_awake: false, on_battery: false })} />);
    expect(screen.queryByTestId("this-mac-awake")).toBeNull();
  });

  it("says it's kept awake, or why not", () => {
    const { rerender } = render(<AwakeLine info={info({ working: 2, kept_awake: true, on_battery: false })} />);
    expect(screen.getByTestId("this-mac-awake").textContent).toContain("Kept awake while 2 AI Juggler sessions work");
    rerender(<AwakeLine info={info({ working: 1, kept_awake: false, on_battery: true })} />);
    expect(screen.getByTestId("this-mac-awake").textContent).toContain("On battery, so not kept awake while 1 AI Juggler session works");
    rerender(<AwakeLine info={info({ working: 1, kept_awake: false, on_battery: false })} />);
    expect(screen.getByTestId("this-mac-awake").textContent).toContain("(off in Settings)");
  });

  it("opens Settings › Sessions in one click", () => {
    const opened = vi.fn();
    const listen = (event: Event) => opened((event as CustomEvent<{ section: string }>).detail.section);
    window.addEventListener("aij:open-settings", listen);
    render(<AwakeLine info={info({ working: 2, kept_awake: true, on_battery: false })} />);
    fireEvent.click(screen.getByRole("button", { name: "Change when the Mac is kept awake" }));
    window.removeEventListener("aij:open-settings", listen);
    expect(opened).toHaveBeenCalledWith("sessions");
  });
});

describe("a launch setting's own restart choice", () => {
  const recipe = (restart: boolean | null): LaunchRecipeInfo => ({
    id: "p-1",
    name: "Overnight",
    account_id: "acc-1",
    account_label: "Work Max",
    provider: "claude-code",
    model: null,
    default_cwd: null,
    restart_on_crash: restart,
  });

  it("shows its choice, and saves a new one (As in Settings saves none)", async () => {
    render(<RecipeOptions provider="claude-code" accountId="acc-1" recipe={recipe(true)} />);
    const select = screen.getByLabelText("If a session crashes") as HTMLSelectElement;
    expect(select.value).toBe("restart");
    await act(async () => {
      fireEvent.change(select, { target: { value: "stay" } });
    });
    expect(restartChoice).toHaveBeenLastCalledWith("p-1", false);
    await act(async () => {
      fireEvent.change(select, { target: { value: "settings" } });
    });
    expect(restartChoice).toHaveBeenLastCalledWith("p-1", null);
  });
});
