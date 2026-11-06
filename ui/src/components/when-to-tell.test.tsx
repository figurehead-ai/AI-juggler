/**
 * AJ-E2904 vitest — Settings › Notifications › When to tell me: a wait's
 * threshold saves, the last warning is always after the first, and Send
 * test sends one and says where to allow notifications.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NotificationSettingsCard } from "./NotificationSettings";

type Seams = { __AIJ_NOTIFICATION_PREFS__?: Record<string, unknown>; __AIJ_NOTIFY_TESTS__?: number };
const w = () => window as unknown as Seams;

beforeEach(() => {
  delete w().__AIJ_NOTIFICATION_PREFS__;
  delete w().__AIJ_NOTIFY_TESTS__;
});
afterEach(cleanup);

describe("AJ-E2904 — when to tell me", () => {
  it("saves the wait, keeps the last warning after the first, and sends a test", async () => {
    render(<NotificationSettingsCard accounts={[]} />);
    const wait = await screen.findByTestId("wait-after");
    expect(wait.getAttribute("data-value")).toBe("5");
    fireEvent.click(wait);
    await act(async () => {
      fireEvent.click(screen.getByTestId("wait-after-0"));
    });
    await waitFor(() => expect(w().__AIJ_NOTIFICATION_PREFS__).toMatchObject({ wait_after_min: 0 }));
    expect(screen.getByRole("status").textContent).toContain("right away");

    fireEvent.click(screen.getByTestId("quota-last"));
    expect(screen.queryByTestId("quota-last-80")).toBeNull();
    expect(screen.getByTestId("quota-last-90")).toBeTruthy();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });

    await act(async () => {
      fireEvent.click(screen.getByTestId("notify-send-test"));
    });
    expect(w().__AIJ_NOTIFY_TESTS__).toBe(1);
    expect(screen.getByTestId("notify-test-sent").textContent).toContain("If nothing appeared");
  });

  it("R1: offers Right away, 2, 5, 10, 15 and 30 minutes, 5 by default, and says Needs you shows a wait at once", async () => {
    render(<NotificationSettingsCard accounts={[]} />);
    const wait = await screen.findByTestId("wait-after");
    expect(wait.getAttribute("data-value")).toBe("5");
    fireEvent.click(wait);
    const offered = ["0", "2", "5", "10", "15", "30"].map((value) => screen.getByTestId(`wait-after-${value}`).textContent?.trim());
    expect(offered.map((words) => words?.replace(/✓/g, "").trim())).toEqual(["Right away", "2 minutes", "5 minutes", "10 minutes", "15 minutes", "30 minutes"]);
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    expect(screen.getByTestId("when-to-tell").textContent).toContain("Needs you and the bell show a wait at once either way.");
  });
});
