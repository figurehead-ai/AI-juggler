/**
 * Wireframe 26 / AJ-E2110 vitest — Settings › the panes: the sidebar
 * (its panes in order, each with its tile; Your Macs once a Mac is
 * paired; the pane shown marked), every older link landing on its pane
 * and section, the last pane viewed coming back; then the policies the
 * user owns: the cleanup default for new profiles (off unless chosen),
 * renewal reminders, the digest's quiet hours and accounts, and Fleet
 * permissions through the same Access sheet as the Fleet screen.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useEffect, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsScreen } from "./Settings";
import { Pane, PaneSection } from "./settings/parts";

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

const PANES = ["General", "Desktop apps", "Accounts", "Sessions", "Switching", "Agent access", "Notifications", "Privacy", "Advanced"];

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

  it("shows Your Macs once a Mac is paired", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    const { getByTestId } = render(<SettingsScreen />);
    await waitFor(() => expect(navNames(getByTestId("settings-nav"))).toContain("Your Macs"));
    expect(navNames(getByTestId("settings-nav"))).toEqual([...PANES.slice(0, 8), "Your Macs", "Advanced"]);
  });

  it("Agent access (AJ-E1702): always there, every pool's level, and the rules you made", async () => {
    const { getByTestId, findByTestId, findAllByTestId } = render(<SettingsScreen focusSection="agents" />);
    expect(getByTestId("settings-nav").querySelector("[aria-current='page']")?.textContent).toBe("Agent access");
    const pane = await findByTestId("settings-pane-agent-access");
    expect(within(pane).getAllByRole("heading", { level: 3 }).map((heading) => heading.textContent)).toEqual(["Pools", "Agent access", "Rules you made", "Rules you made", "Agent tools in sessions"]);
    expect((await findAllByTestId("agent-access-pool")).length).toBeGreaterThan(0);
    expect((await findByTestId("settings-agent-rules")).textContent).toContain("None yet.");
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
      expect(await view.findByTestId(testId), `${pane} › ${testId}`).toBeTruthy();
      view.unmount();
    }
  });

  it("each section has a header and a one-line footer", async () => {
    const { findByTestId } = render(<SettingsScreen focusSection="advanced" />);
    const pane = await findByTestId("settings-pane-advanced");
    const sections = pane.querySelectorAll("section[data-settings-section]");
    // ADR-0023: Local cache, after the Data sections.
    expect(sections.length).toBe(7);
    for (const section of sections) {
      expect(section.querySelector("h3")?.textContent).toBeTruthy();
      expect(section.lastElementChild?.tagName).toBe("P");
    }
  });
});

describe("wireframe 26 — the policies the user owns", () => {
  it("D-08: the cleanup default is notify-only until chosen, and says existing Accounts keep theirs", async () => {
    const { findByTestId, findAllByTestId, getByTestId } = render(<SettingsScreen focusSection="cleanup" />);
    const select = (await findByTestId("cleanup-default")) as HTMLButtonElement;
    await waitFor(() => expect(select.disabled).toBe(false));
    expect(select.getAttribute("data-value")).toBe("notify");
    expect(await findAllByTestId("cleanup-account")).toHaveLength(2);
    fireEvent.click(select);
    await act(async () => {
      fireEvent.click(screen.getByTestId("cleanup-default-tidy"));
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
    const days = await findByTestId("renewal-warn-days");
    await waitFor(() => expect(days.getAttribute("data-value")).toBe("7"));
    const rows = await findAllByTestId("settings-renewal");
    expect(rows).toHaveLength(2);
    expect(rows[1].textContent).toContain("Not set: no renewal reminders");
    fireEvent.click(days);
    await act(async () => {
      fireEvent.click(screen.getByTestId("renewal-warn-days-14"));
    });
    expect((w().__AIJ_APP_SETTINGS__ as Record<string, string>).renewal_warn_days).toBe("14");
    expect(getByTestId("settings-renewal-card").textContent).toContain("start 14 days before");
  });

  it("notifications: quiet hours 22–07 by default, changeable, and an Account can leave the digest", async () => {
    const { findByTestId, getByTestId, findAllByTestId } = render(<SettingsScreen focusSection="notifications" />);
    const start = await findByTestId("quiet-start");
    await waitFor(() => expect(start.getAttribute("data-value")).toBe("22"));
    expect(getByTestId("quiet-end").getAttribute("data-value")).toBe("7");
    fireEvent.click(getByTestId("quiet-start"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("quiet-start-21"));
    });
    expect(w().__AIJ_NOTIFICATION_PREFS__).toMatchObject({ quiet_start: 21, quiet_end: 7 });
    const [work] = await findAllByTestId("digest-account");
    await act(async () => {
      fireEvent.click(within(work).getByRole("checkbox"));
    });
    expect(w().__AIJ_NOTIFICATION_PREFS__).toMatchObject({ digest_muted_accounts: ["claude-work"] });
    expect(getByTestId("settings-notifications").textContent).toContain("left out of the digest");
    fireEvent.click(getByTestId("quiet-end"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("quiet-end-21"));
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

  it("Access offers No access, said as what it means, and saves it", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    w().__AIJ_FLEET_CALLS__ = [];
    const { findByTestId, getByTestId, queryByTestId } = render(<SettingsScreen focusSection="fleet" />);
    const row = await findByTestId("settings-fleet-peer");
    fireEvent.click(within(row).getByRole("button", { name: /Access… for Mac mini/ }));
    const none = getByTestId("fleet-access-none");
    expect(none.textContent).toContain("No access");
    expect(none.textContent).toContain("Mac mini sees nothing of this computer and can't act here.");
    fireEvent.click(within(none).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("fleet-access-save"));
    });
    expect(w().__AIJ_FLEET_CALLS__).toContain("permission:machine-mini:no_access:false");
    await waitFor(() => expect(queryByTestId("fleet-access-sheet")).toBeNull());
    expect((await findByTestId("settings-fleet-peer")).textContent).toContain("No access");
    delete w().__AIJ_FLEET_CALLS__;
  });

  it("a link to Your Macs with none paired says where to pair one", async () => {
    const { findByText, getByTestId } = render(<SettingsScreen focusSection="fleet" />);
    expect(await findByText("No paired Macs. Pair one on the Fleet screen.")).toBeTruthy();
    expect(navNames(getByTestId("settings-nav"))).toContain("Your Macs");
  });
});

describe("a pane's anchor", () => {
  /** A section drawn only once its data is read, as Accounts' Tools on this Mac is. */
  function Late({ after }: { after: number }) {
    const [shown, setShown] = useState(false);
    useEffect(() => {
      const timer = setTimeout(() => setShown(true), after);
      return () => clearTimeout(timer);
    }, [after]);
    return shown ? (
      <PaneSection id="late" title="Drawn late">
        <p>its rows</p>
      </PaneSection>
    ) : null;
  }

  it("lands on a section drawn after the pane opens, not on whichever frame the read beat", async () => {
    render(
      <Pane id="test" title="Test" anchor="late">
        <Late after={60} />
      </Pane>,
    );
    await waitFor(() => expect(document.activeElement).toBe(document.getElementById("settings-late-title")));
  });

  it("stops waiting for a section that never comes", () => {
    vi.useFakeTimers();
    try {
      render(
        <Pane id="test" title="Test" anchor="late">
          <Late after={10_000} />
        </Pane>,
      );
      act(() => void vi.advanceTimersByTime(10_000));
      // Drawn after the wait ended: the pane stays where the user is.
      expect(document.getElementById("settings-late-title")).not.toBeNull();
      expect(document.activeElement).not.toBe(document.getElementById("settings-late-title"));
    } finally {
      vi.useRealTimers();
    }
  });
});
