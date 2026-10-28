/**
 * AJ-E1010 vitest — About & updates: the Beta label and version always;
 * a development build says it never checks; a check shows version and
 * changes before anything installs; install is the user's click; a
 * failed install says why in plain words (version untouched), keeps the
 * technical reason behind Details and offers Try again; "Restart to
 * update" waits with its reason until it is safe and is never forced.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { UpdatesCard } from "./UpdatesCard";
import { channelLabel, shownChannel } from "./ChannelChip";
import { NotificationRegion } from "./NotificationRegion";
import type { UpdateStatusInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_UPDATE_CALLS__ as string[] | undefined) ?? [];
const base: UpdateStatusInfo = {
  current_version: "0.1.0",
  channel: "beta",
  edition: "main",
  app_name: "AI Juggler",
  channels: ["stable", "beta"],
  configured: true,
  check_on_launch: true,
  state: "idle",
  available: null,
  progress: null,
  error: null,
  error_kind: null,
  error_detail: null,
  restart_wait: null,
  backup_path: null,
  checked_at_ms: null,
};
const available = { version: "0.2.0", notes: "## New\n- **History**: find any past session.", date_ms: 1_800_000_000_000 };

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_UPDATE"))) delete w()[key];
  delete w().__AIJ_GOING_BACK__;
  delete w().__AIJ_GOING_BACK_ERROR__;
  localStorage.removeItem("aij.update.noticed");
  localStorage.removeItem("aij.update.restart.later");
});

describe("AJ-E1010 updates", () => {
  it("a development build shows the version and Beta, and never checks", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, configured: false };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-unconfigured']")).not.toBeNull());
    expect(container.querySelector("[data-testid='channel-label']")!.textContent).toBe("Beta");
    expect(container.querySelector("[data-testid='update-version']")!.textContent).toContain("0.1.0");
    expect(container.querySelector("[data-testid='update-check']")).toBeNull();
    expect(calls()).toEqual([]);
  });

  it("About names what this build is on, as your menu does: Nightly, Dev, or the channel (the 0.2 walk, F18)", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, edition: "nightly", channel: "nightly", channels: [] };
    const nightly = render(<UpdatesCard />);
    await waitFor(() => expect(nightly.container.querySelector("[data-testid='channel-label']")!.textContent).toBe("Nightly"));
    nightly.unmount();
    w().__AIJ_UPDATE_STATUS__ = { ...base, channel: "stable" };
    const stable = render(<UpdatesCard />);
    await waitFor(() => expect(stable.container.querySelector("[data-testid='channel-label']")!.textContent).toBe("Stable"));
    expect(channelLabel(shownChannel({ edition: "dev", channel: "stable" }))).toBe("Dev");
  });

  it("a check shows the version and changes; install is the user's click", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    w().__AIJ_UPDATE_AFTER_CHECK__ = { ...base, state: "available", available };
    w().__AIJ_UPDATE_AFTER_INSTALL__ = { ...base, state: "installed", available, progress: 100, backup_path: "/data/backups/before-update.ajbackup" };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-check']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-check']")!);
    });
    expect(container.querySelector("[data-testid='update-available']")!.textContent).toContain("AI Juggler 0.2.0 is available");
    expect(container.querySelector("[data-testid='update-available-channel']")!.textContent).toBe("Beta");
    expect(container.querySelector("[data-testid='update-notes']")!.textContent).toContain("History");
    expect(calls()).toEqual(["check"]);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-install']")!);
    });
    expect(calls()).toEqual(["check", "install"]);
    expect(container.querySelector("[data-testid='update-installed']")!.textContent).toContain("before-update.ajbackup");
  });

  it("a failed install says why in plain words, keeps the reason behind Details, and offers Try again", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      state: "failed",
      available,
      error:
        "The download didn't pass the signature check — it was damaged on the way or isn't from AI Juggler — so it wasn't installed. AI Juggler 0.1.0 is untouched.",
      error_kind: "signature",
      error_detail: "the signature verification failed: Invalid signature",
    };
    w().__AIJ_UPDATE_AFTER_INSTALL__ = { ...base, state: "installed", available, progress: 100 };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-failed']")).not.toBeNull());
    const failed = container.querySelector("[data-testid='update-failed']")!;
    expect(failed.getAttribute("role")).toBe("alert");
    expect(failed.textContent).toContain("Not installed — the download didn't check out");
    expect(failed.textContent).toContain("0.1.0 is untouched");
    const details = container.querySelector<HTMLDetailsElement>("[data-testid='update-failed-details']")!;
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("Invalid signature");
    // The new version and its notes stay readable next to the failure.
    expect(container.querySelector("[data-testid='update-notes']")!.textContent).toContain("History");
    expect(container.querySelector("[data-testid='update-install']")!.textContent).toBe("Try again");
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-install']")!);
    });
    expect(calls()).toEqual(["install"]);
    expect(container.querySelector("[data-testid='update-failed']")).toBeNull();
  });

  it("a failed check says so, and Try again checks again", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      state: "failed",
      error: "Couldn't reach the update server. Check your internet connection, then try again.",
      error_kind: "network",
      error_detail: "error sending request",
    };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-failed']")).not.toBeNull());
    expect(container.querySelector("[data-testid='update-failed']")!.textContent).toContain("Couldn't check for updates");
    expect(container.querySelector("[data-testid='update-install']")).toBeNull();
    const again = container.querySelector("[data-testid='update-check']")!;
    expect(again.textContent).toBe("Try again");
    await act(async () => {
      fireEvent.click(again);
    });
    expect(calls()).toEqual(["check"]);
  });

  it("Restart to update waits, with the reason, until it is safe — never forced", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      state: "installed",
      available,
      restart_wait: "A switch is in progress. Restart to update is offered once it finishes.",
    };
    const first = render(<UpdatesCard />);
    await waitFor(() => expect(first.container.querySelector("[data-testid='update-restart-wait']")).not.toBeNull());
    expect(first.container.querySelector("[data-testid='update-restart-wait']")!.textContent).toContain(
      "Nothing restarts by itself",
    );
    expect(first.container.querySelector("[data-testid='update-restart']")).toBeNull();
    expect(calls()).toEqual([]);
    first.unmount();

    // Safe now: offered, and only the click restarts. The backend still
    // refuses if something started meanwhile, and the card says why.
    w().__AIJ_UPDATE_STATUS__ = { ...base, state: "installed", available };
    w().__AIJ_UPDATE_RESTART_ERROR__ =
      "invalid update: 2 sessions are still running — restarting would stop them. Restart to update is offered once they have finished.";
    const second = render(<UpdatesCard />);
    await waitFor(() => expect(second.container.querySelector("[data-testid='update-restart']")).not.toBeNull());
    expect(second.container.querySelector("[data-testid='update-restart']")!.textContent).toBe("Restart to update");
    expect(calls()).toEqual([]);
    await act(async () => {
      fireEvent.click(second.container.querySelector("[data-testid='update-restart']")!);
    });
    expect(calls()).toEqual(["restart"]);
    // The refusal in plain words: no "invalid update:" framing.
    expect(second.container.querySelector("[data-testid='update-error']")!.textContent).toBe(
      "2 sessions are still running — restarting would stop them. Restart to update is offered once they have finished.",
    );
  });

  it("the launch-time check can be turned off", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-check-on-launch']")).not.toBeNull());
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='update-check-on-launch']")!);
    });
    expect(calls()).toEqual(["check-on-launch:false"]);
    expect((container.querySelector("[data-testid='update-check-on-launch']") as HTMLInputElement).checked).toBe(false);
  });

  it("D-20/D-23: AI Juggler offers stable and beta with what each means; switching forgets the found update", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, state: "available", available };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-channel']")).not.toBeNull());
    const radio = (id: string) =>
      container.querySelector<HTMLInputElement>(`[data-testid='update-channel-${id}']`)!;
    expect(radio("beta").checked).toBe(true);
    expect(container.textContent).toContain("Well-tested releases");
    expect(container.querySelector("[data-testid='update-channel-nightly']")).toBeNull();
    expect(container.querySelector("[data-testid='update-edition-note']")).toBeNull();
    expect(container.textContent).toContain("never goes back to an older version");
    expect(container.querySelector("[data-testid='update-available-channel']")?.textContent).toBe("Beta");
    await act(async () => {
      fireEvent.click(radio("stable"));
    });
    expect(calls()).toEqual(["channel:stable"]);
    expect(radio("stable").checked).toBe(true);
    expect(container.querySelector("[data-testid='update-available']")).toBeNull();
  });

  it("a development build offers no channel (it never checks)", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, configured: false };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-unconfigured']")).not.toBeNull());
    expect(container.querySelector("[data-testid='update-channel']")).toBeNull();
  });

  it("D-23: AI Juggler Nightly names itself, follows nightly with no picker, and says its data is separate", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      edition: "nightly",
      app_name: "AI Juggler Nightly",
      channel: "nightly",
      channels: ["nightly"],
      state: "available",
      available,
    };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-edition-note']")).not.toBeNull());
    expect(container.querySelector("[data-testid='update-version']")!.textContent).toBe(
      "AI Juggler Nightly 0.1.0 · nightly channel",
    );
    expect(container.querySelector("[data-testid='update-channel']")).toBeNull();
    expect(container.querySelector("[data-testid='update-edition-note']")!.textContent).toContain(
      "separate from AI Juggler",
    );
    expect(container.querySelector("[data-testid='update-available']")?.textContent).toContain(
      "AI Juggler Nightly 0.2.0 is available",
    );
    expect(container.querySelector("[data-testid='update-available-channel']")?.textContent).toBe("Nightly");
    expect(container.querySelector("[data-testid='update-check-on-launch']")).not.toBeNull();
    expect(container.textContent).toContain("Installing writes a backup of AI Juggler Nightly's data first");
    expect(container.textContent).toContain("Check for updates when AI Juggler Nightly starts");
  });

  it("D-23: AI Juggler Dev says what it is once, with no channel and no check", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      edition: "dev",
      app_name: "AI Juggler Dev",
      channel: "nightly",
      channels: [],
      configured: false,
    };
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-edition-note']")).not.toBeNull());
    expect(container.querySelector("[data-testid='update-edition-note']")!.textContent).toContain("doesn't update itself");
    expect(container.querySelector("[data-testid='update-version']")!.textContent).toBe(
      "AI Juggler Dev 0.1.0 · built on this computer",
    );
    expect(container.querySelector("[data-testid='update-unconfigured']")).toBeNull();
    expect(container.querySelector("[data-testid='update-channel']")).toBeNull();
    expect(container.querySelector("[data-testid='update-check']")).toBeNull();
  });
});

describe("AJ-E1010 update prompts on Home", () => {
  it("an available update is announced with its Beta label; nothing installs", async () => {
    w().__AIJ_UPDATE_STATUS__ = { ...base, state: "available", available };
    const { container } = render(<NotificationRegion />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-notice']")).not.toBeNull());
    // AJ-E2504: the shared Banner — who and what, then the rest.
    expect(container.querySelector("[data-testid='update-notice'] p")!.textContent).toBe("AI Juggler 0.2.0 is availableBeta");
    expect(container.querySelector("[data-testid='update-notice']")!.textContent).toContain("Nothing is installed until you choose.");
    expect(container.querySelector("[data-testid='update-notice-channel']")!.textContent).toBe("Beta");
    expect(calls()).toEqual([]);
    // Review opens Settings on the update itself, focused.
    const opened: unknown[] = [];
    const listen = (event: Event) => opened.push((event as CustomEvent).detail);
    window.addEventListener("aij:open-settings", listen);
    fireEvent.click(container.querySelector("[data-testid='update-notice-review']")!);
    window.removeEventListener("aij:open-settings", listen);
    expect(opened).toEqual([{ section: "updates" }]);
    expect(calls()).toEqual([]);
    const card = render(<UpdatesCard focus />);
    await waitFor(() => expect(document.activeElement?.id).toBe("settings-updates-title"));
    card.unmount();
  });

  it("Restart to update is offered only once it is safe, and Later puts it away", async () => {
    w().__AIJ_UPDATE_STATUS__ = {
      ...base,
      state: "installed",
      available,
      restart_wait: "Accounts are being brought in. Restart to update is offered once that's done.",
    };
    const held = render(<NotificationRegion />);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(held.container.querySelector("[data-testid='update-ready']")).toBeNull();
    held.unmount();

    w().__AIJ_UPDATE_STATUS__ = { ...base, state: "installed", available };
    const safe = render(<NotificationRegion />);
    await waitFor(() => expect(safe.container.querySelector("[data-testid='update-ready']")).not.toBeNull());
    expect(safe.container.querySelector("[data-testid='update-ready']")!.textContent).toContain("nothing restarts by itself");
    expect(safe.container.querySelector("[data-testid='update-ready-channel']")!.textContent).toBe("Beta");
    expect(calls()).toEqual([]);
    await act(async () => {
      fireEvent.click(safe.container.querySelector("[data-testid='update-ready-restart']")!);
    });
    expect(calls()).toEqual(["restart"]);
    fireEvent.click(safe.container.querySelector("[data-testid='update-ready-later']")!);
    expect(safe.container.querySelector("[data-testid='update-ready']")).toBeNull();
    safe.unmount();
    const again = render(<NotificationRegion />);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(again.container.querySelector("[data-testid='update-ready']")).toBeNull();
  });
});

describe("AJ-E1010 going back to an earlier version", () => {
  it("is hidden while no copy from before an update is kept", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='update-check']")).not.toBeNull());
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector("[data-testid='going-back']")).toBeNull();
  });

  it("says what is lost and kept, asks once, and only then goes back", async () => {
    w().__AIJ_UPDATE_STATUS__ = base;
    w().__AIJ_GOING_BACK__ = [{ schema: 27, version: "0.0.7", kept_at_ms: 1_800_000_000_000 }];
    const { container } = render(<UpdatesCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='going-back']")).not.toBeNull());
    const details = container.querySelector<HTMLDetailsElement>("[data-testid='going-back']")!;
    expect(details.open, "behind a disclosure").toBe(false);
    const prepare = container.querySelector<HTMLButtonElement>("[data-testid='going-back-27']")!;
    expect(prepare.textContent).toBe("Prepare to go back to 0.0.7");
    fireEvent.click(prepare);
    const confirm = container.querySelector("[data-testid='going-back-confirm']")!;
    expect(confirm.textContent).toContain("What changed since");
    expect(confirm.textContent).toContain("nothing is deleted");
    expect(confirm.textContent).toContain("Install 0.0.7 from the releases page");
    expect(document.activeElement?.textContent).toBe("Go back to 0.0.7?");
    expect(calls()).toEqual([]);
    // 0.0.11 walk, Q6: the install line links to that version's release,
    // and opening it changes nothing here.
    const releases = confirm.querySelector<HTMLButtonElement>("[data-testid='going-back-releases']")!;
    expect(releases.textContent).toBe("Open the releases page");
    fireEvent.click(releases);
    expect(calls()).toEqual(["open-releases:0.0.7"]);
    expect(container.querySelector("[data-testid='going-back-confirm']")).not.toBeNull();
    w().__AIJ_UPDATE_CALLS__ = [];
    // Cancel returns focus to the choice; nothing happened.
    fireEvent.click(container.querySelector("[data-testid='going-back-cancel']")!);
    expect(document.activeElement?.getAttribute("data-testid")).toBe("going-back-27");
    expect(calls()).toEqual([]);
    // Refused while something would be cut short — said plainly.
    w().__AIJ_GOING_BACK_ERROR__ =
      "invalid going back: 1 session is still running — stopping AI Juggler would stop it. Going back is possible once it has finished.";
    fireEvent.click(container.querySelector("[data-testid='going-back-27']")!);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='going-back-go']")!);
    });
    expect(calls()).toEqual(["going-back:27"]);
    expect(container.querySelector("[data-testid='going-back-error']")!.textContent).toBe(
      "1 session is still running — stopping AI Juggler would stop it. Going back is possible once it has finished.",
    );
  });
});
