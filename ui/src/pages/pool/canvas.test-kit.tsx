/**
 * The pool canvas tests' shared kit: the seams, a two-member pool with an
 * agent's ask (`seedTeam`) and the pool page rendered (`openPage`).
 */
import { act, render, waitFor } from "@testing-library/react";
import { expect } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { refreshPools, resetPools } from "../../lib/pools";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewMessages } from "../../lib/ipc/messages";
import { resetPreviewCanvas } from "../../lib/ipc/pool-canvas";
import { forgetPaged } from "../../lib/paged";
import { forgetCanvasViews } from "./canvas-view";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ToastHost, resetToasts } from "../../components/Toast";
import { PoolPage } from "../PoolPage";

export type Seams = Record<string, unknown> & { __AIJ_POOL_CALLS__?: string[]; __AIJ_MESSAGES_CALLS__?: string[]; __AIJ_POOL_CANVAS_CALLS__?: string[]; __AIJ_STARTED__?: unknown };
/** The window's seams, read and written through `Reflect`. */
const SEAMS = new Proxy<Seams>({}, {
  get: (_, key) => Reflect.get(window, key),
  set: (_, key, value) => Reflect.set(window, key, value),
  deleteProperty: (_, key) => Reflect.deleteProperty(window, key),
});
export const seams = (): Seams => SEAMS;

function nth(index: number): InstanceInfo {
  const session = MOCK_SESSIONS[index];
  if (!session) throw new Error(`the fixtures have no session ${index}`);
  return session;
}
export const [first, second, third] = [nth(0), nth(1), nth(2)];

/** A two-member pool whose second member's agent asks to message the
 * first; `offerAlways` offers "Always allow here" on that ask. */
export function seedTeam(offerAlways = false): void {
  seams().__AIJ_POOLS__ = {
    pools: [{ id: "pool-team", name: "Team", rules: [{ id: 7, pool_id: "pool-team", action: "claim_task", created_at: Date.now() - 60_000, made_by: "user" }] }],
    members: [
      { session_id: first.id, pool_id: "pool-team", role: "lead" },
      { session_id: second.id, pool_id: "pool-team", role: "member" },
    ],
  };
  const at = Date.now() - 60_000;
  seams().__AIJ_MESSAGES__ = {
    threads: [{ id: "thr-1", pool_id: "pool-team", pool_name: "Team", subject: "Totals", people: [], count: 1, hop: 1, hop_cap: 6, state: "open", state_reason: null, updated_at_ms: at, last_from: "", last_line: "", unread: 0 }],
    messages: {
      "thr-1": [
        { id: "m-1", thread_id: "thr-1", from_name: "B", from_kind: "agent", from_session_id: second.id, kind: "note", subject: "Totals", body: "May I?", hop: 1, redacted: 0, to_all: false, at_ms: at, recipients: [{ id: `session:${first.id}`, name: "A", session_id: first.id, state: "held", via: null, deliverable: false, how: "" }] },
      ],
    },
    needs: [{ key: "ask:ask-1", kind: "ask", pool_id: "pool-team", pool_name: "Team", thread_id: "thr-1", message_id: "m-1", session_id: null, ask_id: "ask-1", offer_always: offerAlways, title: "B wants to message A", detail: "", preview: "May I?", at_ms: at }],
  };
}

export async function openPage(poolId = "pool-team", sessions: InstanceInfo[] = MOCK_SESSIONS) {
  await act(async () => {
    await refreshPools();
  });
  const view = render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId={poolId} sessions={sessions} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
        <ToastHost />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
  await waitFor(() => expect(view.getAllByTestId("canvas-node").length).toBeGreaterThan(0));
  return view;
}


/** Before each test: a desktop window (under 1100 px a pool starts in the
 * List view) and every preview store and seam empty. */
export function resetCanvasTests(): void {
  Object.defineProperty(window, "innerWidth", { value: 1280, configurable: true, writable: true });
  resetPreviewPools();
  resetPreviewMessages();
  resetPreviewCanvas();
  forgetCanvasViews();
  resetPools();
  forgetPaged();
  resetToasts();
  for (const key of ["__AIJ_POOLS__", "__AIJ_POOL_CALLS__", "__AIJ_MESSAGES__", "__AIJ_MESSAGES_CALLS__", "__AIJ_POOL_CANVAS__", "__AIJ_POOL_CANVAS_CALLS__", "__AIJ_STARTED__"]) delete seams()[key];
}
