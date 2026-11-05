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
    // The 0.4 review (44): the figure alone, its label not said again in it.
    expect(view.getByTestId("account-quality-errors").textContent).toBe("Errors and retries9 in 300 turns");
    expect(view.getByTestId("account-quality-stuck").textContent).toContain("Probably stuck2 times");
    expect(view.getByTestId("account-quality-context").textContent).toBe("Context at finish71% on average, over 11 sessions");
    expect(view.getByTestId("account-quality-per-task").textContent).toContain("412k, over 9 tasks");
    // The files' source said once, under the title; a row says its own only where it differs.
    const section = view.getByTestId("account-quality").textContent!;
    expect(section.split("read from each session's own files").length - 1).toBe(1);
    expect(section).toContain("14 sessions AI Juggler ran, read from each session's own files when it ended");
    expect(view.getByTestId("account-quality-stuck").textContent).toContain("sessions it didn't run aren't counted");
    expect(view.queryByTestId("account-quality-left-out")).toBeNull();
    expect(view.getByTestId("account-quality-pending").textContent).toBe("1 session just ended: its figures come in a moment.");
    // D-07: no composite score, and errors never a bare percentage.
    expect(view.getByTestId("account-quality").textContent).not.toMatch(/score/i);
    expect(view.getByTestId("account-quality-errors").textContent).not.toContain("%");
  });

  it("a tool without an error feed or context leaves those rows out, said once (the 0.4 review, 80)", async () => {
    const bare = { ...INFO, tool: "codex", errors_measured: false, errors: 0, turns: 0, context_avg_pct: null, context_sessions: 0, stuck_episodes: 0 };
    const { lines, leftOut } = qualityLines(bare);
    expect(lines.map((line) => [line.key, line.value])).toEqual([["stuck", "Not once"], ["per-task", "412k, over 9 tasks"]]);
    expect(leftOut).toBe("Codex doesn't record errors or context, so those rows are left out.");
    // No finished task is left out too, in the same line.
    expect(qualityLines({ ...bare, finished: 0, tokens_per_finished: null }).leftOut).toBe("Codex doesn't record errors or context, and no task finished in these 7 days, so those rows are left out.");
    expect(qualityLines({ ...INFO, finished: 0, tokens_per_finished: null }).leftOut).toBe("No task finished in these 7 days, so that row is left out.");
    // Drawn: the line under the title, no "not known" rows, and no files source where no row reads them.
    w().__AIJ_ACCOUNT_QUALITY__ = bare;
    const view = render(<AccountQuality folders={["home"]} />);
    await waitFor(() => expect(view.getByTestId("account-quality-left-out").textContent).toContain("Codex doesn't record errors or context"));
    expect(view.queryByTestId("account-quality-errors")).toBeNull();
    expect(view.queryByTestId("account-quality-context")).toBeNull();
    expect(view.getByTestId("account-quality").textContent).not.toContain("own files");
  });

  it("waits until there's something to show", async () => {
    w().__AIJ_ACCOUNT_QUALITY__ = { ...INFO, sessions: 0 };
    const view = render(<AccountQuality folders={["work"]} />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("account-quality")).toBeNull();
  });
});
