/**
 * ADR-0032 Amendment 1 (D-84, D-100): a line of the rail's "Running, not
 * added": its kind by icon, its name, and Add; Not now in its menu.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RailOfferRow } from "./PlaceOffers";

afterEach(() => cleanup());

describe("D-100 a running app not added, in the rail", () => {
  it("shows its kind and name; Add adds it, Not now stops offering it", () => {
    const answers: boolean[] = [];
    render(<RailOfferRow offer={{ id: "app:claude-app:/p/aij-test", label: "Claude app · aij-test", running: true }} onAnswer={(add) => answers.push(add)} />);
    const row = screen.getByTestId("rail-offer");
    expect(row.getAttribute("data-kind")).toBe("desktop");
    expect(screen.getByTestId("app-kind").getAttribute("aria-label")).toBe("Desktop app");
    expect(row.textContent).toContain("Claude app · aij-test, Desktop app, running, not added");
    fireEvent.click(screen.getByRole("button", { name: "Add Claude app · aij-test" }));
    fireEvent.contextMenu(row);
    fireEvent.click(screen.getByTestId("rail-offer-menu-not-now"));
    expect(answers).toEqual([true, false]);
  });

  it("a CLI one wears the CLI mark", () => {
    render(<RailOfferRow offer={{ id: "cli:f1", label: "Claude Code CLI · lab", running: true }} onAnswer={() => undefined} />);
    expect(screen.getByTestId("app-kind").getAttribute("data-kind")).toBe("cli");
  });
});
