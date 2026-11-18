/**
 * D-126 §3: before any session joins, the canvas draws the planned team —
 * the lead on top, the reviewer and everyone in its projects under it,
 * with lines — each card a button that opens it in the inspector (the rc2
 * walk P5); once a session joins, the members are the column again.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_ACCOUNTS, MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { forgetSharedReads } from "../../lib/shared-read";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { PoolPage } from "../PoolPage";
import { plannedPlaces } from "./PlannedTeam";
import { NODE_W } from "./layout";
import { resetCanvasTests, seams } from "./canvas.test-kit";

const POOL = "pool-web";
const rule = (member: string, kind: string, role: string) => ({ member, kind, role, level: null, joined_at: 1, sessions: 0, session_ids: [] });

async function openPlanned() {
  seams().__AIJ_POOLS__ = { pools: [{ id: POOL, name: "web team", rules: [] }], members: [] };
  seams().__AIJ_POOL_RULES__ = {
    [POOL]: { members: [rule("app:cli:claude-work", "app", "member"), rule("app:cli:claude-personal", "app", "reviewer"), rule("all", "everyone", "member")], talk: [], lead_next: "cli:claude-work" },
  };
  await act(async () => {
    await refreshPools();
  });
  return render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId={POOL} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} now={Date.now()} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

beforeEach(() => {
  resetCanvasTests();
  resetPreviewPoolModel();
  forgetSharedReads();
});
afterEach(() => {
  cleanup();
  delete seams().__AIJ_POOL_RULES__;
  resetPreviewPoolModel();
});

describe("the planned team on the diagram (D-126 §3)", () => {
  it("draws the lead on top, the reviewer and everyone under it with lines, and what happens next; no column", async () => {
    const view = await openPlanned();
    const team = await view.findByTestId("planned-team");
    const cards = within(team).getAllByTestId("planned-card");
    expect(cards.map((card) => card.dataset.ruleRef)).toEqual(["app:cli:claude-work", "app:cli:claude-personal", "all"]);
    expect(cards[0]!.textContent).toContain("Lead · its next session in");
    expect(cards[1]!.textContent).toContain("Reviewer · Terminal");
    expect(cards[2]!.textContent).toContain("Member · every new session");
    // From the lead to each, and between the two that may message each other.
    expect(within(team).getByTestId("planned-team-lines").querySelectorAll("path")).toHaveLength(3);
    expect(view.getByTestId("planned-team-next").textContent).toContain("Your team is drawn. No one has joined yet.");
    expect(view.queryByTestId("canvas-rule-nodes")).toBeNull();
  });

  it("each card is a button by its name; a press opens it in the inspector", async () => {
    const view = await openPlanned();
    const team = await view.findByTestId("planned-team");
    const everyone = within(team).getByRole("button", { name: /^Everyone/ });
    fireEvent.click(everyone, { detail: 0 });
    await waitFor(() => expect(view.getByTestId("inspector-title").textContent).toBe(everyone.textContent));
    expect(everyone.getAttribute("aria-pressed")).toBe("true");
  });

  it("a row of up to three under the lead, centred on it", () => {
    const two = plannedPlaces(2, 300);
    expect(two[0]!.x + NODE_W + (two[1]!.x - two[0]!.x - NODE_W) / 2).toBeCloseTo(NODE_W / 2);
    const five = plannedPlaces(5, 300);
    expect(five.map((at) => at.y)).toEqual([300, 300, 300, 500, 500]);
  });
});

describe("once a session has joined", () => {
  it("the members are the column again", async () => {
    seams().__AIJ_POOLS__ = { pools: [{ id: POOL, name: "web team", rules: [] }], members: [{ session_id: MOCK_SESSIONS[0]!.id, pool_id: POOL, role: "member" }] };
    seams().__AIJ_POOL_RULES__ = { [POOL]: { members: [rule("app:cli:claude-personal", "app", "reviewer")], talk: [] } };
    await act(async () => {
      await refreshPools();
    });
    render(
      <ConversationsProvider want={null}>
        <ActionHostProvider host={{}}>
          <PoolPage poolId={POOL} sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} now={Date.now()} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    await screen.findByTestId("canvas-rule-nodes");
    expect(screen.queryByTestId("planned-team")).toBeNull();
    // An App's card in the column is a button too (the rc2 walk P5).
    const app = screen.getByTestId("canvas-rule-node");
    expect(screen.getAllByRole("button", { name: /Claude Code CLI · Personal/ })).toContain(app);
  });
});
