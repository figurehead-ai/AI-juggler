/**
 * AJ-E1025 (#80 review): an App page's tool chip opens the tool's page
 * (AJ-E3104) and leaves the header with it, so keyboard focus must follow
 * into the page — never drop to <body>.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";

// xterm needs a real layout engine; the chip's focus doesn't.
vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

beforeEach(() => {
  // jsdom has no matchMedia (the terminal asks for it).
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
});

it("Enter on About Claude Code opens the tool's page, with focus on its tabs", async () => {
  const user = userEvent.setup();
  const view = render(<App />);
  const rail = await waitFor(() => {
    const found = view.container.querySelector<HTMLElement>("[data-testid='rail']");
    expect(found).not.toBeNull();
    return found!;
  });
  const row = await waitFor(() => {
    const found = [...rail.querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""));
    expect(found).toBeTruthy();
    return found!;
  });
  await user.click(row);
  // A session's header names its tool once, in its App chip ("Claude Code
  // · …", the 0.4 review, 46): the tool's door is on its App's page.
  await user.click(await view.findByTestId("session-account-menu"));
  expect(view.queryByRole("button", { name: "About Claude Code" })).toBeNull();
  await user.click(await view.findByTestId("session-account-open"));
  const chip = await view.findByRole("button", { name: "About Claude Code" });
  chip.focus();
  await user.keyboard("{Enter}");
  // AJ-E3104: the tool's own page (its Apps, sessions, projects and
  // pools); its install and updates are one click on, in Settings.
  await view.findByTestId("provider-overview");
  expect(view.getByTestId("page-name").textContent).toBe("Claude Code");
  expect(view.queryByRole("button", { name: "About Claude Code" })).toBeNull();
  // The opener went away: focus lands on the page's tabs, never on <body>.
  await waitFor(() => expect(document.activeElement?.getAttribute("role")).toBe("tab"));
});
