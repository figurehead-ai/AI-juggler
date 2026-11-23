/**
 * AJ-E1545 step 1 with AJ-E2327's pairing: a desktop profile and the
 * folder evidence says it uses are one row ("Claude app · work",
 * Desktop); an uncertain pairing asks, a certain one doesn't; a folder
 * no profile is paired with stays its own row. Never paired by name: the
 * pairing comes from the backend's evidence only.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { PlacePairingInfo } from "../types/generated/PlacePairingInfo";
import { FirstRun } from "./FirstRun";

const PAIRS: PlacePairingInfo[] = [
  { id: "p-work", label: "work", added: false, certain: true, folder: { tool: "claude-code", name: "work", path: "/Users/you/Library/Application Support/ClaudeCodeProfiles/work", by: null, why: null } },
  { id: "p-lab", label: "lab", added: false, certain: false, folder: { tool: "claude-code", name: ".claude-lab", path: "/Users/you/.claude-lab", by: null, why: null } },
  { id: "p-side", label: "side", added: true, certain: false, folder: { tool: "claude-code", name: ".claude-side", path: "/Users/you/.claude-side", by: null, why: null } },
];

const chosen: string[] = [];
vi.mock("../lib/choose-app-folder", () => ({ chooseAppFolder: (id: string) => (chosen.push(id), Promise.resolve()) }));

vi.mock("../lib/ipc/places", async (actual) => ({ ...(await actual<typeof import("../lib/ipc/places")>()), placesPairings: () => Promise.resolve(PAIRS) }));

const setup = (path: string, origin: string) => ({
  harness: "claude-code",
  path,
  origin,
  login: "logged_in",
  bytes: 1_000_000,
  last_used_secs: null,
  managed: false,
  notes: [],
  signals: [origin],
  in_use: [],
});

beforeEach(() => {
  Reflect.set(window, "__AIJ_SCAN_REPORT__", {
    installs: [setup("/Users/you/Library/Application Support/ClaudeCodeProfiles/work", "manager-tool"), setup("/Users/you/.claude-lab", "home-pattern"), setup("/Users/you/.claude-team-a", "home-pattern"), setup("/Users/you/.claude-side", "home-pattern")],
    probes: [],
    seeds: [],
    desktop_apps: ["Claude"],
    cancelled: false,
  });
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_SCAN_REPORT__");
});

const rowWith = (words: string) => screen.getAllByTestId("first-run-row").find((row) => row.textContent?.includes(words));

it("a desktop profile and its folder are one row, by evidence; an uncertain one asks", async () => {
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · work")).toBeDefined());
  const work = rowWith("Claude app · work")!;
  expect(work.textContent).toContain("Desktop");
  expect(work.querySelector("[data-testid=first-run-unsure]")).toBeNull();
  expect(rowWith("Claude app · lab")?.querySelector("[data-testid=first-run-unsure]")?.textContent).toContain("Is this right?");
  // A folder no profile is paired with keeps its own name, no Desktop chip.
  expect(rowWith("team-a")?.textContent).not.toContain("Desktop");
  expect(screen.getAllByTestId("first-run-row")).toHaveLength(4);
});

it("every row has a Desktop and/or Terminal chip (F1)", async () => {
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · work")).toBeDefined());
  for (const row of screen.getAllByTestId("first-run-row")) {
    const chips = within(row).queryAllByTestId("first-run-chip").map((chip) => chip.textContent);
    expect(chips.length, row.textContent ?? "").toBeGreaterThan(0);
  }
  // Seen in no desktop app: its tool's terminal folder.
  expect(within(rowWith("team-a")!).getAllByTestId("first-run-chip").map((chip) => chip.textContent)).toEqual(["Terminal"]);
});

it("an uncertain pairing offers its fix: Choose its folder… for an App, Change (unpair) before (F2)", async () => {
  chosen.length = 0;
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · side")).toBeDefined());
  fireEvent.click(within(rowWith("Claude app · side")!).getByRole("button", { name: "Choose its folder…" }));
  await waitFor(() => expect(chosen).toEqual(["p-side"]));
  expect(rowWith("Claude app · side")?.textContent).toContain("Its folder was chosen");
  // Not added yet: Change takes the pairing off; the row is the folder.
  fireEvent.click(within(rowWith("Claude app · lab")!).getByRole("button", { name: "Change" }));
  expect(rowWith("Claude app · lab")).toBeUndefined();
  expect(rowWith("claude-lab")?.querySelector("[data-testid=first-run-unsure]")).toBeNull();
  expect(screen.queryAllByText(/Check it on its App page after/)).toHaveLength(0);
});

it("Add app pairs nothing (AJ-E1548 keeps its look)", async () => {
  render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  expect(rowWith("Claude app · work")).toBeUndefined();
});
