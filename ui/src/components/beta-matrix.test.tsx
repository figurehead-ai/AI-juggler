/**
 * AJ-E210 vitest — which switches are tested (Accounts, disclosure):
 * a REAL table (row headers, scoped columns, status in text), honest
 * statuses ("Beta: still being tested" until every part of a switch was
 * tried with the real tools), the one-level-less note on a failed check,
 * plain words (AJ-E1535; D-127: never "proven") and never "lossless"
 * (D-11).
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { BetaMatrix } from "./BetaMatrix";
import type { ValidationCellData } from "../lib/ipc/validation";

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
    status: "In beta: 7 of 8 parts tried with the real tools so far",
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
    status: "In beta: the automatic tests with the real tools aren't in place yet",
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
    expect(table.textContent).toContain("Beta: still being tested");
    expect(container.textContent).not.toMatch(/proven/i);
    expect(table.textContent).toContain("7 of 8 parts tried with the real tools so far");
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
      "None of the 2 switches is tested in every part yet; all are in beta.",
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

  it("a tested switch reads Tested ✓ and its badge is what it carries alone", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = [
      {
        ...CELLS[0]!,
        validated: true,
        badge: "Full story, fresh start",
        dogfood: {
          at: "2026-10-17",
          project_shape: "tiny",
          verdict: "pass",
          source_sha256: "aa",
          target_sha256: "bb",
          note: "target answered the pre-switch question",
        },
        status: "Tested: every part of the switch was tried with the real tools",
      },
      CELLS[1]!,
    ];
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-summary"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "1 of 2 switches tested; 1 still in beta.",
    );
    expect(container.textContent).toContain("Tested ✓");
    expect(container.textContent).toContain("Passed 2026-10-17");
    expect(container.textContent).toContain("Full story, fresh start");
    expect(container.textContent).not.toMatch(/proven/i);
  });

  it("the honest default (no seam): every switch in beta", async () => {
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="beta-matrix-row"]').length).toBe(10),
    );
    // Only switches between the tools AI Juggler has (Claude Code, Codex,
    // OpenCode and ZCode here): a researched pair for a tool it can't run
    // (Gemini) is left out.
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "None of the 10 switches is tested in every part yet; all are in beta.",
    );
    expect(container.textContent).not.toMatch(/gemini/i);
    expect(container.textContent).toContain("ZCode");
    expect(container.textContent).toContain("OpenCode");
    expect(container.textContent).not.toMatch(/lossless/i);
    expect(container.textContent).toContain("Pass with the real tools");
    // Loading state came and went without interaction.
    expect(fireEvent.click(container.querySelector('[data-testid="beta-matrix"]')!)).toBe(true);
  });
});
