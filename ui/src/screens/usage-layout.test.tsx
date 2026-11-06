/**
 * The 0.4 review (37): Monthly caps was a half-width card with an empty
 * half beside it. It is a full line, as every card above it.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { UsageScreen } from "./Usage";
import type { AccountInfo } from "../types/generated/AccountInfo";

afterEach(cleanup);

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
] as unknown as AccountInfo[];

it("gives Monthly caps the whole row", async () => {
  const view = render(<UsageScreen accounts={ACCOUNTS} sessions={[]} onAddAccounts={() => null} />);
  const caps = await view.findByTestId("usage-caps");
  const row = view.getByTestId("usage-caps-row");
  expect(caps.parentElement).toBe(row);
  // One column at every width: no half beside it.
  expect(row.className).not.toMatch(/grid-cols-(?!1\b)/);
});
