/**
 * AJ-E1545 step 1 with AJ-E2327's pairing: a desktop profile and the
 * folder evidence says it uses are one row ("Claude app · work",
 * Desktop); an uncertain pairing asks, a certain one doesn't; a folder
 * no profile is paired with stays its own row. Never paired by name: the
 * pairing comes from the backend's evidence only.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { PlacePairingInfo } from "../types/generated/PlacePairingInfo";
import { FirstRun } from "./FirstRun";

const PAIRS: PlacePairingInfo[] = [
  { id: "p-work", label: "work", added: false, certain: true, folder: { tool: "claude-code", name: "work", path: "/Users/you/Library/Application Support/ClaudeCodeProfiles/work", by: null, why: null } },
  { id: "p-lab", label: "lab", added: false, certain: false, folder: { tool: "claude-code", name: ".claude-lab", path: "/Users/you/.claude-lab", by: null, why: null } },
];

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
    installs: [setup("/Users/you/Library/Application Support/ClaudeCodeProfiles/work", "manager-tool"), setup("/Users/you/.claude-lab", "home-pattern"), setup("/Users/you/.claude-team-a", "home-pattern")],
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
  expect(rowWith("Claude app · lab")?.querySelector("[data-testid=first-run-unsure]")?.textContent).toContain("Is this its folder?");
  // A folder no profile is paired with keeps its own name, no Desktop chip.
  expect(rowWith("team-a")?.textContent).not.toContain("Desktop");
  expect(screen.getAllByTestId("first-run-row")).toHaveLength(3);
});

it("Add app pairs nothing (AJ-E1548 keeps its look)", async () => {
  render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  expect(rowWith("Claude app · work")).toBeUndefined();
});
