/**
 * AJ-E2903 vitest — "How its sessions went" on an Account's page: each
 * signal in words with its source, numerator and denominator (never a
 * bare percentage), the tool without an error feed or context saying so,
 * no composite score, and nothing at all with no sessions in the 7 days.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AccountQuality, qualityLines } from "./account/Quality";
import type { AccountQualityInfo } from "../lib/ipc/quality";

const w = () => window as unknown as { __AIJ_ACCOUNT_QUALITY__?: AccountQualityInfo };
afterEach(() => {
  cleanup();
  delete w().__AIJ_ACCOUNT_QUALITY__;
});

const INFO: AccountQualityInfo = { tool: "claude-code", sessions: 14, pending: 1, errors: 9, turns: 300, errors_measured: true, stuck_episodes: 2, context_avg_pct: 71.4, context_sessions: 11, finished: 9, tokens_per_finished: 412_000 };

describe("AJ-E2903 How its sessions went", () => {
  it("says each signal in words, with where it comes from", async () => {
    w().__AIJ_ACCOUNT_QUALITY__ = INFO;
    const view = render(<AccountQuality folders={["work"]} />);
    await waitFor(() => expect(view.getByTestId("account-quality")).toBeTruthy());
    expect(view.getByTestId("account-quality-errors").textContent).toContain("9 errors or retries / 300 turns");
    expect(view.getByTestId("account-quality-stuck").textContent).toContain("2 times probably stuck");
    expect(view.getByTestId("account-quality-stuck").textContent).toContain("sessions it didn't run aren't counted");
    expect(view.getByTestId("account-quality-context").textContent).toContain("71% on average, over 11 sessions");
    expect(view.getByTestId("account-quality-per-task").textContent).toContain("412k over 9 finished tasks");
    expect(view.getByTestId("account-quality-pending").textContent).toBe("1 session just ended: its figures come in a moment.");
    // D-07: no composite score, and errors never a bare percentage.
    expect(view.getByTestId("account-quality").textContent).not.toMatch(/score/i);
    expect(view.getByTestId("account-quality-errors").textContent).not.toContain("%");
  });

  it("a tool without an error feed or context reports says so", () => {
    const lines = qualityLines({ ...INFO, tool: "codex", errors_measured: false, errors: 0, turns: 0, context_avg_pct: null, context_sessions: 0, finished: 0, tokens_per_finished: null, stuck_episodes: 0 });
    const byKey = Object.fromEntries(lines.map((line) => [line.key, line.value]));
    expect(byKey.errors).toBe("Codex doesn't record errors in its files");
    expect(byKey.context).toBe("Not reported by Codex");
    expect(byKey.stuck).toBe("Not once");
    expect(byKey["per-task"]).toBe("No finished tasks in these 7 days");
  });

  it("waits until there's something to show", async () => {
    w().__AIJ_ACCOUNT_QUALITY__ = { ...INFO, sessions: 0 };
    const view = render(<AccountQuality folders={["work"]} />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("account-quality")).toBeNull();
  });
});
