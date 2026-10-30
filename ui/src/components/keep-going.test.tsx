/**
 * AJ-E1509 vitest — keeping work going: Settings › Sessions' switches
 * (restart off, awake on power on, on battery off and only with the main
 * switch, reopen after a crash on), This Mac's line saying what keeps the
 * Mac awake and why not, with its setting one click away, and a launch
 * setting's own restart choice.
 */
import { pick } from "../test/pick";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionsSettingsCard } from "./SessionsSettings";
import { AwakeLine, OwnLine } from "./this-mac-parts";
import { SmootherCard } from "./SmootherCard";
import { RecipeOptions } from "./RecipeOptions";
import type { SessionPrefsInfo } from "../lib/ipc/session-prefs";
import { mockMachineHealth, type ThisMacInfo } from "../lib/machine";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";

const restartChoice = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock("../lib/ipc/sessions", () => ({ sessionsRestartChoiceSet: restartChoice }));

type Seams = { __AIJ_SESSION_PREFS__?: SessionPrefsInfo; __AIJ_LOGIN_ITEM__?: string };
afterEach(() => {
  cleanup();
  delete (window as unknown as Seams).__AIJ_SESSION_PREFS__;
  delete (window as unknown as Seams).__AIJ_LOGIN_ITEM__;
  localStorage.clear();
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
    const picker = screen.getByRole("button", { name: /^If a session crashes/ });
    expect(picker.getAttribute("data-value")).toBe("restart");
    pick(picker, "Leave it ended");
    await act(async () => {});
    expect(restartChoice).toHaveBeenLastCalledWith("p-1", false);
    pick(picker, "As in Settings");
    await act(async () => {});
    expect(restartChoice).toHaveBeenLastCalledWith("p-1", null);
  });
});

describe("opening AI Juggler at login", () => {
  const seams = () => window as unknown as Seams;

  it("is off until turned on, and saying so", async () => {
    render(<SessionsSettingsCard />);
    const login = (await screen.findByLabelText("Open AI Juggler at login")) as HTMLInputElement;
    expect(login.checked).toBe(false);
    await act(async () => {
      fireEvent.click(login);
    });
    await waitFor(() => expect(seams().__AIJ_LOGIN_ITEM__).toBe("on"));
    expect(screen.getByTestId("pref-open_at_login").textContent).toContain("AI Juggler opens at login.");
  });

  it("can't be turned on in a build that isn't the installed app, and says why", async () => {
    seams().__AIJ_LOGIN_ITEM__ = "unavailable";
    render(<SessionsSettingsCard />);
    const login = (await screen.findByLabelText("Open AI Juggler at login")) as HTMLInputElement;
    expect(login.disabled).toBe(true);
    expect(document.getElementById(login.getAttribute("aria-describedby")!)!.textContent).toBe("Only the installed AI Juggler can open at login.");
  });

  it("is offered on the welcome while it's off, and goes once on", async () => {
    render(<SmootherCard variant="welcome" />);
    const row = await screen.findByTestId("smoother-login-item");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Turn on opening at login" }));
    });
    await waitFor(() => expect(screen.queryByTestId("smoother-login-item")).toBeNull());
    expect(row.isConnected).toBe(false);
    expect(seams().__AIJ_LOGIN_ITEM__).toBe("on");
  });

  it("This Mac says AI Juggler itself, its watchdog and opening at login", () => {
    const own = (watchdog: boolean, atLogin: boolean) =>
      ({ ...info({ working: 0, kept_awake: false, on_battery: false }), own: { rss_bytes: 214_000_000, watchdog, opens_at_login: atLogin } }) as ThisMacInfo;
    const { rerender } = render(<OwnLine info={own(true, true)} />);
    expect(screen.getByTestId("this-mac-own").textContent).toBe("AI Juggler itself: 204 MB · with its crash watchdog · opens at login");
    rerender(<OwnLine info={own(false, false)} />);
    expect(screen.getByTestId("this-mac-own").textContent).toBe("AI Juggler itself: 204 MB");
  });
});
