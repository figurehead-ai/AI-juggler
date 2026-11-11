/**
 * A8 (the AJ-E3306 rc22 walk): an Account's Settings › Renewal date.
 * Typing 11282026 ended on 12/09/2026: the system date control handed
 * over a value on every keystroke (WebKit fills the parts not typed yet
 * with today's), and each was saved. It also drew an unset date as today.
 * Now the date is saved once, whole, on commit; unset reads "Not set".
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AccountSettings } from "./AccountSettings";
import { dateFormat } from "../../lib/date-text";
import { asInput, subscription } from "../../test/fixtures";

const setTerm = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../../lib/ipc/subscriptions", async (original) => ({ ...(await original<typeof import("../../lib/ipc/subscriptions")>()), subscriptionsSetTerm: setTerm }));

afterEach(() => {
  cleanup();
  setTerm.mockClear();
});

const sub = (renewal: string | null) => subscription({ folder: "claude-work", account_id: "acct:anthropic:work", name: "Work", plan: "max-20x", renewal_date: renewal });

const digits = (iso: string) => {
  const [year = "", month = "", day = ""] = iso.split("-");
  return dateFormat().order.map((part) => ({ year, month, day })[part]).join("");
};
const renewalSaves = () => setTerm.mock.calls.filter((call) => (call as unknown[])[1] === "renewal");

function settings(renewal: string | null = null) {
  render(<AccountSettings sub={sub(renewal)} now={Date.now()} onChanged={() => {}} />);
  return asInput(screen.getByTestId("account-renewal-input"));
}

describe("A8 an Account's renewal date", () => {
  it("typed digit by digit (11282026), saves once, with that date", async () => {
    const user = userEvent.setup();
    const input = settings();
    await user.click(input);
    await user.keyboard(digits("2026-11-28"));
    expect(renewalSaves()).toHaveLength(0);
    await user.tab();
    expect(renewalSaves()).toEqual([["acct:anthropic:work", "renewal", "2026-11-28"]]);
  });

  it("never saves while it's being typed: values it passes through save nothing until it's left", async () => {
    const input = settings();
    // What WebKit's date control handed over typing 11, then 28, then 2026
    // (the parts not typed yet filled with today's, 11/09/2026).
    for (const value of ["2026-11-09", "2026-11-28", "2026-11-28"]) fireEvent.change(input, { target: { value } });
    expect(renewalSaves()).toHaveLength(0);
    fireEvent.blur(input);
    expect(renewalSaves()).toEqual([["acct:anthropic:work", "renewal", "2026-11-28"]]);
  });

  it("a partial date saves nothing, and says so", async () => {
    const user = userEvent.setup();
    const input = settings();
    await user.click(input);
    await user.keyboard(digits("2026-11-28").slice(0, 4));
    await user.tab();
    expect(renewalSaves()).toHaveLength(0);
    expect(screen.getByTestId("account-renewal-input-error").textContent).toContain("Nothing was saved");
  });

  it("unset, it shows empty with Not set, not today's date", () => {
    const input = settings(null);
    expect(input.value).toBe("");
    expect(input.placeholder).toBe("Not set");
  });
});
