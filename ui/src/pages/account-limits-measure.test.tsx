/**
 * AJ-E3205 R3, 0.4.4: on the Account's page, with nothing read, its Limits
 * measure in place (`MeasureLimits`, D-39's ask): the reading shows where
 * the button was. Turned off, it says so; a tool with no such check says
 * how its limits come. The App page's own limits offer none.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import * as quota from "../lib/ipc/quota";
import * as probe from "../lib/ipc/probe";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { AccountLimits } from "./AccountPage";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const NOW = Date.UTC(2026, 10, 7, 12);
const folder: AccountInfo = { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false };

function unread(): QuotaDisplayData {
  return {
    account: "claude-work",
    five_hour: null,
    five_hour_omitted_reason: null,
    weekly: null,
    weekly_omitted_reason: null,
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
    unavailable_reason: null,
    credits: null,
    model_weeks: [],
  };
}

const probeIs = (offered: boolean, state: "on" | "off" = "on") =>
  vi.spyOn(probe, "probeStatus").mockResolvedValue({ offered, state, last_run_ms: null, last_outcome: null });

function read(): QuotaDisplayData {
  const window = (percentage: number) => ({ state: "tracking" as const, percentage, badge: "measured", stale_note_ms: null, resets_at_ms: NOW + 3_600_000, reset_basis: "measured" as const, measured_at_ms: NOW, burned_tokens: 0, burned_credits: null, headroom_tokens: null });
  return { ...unread(), five_hour: window(30), weekly: window(20) };
}

it("with nothing read, Measure its limits measures here and the reading shows where it was", async () => {
  const displays = vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  probeIs(true);
  const run = vi.spyOn(probe, "probeRun").mockImplementation(async () => {
    displays.mockResolvedValue(read());
    return { offered: true, state: "on", last_run_ms: NOW, last_outcome: "ran" };
  });
  render(<AccountLimits account={folder} now={NOW} measure />);
  fireEvent.click(await screen.findByTestId("measure-limits-run"));
  expect(run).toHaveBeenCalledWith("claude-work");
  await waitFor(() => expect(screen.queryByTestId("account-limits-measure")).toBeNull(), { timeout: 3_000 });
  expect(screen.getByTestId("account-rings").textContent).toContain("70% left");
});

it("turned off for it, says so with the way to turn it on", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  probeIs(true, "off");
  render(<AccountLimits account={folder} now={NOW} measure />);
  expect((await screen.findByTestId("measure-limits-off")).textContent).toContain("is off here · Turn on ›");
});

it("where the tool has no such check (D-103.4): How it's measured ›, opening where they come from", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  probeIs(false, "off");
  render(<AccountLimits account={folder} now={NOW} measure />);
  const how = await screen.findByTestId("measure-limits-how-open");
  expect(how.textContent).toBe(`How it's measured › — ${folder.label}`);
  expect(screen.queryByTestId("measure-limits-run")).toBeNull();
  expect(screen.getByTestId("account-limits-measure").textContent).not.toContain("Measure its limits");
  fireEvent.click(how);
  expect(screen.getByTestId("measure-limits-how-words").textContent).toMatch(/^No current reading\. Limits come from /);
});

it("without a door (the App page's own limits) shows none", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  render(<AccountLimits account={folder} now={NOW} />);
  await waitFor(() => expect(screen.queryByTestId("account-rings-loading")).toBeNull());
  expect(screen.queryByTestId("account-limits-measure")).toBeNull();
});
