/**
 * 0.3 — choosing in a Picker in a test, as a user does: open it, then
 * click the line whose label (not its hint) is `label`.
 */
import { fireEvent, screen } from "@testing-library/react";

/** The open menu's line labelled `label` (exactly, or matching). */
export function pickerLine(label: string | RegExp): HTMLElement {
  const lines = [...screen.getAllByRole("menuitemradio"), ...screen.queryAllByRole("menuitemcheckbox")];
  const found = lines.find((line) => {
    const text = line.querySelector("[data-choice-label]")?.textContent ?? line.textContent ?? "";
    return typeof label === "string" ? text === label : label.test(text);
  });
  if (!found) throw new Error("no such line in the open menu");
  return found;
}

/** Opens `picker` and picks the line labelled `label`. */
export function pick(picker: HTMLElement, label: string | RegExp): void {
  fireEvent.click(picker);
  fireEvent.click(pickerLine(label));
}
