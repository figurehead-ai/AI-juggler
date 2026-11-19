/**
 * NY1 (the 0.4.6 rc3a walk; the maintainer's "stays until a restart"): a
 * session that stopped with an error, archived and unarchived.
 * - Archived, its rail row left at once, but the bell stayed at 1 for
 *   50 s over its own "You're all caught up"; only Mark all read cleared it.
 * - Unarchived, the rail's Needs you and the bell stayed empty for 22 s
 *   and more, while Home, opened, read the need back and said
 *   "1 session needs you".
 * The window followed the backend only through its change event, which
 * it doesn't always hear (a window not shown holds no listener, AJ-E805;
 * two emissions can land out of order), and each surface kept its own
 * copy: the bell the summary's count, the rail and Home each their own
 * read of the list. Now an archive or an unarchive reads them again
 * itself, the bell counts the rows, and the list is one wherever shown.
 */
import { act, cleanup, render, waitFor, type RenderResult } from "@testing-library/react";
import type { JSX } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { NoticeInfo } from "../types/generated/NoticeInfo";
import type { NoticeSummaryInfo } from "../types/generated/NoticeSummaryInfo";
import type { PageRequest } from "../types/generated/PageRequest";

const CHANGED = "notifications://changed";

/** "“web · started 9:23 PM” stopped with an error", as needs.rs has it. */
const crashed = (id: number, session: string): NoticeInfo => ({
  id,
  kind: "failed",
  needs: true,
  title: "“web · started 9:23 PM” stopped with an error",
  detail: "web · AI Juggler · Work",
  project_id: null,
  project: null,
  target: { session_id: session, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null },
  facts: { window: null, resets_at_ms: null, sessions: null, version: null, channel: null, attempt: null, tries: null, account: null },
  action: "open",
  meter: null,
  created_ms: 1,
  unread: true,
  subject: `s:${session}`,
  snoozed: false,
  snoozed_until_ms: null,
  muted: false,
  dismissed: false,
});

/** The backend: its needs and its summary, and whether the window hears
 * its change event (the walk's window didn't: not shown, AJ-E805). */
const backend = { needs: [] as NoticeInfo[], version: 1, heard: true };
const told = new Map<string, (message: { payload: unknown }) => void>();

function summary(): NoticeSummaryInfo {
  const count = backend.needs.length;
  return { count, unread: count, snoozed: 0, muted: 0, quiet: false, quiet_until: null, banners: 0, digest: { finished: 0, restarted: 0, resets: 0, updates: 0, next_reset_account: null, next_reset_ms: null }, version: backend.version };
}

/** The model changed: the backend works it out and tells the window. */
function changed(needs: NoticeInfo[]): void {
  backend.needs = needs;
  backend.version += 1;
  if (backend.heard) told.get(CHANGED)?.({ payload: summary() });
}

const X = crashed(7, "s-x");

/** What the window's calls carry, here. */
type Args = { section?: string; request?: PageRequest; sessionId?: string };

async function answer(name: string, args: Args = {}): Promise<unknown> {
  switch (name) {
    case "notifications_summary":
      return summary();
    case "notifications_page": {
      const rows = args.section === "needs" ? backend.needs : [];
      const limit = args.request?.limit ?? 50;
      return { items: rows.slice(0, limit), next_cursor: null, total: rows.length };
    }
    case "instance_archive":
      changed(backend.needs.filter((row) => row.target.session_id !== args.sessionId));
      return 1;
    case "instance_unarchive":
      changed([X]);
      return 2;
    default:
      throw new Error(`not in this test: ${name}`);
  }
}

vi.mock("../lib/ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/ipc/transport")>()),
  liveBackend: true,
  call: (name: string, args?: Args) => answer(name, args),
  listen: async (event: string, handler: (message: { payload: unknown }) => void) => {
    told.set(event, handler);
    return () => told.delete(event);
  },
}));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), hasLiveBackend: () => true }));

