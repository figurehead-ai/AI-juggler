import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { saveOptimistic } from "../lib/optimistic";

/**
 * §4.15, §8 (the 2026-10-27 audit): a setting whose save fails goes back
 * to what it was and says why — it never keeps showing a value that was
 * never saved.
 */

const failing = vi.hoisted(() => ({ prefs: false, kinds: false }));

vi.mock("../lib/bridge", async (original) => {
  const real = await original<typeof import("../lib/bridge")>();
  return {
    ...real,
    notificationPrefsGet: async () => ({ quiet_start: 22, quiet_end: 7, digest_excluded: [] }),
    notificationPrefsSet: async (next: unknown) => {
      if (failing.prefs) throw { code: "io", retryable: true, user_message: "AI Juggler couldn't read or save its data. Try again." };
      return next;
    },
  };
});

vi.mock("../lib/ipc/notifications", async (original) => {
  const real = await original<typeof import("../lib/ipc/notifications")>();
  return {
    ...real,
    notificationsGet: async () => ({ waiting: false, stuck: false, limit: false, finished: false, this_mac: false }),
    notificationsSet: async (next: unknown) => {
      if (failing.kinds) throw { code: "io", retryable: true, user_message: "AI Juggler couldn't read or save its data. Try again." };
      return next;
    },
  };
});

afterEach(() => {
  cleanup();
  failing.prefs = false;
  failing.kinds = false;
});

describe("a failed settings save", () => {
  it("puts the quiet hours back and says why", async () => {
    failing.prefs = true;
    const { NotificationSettingsCard } = await import("./NotificationSettings");
    const view = render(<NotificationSettingsCard accounts={[]} />);
    const start = await view.findByTestId("quiet-start");
    expect(start.getAttribute("data-value")).toBe("22");
    fireEvent.click(start);
    fireEvent.click(screen.getByTestId("quiet-start-20"));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("The setting is back to what it was"));
    expect(view.getByTestId("quiet-start").getAttribute("data-value")).toBe("22");
  });

  it("puts a notification switch back and says why", async () => {
    failing.kinds = true;
    const { NotificationSettingsCard } = await import("./NotificationSettings");
    const view = render(<NotificationSettingsCard accounts={[]} />);
    const waiting = (await view.findByLabelText(/A session waits for you/)) as HTMLInputElement;
    expect(waiting.checked).toBe(false);
    fireEvent.click(waiting);
    await waitFor(() => expect(view.getByText(/The setting is back to what it was/)).toBeTruthy());
    expect((view.getByLabelText(/A session waits for you/) as HTMLInputElement).checked).toBe(false);
  });

  it("the helper shows the new value at once, and the old one after a failure", async () => {
    const shown: number[] = [];
    let said = "";
    await saveOptimistic({ previous: 1, next: 2, show: (v) => shown.push(v), save: async () => Promise.reject(new Error("disk full")), onFailed: (w) => (said = w) });
    expect(shown).toEqual([2, 1]);
    expect(said).toBe("Couldn't save that: disk full. The setting is back to what it was.");
  });
});
