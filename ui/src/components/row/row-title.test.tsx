/**
 * D-96 D at rail width: an untitled session's name ("billing · Work ·
 * started 1:14 AM") is cut at the row's end, its time first; the whole
 * name is in the title's tooltip, in every density, as Home's rows keep
 * a cut detail whole in theirs.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionRow } from "./SessionRow";
import { itemsOf } from "../../lib/rail-items";
import { instance } from "../../test/fixtures";

afterEach(cleanup);

describe("a row's cut title", () => {
  it.each(["cozy", "compact"] as const)("keeps its whole name in its tooltip (%s)", (density) => {
    const session = instance({ id: "s-bill", cwd: "/Users/you/Dev/billing-reconciliation-service", account_label: "Work", git_branch: "main" });
    const [item] = itemsOf({ sessions: [session], conversations: [], projects: [], seen: {} });
    render(<SessionRow item={item!} density={density} now={Date.now()} onSelect={vi.fn()} />);
    const title = screen.getByTestId("row-title");
    expect(title.className).toContain("truncate");
    expect(title.getAttribute("title")).toBe(item!.title);
    expect(title.getAttribute("title")).toContain("started");
  });
});
