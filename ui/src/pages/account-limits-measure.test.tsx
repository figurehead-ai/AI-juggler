/**
 * AJ-E3205 R3: "Measure its limits" opens the Account's page; there, with
 * nothing read, its Limits offer "Measure its limits ›" where measuring can
 * be switched on (its App's Settings), and otherwise say how they come.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import * as quota from "../lib/ipc/quota";
import * as capture from "../lib/ipc/capture";
import { openWhereMeasured } from "../lib/measure-limits";
import { rememberedTabId } from "./registry";
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

const offered = (yes: boolean) => vi.spyOn(capture, "captureStatus").mockResolvedValue({ offered: yes, state: "off", file: null, reason: yes ? null : "AI Juggler's own copy" });

it("with nothing read, offers Measure its limits › where it can be switched on", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  offered(true);
  const measure = vi.fn();
  render(<AccountLimits account={folder} now={NOW} onMeasure={measure} />);
  fireEvent.click(await screen.findByTestId("account-limits-measure"));
  expect(measure).toHaveBeenCalledTimes(1);
});

it("where it can't be switched on, says how they come instead", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  offered(false);
  render(<AccountLimits account={folder} now={NOW} onMeasure={() => {}} />);
  expect((await screen.findByTestId("account-limits-how")).textContent).toBe("They show once a session on it reports them.");
  expect(screen.queryByTestId("account-limits-measure")).toBeNull();
});

it("a door to measuring opens its App on Settings when the switch is there, else on Overview", async () => {
  const open = vi.fn();
  offered(true);
  await openWhereMeasured("claude-work", open);
  expect(rememberedTabId("app")).toBe("settings");
  offered(false);
  await openWhereMeasured("claude-work", open);
  expect(rememberedTabId("app")).toBe("overview");
  expect(open).toHaveBeenCalledTimes(2);
});

it("without a door (the App page's own limits) shows none", async () => {
  vi.spyOn(quota, "quotaDisplay").mockResolvedValue(unread());
  render(<AccountLimits account={folder} now={NOW} />);
  await waitFor(() => expect(screen.queryByTestId("account-rings-loading")).toBeNull());
  expect(screen.queryByTestId("account-limits-measure")).toBeNull();
});
