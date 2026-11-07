/**
 * 0.4.1 (the maintainer's finding): Go to it opens a Claude app session
 * itself through its profile's own link. Its click never lets macOS ask;
 * when opening it would need macOS to ask once, the toast says so and
 * offers "Open the session itself", and only that click asks.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { showGoTo } from "./go-to";
import { resetToasts, toastEntries } from "../components/Toast";

afterEach(() => resetToasts());

describe("Go to it, when macOS would ask first", () => {
  it("offers to open the session itself, and only that click goes again with asking allowed", async () => {
    const again = vi.fn(async () => ({ outcome: "Opened the session in Claude · Work", reached: "thread" }));
    showGoTo({ outcome: "Brought Claude · Work forward. To open the session itself, AI Juggler needs to send Claude · Work a link: macOS asks you once", reached: "consent" }, again);
    const toast = toastEntries()[0]!.options;
    expect(toast.testId).toBe("go-to-consent");
    expect(toast.action?.label).toBe("Open the session itself");
    expect(again).not.toHaveBeenCalled();
    toast.action!.run();
    expect(again).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => expect(toastEntries()[0]!.options.message).toBe("Opened the session in Claude · Work"));
  });

  it("without a way to go again, it only says what happened", () => {
    showGoTo({ outcome: "Brought Claude · Work forward; pick the session in its sidebar", reached: "app" });
    expect(toastEntries()[0]!.options.action).toBeUndefined();
  });
});
