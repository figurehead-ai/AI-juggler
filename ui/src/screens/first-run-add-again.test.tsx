/**
 * F19 (0.5.1 PL1 D2): after Remove, one tool's Add an app found nothing
 * to add: it went straight to "Add a fresh app.", and its Look again
 * didn't see the tool's desktop profiles open and not added (a removed
 * one is offered again while it runs). Both list them now, each with Add.
 */
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { FirstRun } from "./FirstRun";

vi.mock("../components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

type Offer = { id: string; label: string; running: boolean };
const places = (offers: Offer[]) => Object.assign(window, { __AIJ_PLACES__: { places: [], offers } });
const WORK = { id: "app:claude-app:/Users/you/Library/Application Support/ClaudeProfiles/work", label: "Claude app · work", running: true };
const CODEX = { id: "app:codex-app:", label: "Codex app", running: true };

beforeEach(() => {
  // Nothing new on disk: every folder of the tool is in already.
  Object.assign(window, { __AIJ_SCAN_REPORT__: { installs: [], probes: [], seeds: [], desktop_apps: ["Claude.app"], cancelled: false } });
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_SCAN_REPORT__");
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

it("one tool's Add lists its app's profiles open and not added, not another app's", async () => {
  places([WORK, CODEX]);
  const view = render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
  const desktop = await view.findByTestId("first-run-desktop");
  const rows = within(desktop).getAllByTestId("desktop-offer");
  expect(rows.map((row) => row.textContent)).toEqual(["Claude app · workAddNot now"]);
  expect(view.queryByText("Add a fresh app.")).toBeNull();
});

it("with none, it opens on a fresh app; Look again then finds a profile opened since", async () => {
  places([]);
  const view = render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
  expect(await view.findByText("Add a fresh app.")).toBeTruthy();
  places([WORK]);
  await act(async () => {
    fireEvent.click(view.getByTestId("first-run-look-again"));
  });
  const desktop = await view.findByTestId("first-run-desktop");
  expect(within(desktop).getByTestId("desktop-offer").textContent).toContain("Claude app · work");
});
