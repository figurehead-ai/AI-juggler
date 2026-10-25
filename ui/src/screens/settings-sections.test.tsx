/**
 * Wireframe 26 / AJ-E2110 vitest — Settings › the panes: the sidebar
 * (its panes in order, each with its tile; Your Macs once a Mac is
 * paired; the pane shown marked), every older link landing on its pane
 * and section, the last pane viewed coming back; then the policies the
 * user owns: the cleanup default for new profiles (off unless chosen),
 * renewal reminders, the digest's quiet hours and accounts, and Fleet
 * permissions through the same Access sheet as the Fleet screen.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SettingsScreen } from "./Settings";

const w = () => window as unknown as Record<string, unknown>;

const PEER = {
  machine_id: "machine-mini",
  label: "Mac mini",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view",
  permission_label: "View only",
  ask_each_time: false,
  paired_via: "code",
  paired_at_ms: 1,
  last_seen_ms: null,
  connection: "link_off",
  protocol: null,
  connection_detail: null,
};

const PANES = ["General", "Desktop apps", "Accounts", "Sessions", "Switching", "Notifications", "Privacy", "Advanced"];

afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_APP_SETTINGS__", "__AIJ_NOTIFICATION_PREFS__", "__AIJ_CLEANUP_DEFAULT__", "__AIJ_FLEET_PEERS__", "__AIJ_CLEANUP_STATUS__", "__AIJ_CLEANUP_STATUS_ERROR__"]) {
    delete w()[key];
  }
  localStorage.clear();
});

const navNames = (nav: HTMLElement) => within(nav).getAllByRole("button").map((button) => button.textContent);

describe("AJ-E2110 — Settings panes", () => {
  it("lists the panes in order, each with its tile, and marks the one shown", async () => {
    const { getByTestId, findByTestId } = render(<SettingsScreen />);
    const nav = getByTestId("settings-nav");
    expect(nav.closest("nav")?.getAttribute("aria-label")).toBe("Settings");
    expect(navNames(nav)).toEqual(PANES);
    for (const button of within(nav).getAllByRole("button")) {
      expect(button.querySelector("span[aria-hidden='true'] svg")).not.toBeNull();
    }
    expect(within(nav).getByRole("button", { name: "General" }).getAttribute("aria-current")).toBe("page");
    await findByTestId("settings-pane-general");
    fireEvent.click(within(nav).getByRole("button", { name: "Notifications" }));
    expect(within(nav).getByRole("button", { name: "Notifications" }).getAttribute("aria-current")).toBe("page");
    expect(within(nav).getByRole("button", { name: "General" }).getAttribute("aria-current")).toBeNull();
    const pane = await findByTestId("settings-pane-notifications");
    expect(within(pane).getByRole("heading", { level: 2 }).textContent).toBe("Notifications");
    await waitFor(() => expect(getByTestId("settings-notifications").textContent).toContain("Quiet hours"));
  });

  it("shows Your Macs once a Mac is paired, and never Agent access on this line", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    const { getByTestId } = render(<SettingsScreen />);
    await waitFor(() => expect(navNames(getByTestId("settings-nav"))).toContain("Your Macs"));
    expect(navNames(getByTestId("settings-nav"))).toEqual([...PANES.slice(0, 7), "Your Macs", "Advanced"]);
    expect(navNames(getByTestId("settings-nav"))).not.toContain("Agent access");
  });

  it("every older link lands on its pane and section", async () => {
    const links: Array<[string, string, string]> = [
      ["general", "general", "settings-pane-general"],
      ["updates", "general", "settings-about"],
      ["account", "advanced", "settings-account"],
      ["packs", "switching", "settings-packs"],
      ["sessions", "sessions", "settings-sessions"],
      ["cleanup", "advanced", "settings-cleanup"],
      ["renewal", "accounts", "settings-renewal"],
      ["notifications", "notifications", "settings-notifications"],
      ["fleet", "macs", "settings-fleet"],
      ["data", "advanced", "settings-data"],
      ["privacy", "privacy", "settings-reads"],
      ["last-messages", "desktop-apps", "settings-last-messages"],
      ["desktop-apps", "desktop-apps", "settings-show-from"],
    ];
    for (const [section, pane, anchor] of links) {
      const view = render(<SettingsScreen focusSection={section} />);
      expect(view.container.querySelector("[aria-current='page']")?.getAttribute("data-pane"), section).toBe(pane);
      await waitFor(() => expect(view.container.querySelector(`#${anchor}, [data-testid='${anchor}']`), section).not.toBeNull());
      const title = document.getElementById(`${anchor}-title`);
      if (title && section !== pane) await waitFor(() => expect(document.activeElement, section).toBe(title));
      view.unmount();
    }
  });

  it("opened for Cloud, lands on Advanced › Figurehead & Cloud", async () => {
    const { container, findByTestId } = render(<SettingsScreen focusCloud />);
    expect(container.querySelector("[aria-current='page']")?.textContent).toBe("Advanced");
    const section = (await findByTestId("account-cloud")).closest("section")!;
    expect(within(section).getAllByRole("heading")[0]!.textContent).toBe("Figurehead & Cloud");
  });

  it("⌘, comes back to the pane viewed last", async () => {
    const first = render(<SettingsScreen />);
    fireEvent.click(within(first.getByTestId("settings-nav")).getByRole("button", { name: "Privacy" }));
    await first.findByTestId("settings-pane-privacy");
    first.unmount();
    const again = render(<SettingsScreen />);
    expect(again.container.querySelector("[aria-current='page']")?.textContent).toBe("Privacy");
    await again.findByTestId("settings-pane-privacy");
  });

  it("every setting that moved still has a home", async () => {
    const homes: Array<[string, string]> = [
      ["general", "settings-appearance"],
      ["general", "settings-editor"],
      ["general", "settings-updates"],
      ["desktop-apps", "desktop-terminal-tabs"],
      ["accounts", "settings-renewal-card"],
      ["sessions", "settings-sessions"],
      ["switching", "pack-secrets"],
      ["notifications", "settings-notifications"],
      ["privacy", "settings-privacy"],
      ["advanced", "settings-cleanup"],
      ["advanced", "settings-storage"],
      ["advanced", "settings-backup"],
      ["advanced", "settings-diagnostics"],
      ["advanced", "account-cloud"],
      ["advanced", "settings-uninstall"],
    ];
    for (const [pane, testId] of homes) {
      const view = render(<SettingsScreen focusSection={pane} />);
      expect(await view.findByTestId(testId, {}, { timeout: 3000 }), `${pane} › ${testId}`).toBeTruthy();
      view.unmount();
    }
  });

  it("each section has a header and a one-line footer", async () => {
    const { findByTestId } = render(<SettingsScreen focusSection="advanced" />);
    const pane = await findByTestId("settings-pane-advanced");
    const sections = pane.querySelectorAll("section[data-settings-section]");
    expect(sections.length).toBe(6);
    for (const section of sections) {
      expect(section.querySelector("h3")?.textContent).toBeTruthy();
      expect(section.lastElementChild?.tagName).toBe("P");
    }
  });
});

describe("wireframe 26 — the policies the user owns", () => {
  it("D-08: the cleanup default is notify-only until chosen, and says existing Accounts keep theirs", async () => {
    const { findByTestId, findAllByTestId, getByTestId } = render(<SettingsScreen focusSection="cleanup" />);
    const select = (await findByTestId("cleanup-default")) as HTMLSelectElement;
    await waitFor(() => expect(select.disabled).toBe(false));
    expect(select.value).toBe("notify");
    expect(await findAllByTestId("cleanup-account")).toHaveLength(2);
    await act(async () => {
      fireEvent.change(select, { target: { value: "tidy" } });
    });
    expect(w().__AIJ_CLEANUP_DEFAULT__).toBe(true);
    expect(getByTestId("settings-cleanup").textContent).toContain("Existing Accounts keep their own choice");
  });

  it("AJ-E508 Req.4: Settings › Cleanup has the global pause switch, off by default, and pausing says so", async () => {
    const { findByTestId, getByTestId } = render(<SettingsScreen focusSection="cleanup" />);
    const pause = (await findByTestId("settings-cleanup-pause")) as HTMLInputElement;
    await waitFor(() => expect(pause.disabled).toBe(false));
    expect(pause.checked).toBe(false);
    await act(async () => {
      fireEvent.click(pause);
    });
    await waitFor(() => expect(pause.checked).toBe(true));
    expect(getByTestId("settings-cleanup").textContent).toContain("Automatic cleanup is paused.");
  });

  it("AJ-E508 Req.4: if the status can't be read, it says so and Try again recovers", async () => {
    w().__AIJ_CLEANUP_STATUS_ERROR__ = { code: "io", retryable: true, user_message: "The status could not be read." };
    const { getByTestId, findByTestId } = render(<SettingsScreen focusSection="cleanup" />);
    const error = await findByTestId("settings-cleanup-pause-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error.textContent).toContain("The status could not be read.");
    expect((getByTestId("settings-cleanup-pause") as HTMLInputElement).disabled).toBe(true);
    delete w().__AIJ_CLEANUP_STATUS_ERROR__;
    await act(async () => {
      fireEvent.click(within(error).getByRole("button", { name: "Try again" }));
    });
    await waitFor(() => expect((getByTestId("settings-cleanup-pause") as HTMLInputElement).disabled).toBe(false));
    expect(document.querySelector("[data-testid='settings-cleanup-pause-error']")).toBeNull();
  });

  it("AJ-E508 Req.4: a pause made elsewhere shows as on", async () => {
    w().__AIJ_CLEANUP_STATUS__ = { paused: true, suspended: false, suspended_until_ms: null, tier2_pending: [] };
    const { findByTestId } = render(<SettingsScreen focusSection="cleanup" />);
    const pause = (await findByTestId("settings-cleanup-pause")) as HTMLInputElement;
    await waitFor(() => expect(pause.checked).toBe(true));
  });

  it("D-12: renewal reminders default to 7 days and every Account shows its own date, or that it has none", async () => {
    const { findByTestId, findAllByTestId, getByTestId } = render(<SettingsScreen focusSection="renewal" />);
    const days = (await findByTestId("renewal-warn-days")) as HTMLSelectElement;
    await waitFor(() => expect(days.value).toBe("7"));
    const rows = await findAllByTestId("settings-renewal");
    expect(rows).toHaveLength(2);
    expect(rows[1].textContent).toContain("Not set: no renewal reminders");
    await act(async () => {
      fireEvent.change(days, { target: { value: "14" } });
    });
    expect((w().__AIJ_APP_SETTINGS__ as Record<string, string>).renewal_warn_days).toBe("14");
    expect(getByTestId("settings-renewal-card").textContent).toContain("start 14 days before");
  });

  it("notifications: quiet hours 22–07 by default, changeable, and an Account can leave the digest", async () => {
    const { findByTestId, getByTestId, findAllByTestId } = render(<SettingsScreen focusSection="notifications" />);
    const start = (await findByTestId("quiet-start")) as HTMLSelectElement;
    await waitFor(() => expect(start.value).toBe("22"));
    expect((getByTestId("quiet-end") as HTMLSelectElement).value).toBe("7");
    await act(async () => {
      fireEvent.change(getByTestId("quiet-start"), { target: { value: "21" } });
    });
    expect(w().__AIJ_NOTIFICATION_PREFS__).toMatchObject({ quiet_start: 21, quiet_end: 7 });
    const [work] = await findAllByTestId("digest-account");
    await act(async () => {
      fireEvent.click(within(work).getByRole("checkbox"));
    });
    expect(w().__AIJ_NOTIFICATION_PREFS__).toMatchObject({ digest_muted_accounts: ["claude-work"] });
    expect(getByTestId("settings-notifications").textContent).toContain("left out of the digest");
    await act(async () => {
      fireEvent.change(getByTestId("quiet-end"), { target: { value: "21" } });
    });
    expect(getByTestId("quiet-hours").textContent).toContain("No quiet hours");
  });

  it("Fleet permissions: each paired Mac's access, changed through the Access sheet", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    const { findByTestId, getByTestId } = render(<SettingsScreen focusSection="fleet" />);
    const row = await findByTestId("settings-fleet-peer");
    expect(row.textContent).toContain("Mac mini");
    expect(row.textContent).toContain("View only");
    fireEvent.click(within(row).getByRole("button", { name: /Access… for Mac mini/ }));
    expect(getByTestId("fleet-access-sheet")).toBeTruthy();
  });

  it("a link to Your Macs with none paired says where to pair one", async () => {
    const { findByText, getByTestId } = render(<SettingsScreen focusSection="fleet" />);
    expect(await findByText("No paired Macs. Pair one on the Fleet screen.")).toBeTruthy();
    expect(navNames(getByTestId("settings-nav"))).toContain("Your Macs");
  });
});
