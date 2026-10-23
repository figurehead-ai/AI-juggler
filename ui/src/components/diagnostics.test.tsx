/**
 * AJ-E1517 vitest — Settings › Report a problem: the bug form opens; the
 * diagnostics are shown file by file with what's left out named before
 * anything is saved; cancelling (or the save sheet) saves nothing; saving
 * saves the preview shown and says where.
 */
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DiagnosticsCard } from "./DiagnosticsCard";

const w = () => window as unknown as Record<string, unknown>;

afterEach(cleanup);
beforeEach(() => {
  for (const key of ["__AIJ_DIAGNOSTICS__", "__AIJ_DIAGNOSTICS_DEST__", "__AIJ_DIAGNOSTICS_SAVED__", "__AIJ_BUG_FORM_OPENED__"]) delete w()[key];
});

describe("AJ-E1517 — Report a problem", () => {
  it("opens the bug form", async () => {
    const { getByTestId } = render(<DiagnosticsCard />);
    await act(async () => {
      fireEvent.click(getByTestId("report-bug"));
    });
    expect(w().__AIJ_BUG_FORM_OPENED__).toBe(1);
  });

  it("shows every file and what's left out before anything is saved", async () => {
    const { getByTestId, findByTestId, getAllByTestId, queryByTestId } = render(<DiagnosticsCard />);
    await act(async () => {
      fireEvent.click(getByTestId("diagnostics-show"));
    });
    const preview = await findByTestId("diagnostics-preview");
    expect(getAllByTestId("diagnostics-file").map((row) => row.querySelector("summary span")?.textContent)).toEqual([
      "about.json",
      "app-settings.json",
      "logs/supervision.jsonl",
    ]);
    expect(within(preview).getByLabelText("about.json (read-only)").textContent).toContain('"schema_on_disk": 31');
    expect(getByTestId("diagnostics-left-out").textContent).toContain("3 settings that hold tokens, addresses or paths");
    expect(w().__AIJ_DIAGNOSTICS_SAVED__).toBeUndefined();
    // Cancelled at the save sheet: nothing saved, the preview stays.
    await act(async () => {
      fireEvent.click(getByTestId("diagnostics-save"));
    });
    expect(w().__AIJ_DIAGNOSTICS_SAVED__).toBeUndefined();
    expect(queryByTestId("diagnostics-preview")).not.toBeNull();
    fireEvent.click(getByTestId("diagnostics-cancel"));
    expect(queryByTestId("diagnostics-preview")).toBeNull();
  });

  it("saves the preview shown and says where", async () => {
    w().__AIJ_DIAGNOSTICS_DEST__ = "/Users/someone/Desktop/d.zip";
    const { getByTestId, findByTestId, queryByTestId } = render(<DiagnosticsCard />);
    await act(async () => {
      fireEvent.click(getByTestId("diagnostics-show"));
    });
    await findByTestId("diagnostics-preview");
    await act(async () => {
      fireEvent.click(getByTestId("diagnostics-save"));
    });
    expect(w().__AIJ_DIAGNOSTICS_SAVED__).toEqual({ id: "preview", dest: "/Users/someone/Desktop/d.zip" });
    expect(queryByTestId("diagnostics-preview")).toBeNull();
    expect(getByTestId("settings-diagnostics").textContent).toContain("Saved to ~/Desktop/d.zip");
  });
});
