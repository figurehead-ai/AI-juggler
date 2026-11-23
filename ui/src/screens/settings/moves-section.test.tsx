/**
 * AJ-E2325 vitest — Settings › Desktop apps › Moves: "Keep move backups
 * for" names its four choices in days, 7 days by default, and a choice is
 * kept as the setting the sweep reads; no moves, no section.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { KEEP_KEY, MovesSection } from "./MovesSection";
import type { MoveRecordInfo } from "../../lib/ipc/switch-move";
import { appSettingGet } from "../../lib/ipc/settings";


const one: MoveRecordInfo = {
  move_id: "m1",
  conversation: "c1",
  title: "Fix the parser",
  from: "A",
  to: "B",
  state: "committed",
  at_ms: 1,
  detail: null,
  can_undo: true,
  undo_why: null,
  can_finish: false,
  has_backup: true,
  kept_until_ms: null,
  timeline: [],
  kept: [],
};

afterEach(() => {
  cleanup();
  delete window.__AIJ_MOVES__;
  Reflect.deleteProperty(window, "__AIJ_APP_SETTINGS__");
});

describe("Moves in Settings", () => {
  it("no moves: no section", async () => {
    window.__AIJ_MOVES__ = [];
    const { container } = render(<MovesSection />);
    await waitFor(() => expect(container.innerHTML).toBe(""));
  });

  it("keeps backups 7 days by default, names the choices in days, and keeps a choice", async () => {
    window.__AIJ_MOVES__ = [one];
    render(<MovesSection />);
    const group = await screen.findByTestId("moves-keep-choice");
    const names = within(group).getAllByRole("radio").map((radio) => radio.textContent);
    expect(names).toEqual(["1 day", "7 days", "30 days", "Until I delete them"]);
    expect(within(group).getByRole("radio", { name: "7 days" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(within(group).getByRole("radio", { name: "1 day" }));
    await waitFor(async () => expect(await appSettingGet(KEEP_KEY)).toBe("1"));
    expect(screen.getByTestId("move-kept-until").textContent).toBe("Backup kept until you delete it");
  });
});
