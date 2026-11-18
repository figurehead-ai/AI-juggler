/**
 * D-126 §4b (the rc1 X2 lab: fifty members wrapped down a narrow column
 * with a sideways scrollbar): the roles name three holders, then how many
 * more; who holds each role is one list, a page at a time, with a search.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { RoleCards, holdersLine, holdersOf } from "./setup/RoleCards";
import { RoleHolders, type HolderRow } from "./RoleHolders";

afterEach(cleanup);

const fifty: HolderRow[] = Array.from({ length: 50 }, (_, at) => ({ key: `s-${at}`, name: `web (${at + 1})`, when: "started 5:16 PM", role: at === 0 ? "lead" : "member", select: { kind: "node", id: `s-${at}` } }));

describe("who holds each role, at fifty members (X2 §4b)", () => {
  it("a role names three, then how many more, in one column in the panel", () => {
    expect(holdersLine(fifty.slice(1).map((row) => row.name))).toBe("Now: web (2), web (3), web (4) and 46 more");
    expect(holdersLine([])).toBe("No one yet");
    render(<RoleCards narrow holders={holdersOf(fifty)} />);
    const list = screen.getByRole("list", { name: "What each role does" });
    expect(list.className).not.toContain("sm:grid-cols-2");
    expect(within(screen.getAllByTestId("role-card")[1]!).getByTestId("role-card-holders").textContent).toBe("Now: web (2), web (3), web (4) and 46 more");
  });

  it("one list, ten at a time, each a row of its own that opens its member; Show more; a search", () => {
    const open = vi.fn();
    render(<RoleHolders rows={fifty} onSelect={open} />);
    const list = screen.getByRole("list", { name: "Who holds each role" });
    expect(within(list).getAllByRole("button")).toHaveLength(10);
    expect(within(list).getAllByRole("button")[0]!.textContent).toBe("web (1) · started 5:16 PMLead");
    fireEvent.click(screen.getByRole("button", { name: "Show 10 more" }));
    expect(within(list).getAllByRole("button")).toHaveLength(20);
    fireEvent.change(screen.getByRole("searchbox", { name: "Find a member" }), { target: { value: "web (37)" } });
    const found = within(list).getAllByRole("button");
    expect(found).toHaveLength(1);
    fireEvent.click(found[0]!);
    expect(open).toHaveBeenCalledWith({ kind: "node", id: "s-36" });
    fireEvent.change(screen.getByRole("searchbox", { name: "Find a member" }), { target: { value: "nobody" } });
    expect(screen.getByTestId("role-holders").textContent).toContain("No member matches");
  });

  it("a few members: no search, no Show more", () => {
    render(<RoleHolders rows={fifty.slice(0, 3)} />);
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.queryByTestId("role-holders-more")).toBeNull();
  });
});
