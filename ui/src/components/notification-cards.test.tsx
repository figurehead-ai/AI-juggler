/**
 * Design review 29–31 and 74 vitest — Settings › Notifications: a card
 * per part, headed in the section-title style; quiet hours and the digest
 * folded behind one line that says their values; no "macOS" or "parked".
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { moreWords, NotificationSettingsCard } from "./NotificationSettings";

afterEach(cleanup);

describe("Notifications, design review 29–31 and 74", () => {
  it("a card each, section-title headings, quiet hours and the digest folded with their values", async () => {
    render(<NotificationSettingsCard accounts={[]} />);
    const more = (await screen.findByTestId("notifications-more")) as HTMLDetailsElement;
    expect(more.open).toBe(false);
    expect(more.querySelector("summary")!.textContent).toBe("More: quiet hours 22:00–07:00");
    expect(more.contains(screen.getByTestId("quiet-hours"))).toBe(true);
    expect(more.contains(screen.getByTestId("digest-accounts"))).toBe(true);
    const root = screen.getByTestId("settings-notifications");
    const cards = [screen.getByTestId("system-notifications"), screen.getByTestId("when-to-tell"), more].map((part) => part.closest(".rounded-card"));
    expect(new Set(cards).size).toBe(3);
    for (const legend of root.querySelectorAll("legend")) expect(legend.className).toContain("text-section");
    expect(root.textContent).not.toMatch(/macOS|parked/);
    expect(root.textContent).toContain("a session waiting to resume");
  });

  it("the warning pickers say the level once", async () => {
    render(<NotificationSettingsCard accounts={[]} />);
    expect((await screen.findByTestId("quota-first")).textContent).toMatch(/^\d+ %$/);
  });

  it("the fold's words", () => {
    const accounts = [{ id: "a" }, { id: "b" }, { id: "c" }];
    expect(moreWords({ quiet_start: 22, quiet_end: 7, digest_muted_accounts: [] }, accounts)).toBe("quiet hours 22:00–07:00, all accounts");
    expect(moreWords({ quiet_start: 3, quiet_end: 3, digest_muted_accounts: ["b", "gone"] }, accounts)).toBe("no quiet hours, 1 account left out");
    expect(moreWords({ quiet_start: 3, quiet_end: 3, digest_muted_accounts: ["a", "b"] }, accounts)).toBe("no quiet hours, 2 accounts left out");
  });
});
