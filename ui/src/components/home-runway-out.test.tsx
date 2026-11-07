/**
 * 0.4.1 (the maintainer's "Out" everywhere): Home's Runway by account says
 * Out for an Account with nothing left, as the App and Account pages and
 * the rail do; a low one keeps its figure.
 */
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { HomeRunway } from "./HomeRunway";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { forecastDisplay, quotaDisplay, windowDisplay } from "../lib/wire";

afterEach(cleanup);

const folder = (id: string, label: string): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
});
const display = (account: string, used: number): QuotaDisplayData => ({
  ...quotaDisplay(account),
  five_hour: windowDisplay(used >= 100 ? "exhausted" : "tracking", {
    percentage: used,
  }),
  weekly: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: forecastDisplay("learning", { badge: "" }),
});

describe("Home's runway says Out", () => {
  it('an Account with nothing left shows the Out chip, not "0% left"; a low one keeps its figure', async () => {
    render(
      <HomeRunway
        accounts={[folder("work", "Work"), folder("side", "Side")]}
        quota={[display("work", 100), display("side", 85)]}
        sessions={[]}
      />,
    );
    const rows = await screen.findAllByTestId("home-runway-account");
    const work = rows.find((row) => row.textContent?.includes("Work"))!;
    const side = rows.find((row) => row.textContent?.includes("Side"))!;
    expect(within(work).getByTestId("home-runway-out").textContent).toContain(
      "Out",
    );
    // The figure gives way to the chip; the line below still names the window.
    expect(within(work).queryByText("0% left", { exact: true })).toBeNull();
    expect(within(side).queryByTestId("home-runway-out")).toBeNull();
    expect(side.textContent).toContain("15% left");
  });
});
