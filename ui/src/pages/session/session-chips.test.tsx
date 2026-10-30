/**
 * 0.3 polish — a session's header has two chips: its state, and one
 * Account menu (the review: "Running in AI Juggler", "Profile: Work Max"
 * and "Account ▾" beside a facts line that said the account again). The
 * menu says which account, profile and folder, and opens the profile's
 * page (the profile chip it replaced was no door). Where it runs is on
 * the facts line.
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

it("shows two chips: the state, and the Account menu named by its profile", () => {
  const view = render(<SessionChips model={model} account={signedIn} now={Date.now()} />);
  expect(view.getByTestId("session-state-chip").textContent).toBe("Idle");
  const menu = view.getByTestId("session-account-menu");
  expect(menu.getAttribute("aria-label")).toBe("Account: Work");
  expect(menu.getAttribute("aria-haspopup")).toBe("menu");
  for (const gone of ["session-where-chip", "session-profile-chip", "session-account-chip", "session-folder-chip", "session-more-chips"]) expect(view.queryByTestId(gone)).toBeNull();
});

it("opens to the account, the profile, the folder and the door to the profile's page", () => {
  const openAccountById = vi.fn();
  const view = render(
    <ActionHostProvider host={{ openAccountById }}>
      <SessionChips model={model} account={signedIn} now={Date.now()} />
    </ActionHostProvider>,
  );
  fireEvent.click(view.getByTestId("session-account-menu"));
  const panel = view.getByTestId("session-account-panel");
  expect(panel.getAttribute("role")).toBe("menu");
  expect(panel.textContent).toContain("Work Max");
  expect(panel.textContent).toContain("you@example.com · max-20x");
  // The account's name isn't the profile's here: both are said.
  expect(within(panel).getByTestId("session-account-profile").textContent).toBe("ProfileWork");
  expect(within(panel).getByTestId("session-account-folder").textContent).toContain("ai-juggler");
  fireEvent.click(within(panel).getByTestId("session-account-open"));
  expect(openAccountById).toHaveBeenCalledWith("claude-work");
  expect(view.queryByTestId("session-account-panel")).toBeNull();
});

it("says a profile once when the account is named for it, and works before sign-in is known", () => {
  const same = { ...(signedIn as object), profiles: [{ id: "claude-work", label: "Work" }] } as never;
  let view = render(<SessionChips model={model} account={same} now={Date.now()} />);
  fireEvent.click(view.getByTestId("session-account-menu"));
  expect(view.queryByTestId("session-account-profile")).toBeNull();
  cleanup();
  view = render(<SessionChips model={model} account={undefined} now={Date.now()} />);
  fireEvent.click(view.getByTestId("session-account-menu"));
  expect(view.getByTestId("session-account-profile").textContent).toBe("ProfileWork");
  // No host door: no line that does nothing.
  expect(view.queryByTestId("session-account-open")).toBeNull();
});

it("says where it runs on the facts line", () => {
  const view = render(<SessionWhere model={{ ...model, live: true, places: [] } as SessionModel} now={Date.now()} />);
  expect(view.getByTestId("session-where").textContent).toBe("Running in AI Juggler");
});
