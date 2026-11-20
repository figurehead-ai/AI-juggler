/**
 * AJ-E1612 (dry run 2): after a sign-in ended, Home (opened again) said 9
 * and the bell 9, while the rail's Needs you still said 12 and listed the
 * three Apps just signed in. Every Needs-you surface reads the one list
 * (lib/needs-you, `notices:needs`) and the one summary, and follows the
 * backend's one change event: the summary, whose version moving reads
 * every list again (lib/notices-store). The backend's half — the version
 * moves as the window reads it — is centre/tests.rs and sign_in_ended.rs.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { resetNoticeStandIn, standInNeeds, type StandInNeed } from "../lib/ipc/notices";
import { applySummary, resetNotices } from "../lib/notices-store";
import { forgetPaged } from "../lib/paged";
import { NotificationBell } from "./NotificationBell";
import { WorkflowRail } from "./WorkflowRail";

const TARGET = { session_id: null, conversation_id: null, project_id: null, page: null, task_id: null, pool_id: null };

/** "<App> needs its sign-in", as needs.rs words it. */
function signIn(app: string, account: string): StandInNeed {
  return {
    key: `signin:${account}`,
    kind: "signin",
    title: `${app} needs its sign-in`,
    detail: "OpenCode's own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password.",
    project_id: null,
    project: null,
    target: { ...TARGET, account_id: account },
    action: "sign-in",
  };
}

const LAB = [signIn("OpenCode CLI · lab A", "oc-a"), signIn("OpenCode CLI · lab B", "oc-b"), signIn("Kimi Code CLI · lab A", "kimi-a")];

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  resetNoticeStandIn();
  resetNotices();
  forgetPaged();
});
afterEach(cleanup);

function shell() {
  return render(
    <>
      <NotificationBell sessions={[]} />
      <WorkflowRail
        density="comfortable"
        onCycleDensity={() => {}}
        projects={[]}
        sessions={[]}
        loginTerminals={[]}
        selectedId={null}
        onSelectSession={() => {}}
        onSelectProject={() => {}}
        onNewSession={() => {}}
        onNewProject={() => {}}
        header={() => null}
        footer={() => null}
      />
    </>,
  );
}

/** The head's count, from its words ("Needs you 3"): 0.4.6's head has no
 * count of its own to find (the fix goes there too). */
const railCount = (view: ReturnType<typeof shell>) => /Needs you\D*(\d+)/.exec(view.getByTestId("rail-needs-you").textContent ?? "")?.[1];
const railRows = (view: ReturnType<typeof shell>) => view.getAllByTestId("rail-notice").map((row) => row.textContent ?? "");

it("a sign-in's end takes its row out of the rail's Needs you and its count, with the bell, on the one change event", async () => {
  standInNeeds(LAB); // the backend's tick before: three Apps need their sign-in
  const view = shell();
  await waitFor(() => expect(railCount(view)).toBe("3"));
  expect(view.getByTestId("bell-count").textContent).toBe("3");
  expect(railRows(view).some((row) => row.includes("OpenCode CLI · lab A needs its sign-in"))).toBe(true);

  // OpenCode lab A's Sign in terminal ended: the backend works Needs you
  // out again and tells the window (its summary, the version moved).
  act(() => applySummary(standInNeeds(LAB.slice(1))));
  await waitFor(() => expect(railCount(view)).toBe("2"));
  expect(view.getByTestId("bell-count").textContent).toBe("2");
  expect(railRows(view).some((row) => row.includes("OpenCode CLI · lab A needs its sign-in"))).toBe(false);
  expect(railRows(view).some((row) => row.includes("OpenCode CLI · lab B needs its sign-in"))).toBe(true);

  // And the next one (Kimi Code lab A), as the dry run went on.
  act(() => applySummary(standInNeeds([LAB[1]!])));
  await waitFor(() => expect(railCount(view)).toBe("1"));
  expect(view.getByTestId("bell-count").textContent).toBe("1");
  expect(railRows(view)).toHaveLength(1);
});
