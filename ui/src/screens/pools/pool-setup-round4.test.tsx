/**
 * AJ-E3011 round 4 (docs/qa/evidence/AJ-E3011/walks/after3-run1.md, its
 * "Round 4" list, item 1 as main's ruling replaced it).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { resetPreviewPools, scopeOf, type PoolInfo } from "../../lib/ipc/pools";
import { draftOf, type Draft } from "../../lib/new-pool";
import { nextLine, shapeOfDraft } from "../../lib/pool-sentences";
import { templateById } from "../../lib/pool-templates";
import { resetPending, settleAdds, settleRoles, usePendingAdds, usePendingRoles, withPendingAdd, withPendingRole } from "../../lib/pool-pending";
import { InspectPool } from "../../pages/pool/InspectPool";
import { InspectRule } from "../../pages/pool/InspectRule";
import { flushHeld, resetToasts, undoLatest } from "../../components/Toast";
import { RuleNodes } from "../../pages/pool/RuleNodes";
import { NewPool } from "./NewPool";
import { NewPoolReview } from "./NewPoolReview";

beforeEach(() => {
  Object.assign(window, { __AIJ_AGENT_BRIDGE__: undefined });
  resetPreviewAgentBridge();
  resetPending();
  resetToasts();
});
afterEach(cleanup);

const app = (ref: string, label: string, role: "member" | "reviewer" = "member") => ({ ref, kind: "app" as const, label, role, level: null, reach: "later" as const });

function nextTo(title: string): void {
  for (let n = 0; n < 9 && screen.queryByTestId("new-pool-step-title")?.textContent !== title; n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
  expect(screen.getByTestId("new-pool-step-title").textContent).toBe(title);
}

describe("AJ-E3011 round 4", () => {
  it("#1 (D-30): New pool asks the one question as its own step while unanswered; Yes turns them on", async () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    await waitFor(() => expect(screen.getAllByTestId("new-pool-step").map((step) => step.textContent)).toContain("8Team tools"));
    nextTo("Team tools");
    expect(screen.getByTestId("new-pool-tools").querySelector("h3")?.textContent).toBe("Let the sessions in your pools message each other and share notes and tasks?");
    await act(async () => fireEvent.click(screen.getByTestId("new-pool-tools-yes")));
    await waitFor(() => expect(screen.queryByTestId("new-pool-tools")).toBeNull());
    // Answered here: the step stays, and says where they stand.
    expect(screen.getByTestId("new-pool-tools-answered").textContent).toBe("On: they can message each other and share notes and tasks.");
  });

  it("#1 (D-30): once answered, New pool never asks again", async () => {
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: true, on: true } });
    resetPreviewAgentBridge();
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.getAllByTestId("new-pool-step").map((step) => step.textContent)).not.toContain("8Team tools");
    expect(screen.getAllByTestId("new-pool-step")).toHaveLength(8);
  });

  it("AJ-E3013: from a folder others can write, New pool's step says why the tools can't run and what to do, with no Yes; not in this build, no step", async () => {
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: false, on: false, available: false, unavailable: "unsafe_location", unsafe_folder: "~/Downloads" } });
    resetPreviewAgentBridge();
    const view = render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    await waitFor(() => expect(screen.getAllByTestId("new-pool-step").map((step) => step.textContent)).toContain("8Team tools"));
    nextTo("Team tools");
    expect(screen.getByTestId("new-pool-tools-line").textContent).toBe("AI Juggler's team tools can't run from a folder others can write (~/Downloads). Move AI Juggler to Applications, then open it again.");
    for (const button of ["new-pool-tools-yes", "new-pool-tools-not-now", "new-pool-tools-on"]) expect(screen.queryByTestId(button)).toBeNull();
    view.unmount();
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: false, on: false, available: false, unavailable: "missing" } });
    resetPreviewAgentBridge();
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} project="proj-ai-juggler" onClose={() => undefined} />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(screen.getAllByTestId("new-pool-step")).toHaveLength(8);
  });

  it("#3: what happens next, in one line, from the shared sentences: Review and the diagram alike", () => {
    const draft: Draft = { ...draftOf(templateById("blank"), "p-web"), members: [app("app:cli:work", "Claude Code · Work"), app("app:cli:codex", "Codex · Personal", "reviewer")], leadNext: "app:cli:work" };
    const line = nextLine(shapeOfDraft(draft, (_, label) => label), "web");
    expect(line).toBe("Open Claude Code · Work in web: it leads; Codex · Personal checks its work; their messages show here");
    render(<NewPoolReview draft={draft} name="web team" where="web" />);
    expect(screen.getByTestId("new-pool-review-next").textContent).toBe(`What happens next: ${line}.`);
  });

  it("#5: an add shows at once, pending, and settles when read back; refused, it rolls back", async () => {
    const rule = { member: "app:cli:work", kind: "app", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] };
    let resolve: () => void = () => undefined;
    const added = withPendingAdd({ poolId: "p", ref: "app:cli:codex", label: "Codex · Personal", kind: "app" }, () => new Promise<void>((done) => (resolve = done)));
    const view = render(<RuleNodes poolId="p" rules={[rule]} watched={[]} labelOf={() => "Claude Code · Work"} selection={{ kind: "none" }} onSelect={() => undefined} />);
    // At once (the same frame): its card, pending.
    expect(view.getByTestId("canvas-rule-node-adding").textContent).toContain("Codex · Personal");
    expect(view.getByTestId("canvas-rule-node-adding").getAttribute("aria-busy")).toBe("true");
    resolve();
    await added;
    // Read back: the real card takes its place.
    view.rerender(<RuleNodes poolId="p" rules={[rule, { ...rule, member: "app:cli:codex" }]} watched={[]} labelOf={(one) => (one.member === "app:cli:codex" ? "Codex · Personal" : "Claude Code · Work")} selection={{ kind: "none" }} onSelect={() => undefined} />);
    await waitFor(() => expect(view.queryByTestId("canvas-rule-node-adding")).toBeNull());
    // Refused: gone at once, and the caller is told why.
    await expect(withPendingAdd({ poolId: "p", ref: "app:cli:kimi", label: "Kimi", kind: "app" }, () => Promise.reject(new Error("refused: sealed")))).rejects.toThrow("refused: sealed");
    expect(view.queryByTestId("canvas-rule-node-adding")).toBeNull();
  });

  it("#5: a role change shows at once, pending, and settles on the role read back", async () => {
    function Probe() {
      const roles = usePendingRoles("p");
      const adding = usePendingAdds("p");
      return (
        <>
          <output data-testid="probe-roles">{[...roles].map(([ref, role]) => `${ref}=${role}`).join(",")}</output>
          <output data-testid="probe-adding">{adding.length}</output>
        </>
      );
    }
    render(<Probe />);
    const probe = () => ({ roles: screen.getByTestId("probe-roles").textContent, adding: screen.getByTestId("probe-adding").textContent });
    let resolve: () => void = () => undefined;
    const changed = withPendingRole({ poolId: "p", ref: "s1", role: "reviewer" }, () => new Promise<void>((done) => (resolve = done)));
    await waitFor(() => expect(probe().roles).toBe("s1=reviewer"));
    resolve();
    await changed;
    act(() => settleRoles("p", new Map([["s1", "reviewer"]])));
    await waitFor(() => expect(probe().roles).toBe(""));
    // Nothing to settle: nothing changes (the screen reads the list by identity).
    act(() => settleAdds("p", new Set()));
    expect(probe().adding).toBe("0");
  });

  it("Take out of the pool is held, like Stop: Undo in time keeps the member, letting it run takes it out", async () => {
    const pool: PoolInfo = { id: "pool-x", name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
    const rule = { member: "app:cli:codex", kind: "app", role: "reviewer", level: null, joined_at: 1, sessions: 0, session_ids: [] };
    Object.assign(window, { __AIJ_POOLS__: { pools: [{ id: "pool-x", name: "web team" }] } });
    resetPreviewPools();
    const calls = (): string[] => {
      const seen: unknown = Reflect.get(window, "__AIJ_POOL_CALLS__");
      return Array.isArray(seen) ? seen.map(String) : [];
    };
    let gone = 0;
    render(<InspectRule pool={pool} rule={rule} watched={undefined} label="Codex · Personal" onChanged={() => undefined} onGone={() => (gone += 1)} />);
    fireEvent.click(screen.getByTestId("inspect-rule-leave"));
    expect(gone).toBe(1);
    // Held: nothing sent; Undo keeps it.
    expect(undoLatest()).toBe(true);
    await act(async () => flushHeld());
    expect(calls()).not.toContain("leave:pool-x:app:cli:codex");
    // Again, and let it run.
    fireEvent.click(screen.getByTestId("inspect-rule-leave"));
    await act(async () => flushHeld());
    await waitFor(() => expect(calls()).toContain("leave:pool-x:app:cli:codex"));
  });

  it("#6: the rarer settings are under one More settings, closed at first", () => {
    const pool: PoolInfo = { id: "p", name: "web team", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 0, ...scopeOf("user", null) };
    render(<InspectPool pool={pool} project={undefined} now={0} onDeleted={() => undefined} />);
    const more = screen.getByTestId("pool-more-fold");
    expect(more.hasAttribute("open")).toBe(false);
    expect(more.querySelector("summary")?.textContent).toContain("More settings");
    for (const inside of ["pool-seal-section", "pool-spans-macs"]) expect(more.querySelector(`[data-testid="${inside}"]`)).not.toBeNull();
    expect(more.textContent).toContain("Pools its agents may write to");
  });
});
