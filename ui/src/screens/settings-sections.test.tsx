/**
 * Wireframe 26 vitest — Settings › the policies the user owns: the
 * section list (every section stays on the page; the list jumps and
 * shows where you are), the cleanup default for new Accounts (off unless
 * chosen), renewal reminders, the digest's quiet hours and Accounts, and
 * Fleet permissions through the same Access sheet as the Fleet screen.
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

afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_APP_SETTINGS__", "__AIJ_NOTIFICATION_PREFS__", "__AIJ_CLEANUP_DEFAULT__", "__AIJ_FLEET_PEERS__", "__AIJ_CLEANUP_STATUS__", "__AIJ_CLEANUP_STATUS_ERROR__"]) {
    delete w()[key];
  }
});

describe("wireframe 26 — Settings sections", () => {
  it("lists every section in order, keeps them all on the page, and jumps to one", async () => {
    const { container, getByTestId } = render(<SettingsScreen />);
    const nav = getByTestId("settings-nav");
    const titles = within(nav)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(titles).toEqual([
      "General",
      "Account & Cloud",
      "Context packs & secrets",
      "Sessions",
      "Cleanup",
      "Accounts & renewal dates",
      "Notifications",
      "Fleet permissions",
      "Data: backup & retention",
    ]);
    expect(container.querySelectorAll("[data-settings-section]")).toHaveLength(9);
    expect(within(nav).getByRole("button", { name: "General" }).getAttribute("aria-current")).toBe("true");
    fireEvent.click(within(nav).getByRole("button", { name: "Notifications" }));
    expect(within(nav).getByRole("button", { name: "Notifications" }).getAttribute("aria-current")).toBe("true");
    expect(document.activeElement?.id).toBe("settings-notifications-title");
    await waitFor(() => expect(getByTestId("settings-notifications").textContent).toContain("Quiet hours"));
  });

  it("starts on Account & Cloud when opened for it", () => {
    const { getByTestId } = render(<SettingsScreen focusCloud />);
    expect(
      within(getByTestId("settings-nav")).getByRole("button", { name: "Account & Cloud" }).getAttribute("aria-current"),
    ).toBe("true");
  });

  it("D-08: the cleanup default is notify-only until chosen, and says existing Accounts keep theirs", async () => {
    const { getByTestId, findAllByTestId } = render(<SettingsScreen />);
    const select = getByTestId("cleanup-default") as HTMLSelectElement;
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
    const { getByTestId } = render(<SettingsScreen />);
    const pause = getByTestId("settings-cleanup-pause") as HTMLInputElement;
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
    const { getByTestId, findByTestId } = render(<SettingsScreen />);
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
    const { getByTestId } = render(<SettingsScreen />);
    await waitFor(() => expect((getByTestId("settings-cleanup-pause") as HTMLInputElement).checked).toBe(true));
  });

  it("D-12: renewal reminders default to 7 days and every Account shows its own date, or that it has none", async () => {
    const { getByTestId, findAllByTestId } = render(<SettingsScreen />);
    const days = getByTestId("renewal-warn-days") as HTMLSelectElement;
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
    const { getByTestId, findAllByTestId } = render(<SettingsScreen />);
    await waitFor(() => expect((getByTestId("quiet-start") as HTMLSelectElement).value).toBe("22"));
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
    const { findByTestId, getByTestId } = render(<SettingsScreen />);
    const row = await findByTestId("settings-fleet-peer");
    expect(row.textContent).toContain("Mac mini");
    expect(row.textContent).toContain("View only");
    fireEvent.click(within(row).getByRole("button", { name: /Access… for Mac mini/ }));
    expect(getByTestId("fleet-access-sheet")).toBeTruthy();
  });

  it("with no paired Macs, says where to pair one", async () => {
    const { findByText } = render(<SettingsScreen />);
    expect(await findByText("No paired Macs. Pair one on the Fleet screen.")).toBeTruthy();
  });
});
