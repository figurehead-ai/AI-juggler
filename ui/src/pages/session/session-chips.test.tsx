/**
 * The 0.2 walk, F18 — a session's header folded its account and folder
 * behind a bare "+2" (its name said "its account and folder", the eye
 * saw a number). The chip now says what it holds.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { SessionChips } from "./SessionHead";
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
  profileLabel: "Work",
} as unknown as SessionModel;

it("names what the folded chip holds, in words, and opens it", () => {
  const view = render(<SessionChips model={model} account={undefined} now={Date.now()} />);
  const more = view.getByTestId("session-more-chips");
  expect(more.textContent).toBe("Folder▾");
  expect(more.getAttribute("aria-label")).toBeNull();
  fireEvent.click(more);
  expect(more.getAttribute("aria-expanded")).toBe("true");
});

it("with its account too: \"Account and folder\"", () => {
  const account = { state: "signed_in", title: "Work Max", label: "Work Max", email: null, organisation: null, plan: null, service: null, profiles: [] } as never;
  const view = render(<SessionChips model={model} account={account} now={Date.now()} />);
  expect(view.getByTestId("session-more-chips").textContent).toBe("Account and folder▾");
});
