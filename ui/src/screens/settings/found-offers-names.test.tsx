/**
 * 0.5.1 F99 (F-PL1-15 b, the F95 class): Settings › Desktop apps › Found
 * on this computer. Every row had an "Add" and a "Not now" with the same
 * names, so a screen reader couldn't tell which App a button adds. Each
 * is named with its App now; the visible words stay "Add" and "Not now".
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { DesktopAppsPane } from "./DesktopAppsPane";
import { ConversationsProvider } from "../../lib/conversations";
import { resetToasts } from "../../components/Toast";

/** The preview's places seam (the same one keep-alive.test sets). */
function seed(offers: { id: string; label: string }[]): void {
  Object.defineProperty(window, "__AIJ_PLACES__", { value: { places: [], offers }, configurable: true, writable: true });
}

afterEach(() => {
  cleanup();
  resetToasts();
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

describe("F99 Found on this computer: each button named with its App", () => {
  it("finds Add and Not now by role and name, keeps the visible words, and acts on that row", async () => {
    seed([
      { id: "app:claude-app:/Users/you/Profiles/b-lab", label: "Claude app · personal" },
      { id: "app:claude-app:/Users/you/Profiles/c-lab", label: "Claude app · work" },
    ]);
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const add = await screen.findByRole("button", { name: "Add Claude app · personal" });
    const notNow = screen.getByRole("button", { name: "Not now, Claude app · work" });
    expect(screen.getByRole("button", { name: "Add Claude app · work" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Not now, Claude app · personal" })).toBeTruthy();
    // The visible words are unchanged, and each name starts with them
    // (axe's label-content-name-mismatch).
    expect(add.textContent).toBe("Add");
    expect(notNow.textContent).toBe("Not now");
    // No two buttons in the section share a name any more.
    const names = screen.getAllByTestId(/^desktop-offer-(add|not-now)$/).map((button) => button.getAttribute("aria-label"));
    expect(new Set(names).size).toBe(4);
    // Not now on the work row answers that row only.
    fireEvent.click(notNow);
    await waitFor(() => expect(screen.queryByRole("button", { name: "Add Claude app · work" })).toBeNull());
    expect(screen.getByRole("button", { name: "Add Claude app · personal" })).toBeTruthy();
  });
});
