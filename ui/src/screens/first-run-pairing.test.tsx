/**
 * AJ-E1545 step 1 with AJ-E2327's pairing: a desktop profile and the
 * folder evidence says it uses are one row ("Claude app · work",
 * Desktop); an uncertain pairing asks, a certain one doesn't; a folder
 * no profile is paired with stays its own row. Never paired by name: the
 * pairing comes from the backend's evidence only.
 *
 * 0.5.1 F35 (PL1 C3): the rows follow the backend's own words: its label
 * ("Claude app · work"), the tool's id and the folder's full path (its
 * `folder` is for the words: "Claude Code", `~/…`). A profile with no
 * folder row is a row of its own in its tool's group, with the Desktop
 * chip, unticked until the user ticks it (D-84), counted in "Bring in N
 * apps"; no separate "Open now, not added yet" block.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PlacePairingInfo } from "../types/generated/PlacePairingInfo";
import { FirstRun } from "./FirstRun";
import { PLACES_CHANGED } from "../lib/ipc/places";

// As the backend sends them: `folder` in words (the tool's name, `~`), the
// tool's id and the full path beside it.
const pairing = (name: string, folder: string | null, extra: Partial<PlacePairingInfo> = {}): PlacePairingInfo => ({
  id: `p-${name}`,
  label: `Claude app · ${name}`,
  added: false,
  certain: false,
  folder: { tool: "Claude Code", name: folder?.split("/").pop() ?? null, path: folder?.replace("/Users/you", "~") ?? null, by: folder ? "named" : null, why: folder ? null : "None of its sessions is open." },
  tool: "claude-code",
  path: folder,
  running: true,
  ...extra,
});
const PAIRS: PlacePairingInfo[] = [
  pairing("work", "/Users/you/Library/Application Support/ClaudeCodeProfiles/work", { certain: true }),
  pairing("lab", "/Users/you/.claude-lab"),
  pairing("side", "/Users/you/.claude-side", { added: true }),
];

const chosen: Array<[string, boolean | undefined]> = [];
vi.mock("../lib/choose-app-folder", () => ({
  chooseAppFolder: (id: string, _label: string, options?: { added?: boolean }) => (chosen.push([id, options?.added]), Promise.resolve(true)),
}));

// The backend's pairings, read again when a place changes (a folder
// chosen before Add pairs the profile for sure: F2).
let pairs: PlacePairingInfo[] = PAIRS;
const added: string[] = [];
vi.mock("../lib/ipc/places", async (actual) => ({
  ...(await actual<typeof import("../lib/ipc/places")>()),
  placesPairings: () => Promise.resolve(pairs),
  placesOfferAnswer: (id: string, add: boolean) => (add && added.push(id), Promise.resolve({ id, folder: { tool: null, name: null, path: null, by: null, why: null } })),
}));

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

it("an uncertain pairing offers Choose its folder…, for an App and before it's added (F2)", async () => {
  chosen.length = 0;
  pairs = PAIRS;
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · side")).toBeDefined());
  fireEvent.click(within(rowWith("Claude app · side")!).getByRole("button", { name: "Choose its folder…" }));
  await waitFor(() => expect(chosen).toEqual([["p-side", true]]));
  expect(rowWith("Claude app · side")?.textContent).toContain("Its folder was chosen");
  // Not added yet: the same one action, the pick kept until its Add.
  const lab = rowWith("Claude app · lab")!;
  expect(within(lab).queryByRole("button", { name: "Change" })).toBeNull();
  fireEvent.click(within(lab).getByRole("button", { name: "Choose its folder…" }));
  await waitFor(() => expect(chosen).toEqual([["p-side", true], ["p-lab", false]]));
  expect(rowWith("Claude app · lab")?.textContent).toContain("Its folder was chosen");
  expect(screen.queryAllByText(/Check it on its App page after/)).toHaveLength(0);
});

it("a folder chosen before Add moves the profile to that folder's row, sure (F2)", async () => {
  pairs = PAIRS;
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · lab")).toBeDefined());
  // The backend now pairs lab with the folder the user chose, for sure.
  pairs = PAIRS.map((row) =>
    row.id === "p-lab" ? { ...row, certain: true, path: "/Users/you/.claude-team-a", folder: { ...row.folder, name: ".claude-team-a", path: "~/.claude-team-a", by: "recorded" } } : row,
  );
  window.dispatchEvent(new CustomEvent(PLACES_CHANGED));
  await waitFor(() => expect(rowWith("Claude app · lab")?.textContent).toContain("Desktop"));
  expect(rowWith("Claude app · lab")?.querySelector("[data-testid=first-run-unsure]")).toBeNull();
  // Its old folder is its own row again, nothing added.
  expect(rowWith("claude-lab")?.querySelector("[data-testid=first-run-unsure]")).toBeNull();
  expect(screen.getAllByTestId("first-run-row")).toHaveLength(4);
  pairs = PAIRS;
});

it("Add app pairs nothing (AJ-E1548 keeps its look)", async () => {
  render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  expect(rowWith("Claude app · work")).toBeUndefined();
});

describe("F35: every desktop profile is a row in its tool's group", () => {
  const SOLO = pairing("design", null);
  beforeEach(() => {
    added.length = 0;
    pairs = [...PAIRS, SOLO];
  });
  afterEach(() => {
    pairs = PAIRS;
  });

  it("a profile with no folder row is its own row, Desktop chip, unticked, and no separate block", async () => {
    render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await screen.findByTestId("first-run-found");
    await waitFor(() => expect(rowWith("Claude app · design")).toBeDefined());
    const design = rowWith("Claude app · design")!;
    expect(design.closest("[data-testid=first-run-group-claude-code]")).not.toBeNull();
    expect(within(design).getAllByTestId("first-run-chip").map((chip) => chip.textContent)).toEqual(["Desktop"]);
    expect(within(design).getByRole<HTMLInputElement>("checkbox").checked).toBe(false);
    expect(design.textContent).toContain("Open now");
    expect(screen.queryByTestId("first-run-desktop")).toBeNull();
    expect(screen.queryByText("Open now, not added yet")).toBeNull();
    // Four folders (work and lab are their profiles' rows) and design.
    expect(screen.getAllByTestId("first-run-row")).toHaveLength(5);
  });

  it("Bring in N counts a ticked profile, and Bring in adds it and only what was ticked (D-84)", async () => {
    render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await screen.findByTestId("first-run-found");
    await waitFor(() => expect(rowWith("Claude app · design")).toBeDefined());
    // The profile rows aren't ticked for the user, a paired one neither.
    expect(within(rowWith("Claude app · work")!).getByRole<HTMLInputElement>("checkbox").checked).toBe(false);
    const before = Number(/Bring in (\d+)/.exec(screen.getByTestId("first-run-bring-in").textContent ?? "")?.[1] ?? 0);
    fireEvent.click(within(rowWith("Claude app · design")!).getByRole("checkbox"));
    fireEvent.click(within(rowWith("Claude app · work")!).getByRole("checkbox"));
    expect(screen.getByTestId("first-run-bring-in").textContent).toBe(`Bring in ${before + 2} apps`);
    await act(async () => {
      fireEvent.click(screen.getByTestId("first-run-bring-in"));
    });
    await waitFor(() => expect([...added].sort()).toEqual(["p-design", "p-work"]));
  });

  it("the group's All ticks its profiles too", async () => {
    render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await screen.findByTestId("first-run-found");
    await waitFor(() => expect(rowWith("Claude app · design")).toBeDefined());
    fireEvent.click(screen.getByTestId("first-run-all-claude-code"));
    expect(within(rowWith("Claude app · design")!).getByRole<HTMLInputElement>("checkbox").checked).toBe(true);
    expect(within(rowWith("Claude app · work")!).getByRole<HTMLInputElement>("checkbox").checked).toBe(true);
  });
});

it("F35: a profile whose folder is in AI Juggler already keeps a row to tick", async () => {
  Reflect.set(window, "__AIJ_SCAN_REPORT__", {
    installs: [{ ...setup("/Users/you/Library/Application Support/ClaudeCodeProfiles/work", "manager-tool"), managed: true }],
    probes: [],
    seeds: [],
    desktop_apps: ["Claude"],
    cancelled: false,
  });
  pairs = [PAIRS[0]!];
  render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
  await screen.findByTestId("first-run-found");
  await waitFor(() => expect(rowWith("Claude app · work")).toBeDefined());
  expect(within(rowWith("Claude app · work")!).getByRole<HTMLInputElement>("checkbox").disabled).toBe(false);
  expect(screen.getAllByTestId("first-run-row")).toHaveLength(2);
  pairs = PAIRS;
});
