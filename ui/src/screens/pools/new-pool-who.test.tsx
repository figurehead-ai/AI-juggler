/**
 * AJ-E3007, AJ-E3011, D-126: New pool's Who's in — paged, the shell-only
 * App said, and (D-126) the Apps as logo cards.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { draftOf, type Candidate, type Draft } from "../../lib/new-pool";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { templateById } from "../../lib/pool-templates";
import { CANDIDATES_PAGE, NewPoolWho } from "./NewPoolWho";

afterEach(cleanup);

/** rc27 live pass 2: step 2 drew 143 session rows at once and was slow. A
 * group shows a page, and Show more adds the next. */
describe("New pool's Who list is paged", () => {
  it("draws a page of a long group, and Show more adds another", () => {
    const rows: Candidate[] = Array.from({ length: 143 }, (_, i) => ({
      ref: `sess-${i}`,
      kind: "session",
      label: `Session ${i}`,
      detail: "lab · ended",
      group: "recent",
      reach: "full",
      blocked: null,
      lastMs: 143 - i,
    }));
    const candidates = { session: rows, app: [], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    render(<NewPoolWho draft={draftOf(templateById("feature-team"), null)} onDraft={() => undefined} candidates={candidates} onFix={() => undefined} />);
    const list = screen.getByTestId("new-pool-candidates");
    expect(within(list).getAllByRole("listitem")).toHaveLength(CANDIDATES_PAGE);
    fireEvent.click(screen.getByTestId("new-pool-more-recent"));
    expect(within(list).getAllByRole("listitem")).toHaveLength(2 * CANDIDATES_PAGE);
    expect(screen.getByTestId("new-pool-more-recent").textContent).toContain(`${143 - 2 * CANDIDATES_PAGE} more`);
    // A search pages its matches the same way.
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Session 1" } });
    expect(within(list).getAllByRole("listitem").length).toBeLessThanOrEqual(CANDIDATES_PAGE);
  });
});

// The P8 walk (rc32, step 4): adding a Kimi Code App left the counters at
// "0 full, 0 shell only", and its row said only "new ones join".
describe("P8 an App member that works through its shell, in the wizard", () => {
  it("its row says Shell only beside new ones join, and the side panel names it", () => {
    const why = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";
    const kimi: Candidate = { ref: "app:cli:kimi-1", kind: "app", label: "Kimi Code · Lab", detail: "none open now", group: "all", reach: "later", blocked: null, shell: why, lastMs: 0 };
    const candidates = { session: [], app: [kimi], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    let draft: Draft = { ...draftOf(templateById("blank"), null), members: [] };
    const { rerender } = render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: /Apps/ }));
    const row = screen.getByTestId("new-pool-candidate");
    expect(row.textContent).toContain("new ones join");
    expect(within(row).getByTestId("shell-only").getAttribute("aria-description")).toBe(why);
    fireEvent.click(within(row).getByRole("checkbox"));
    rerender(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    // Round 2 (#6, #7): in words, without the jargon counters.
    expect(screen.getByTestId("new-pool-side-words").textContent).toBe("1 app");
    expect(screen.getByTestId("new-pool-side-members").textContent).toContain("Kimi Code · Lab");
  });
});
