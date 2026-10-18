/**
 * AJ-E210 vitest — the Beta validation matrix (Settings → Beta):
 * a REAL table (row headers, scoped columns, status in text), honest
 * statuses ("Beta — not yet validated" everywhere until a live cell
 * + real CI row graduate a pair), the ceiling-lowering disclosure on
 * failed cells, and the copy rule (never "lossless", D-11).
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
    badge: "F1 · Beta",
    validated: false,
    mechanism_green: true,
    ci_green: true,
    dogfood: null,
    status: "CI fixtures green · live dogfood pending — open beta debt",
    notes: [],
  },
  {
    from: "codex",
    to: "codex",
    mechanism: "transplant",
    researched_ceiling: "F1",
    advertised: "F0",
    badge: "F0 · Beta",
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
    status: "Live cell FAILED 2026-10-16 — S-F4: rules ring lost; ceiling lowered one level",
    notes: ["codex adapter not landed — mechanism fixtures only"],
  },
  {
    from: "claude-code",
    to: "gemini-cli",
    mechanism: "pack-only",
    researched_ceiling: "F0",
    advertised: "F0",
    badge: "F0 · Beta",
    validated: false,
    mechanism_green: true,
    ci_green: false,
    dogfood: null,
    status: "Real-adapter fixtures not landed · live dogfood pending — open beta debt",
    notes: ["pack-only: no target adapter planned for beta (SPEC:140 adapter-first)"],
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
    expect(table.querySelector("thead th[scope='col']")!.textContent).toBe("Pair");
    const rowHeader = table.querySelector("tbody th[scope='row']")!;
    expect(rowHeader.textContent).toContain("claude-code → claude-code");
    expect(table.textContent).toContain("Beta — not yet validated");
    expect(table.textContent).toContain("open beta debt");
    expect(table.textContent).not.toMatch(/lossless/i);
  });

  it("the summary counts open beta debt; the failing cell shows its lowered ceiling", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = CELLS;
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-summary"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "All 3 provider pairs ship in beta",
    );
    // The failed cell's disclosure: advertised F0, researched F1,
    // the reason visible.
    const failedRow = Array.from(
      container.querySelectorAll<HTMLTableRowElement>('[data-testid="beta-matrix-row"]'),
    ).find((row) => row.textContent!.includes("codex → codex"))!;
    expect(failedRow.textContent).toContain("F0 · Beta");
    expect(failedRow.textContent).toContain("lowered from F1 — a live cell failed");
    expect(failedRow.textContent).toContain("FAIL · 2026-10-16 (monorepo)");
    // The notes ride along verbatim.
    expect(failedRow.textContent).toContain("codex adapter not landed");
  });

  it("a validated pair reads Validated ✓ and graduates the badge", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = [
      {
        ...CELLS[0]!,
        validated: true,
        badge: "F1 · Validated",
        dogfood: {
          at: "2026-10-17",
          project_shape: "tiny",
          verdict: "pass",
          source_sha256: "aa",
          target_sha256: "bb",
          note: "target answered the pre-switch question",
        },
        status: "Validated — live cell passed 2026-10-17 (tiny shape)",
      },
      CELLS[1]!,
    ];
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="beta-matrix-summary"]')).not.toBeNull(),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "1 of 2 pairs validated · 1 still open beta debt",
    );
    expect(container.textContent).toContain("Validated ✓");
    expect(container.textContent).toContain("pass · 2026-10-17 (tiny)");
    expect(container.textContent).toContain("F1 · Validated");
  });

  it("the honest default (no seam): every pair open beta debt", async () => {
    const { container } = render(<BetaMatrix />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="beta-matrix-row"]').length).toBe(11),
    );
    expect(container.querySelector('[data-testid="beta-matrix-summary"]')!.textContent).toContain(
      "All 11 provider pairs ship in beta",
    );
    expect(container.textContent).not.toMatch(/lossless/i);
    expect(container.textContent).toContain("real row green");
    // Loading state came and went without interaction.
    expect(fireEvent.click(container.querySelector('[data-testid="beta-matrix"]')!)).toBe(true);
  });
});
