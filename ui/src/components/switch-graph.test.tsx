/**
 * AJ-E209 vitest — the badged ranked switch entry, the Switch Graph
 * timeline, and the beta-copy lint (D-04/D-11: "beta" must never read
 * as "lossless"; cross-Provider rows always carry the beta badge).
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SwitchGraph } from "./SwitchGraph";
import { SwitchTargetRows } from "./SwitchTargetRows";
import type { SwitchTargetData, WorkGraphData } from "../lib/bridge";
import graphSource from "./SwitchGraph.tsx?raw";
import rowsSource from "./SwitchTargetRows.tsx?raw";
import flowSource from "./SwitchFlow.tsx?raw";
import backFlowSource from "./SwitchBackFlow.tsx?raw";
import bridgeSource from "../lib/bridge.ts?raw";

const w = () => window as unknown as Record<string, unknown>;

beforeEach(() => {
  delete w().__AIJ_SWITCH_GRAPH__;
  delete w().__AIJ_EPOCH_RESUME_CALLS__;
  delete w().__AIJ_PACK_EXPORT_CALLS__;
  delete w().__AIJ_EPOCH_RESUME_FAIL__;
  delete w().__AIJ_PACK_EXPORT_FAIL__;
});

afterEach(() => {
  delete w().__AIJ_SWITCH_GRAPH__;
  delete w().__AIJ_EPOCH_RESUME_CALLS__;
  delete w().__AIJ_PACK_EXPORT_CALLS__;
  delete w().__AIJ_EPOCH_RESUME_FAIL__;
  delete w().__AIJ_PACK_EXPORT_FAIL__;
});

const TARGETS: SwitchTargetData[] = [
  {
    recipe_id: "r-work",
    account_id: "a-work",
    account_label: "Work Max",
    badge: "F1 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["todo list", "subagent state"],
    score: 0.62,
    top_factors: ["80% headroom", "usual account for this repo"],
    factors: [
      ["80% headroom", 0.36],
      ["usual account for this repo", 0.2],
      ["fidelity 1 · Beta", 0.15],
      ["≈$0.12 replay", 0.07],
      ["quiet", 0.1],
    ],
    confidence: "Computed",
    tte_min: 262,
    suggested: true,
  },
  {
    recipe_id: "r-lab",
    account_id: "a-lab",
    account_label: "Lab",
    badge: "F2 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["rewind history"],
    filtered_reason: "exhausted — resets in 3h",
    top_factors: [],
    factors: [],
    confidence: "Measured",
    suggested: false,
  },
];

describe("AJ-E209 SwitchTargetRows — the badged ranked entry", () => {
  it("renders badge + Not yet validated + Drop list + top factors on the cross-Provider row", () => {
    const picks: SwitchTargetData[] = [];
    const { container } = render(
      <SwitchTargetRows targets={TARGETS} onPick={(target) => picks.push(target)} />,
    );
    const badge = container.querySelector('[data-testid="fidelity-badge"]')!;
    expect(badge.textContent).toContain("F1 · Beta");
    expect(badge.textContent).toContain("Not yet validated");
    expect(badge.getAttribute("data-validated")).toBe("false");
    expect(
      container.querySelector('[data-testid="target-drop-list"]')!.textContent,
    ).toContain("Doesn't carry: todo list, subagent state");
    expect(container.textContent).toContain("80% headroom · usual account for this repo");
    expect(container.textContent).toContain("exhausts ≈262 min");
    expect(container.textContent).toContain("Computed");
    expect(container.textContent).toContain("Suggested");
  });

  it("the Why expander shows every input incl. switch cost; click is suggest-only", () => {
    const picks: SwitchTargetData[] = [];
    const { container } = render(
      <SwitchTargetRows targets={TARGETS} onPick={(target) => picks.push(target)} />,
    );
    fireEvent.click(container.querySelector("button[aria-expanded]")!);
    expect(container.textContent).toContain("≈$0.12 replay");
    // The suggested row's click is the ONLY action — it hands the
    // target to the host (the E205 flow), nothing executes here.
    fireEvent.click(container.querySelector("button[data-row]:not(:disabled)")!);
    expect(picks.map((target) => target.recipe_id)).toEqual(["r-work"]);
  });

  it("hard-filtered rows stay visible, greyed, with their reason — never clickable", () => {
    const picks: SwitchTargetData[] = [];
    const { container } = render(
      <SwitchTargetRows targets={TARGETS} onPick={(target) => picks.push(target)} />,
    );
    const filtered = Array.from(
      container.querySelectorAll<HTMLButtonElement>("button[data-row]"),
    ).find((button) => button.disabled)!;
    expect(filtered.textContent).toContain("exhausted — resets in 3h");
    fireEvent.click(filtered);
    expect(picks).toEqual([]);
  });
});

const LINEAR_BACK: WorkGraphData[] = [
  {
    title: "Login refactor",
    epochs: [
      { id: "ep-a", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-1" },
      { id: "ep-b", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-2" },
      { id: "ep-a2", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-3" },
    ],
    edges: [
      { from: "ep-a", to: "ep-b", label: "transplant F1 · 12:04", switch_back: false },
      { from: "ep-b", to: "ep-a2", label: "switch-back ↺ · F1", switch_back: true },
    ],
    collapsed: 2,
    hop_note: "3 switches so far ≈41k tokens",
  },
];

describe("AJ-E209 SwitchGraph — the timeline", () => {
  it("renders the A→B→A shape: labelled arrows, the switch-back ↺, collapse pill, hop note", async () => {
    w().__AIJ_SWITCH_GRAPH__ = LINEAR_BACK;
    const { container } = render(<SwitchGraph projectId="p1" onOpenSession={() => {}} />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="switch-graph-node"]').length).toBe(3),
    );
    const edges = Array.from(container.querySelectorAll('[data-testid="switch-graph-edge"]'));
    expect(edges.map((edge) => edge.getAttribute("aria-label"))).toEqual([
      "transplant F1 · 12:04",
      "switch-back ↺ · F1",
    ]);
    expect(container.querySelector('[data-testid="switch-graph-collapse"]')!.textContent).toBe(
      "+2 switches",
    );
    expect(container.querySelector('[data-testid="switch-graph-hop-note"]')!.textContent).toBe(
      "3 switches so far ≈41k tokens",
    );
  });

  it("nodes are buttons (keyboard-first) with ↑/↓ roving focus", async () => {
    w().__AIJ_SWITCH_GRAPH__ = LINEAR_BACK;
    const { container } = render(<SwitchGraph projectId="p1" onOpenSession={() => {}} />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="switch-graph-node"]').length).toBe(3),
    );
    const nodes = Array.from(
      container.querySelectorAll<HTMLButtonElement>('[data-testid="switch-graph-node"]'),
    );
    expect(nodes.every((node) => node.tagName === "BUTTON")).toBe(true);
    nodes[0]!.focus();
    fireEvent.keyDown(container.querySelector('[role="group"]')!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(nodes[1]);
    fireEvent.keyDown(container.querySelector('[role="group"]')!, { key: "ArrowUp" });
    expect(document.activeElement).toBe(nodes[0]);
  });

  it("the node menu offers Resume only when the transcript survives; ⌘R resumes the open node", async () => {
    w().__AIJ_SWITCH_GRAPH__ = LINEAR_BACK;
    const opened: string[] = [];
    const { container } = render(
      <SwitchGraph projectId="p1" onOpenSession={(id) => opened.push(id)} />,
    );
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="switch-graph-node"]').length).toBe(3),
    );
    const nodes = Array.from(
      container.querySelectorAll<HTMLButtonElement>('[data-testid="switch-graph-node"]'),
    );
    // Open the resumable middle node (B on Personal) and resume via ⌘R.
    fireEvent.click(nodes[1]!);
    const menu = container.querySelector('[data-testid="switch-graph-node-menu"]')!;
    expect(menu.textContent).toContain("Personal");
    const resume = container.querySelector('[data-testid="epoch-resume-button"]')!;
    expect(resume.textContent).toContain("Resume (⌘R)");
    fireEvent.keyDown(window, { key: "r", metaKey: true });
    await waitFor(() => expect(w().__AIJ_EPOCH_RESUME_CALLS__).toEqual(["ep-b"]));
    // The new instance opens (suggest-chosen: the user's click started it).
    expect(opened.length).toBe(1);
  });

  it("a failed edge is dashed-labelled, opens the §9 reason + source-safe recovery", async () => {
    w().__AIJ_SWITCH_GRAPH__ = [
      {
        title: "Payments fix",
        epochs: [
          { id: "ep-src", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-src" },
          { id: "ep-dst", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-dst" },
        ],
        edges: [
          {
            from: "ep-src",
            to: null,
            label: "transplant F1 · target launch failed",
            failed_reason: "S-F6: target window closed mid-launch",
            switch_back: false,
          },
        ],
        collapsed: 0,
      },
    ];
    const opened: string[] = [];
    const { container } = render(
      <SwitchGraph projectId="p1" onOpenSession={(id) => opened.push(id)} />,
    );
    await waitFor(() =>
      expect(container.querySelector('[data-testid="switch-graph-failed-edge"]')).not.toBeNull(),
    );
    fireEvent.click(container.querySelector('[data-testid="switch-graph-failed-edge"]')!);
    const detail = container.querySelector('[data-testid="switch-graph-failed-detail"]')!;
    expect(detail.textContent).toContain("S-F6: target window closed mid-launch");
    expect(detail.textContent).toContain("the source stayed resumable");
    fireEvent.click(
      Array.from(detail.querySelectorAll<HTMLButtonElement>("button")).find((button) =>
        button.textContent!.includes("Open the source epoch"),
      )!,
    );
    expect(opened).toEqual(["sess-src"]);
  });

  it("Export pack fires only from the click and lands its path in the toast", async () => {
    w().__AIJ_SWITCH_GRAPH__ = LINEAR_BACK;
    const { container } = render(<SwitchGraph projectId="p1" onOpenSession={() => {}} />);
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="switch-graph-node"]').length).toBe(3),
    );
    fireEvent.click(container.querySelectorAll('[data-testid="switch-graph-node"]')[0]!);
    expect(w().__AIJ_PACK_EXPORT_CALLS__).toBeUndefined();
    fireEvent.click(container.querySelector('[data-testid="epoch-export-button"]')!);
    await waitFor(() => expect(w().__AIJ_PACK_EXPORT_CALLS__).toEqual(["ep-a"]));
    await waitFor(() =>
      expect(container.querySelector('[data-testid="pack-export-toast"]')!.textContent).toContain(
        "Pack exported to ",
      ),
    );
  });

  it("renders the honest empty state when no switches exist", async () => {
    w().__AIJ_SWITCH_GRAPH__ = [];
    const { container } = render(<SwitchGraph projectId="p1" onOpenSession={() => {}} />);
    await waitFor(() =>
      expect(container.querySelector('[data-testid="switch-graph-empty"]')).not.toBeNull(),
    );
    expect(container.textContent).toContain("No switches yet — your work starts here.");
  });
});

describe("AJ-E209 copy lint — beta is never lossless (D-04/D-11)", () => {
  const switchSources = [graphSource, rowsSource, flowSource, backFlowSource];

  it("no switch surface copy claims lossless or exact fidelity", () => {
    // "lossless"/"exactly like"/"perfect copy" are banned below F4 —
    // and nothing is F4-validated yet, so they are banned outright.
    for (const source of switchSources) {
      expect(source, source.slice(0, 60)).not.toMatch(/lossless/i);
      expect(source, source.slice(0, 60)).not.toMatch(/exactly like/i);
      expect(source, source.slice(0, 60)).not.toMatch(/perfect cop/i);
    }
  });

  it("cross-Provider badges always carry the beta qualifier in copy and fixtures", () => {
    // Component copy says it explicitly…
    expect(rowsSource).toContain("Not yet validated");
    expect(rowsSource).toContain("Badges are per provider pair and beta-validated");
    // …and the bridge fixtures honour it (cross-Provider ⇒ Beta).
    const badges = Array.from(bridgeSource.matchAll(/badge: "(F\d[^"]*)"/g)).map(
      (m) => m[1] as string,
    );
    expect(badges.length).toBeGreaterThan(0);
    for (const badge of badges) {
      expect(badge).toMatch(/Beta|Validated/);
      expect(badge).not.toMatch(/lossless/i);
    }
  });
});
