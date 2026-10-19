/**
 * AJ-E1103 vitest — interface usage-stats events: what the switch
 * target list and the update notice count, through the preview seam
 * (in the app the backend accepts only schema values, and only once the
 * user opted in).
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SwitchTargetRows } from "../components/SwitchTargetRows";
import type { SwitchTargetData } from "./bridge";

type Seen = { metric: string; dims: Record<string, string>; value?: number };
const w = () => window as unknown as { __AIJ_STATS_EVENTS__?: Seen[] };

const target = (recipe: string, suggested: boolean): SwitchTargetData => ({
  recipe_id: recipe,
  provider: "codex",
  account_id: `a-${recipe}`,
  account_label: `Account ${recipe}`,
  badge: "F1 · Beta",
  validated: false,
  cross_provider: false,
  drops: [],
  top_factors: [],
  factors: [],
  confidence: "Computed",
  suggested,
});

beforeEach(() => {
  delete w().__AIJ_STATS_EVENTS__;
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_STATS_EVENTS__;
});

describe("AJ-E1103 interface usage-stats events", () => {
  it("a suggestion counts as shown, then accepted", () => {
    const { getByText } = render(
      <SwitchTargetRows targets={[target("one", true), target("two", false)]} onPick={() => undefined} />,
    );
    fireEvent.click(getByText("Account one"));
    expect(w().__AIJ_STATS_EVENTS__).toEqual([
      { metric: "usage.suggestion", dims: { provider: "codex", action: "shown" }, value: undefined },
      { metric: "usage.suggestion", dims: { provider: "codex", action: "accepted" }, value: undefined },
    ]);
  });

  it("picking another target counts the suggestion as dismissed", () => {
    const { getByText } = render(
      <SwitchTargetRows targets={[target("one", true), target("two", false)]} onPick={() => undefined} />,
    );
    fireEvent.click(getByText("Account two"));
    expect(w().__AIJ_STATS_EVENTS__?.map((e) => e.dims.action)).toEqual(["shown", "dismissed"]);
  });

  it("no suggestion, nothing counted; never a label or path", () => {
    const { getByText } = render(
      <SwitchTargetRows targets={[target("two", false)]} onPick={() => undefined} />,
    );
    fireEvent.click(getByText("Account two"));
    expect(w().__AIJ_STATS_EVENTS__ ?? []).toEqual([]);
  });
});
