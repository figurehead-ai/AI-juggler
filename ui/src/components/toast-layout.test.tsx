/**
 * The rc31 NY1 walk: on the archive and unarchive toasts, "Undo (3s)" sat
 * over "…archived — nothing deleted.": the sentence after the name was
 * kept on one line, and Undo beside it. A control never covers text
 * (tests/visual/toast-narrow.spec.ts measures it in a browser).
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { resetToasts, showToast, ToastHost } from "./Toast";

afterEach(() => {
  cleanup();
  resetToasts();
});

describe("a toast's layout", () => {
  it("an archive toast puts Undo under its words, and its sentence wraps", () => {
    render(<ToastHost />);
    const message = "web · Work · work · started 7:04 PM: archived — nothing deleted.";
    act(() => void showToast({ message, actionBelow: true, undo: { onUndo: () => {} } }));
    const undo = screen.getByRole("button", { name: /Undo/ });
    const words = screen.getByText((_, el) => el?.tagName === "P" && el.textContent === message);
    expect(words.parentElement?.contains(undo), "Undo in the words' column, under them").toBe(true);
    expect(words.compareDocumentPosition(undo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    for (const kept of words.querySelectorAll(".whitespace-nowrap")) expect(kept.textContent).not.toContain("nothing deleted");
    expect([...words.querySelectorAll(".whitespace-nowrap")].map((kept) => kept.textContent)).toEqual(["web", "Work", "work", "started 7:04 PM:"]);
  });

});
