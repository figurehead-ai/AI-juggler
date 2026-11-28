/**
 * Design review 29–31 and 74 vitest — Settings › Notifications: a card
 * per part, headed in the section-title style; quiet hours and the digest
 * folded behind one line that says their values; no "macOS" or "parked".
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { moreWords, NotificationSettingsCard } from "./NotificationSettings";
import { clocksWhole, hourWords } from "../lib/time-words";
import { quietHours } from "./watches/watches-view";

afterEach(cleanup);

const twelveHour = new Intl.DateTimeFormat([], { hour: "numeric" }).resolvedOptions().hourCycle?.startsWith("h1");
/** Quiet hours as the one rule writes them (0.5.2 G52). */
const quiet = clocksWhole(`quiet hours ${hourWords(22)}–${hourWords(7)}`);

describe("Notifications, design review 29–31 and 74", () => {
  it("a card each, section-title headings, quiet hours and the digest folded with their values", async () => {
    render(<NotificationSettingsCard accounts={[]} />);
    const more = (await screen.findByTestId("notifications-more"));
    expect(more.hasAttribute("data-open")).toBe(false);
    expect(more.querySelector("button[aria-expanded]")!.textContent).toBe(`More: ${quiet}`);
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
    expect(moreWords({ quiet_start: 22, quiet_end: 7, digest_muted_accounts: [] }, accounts)).toBe(`${quiet}, all accounts`);
    expect(moreWords({ quiet_start: 3, quiet_end: 3, digest_muted_accounts: ["b", "gone"] }, accounts)).toBe("no quiet hours, 1 account left out");
    expect(moreWords({ quiet_start: 3, quiet_end: 3, digest_muted_accounts: ["a", "b"] }, accounts)).toBe("no quiet hours, 2 accounts left out");
  });

  // 0.5.2 G52: Settings said "22:00" beside the Watches form's "10:00 PM".
  it("quiet hours follow the system's clock, as the Watches form writes them", async () => {
    const form = quietHours({ quiet_start: 22, quiet_end: 7 });
    expect(hourWords(22)).toBe(form?.from);
    expect(hourWords(7)).toBe(form?.to);
    if (twelveHour) {
      expect(moreWords({ quiet_start: 22, quiet_end: 7, digest_muted_accounts: [] }, [])).toMatch(/^quiet hours 10:00\s?PM–7:00\s?AM$/u);
      expect(hourWords(0)).toMatch(/^12:00\s?AM$/u);
    }
    render(<NotificationSettingsCard accounts={[]} />);
    (await screen.findByTestId("notifications-more")).querySelector<HTMLButtonElement>("button[aria-expanded]")!.click();
    const from = await screen.findByTestId("quiet-start");
    expect(from.textContent).toContain(hourWords(22));
    expect(screen.getByTestId("quiet-end").textContent).toContain(hourWords(7));
    if (twelveHour) expect(from.textContent).not.toContain("22:00");
  });
});
