/**
 * AJ-E210 vitest — which switches are proven (Accounts, disclosure):
 * a REAL table (row headers, scoped columns, status in text), honest
 * statuses ("Beta — not proven yet" everywhere until a hand check and
 * the real-tool tests prove a switch), the one-level-less note on a
 * failed check, plain words (AJ-E1535) and never "lossless" (D-11).
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { BetaMatrix } from "./BetaMatrix";
import type { ValidationCellData } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

beforeEach(() => {
  delete w().__AIJ_VALIDATION_MATRIX__;
});

const CELLS: ValidationCellData[] = [
  {
    from: "claude-code",
    to: "claude-code",
    mechanism: "transplant",
    researched_ceiling: "F1",
    advertised: "F1",
    badge: "Full story, fresh start · Beta",
    validated: false,
    mechanism_green: true,
    ci_green: true,
    dogfood: null,
    status: "Automatic tests pass with the real tools; not tried by hand yet",
    notes: [],
  },
  {
    from: "codex",
    to: "codex",
    mechanism: "transplant",
    researched_ceiling: "F1",
    advertised: "F0",
    badge: "Handover notes · Beta",
    validated: false,
    mechanism_green: true,
    ci_green: false,
    dogfood: {
      at: "2026-10-16",
      project_shape: "monorepo",
      verdict: "fail",
      source_sha256: "aa",
      target_sha256: "bb",
      note: "S-F4: rules ring lost",
    },
    status: "Failed when tried by hand 2026-10-16: the rules didn't carry. It promises one level less until fixed",
    notes: ["Tested with stand-in tools only"],
  },
  {
    from: "claude-code",
    to: "gemini-cli",
    mechanism: "pack-only",
    researched_ceiling: "F0",
    advertised: "F0",
    badge: "Handover notes · Beta",
    validated: false,
    mechanism_green: true,
    ci_green: false,
    dogfood: null,
    status: "Automatic tests with the real tools aren't in place; not tried by hand yet",
    notes: ["Handover notes only: starting this tool from a story isn't planned for the beta"],
  },
];

describe("AJ-E210 BetaMatrix — the disclosure table", () => {
  it("renders a real table with row headers and text statuses (never colour-only)", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = CELLS;
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-table"]')).not.toBeNull(),
    );
    const table = container.querySelector('[data-testid="beta-matrix-table"]')!;
    expect(table.tagName).toBe("TABLE");
    expect(table.querySelector("thead th[scope='col']")!.textContent).toBe("Switch");
    const rowHeader = table.querySelector("tbody th[scope='row']")!;
    expect(rowHeader.textContent).toBe("Claude Code → Claude Code");
    expect(table.textContent).toContain("Beta — not proven yet");
    expect(table.textContent).toContain("not tried by hand yet");
    expect(table.textContent).not.toMatch(/lossless/i);
    // Plain words (AJ-E1535): no developer terms anywhere in the table.
    expect(container.textContent).not.toMatch(/\bF[0-4]\b|dogfood|CI row|ceiling|tool pairs|beta debt|live cell/i);
  });

  it("the summary counts what's in beta; a failed hand check shows it promises one level less", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = CELLS;
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-summary"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "None of the 2 switches is proven yet; all are in beta.",
    );
    // The failed cell's disclosure: advertised F0, researched F1,
    // the reason visible.
    const failedRow = Array.from(
      container.querySelectorAll<HTMLTableRowElement>('[data-testid="beta-matrix-row"]'),
    ).find((row) => row.textContent!.includes("Codex → Codex"))!;
    expect(failedRow.textContent).toContain("Handover notes · Beta");
    expect(failedRow.textContent).toContain("one level less — a hand check failed");
    expect(failedRow.textContent).toContain("Failed 2026-10-16");
    // The notes ride along verbatim.
    expect(failedRow.textContent).toContain("Tested with stand-in tools only");
  });

  it("a proven switch reads Proven ✓ and graduates the badge", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = [
      {
        ...CELLS[0]!,
        validated: true,
        badge: "Full story, fresh start · Proven",
        dogfood: {
          at: "2026-10-17",
          project_shape: "tiny",
          verdict: "pass",
          source_sha256: "aa",
          target_sha256: "bb",
          note: "target answered the pre-switch question",
        },
        status: "Proven: tried by hand 2026-10-17 on a tiny project",
      },
      CELLS[1]!,
    ];
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-summary"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "1 of 2 switches proven; 1 still in beta.",
    );
    expect(container.textContent).toContain("Proven ✓");
    expect(container.textContent).toContain("Passed 2026-10-17");
    expect(container.textContent).toContain("Full story, fresh start · Proven");
  });

  it("the honest default (no seam): every switch in beta", async () => {
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="beta-matrix-row"]').length).toBe(5),
    );
    // Only switches between the tools AI Juggler has (Claude Code, Codex
    // and OpenCode here): a researched pair for a tool it can't run is
    // left out.
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "None of the 5 switches is proven yet; all are in beta.",
    );
    expect(container.textContent).not.toMatch(/zcode|gemini/i);
    expect(container.textContent).toContain("OpenCode");
    expect(container.textContent).not.toMatch(/lossless/i);
    expect(container.textContent).toContain("Pass with the real tools");
    // Loading state came and went without interaction.
    expect(fireEvent.click(container.querySelector('[data-testid="beta-matrix"]')!)).toBe(true);
  });
});
