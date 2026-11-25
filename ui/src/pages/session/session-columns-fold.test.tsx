/**
 * AJ-E1504 R2 (0.5.1 F89): ⌘⇧I folds a session page to its header line:
 * the side column steps aside so the conversation gets the room, and
 * Switch account… (D-143) stays one visible click.
 */
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { SessionColumns } from "./SessionColumns";
import { PageFolded } from "../page-folded";

afterEach(cleanup);

function page(folded: boolean) {
  return render(
    <PageFolded.Provider value={folded}>
      <SessionColumns main={<p>its terminal</p>} side={<p>Where it runs</p>} lead={<button type="button">Switch account…</button>} />
    </PageFolded.Provider>,
  );
}

describe("a session page folded (⌘⇧I)", () => {
  it("shows its side column while open", () => {
    const view = page(false);
    expect(view.getByTestId("session-side").textContent).toContain("Where it runs");
    expect(view.getByText("its terminal")).toBeTruthy();
  });

  it("hides the side column when folded, keeping the conversation and Switch account…", () => {
    const view = page(true);
    expect(view.queryByTestId("session-side")).toBeNull();
    expect(view.queryByText("Where it runs")).toBeNull();
    expect(view.getByText("its terminal")).toBeTruthy();
    expect(view.getByRole("button", { name: "Switch account…" })).toBeTruthy();
    expect(view.getByTestId("session-overview").getAttribute("data-folded")).toBe("true");
  });
});
