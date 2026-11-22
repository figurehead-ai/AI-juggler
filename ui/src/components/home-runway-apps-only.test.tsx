/**
 * The 0.5.0 re-walk, R1: Home showed "This screen hit a problem" on lab
 * PL1, whose Accounts hold only desktop apps (two lab Claude app profiles,
 * no CLI folder). Runway by account read its first folder's tool, and there
 * was none. Such an Account is listed, with nothing to measure from a CLI.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { HomeRunway } from "./HomeRunway";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
});

it("lists an Account that only desktop apps use, without a CLI folder, and doesn't crash", async () => {
  Object.assign(window, {
    __AIJ_FIXTURE__: {
      subscriptions: [
        {
          key: "k-personal",
          tool: "claude-code",
          state: "signed_in",
          email: "p…@example.com",
          profiles: [],
          apps: ["app:claude:personal"],
          organisation: null,
          reason: null,
          plan: null,
          renewal_date: null,
          fee_usd: null,
          differs: [],
          choices: [],
        },
      ],
    },
  });
  render(<HomeRunway accounts={[]} quota={[]} sessions={[]} />);
  const rows = await screen.findAllByTestId("home-runway-account");
  expect(rows).toHaveLength(1);
  // Nothing a CLI could measure here: no Measure its limits.
  expect(rows[0]!.textContent).not.toContain("Measure");
});
