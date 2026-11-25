/**
 * 0.5.1 F27 (AJ-E1553, D-84: one place per thing): an App whose sign-in
 * ended early was in the rail twice, its Sign-ins entry ("Didn't finish")
 * and a Needs you line "… needs its sign-in". The rail shows it once, as
 * the Sign-ins entry; the bell keeps the need; another App's need stays,
 * and the line comes back once the entry leaves.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { resetNoticeStandIn, standInNeeds, type StandInNeed } from "../lib/ipc/notices";
import { applySummary, resetNotices } from "../lib/notices-store";
import { forgetPaged } from "../lib/paged";
import { refreshSignIns, resetSignInsForTests } from "../lib/sign-ins-store";
import { signInApps, withoutSignInsShown } from "../lib/rail-sign-ins";
import type { SignInInfo } from "../lib/ipc/sign-ins";
import { NotificationBell } from "./NotificationBell";
import { WorkflowRail } from "./WorkflowRail";

const TARGET = { session_id: null, conversation_id: null, project_id: null, page: null, task_id: null, pool_id: null };

function need(app: string, account: string): StandInNeed {
  return {
    key: `signin:${account}`,
    kind: "signin",
    title: `${app} needs its sign-in`,
    detail: "Its own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password.",
    project_id: null,
    project: null,
    target: { ...TARGET, account_id: account },
    action: "sign-in",
  };
}

function signIn(account: string, state: SignInInfo["state"]): SignInInfo {
  return { id: `login-${account}`, account_id: account, tool: "claude", title: "Sign in · Claude Code 2", state, page_host: null, code: null, pastes_code: false, last_lines: [], started_at: 1 };
}

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  resetNoticeStandIn();
  resetNotices();
  forgetPaged();
  resetSignInsForTests();
});
afterEach(() => {
  cleanup();
  delete window.__AIJ_SIGN_INS__;
});

function rail(terminals: { id: string; title: string; ended: boolean; account?: string; state?: string }[]) {
  return render(
    <>
      <NotificationBell sessions={[]} />
      <WorkflowRail
        density="comfortable"
        onCycleDensity={() => {}}
        projects={[]}
        sessions={[]}
        loginTerminals={terminals}
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

const rows = (view: ReturnType<typeof rail>) => view.queryAllByTestId("rail-notice").map((row) => row.textContent ?? "");

it("an App whose sign-in didn't finish is in the rail once, as its Sign-ins entry", async () => {
  window.__AIJ_SIGN_INS__ = [signIn("cc-2", "ended")];
  await refreshSignIns();
  standInNeeds([need("Claude Code 2", "cc-2"), need("Codex", "codex-1")]);
  const view = rail([{ id: "login-cc-2", title: "Sign in · Claude Code 2", ended: true, account: "cc-2", state: "ended" }]);
  await waitFor(() => expect(rows(view).some((row) => row.includes("Codex needs its sign-in"))).toBe(true));
  expect(view.getAllByTestId("rail-sign-in")).toHaveLength(1);
  expect(view.getByTestId("rail-sign-in").textContent).toContain("Didn't finish");
  expect(rows(view).some((row) => row.includes("Claude Code 2 needs its sign-in"))).toBe(false);
  // The bell still counts both.
  expect(view.getByTestId("bell-count").textContent).toBe("2");
});

it("the Needs you line comes back once the Sign-ins entry leaves", async () => {
  window.__AIJ_SIGN_INS__ = [signIn("cc-2", "ended")];
  await refreshSignIns();
  standInNeeds([need("Claude Code 2", "cc-2")]);
  const view = rail([]);
  await waitFor(() => expect(view.getByTestId("bell-count").textContent).toBe("1"));
  expect(rows(view).some((row) => row.includes("Claude Code 2 needs its sign-in"))).toBe(false);
  window.__AIJ_SIGN_INS__ = [];
  await act(async () => {
    await refreshSignIns();
  });
  act(() => applySummary(standInNeeds([need("Claude Code 2", "cc-2")])));
  await waitFor(() => expect(rows(view).some((row) => row.includes("Claude Code 2 needs its sign-in"))).toBe(true));
});

/** 0.5.1 F34 (AJ-E3104): the Sign-ins entry is that need's one place in
 * the rail, and the rail's count includes it: the rail, the bell (and
 * Home, which reads the bell's count) say one number, while the entry
 * shows and after it leaves. */
const railCount = (view: ReturnType<typeof rail>) => Number(/Needs you\D*(\d+)/.exec(view.queryByTestId("rail-needs-you")?.textContent ?? "")?.[1] ?? 0);

it("the rail's Needs you count includes a sign-in its Sign-ins entry shows, as the bell's does", async () => {
  window.__AIJ_SIGN_INS__ = [signIn("cc-2", "ended")];
  await refreshSignIns();
  standInNeeds([need("Claude Code 2", "cc-2"), need("Codex", "codex-1")]);
  const view = rail([{ id: "login-cc-2", title: "Sign in · Claude Code 2", ended: true, account: "cc-2", state: "ended" }]);
  await waitFor(() => expect(view.getByTestId("bell-count").textContent).toBe("2"));
  await waitFor(() => expect(railCount(view)).toBe(2));
  expect(rows(view).some((row) => row.includes("Claude Code 2 needs its sign-in"))).toBe(false);
});

it("the rail and the bell agree on one sign-in shown only as its entry, and after the entry leaves", async () => {
  window.__AIJ_SIGN_INS__ = [signIn("cc-2", "running")];
  await refreshSignIns();
  standInNeeds([need("Claude Code 2", "cc-2")]);
  const view = rail([{ id: "login-cc-2", title: "Sign in · Claude Code 2", ended: false, account: "cc-2", state: "running" }]);
  await waitFor(() => expect(view.getByTestId("bell-count").textContent).toBe("1"));
  await waitFor(() => expect(railCount(view)).toBe(1));
  expect(rows(view)).toEqual([]);
  window.__AIJ_SIGN_INS__ = [];
  await act(async () => {
    await refreshSignIns();
  });
  act(() => applySummary(standInNeeds([need("Claude Code 2", "cc-2")])));
  await waitFor(() => expect(rows(view).some((row) => row.includes("Claude Code 2 needs its sign-in"))).toBe(true));
  expect(railCount(view)).toBe(1);
  expect(view.getByTestId("bell-count").textContent).toBe("1");
});

it("leaves out only a sign-in need of an App the Sign-ins show (pure)", () => {
  const apps = signInApps([signIn("a", "running"), signIn("b", "signed_in"), { ...signIn("c", "ended"), account_id: "" }]);
  expect([...apps]).toEqual(["a"]);
  const lines = [
    { kind: "signin", target: { ...TARGET, account_id: "a" } },
    { kind: "signin", target: { ...TARGET, account_id: "b" } },
    { kind: "quota", target: { ...TARGET, account_id: "a" } },
  ];
  expect(withoutSignInsShown(lines, apps).map((line) => `${line.kind}:${line.target.account_id}`)).toEqual(["signin:b", "quota:a"]);
});
