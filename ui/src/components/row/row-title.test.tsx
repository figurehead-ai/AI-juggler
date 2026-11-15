/**
 * D-96 D at rail width: an untitled session's name ("billing · Work ·
 * started 1:14 AM") is cut at the row's end, its time first; the whole
 * name is in the title's tooltip, in every density, as Home's rows keep
 * a cut detail whole in theirs.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionRow } from "./SessionRow";
import { RowProject } from "./RowParts";
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

/** 0.4.5 (persona D3's walk: "api · 2 d a…" with room to spare): beside
 * its project a row's time is never capped or shrunk; the project gives
 * way. Its cap was a percentage of a line sized by its own content, so
 * beside a project narrower than 2.5ch it equalled the time's width and
 * the app's WebKit, rounding, cut it (no test browser reproduces that
 * rounding, so the cause is held here). */
describe("a row's time beside its project", () => {
  it("stays whole: the time doesn't shrink and has no cap; the project gives way", () => {
    const session = instance({ id: "s-api", cwd: "/Users/you/Dev/api" });
    const [item] = itemsOf({ sessions: [session], conversations: [], projects: [], seen: {} });
    render(<RowProject item={{ ...item!, project: "api", projectId: null }} rest="2 d ago" whole />);
    const when = screen.getByTestId("row-when");
    expect(when.textContent).toBe(" · 2 d ago");
    expect(when.className).toContain("shrink-0");
    expect(when.className).not.toMatch(/max-w-|truncate/);
    expect(screen.getByText("api").className).toMatch(/min-w-\[2\.5ch\] shrink\b/);
  });

  it("alone on its line, it may still be cut at the line's end", () => {
    const session = instance({ id: "s-none", cwd: "/Users/you/Dev/api" });
    const [item] = itemsOf({ sessions: [session], conversations: [], projects: [], seen: {} });
    render(<RowProject item={{ ...item!, project: null, projectId: null }} rest="2 d ago" whole />);
    expect(screen.getByTestId("row-when").className).toContain("truncate");
  });
});
