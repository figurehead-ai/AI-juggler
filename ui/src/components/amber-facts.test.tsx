/**
 * D-126 — amber only when the user must act. A plain fact on the pool
 * screens and on Home is said in neutral words: an App that is only
 * watched, a file the connector leaves alone, an Account low on room
 * that no live session needs. Amber stays for what waits on the user.
 */
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { roomTone } from "../lib/attention";
import { forecastDisplay, quotaDisplay, windowDisplay } from "../lib/wire";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { instance } from "../test/fixtures";
import { ConnectorFiles } from "../pages/pool/connector/ConnectorFiles";
import { ConnectorRowState } from "../pages/pool/connector/ConnectorRow";
import { HomeRunway } from "./HomeRunway";

afterEach(cleanup);

describe("a room meter's colour", () => {
  it("is amber only at L2 (a live session on 10 % or less), red once out under one", () => {
    expect(roomTone(15, 0).text).toBe("text-ink");
    expect(roomTone(8, 0).text).toBe("text-ink");
    expect(roomTone(8, 1).text).toBe("text-warning");
    expect(roomTone(0, 1).bar).toBe("bg-danger");
    expect(roomTone(null, 3).bar).toBe("bg-accent");
  });
});

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
  five_hour: windowDisplay("tracking", { percentage: used }),
  weekly: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: forecastDisplay("learning", { badge: "" }),
});

describe("Home's runway", () => {
  it("says a low Account's figure in ink when nothing live needs it (was amber at 25 % or less)", async () => {
    render(<HomeRunway accounts={[folder("side", "Side")]} quota={[display("side", 85)]} sessions={[]} />);
    const row = (await screen.findAllByTestId("home-runway-account"))[0]!;
    expect(within(row).getByText("15% left").className).toContain("text-ink");
    expect(within(row).getByText("15% left").className).not.toContain("text-warning");
  });

  it("turns amber when a live session is on it at 10 % or less", async () => {
    const live = instance({ account_id: "work", status: "running" });
    render(<HomeRunway accounts={[folder("work", "Work")]} quota={[display("work", 92)]} sessions={[live]} />);
    const row = (await screen.findAllByTestId("home-runway-account"))[0]!;
    expect(within(row).getByText("8% left").className).toContain("text-warning");
  });
});

describe("the pool's connector words", () => {
  it("says an App is only watched in ink, its Add still offered", () => {
    render(<ConnectorRowState status={{ app_id: "a", state: "absent", changed: [], why_not: null, folder: null, picked_up: null, attests: false, can_wake: false }} bridge={null} testId="row" busy={false} failed={null} onReview={() => {}} onGuide={() => {}} />);
    const words = screen.getByText("Watched: can't talk in the pool yet.");
    expect(words.className).toContain("text-ink");
    expect(words.className).not.toContain("text-warning");
    expect(screen.getByTestId("row-add")).toBeTruthy();
  });

  it("says a file it leaves alone in muted words", () => {
    render(<ConnectorFiles testId="c" files={[{ path: "/u/.claude/settings.json", action: "refuse", before: "{}", after: "{}" }]} />);
    const line = screen.getByTestId("c-file").firstElementChild!;
    expect(line.className).toContain("text-muted");
    expect(line.className).not.toContain("text-warning");
  });
});
