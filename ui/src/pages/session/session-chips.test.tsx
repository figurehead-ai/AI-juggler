/**
 * 0.3 polish — a session's header has two chips: its state, and one
 * App menu (the review: "Running in AI Juggler", "Profile: Work Max" and
 * "Account ▾" beside a facts line that said the account again). D-62: the
 * chip is named by its App; the menu says which App, account and working
 * folder, and opens the App's page (the profile chip it replaced was no
 * door). Where it runs is on the facts line.
 */
import { OPEN_APP } from "../../lib/apps";
import { OPEN_ACCOUNT } from "../../lib/accounts-page";
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { SessionChips, SessionWhere } from "./SessionHead";
import type { SessionModel } from "./model";

afterEach(cleanup);

const model = {
  live: false,
  state: { words: "Idle", tone: "neutral", since: null, working: false, asks: false },
  places: [],
  tool: "claude-code",
  sessionId: "s1",
  viewedMs: null,
  cwd: "/Users/you/Dev/ai-juggler",
  profileId: "claude-work",
  profileLabel: "Work",
} as unknown as SessionModel;

const signedIn = { state: "signed_in", account_id: "acct:anthropic:work", tool: "claude-code", title: "Work Max", label: "Work Max", email: "you@example.com", organisation: null, plan: "max-20x", service: null, profiles: [{ id: "claude-work", label: "Work Max" }] } as never;

it("shows two chips: the state, and the App menu named by its App", () => {
  const view = render(<SessionChips model={model} account={signedIn} now={Date.now()} />);
  expect(view.getByTestId("session-state-chip").textContent).toBe("Idle");
  const menu = view.getByTestId("session-account-menu");
  expect(menu.getAttribute("aria-label")).toBe("App: Claude Code · Work");
  expect(menu.textContent).toBe("Claude Code · Work");
  expect(menu.getAttribute("aria-haspopup")).toBe("menu");
  for (const gone of ["session-where-chip", "session-profile-chip", "session-account-chip", "session-folder-chip", "session-more-chips"]) expect(view.queryByTestId(gone)).toBeNull();
});

it("opens to the App, its account, the working folder and a door to each of their pages (A3)", () => {
  const opened = vi.fn();
  const listen = (event: Event) => opened(event.type, (event as CustomEvent<{ id: string }>).detail.id);
  window.addEventListener(OPEN_APP, listen);
  window.addEventListener(OPEN_ACCOUNT, listen);
  const view = render(
    <ActionHostProvider host={{}}>
      <SessionChips model={model} account={signedIn} now={Date.now()} />
    </ActionHostProvider>,
  );
  fireEvent.click(view.getByTestId("session-account-menu"));
  const panel = view.getByTestId("session-account-panel");
  expect(panel.getAttribute("role")).toBe("menu");
  expect(within(panel).getByTestId("session-account-profile").textContent).toBe("Claude Code · Work");
  // The account is the subscription behind the App, named its own way
  // (D-62: its plan here), never by the App's folder.
  expect(within(panel).getByTestId("session-account-account").textContent).toBe("AccountClaude Max 20x · you@example.com");
  expect(within(panel).getByTestId("session-account-folder").textContent).toBe("Working folder~/Dev/ai-juggler");
  expect(within(panel).getByTestId("session-account-open").textContent).toContain("Open Claude Code · Work's page");
  expect(within(panel).getByTestId("session-account-open-account").textContent).toContain("Open Claude Max 20x's page");
  fireEvent.click(within(panel).getByTestId("session-account-open"));
  expect(view.queryByTestId("session-account-panel")).toBeNull();
  fireEvent.click(view.getByTestId("session-account-menu"));
  fireEvent.click(view.getByTestId("session-account-open-account"));
  window.removeEventListener(OPEN_APP, listen);
  window.removeEventListener(OPEN_ACCOUNT, listen);
  // A terminal session's App is its folder's CLI App; its Account, the
  // subscription it's signed in to.
  expect(opened.mock.calls).toEqual([
    [OPEN_APP, "cli:claude-work"],
    [OPEN_ACCOUNT, "acct:anthropic:work"],
  ]);
});

it("names a desktop App by its app and its config folder, and works before sign-in is known", () => {
  const desktop = { ...model, places: [{ app: "Claude", app_id: "claude", profile: "Work", instance: "b-work", pid: 7, kind: "desktop", app_pid: 6, label: "Claude · b-work" }] } as SessionModel;
  let view = render(<SessionChips model={desktop} account={signedIn} now={Date.now()} />);
  expect(view.getByTestId("session-account-menu").getAttribute("aria-label")).toBe("App: Claude app · Work");
  cleanup();
  view = render(<SessionChips model={model} account={undefined} now={Date.now()} />);
  fireEvent.click(view.getByTestId("session-account-menu"));
  expect(view.getByTestId("session-account-profile").textContent).toBe("Claude Code · Work");
  expect(view.queryByTestId("session-account-account")).toBeNull();
  // Before its sign-in is known: its App's page, no Account's yet.
  expect(view.getByTestId("session-account-open")).toBeTruthy();
  expect(view.queryByTestId("session-account-open-account")).toBeNull();
});

it("says where it runs on the facts line", () => {
  const view = render(<SessionWhere model={{ ...model, live: true, places: [] } as SessionModel} now={Date.now()} />);
  expect(view.getByTestId("session-where").textContent).toBe("Running in AI Juggler");
});
