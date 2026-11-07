/**
 * AJ-E3205 R3: "Measure its limits" opens the Account's page; there, with
 * nothing read, its Limits offer "Measure its limits ›", which opens its
 * App on Settings, where measuring is turned on. Once read, no door.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import * as quota from "../lib/ipc/quota";
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

it("with nothing read, offers Measure its limits ›, which calls its door", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  const measure = vi.fn();
  render(<AccountLimits account={folder} now={NOW} onMeasure={measure} />);
  fireEvent.click(await screen.findByTestId("account-limits-measure"));
  expect(measure).toHaveBeenCalledTimes(1);
});

it("without a door (the App page's own limits) shows none", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  render(<AccountLimits account={folder} now={NOW} />);
  await waitFor(() => expect(screen.queryByTestId("account-rings-loading")).toBeNull());
  expect(screen.queryByTestId("account-limits-measure")).toBeNull();
});
