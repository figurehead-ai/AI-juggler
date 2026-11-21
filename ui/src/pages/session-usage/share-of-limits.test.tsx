import { cleanup, render, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { UsageSessionDetailInfo } from "../../types/generated/UsageSessionDetailInfo";
import type { UsageWindowShareInfo } from "../../types/generated/UsageWindowShareInfo";

/**
 * SK-22: a session's Usage tab drew the 5-hour window and the week as full
 * bars ("32k of 32k · 100%") for an App whose limits AI Juggler doesn't
 * know: the share is of the App's own use, but a full bar reads "at the
 * limit". With no known limit, it's said in words, with why; a known
 * limit keeps its bar.
 */

const NO_LIMIT = "OpenCode doesn't report its limits to AI Juggler yet";

const window = (name: "five_hour" | "weekly", no_limit: string | null): UsageWindowShareInfo => ({
  window: name,
  since_ms: Date.now() - 3_600_000,
  basis: "rolling",
  session_io: 32_000,
  profile_io: 32_000,
  no_limit,
});

const detail = (no_limit: string | null): UsageSessionDetailInfo => ({
  turns: [{ at_ms: Date.now() - 60_000, model: "m", tokens: { input: 30_000, output: 2_000, cache_read: 0, cache_write: 0, reasoning: 0 }, usd: null }],
  truncated: false,
  totals: { sessions: 1, active_hours: 1, messages: 1, tokens: { input: 30_000, output: 2_000, cache_read: 0, cache_write: 0, reasoning: 0 }, usd: null, unpriced: true },
  by_model: [],
  windows: [window("five_hour", no_limit), window("weekly", no_limit)],
});

let answer = detail(NO_LIMIT);

vi.mock("../../lib/ipc/usageHistory", async (original) => {
  const real = await original<typeof import("../../lib/ipc/usageHistory")>();
  return { ...real, usageSession: async () => answer };
});

afterEach(cleanup);

describe("a session's share of the limits", () => {
  it("is words, with why, when the App's limits aren't known: no bar reads as at the limit", async () => {
    answer = detail(NO_LIMIT);
    const { SessionUsage } = await import("./SessionUsage");
    const view = render(<SessionUsage profile="oc-1" conversation="ses-1" profileLabel="OpenCode · R3" />);
    const card = await view.findByTestId("session-usage-share");
    await waitFor(() => expect(within(card).getAllByTestId("session-usage-share-words")).toHaveLength(2));
    expect(within(card).queryByTestId("share-bar")).toBeNull();
    expect(card.textContent).not.toContain("100%");
    expect(card.textContent).toContain("Its share of the app's use");
    expect(card.textContent).not.toContain("Its share of the limits");
    expect(card.textContent).toContain("This session: 32k of the 32k new tokens AI Juggler saw on this app");
    expect(card.textContent).toContain("The last 5 hours");
    expect(card.textContent).toContain("The last 7 days");
    // Why, once, in the adapter's words.
    expect(within(card).getAllByTestId("session-usage-no-limit").map((line) => line.textContent)).toEqual([`${NO_LIMIT}.`]);
  });

  it("keeps its bars against a limit the App's tool reports", async () => {
    answer = detail(null);
    const { SessionUsage } = await import("./SessionUsage");
    const view = render(<SessionUsage profile="cc-1" conversation="c-1" profileLabel="Work" />);
    const card = await view.findByTestId("session-usage-share");
    await waitFor(() => expect(within(card).getAllByTestId("share-bar")).toHaveLength(2));
    expect(card.textContent).toContain("Its share of the limits");
    expect(within(card).queryByTestId("session-usage-share-words")).toBeNull();
    expect(within(card).queryByTestId("session-usage-no-limit")).toBeNull();
  });
});
