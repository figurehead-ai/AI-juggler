/**
 * A8 (the AJ-E3306 rc22 walk): the shared date field saves a whole, real
 * day once, on commit (leaving it, Enter, or the calendar), never while
 * it's typed; an unset one is empty with "Not set", never today.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DateField } from "./DateField";
import { dateFormat } from "../../lib/date-text";
import { asButton, asInput } from "../../test/fixtures";

afterEach(cleanup);

/** 2026-11-28 as its digits, in this machine's order (11282026 in the US). */
function digitsOf(iso: string): string {
  const [year = "", month = "", day = ""] = iso.split("-");
  return dateFormat().order.map((part) => ({ year, month, day })[part]).join("");
}

function field(value: string | null = null, extra: { min?: string; max?: string } = {}) {
  const onCommit = vi.fn();
  render(
    <>
      <DateField value={value} onCommit={onCommit} ariaLabel="Renewal date" testId="day" {...extra} />
      <button type="button">elsewhere</button>
    </>,
  );
  return { onCommit, input: asInput(screen.getByTestId("day")) };
}

describe("A8 the shared date field", () => {
  it("typed digit by digit, saves once, with that date, when it's left", async () => {
    const user = userEvent.setup();
    const { onCommit, input } = field();
    await user.type(input, digitsOf("2026-11-28"));
    expect(onCommit).not.toHaveBeenCalled();
    await user.tab();
    await user.tab();
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("2026-11-28");
  });

  it("Enter commits; leaving after it doesn't save again", async () => {
    const user = userEvent.setup();
    const { onCommit, input } = field();
    await user.type(input, `${digitsOf("2026-11-28")}{Enter}`);
    await user.click(screen.getByText("elsewhere"));
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("2026-11-28");
  });

  it("a partial or impossible date saves nothing and says why", async () => {
    const user = userEvent.setup();
    const { onCommit, input } = field("2026-03-01");
    await user.clear(input);
    await user.type(input, digitsOf("2026-11-28").slice(0, 4));
    await user.click(screen.getByText("elsewhere"));
    expect(onCommit).not.toHaveBeenCalled();
    expect(screen.getByTestId("day-error").textContent).toMatch(/^Not a whole date yet: type it as .+\. Nothing was saved\.$/);
    expect(input.getAttribute("aria-invalid")).toBe("true");
    await user.clear(input);
    await user.type(input, digitsOf("2026-02-30"));
    await user.click(screen.getByText("elsewhere"));
    expect(onCommit).not.toHaveBeenCalled();
    expect(screen.getByTestId("day-error").textContent).toContain("There's no such day");
    // Escape puts the saved one back.
    await user.click(input);
    await user.keyboard("{Escape}");
    expect(input.value).not.toBe("");
    expect(screen.queryByTestId("day-error")).toBeNull();
  });

  it("unset, it is empty and says Not set, never today", () => {
    const { input } = field(null);
    expect(input.value).toBe("");
    expect(input.placeholder).toBe("Not set");
    expect(input.dataset.value).toBe("");
  });

  it("emptied and left, it is unset", async () => {
    const user = userEvent.setup();
    const { onCommit, input } = field("2026-03-01");
    await user.clear(input);
    await user.click(screen.getByText("elsewhere"));
    expect(onCommit).toHaveBeenCalledWith(null);
  });

  it("a day picked from the calendar saves it at once; days outside the range can't be picked", async () => {
    const user = userEvent.setup();
    const { onCommit } = field("2026-11-10", { min: "2026-11-05", max: "2026-11-20" });
    await user.click(screen.getByTestId("day-calendar"));
    const month = screen.getByTestId("day-month");
    expect(within(month).getByRole("table")).toBeTruthy();
    const chosen = asButton(month.querySelector('[data-day="2026-11-10"]'));
    expect(document.activeElement).toBe(chosen);
    expect(asButton(month.querySelector('[data-day="2026-11-04"]')).disabled).toBe(true);
    // The arrows move a day, a week.
    fireEvent.keyDown(chosen, { key: "ArrowRight" });
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    expect(document.activeElement?.getAttribute("data-day")).toBe("2026-11-18");
    await user.keyboard("{Enter}");
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith("2026-11-18");
    expect(screen.queryByTestId("day-month")).toBeNull();
  });

  it("Escape closes the calendar and gives focus back to its button", async () => {
    const user = userEvent.setup();
    field("2026-11-10");
    await user.click(screen.getByTestId("day-calendar"));
    await user.keyboard("{Escape}");
    expect(screen.queryByTestId("day-month")).toBeNull();
    expect(document.activeElement).toBe(screen.getByTestId("day-calendar"));
  });
});
