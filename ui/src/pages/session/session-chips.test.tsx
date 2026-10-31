/**
 * 0.3 polish — a session's header has two chips: its state, and one
 * App menu (the review: "Running in AI Juggler", "Profile: Work Max" and
 * "Account ▾" beside a facts line that said the account again). D-62: the
 * chip is named by its App; the menu says which App, account and working
 * folder, and opens the App's page (the profile chip it replaced was no
 * door). Where it runs is on the facts line.
 */
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

const signedIn = { state: "signed_in", title: "Work Max", label: "Work Max", email: "you@example.com", organisation: null, plan: "max-20x", service: null, profiles: [{ id: "claude-work", label: "Work Max" }] } as never;

it("shows two chips: the state, and the App menu named by its App", () => {
  const view = render(<SessionChips model={model} account={signedIn} now={Date.now()} />);
  expect(view.getByTestId("session-state-chip").textContent).toBe("Idle");
  const menu = view.getByTestId("session-account-menu");
  expect(menu.getAttribute("aria-label")).toBe("App: Claude Code · Work");
  expect(menu.textContent).toBe("Claude Code · Work");
  expect(menu.getAttribute("aria-haspopup")).toBe("menu");
  for (const gone of ["session-where-chip", "session-profile-chip", "session-account-chip", "session-folder-chip", "session-more-chips"]) expect(view.queryByTestId(gone)).toBeNull();
});

it("opens to the App, its account, the working folder and the door to the App's page", () => {
  const openAccountById = vi.fn();
  const view = render(
    <ActionHostProvider host={{ openAccountById }}>
      <SessionChips model={model} account={signedIn} now={Date.now()} />
    </ActionHostProvider>,
  );
  fireEvent.click(view.getByTestId("session-account-menu"));
  const panel = view.getByTestId("session-account-panel");
  expect(panel.getAttribute("role")).toBe("menu");
  expect(within(panel).getByTestId("session-account-profile").textContent).toBe("Claude Code · Work");
  // The account is the subscription behind the App.
  expect(within(panel).getByTestId("session-account-account").textContent).toBe("AccountWork Max · you@example.com · max-20x");
  expect(within(panel).getByTestId("session-account-folder").textContent).toBe("Working folder~/Dev/ai-juggler");
  expect(within(panel).getByTestId("session-account-open").textContent).toContain("Open Claude Code · Work's page");
  fireEvent.click(within(panel).getByTestId("session-account-open"));
  expect(openAccountById).toHaveBeenCalledWith("claude-work");
  expect(view.queryByTestId("session-account-panel")).toBeNull();
});

it("names a desktop App by its app and instance, and works before sign-in is known", () => {
  const desktop = { ...model, places: [{ app: "Claude", app_id: "claude", profile: "Work", instance: "b-work", pid: 7, kind: "desktop", app_pid: 6, label: "Claude · b-work" }] } as SessionModel;
  let view = render(<SessionChips model={desktop} account={signedIn} now={Date.now()} />);
  expect(view.getByTestId("session-account-menu").getAttribute("aria-label")).toBe("App: Claude app · b-work");
  cleanup();
  view = render(<SessionChips model={model} account={undefined} now={Date.now()} />);
  fireEvent.click(view.getByTestId("session-account-menu"));
  expect(view.getByTestId("session-account-profile").textContent).toBe("Claude Code · Work");
  expect(view.queryByTestId("session-account-account")).toBeNull();
  // No host door: no line that does nothing.
  expect(view.queryByTestId("session-account-open")).toBeNull();
});

it("says where it runs on the facts line", () => {
  const view = render(<SessionWhere model={{ ...model, live: true, places: [] } as SessionModel} now={Date.now()} />);
  expect(view.getByTestId("session-where").textContent).toBe("Running in AI Juggler");
});
