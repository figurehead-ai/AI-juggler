/**
 * AJ-E1203 vitest — Settings: Appearance stays in sync with ⌘⇧L; Account
 * & Cloud signed out (two paths, desktop never lesser), linking (code,
 * denied → Try again), the consent screen's defaults (backup + discovery
 * on, usage sync off and disabled), a paid-locked feature (disabled +
 * Upgrade), the unlinked notice, and an Unlink confirmation that names
 * what stays.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SettingsScreen } from "../screens/Settings";
import { setTheme } from "../theme";

const w = () => window as unknown as Record<string, unknown>;

const LINKED = {
  state: "linked",
  email: "you@example.com",
  plan: "Free",
  device_name: "Studio Mac",
  features: [
    { key: "cloudBackup", access: "included", tier: "free", on: false },
    { key: "fleetDiscovery", access: "included", tier: "free", on: false },
    { key: "usageSync", access: "included", tier: "free", on: false },
  ],
  link: null,
  refreshed_at_ms: 1,
  offline: false,
  notice: null,
  consent_version: 0,
  consent_current: 1,
};

afterEach(cleanup);
beforeEach(() => {
  // jsdom has no matchMedia; the theme controller asks it.
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  })) as unknown as typeof window.matchMedia;
  for (const key of ["__AIJ_FIGUREHEAD__", "__AIJ_FIGUREHEAD_APPROVE__", "__AIJ_FIGUREHEAD_FEATURES__", "__AIJ_FLEET_CALLS__"]) {
    delete w()[key];
  }
});

const switchOf = (root: HTMLElement, key: string) =>
  root.querySelector<HTMLButtonElement>(`[data-key="${key}"] [role="switch"]`)!;

describe("Settings — Appearance", () => {
  it("shows This Mac in the menu bar by default, and turns it off", async () => {
    const view = render(<SettingsScreen />);
    const toggle = await view.findByTestId("menu-bar-switch");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(view.getByTestId("settings-menu-bar").textContent).toContain("A number shows only when something needs a glance");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("false");
  });

  it("follows ⌘⇧L and writes the choice", async () => {
    const view = render(<SettingsScreen />);
    act(() => setTheme("light"));
    await waitFor(() =>
      expect((view.getByTestId("appearance-light") as HTMLInputElement).checked).toBe(true),
    );
    fireEvent.click(view.getByTestId("appearance-system"));
    expect(localStorage.getItem("aij.theme")).toBe("system");
  });
});

describe("Settings — Account & Cloud", () => {
  it("signed out explains both paths and never calls the desktop path lesser", async () => {
    const view = render(<SettingsScreen />);
    const card = await view.findByTestId("cloud-signed-out");
    expect(card.textContent).toContain("You can do everything in AI Juggler without an account");
    expect(card.textContent).toContain("never leave this Mac");
    expect(view.getByTestId("cloud-connect")).toBeTruthy();
    fireEvent.click(view.getByTestId("cloud-use-code"));
    expect(view.getByLabelText("Pairing code from figurehead.ai")).toBeTruthy();
  });

  it("a refused pairing code says why", async () => {
    const view = render(<SettingsScreen />);
    fireEvent.click(await view.findByTestId("cloud-use-code"));
    fireEvent.change(view.getByTestId("cloud-code"), { target: { value: "BAD" } });
    await act(async () => {
      fireEvent.click(view.getByTestId("cloud-claim"));
    });
    expect((await view.findByRole("alert")).textContent).toContain("wrong or has expired");
  });

  it("linking shows the code, and a denial offers Try again", async () => {
    const view = render(<SettingsScreen />);
    const connect = await view.findByTestId("cloud-connect");
    await act(async () => {
      fireEvent.click(connect);
    });
    expect((await view.findByTestId("cloud-user-code")).textContent).toBe("QVM3-GABM");
    w().__AIJ_FIGUREHEAD_APPROVE__ = "denied";
    const failed = await view.findByTestId("cloud-link-failed", {}, { timeout: 3000 });
    expect(failed.getAttribute("data-phase")).toBe("denied");
    expect(failed.textContent).toContain("nothing was linked");
    expect(view.getByTestId("cloud-retry")).toBeTruthy();
  });

  it("the consent screen defaults backup and discovery on; usage sync off here (it's set in Privacy)", async () => {
    w().__AIJ_FIGUREHEAD__ = { ...LINKED };
    const view = render(<SettingsScreen />);
    await view.findByTestId("cloud-consent");
    expect(switchOf(view.container, "cloudBackup").getAttribute("aria-checked")).toBe("true");
    expect(switchOf(view.container, "fleetDiscovery").getAttribute("aria-checked")).toBe("true");
    const usage = switchOf(view.container, "usageSync");
    expect(usage.getAttribute("aria-checked")).toBe("false");
    expect(usage.disabled).toBe(true);
    expect(view.getByTestId("cloud-feature-locked").textContent).toBe(
      "Turn on in Settings › Privacy, while sharing usage stats",
    );
    for (const row of view.getAllByTestId("cloud-feature")) {
      expect(row.textContent).toContain("Sends:");
    }
    await act(async () => {
      fireEvent.click(view.getByTestId("cloud-turn-on"));
    });
    await view.findByTestId("cloud-connected");
    expect(w().__AIJ_FLEET_CALLS__).toContain("fh-choices:cloudBackup,fleetDiscovery");
  });

  it("a paid feature the plan lacks is a disabled toggle with Upgrade", async () => {
    w().__AIJ_FIGUREHEAD__ = {
      ...LINKED,
      consent_version: 1,
      features: [...LINKED.features, { key: "transcriptVault", access: "upgrade", tier: "paid", on: false }],
    };
    const view = render(<SettingsScreen />);
    await view.findByTestId("cloud-connected");
    expect(switchOf(view.container, "transcriptVault").disabled).toBe(true);
    expect(view.getByText("Paid")).toBeTruthy();
    fireEvent.click(view.getByTestId("cloud-upgrade"));
    expect(w().__AIJ_FLEET_CALLS__).toContain("fh-open:upgrade");
  });

  it("an unlinked Mac says so once and keeps local data", async () => {
    w().__AIJ_FIGUREHEAD__ = {
      ...LINKED,
      state: "signed_out",
      email: null,
      features: [],
      notice: "This Mac was unlinked from Figurehead.",
    };
    const view = render(<SettingsScreen />);
    const notice = await view.findByTestId("cloud-notice");
    expect(notice.textContent).toContain("unlinked");
    expect(notice.textContent).toContain("Everything on this Mac is unchanged");
    await act(async () => {
      fireEvent.click(view.getByText("Dismiss"));
    });
    await waitFor(() => expect(view.queryByTestId("cloud-notice")).toBeNull());
  });

  it("Unlink asks first, focuses Cancel, and says what stays", async () => {
    w().__AIJ_FIGUREHEAD__ = { ...LINKED, consent_version: 1 };
    const view = render(<SettingsScreen />);
    fireEvent.click(await view.findByTestId("cloud-unlink"));
    const sheet = view.getByTestId("cloud-unlink-confirm");
    expect(sheet.textContent).toContain("Everything stored on this Mac stays");
    expect(document.activeElement?.textContent).toBe("Cancel");
    await act(async () => {
      fireEvent.click(view.getByTestId("cloud-unlink-yes"));
    });
    await view.findByTestId("cloud-signed-out");
    expect(w().__AIJ_FLEET_CALLS__).toContain("fh-unlink");
  });
});