beforeEach(() => {
  backend.needs = [X];
  backend.version = 1;
  backend.heard = true;
  told.clear();
  // The setup file loaded these with the real transport: again, with this one.
  vi.resetModules();
});
afterEach(cleanup);

/** The bell, and Needs you as a surface shows it (the rail; Home). */
async function surfaces() {
  const { NotificationBell } = await import("./NotificationBell");
  const { useBackendNeeds } = await import("../lib/needs-you");
  const history = await import("../lib/ipc/session-history");
  const store = await import("../lib/notices-store");
  function NeedsYou({ id }: { id: string }): JSX.Element {
    const needs = useBackendNeeds();
    return <ul data-testid={id}>{needs.loading ? null : needs.rows.map((row) => <li key={row.id}>{row.subject}</li>)}</ul>;
  }
  const view = render(
    <>
      <NotificationBell sessions={[]} />
      <NeedsYou id="rail" />
    </>,
  );
  const bell = () => view.queryByTestId("bell-count")?.textContent ?? "0";
  const rows = (id: string) => [...(view.queryByTestId(id)?.querySelectorAll("li") ?? [])].map((row) => row.textContent);
  const openHome = (): RenderResult => render(<NeedsYou id="home" />, { container: view.container.appendChild(document.createElement("div")) });
  await waitFor(() => expect(rows("rail")).toEqual(["s:s-x"]));
  await waitFor(() => expect(bell()).toBe("1"));
  return { view, bell, rows, openHome, history, store };
}

it("run 2: archived while its event isn't heard, the bell's count leaves with the rail's row", async () => {
  const { bell, rows, history } = await surfaces();
  backend.heard = false;
  await act(async () => void (await history.instanceArchive("s-x")));
  await waitFor(() => expect(rows("rail")).toEqual([]));
  // The walk: the badge stayed at 1 for 50 s, its panel caught up.
  await waitFor(() => expect(bell()).toBe("0"));
});

it("run 1: unarchived while its event isn't heard, the need is back in the rail and the bell at once, as Home has it", async () => {
  const { bell, rows, openHome, history } = await surfaces();
  await act(async () => void (await history.instanceArchive("s-x")));
  await waitFor(() => expect(bell()).toBe("0"));
  expect(rows("rail")).toEqual([]);
  backend.heard = false;
  await act(async () => void (await history.instanceUnarchive("s-x")));
  // The walk: no Needs you in the rail and no badge for 22 s and more.
  await waitFor(() => expect(rows("rail")).toEqual(["s:s-x"]));
  await waitFor(() => expect(bell()).toBe("1"));
  openHome();
  await waitFor(() => expect(rows("home")).toEqual(["s:s-x"]));
});

it("one list: Home, opened, reads a change no event told of, and the rail and the bell show it too", async () => {
  const { bell, rows, openHome } = await surfaces();
  // The model changed while the window heard nothing (not shown).
  backend.heard = false;
  changed([]);
  expect(rows("rail")).toEqual(["s:s-x"]);
  openHome();
  await waitFor(() => expect(rows("home")).toEqual([]));
  // The walk: Home said "1 session needs you" while the rail and the bell
  // said nothing (here the other way round, the same split).
  await waitFor(() => expect(rows("rail")).toEqual([]));
  await waitFor(() => expect(bell()).toBe("0"));
});

it("one count: an older summary landing after a newer one never leaves the bell counting a need its list doesn't show", async () => {
  const { bell, rows, store } = await surfaces();
  const before = summary();
  await act(async () => changed([]));
  await waitFor(() => expect(rows("rail")).toEqual([]));
  await waitFor(() => expect(bell()).toBe("0"));
  // Two emissions out of order: the one from before the change lands last.
  act(() => store.applySummary(before));
  await waitFor(() => expect(rows("rail")).toEqual([]));
  await new Promise((done) => setTimeout(done, 50));
  expect(bell()).toBe("0");
});
