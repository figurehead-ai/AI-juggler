/**
 * AJ-E1025 (#80 review): the header's tool chip opens the tool's view and
 * leaves the header with it, so keyboard focus must follow into the
 * inspector — never drop to <body>.
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

it("Enter on About Claude Code opens Settings › Accounts at its tools, with focus on that section", async () => {
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
  const chip = await view.findByRole("button", { name: "About Claude Code" });
  chip.focus();
  await user.keyboard("{Enter}");
  // 0.1.5: the tool page folded into Settings › Accounts (its install
  // and accounts), where the chip now leads.
  const tool = await view.findByTestId("settings-tool");
  expect(tool.textContent).toContain("Claude Code");
  expect(view.queryByRole("button", { name: "About Claude Code" })).toBeNull();
  // The opener went away: focus lands on the section, never on <body>.
  await waitFor(() => expect(document.activeElement?.id).toBe("settings-tools-title"));
  expect(document.activeElement?.textContent).toBe("Tools on this computer");
});
