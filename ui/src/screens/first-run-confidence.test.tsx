/**
 * AJ-E2906 vitest — first run says how sure it is: each found setup has
 * one word (Found, Likely or Guessed: the weaker of its name and its
 * sign-in) and where it came from in words; Details lists each field
 * with its own word and source and the scan's notes; Guessed says it can
 * be renamed; a sign-in that may have expired says so; Look again is
 * always there and keeps the ticks on setups still found.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FirstRun } from "./FirstRun";
import { fieldRows, rowConfidence, sourceWords } from "./first-run-confidence";
import type { DiscoveredInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const NOW = Math.floor(Date.now() / 1000);

function setup(path: string, origin: string, inference: Partial<DiscoveredInfo["inference"]> = {}, extra: Partial<DiscoveredInfo> = {}): DiscoveredInfo {
  return {
    harness: "claude-code",
    path,
    origin,
    set_by: null,
    login: "logged_in",
    bytes: 1000,
    last_used_secs: NOW - 3600,
    managed: false,
    notes: [],
    signals: [origin],
    in_use: [],
    inference: {
      purpose: "Main",
      purpose_confidence: "found",
      purpose_rule: "usual-folder",
      login: "signed-in",
      login_confidence: "found",
      login_rule: "sign-in-present",
      activity: "active",
      activity_confidence: "found",
      activity_rule: "newest-conversation",
      activity_words: "Active this week",
      keychain_bound: false,
      ...inference,
    },
    ...extra,
  };
}

const USUAL = setup("/Users/you/.claude", "vendor-default", {}, { notes: ["Claude Code's standard location."] });
const WORK = setup("/Users/you/.claude-work", "home-pattern", { purpose: "Work", purpose_confidence: "likely", purpose_rule: "name-word" });
const ODD = setup("/Users/you/.claude-x", "env-override", { purpose: "X", purpose_confidence: "guessed", purpose_rule: "folder-name" }, { login: "logged_out", last_used_secs: NOW - 100 * 86_400 });
const STALE = setup(
  "/Users/you/.claude-old",
  "other-manager",
  { purpose: "Old", purpose_confidence: "found", purpose_rule: "usual-folder", login: "maybe-expired", login_confidence: "likely", login_rule: "sign-in-empty" },
  { login: "logged_out", set_by: "cc-switch", last_used_secs: NOW - 200 * 86_400 },
);
const report = (installs: DiscoveredInfo[]) => ({ installs, probes: [], seeds: [], desktop_apps: [], cancelled: false });

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  w().__AIJ_SCAN_REPORT__ = report([USUAL, WORK, ODD, STALE]);
});

describe("AJ-E2906 the words", () => {
  it("the row's word is the weaker of its name and its sign-in", () => {
    expect([USUAL, WORK, ODD, STALE].map(rowConfidence)).toEqual(["found", "likely", "guessed", "likely"]);
  });

  it("says where each came from, never an internal id", () => {
    expect(sourceWords(USUAL, "Claude Code")).toBe("Claude Code's usual folder");
    expect(sourceWords(WORK, "Claude Code")).toBe("a folder named like a Claude Code setup");
    expect(sourceWords(ODD, "Claude Code")).toBe("set in your shell's settings");
    expect(sourceWords(STALE, "Claude Code")).toBe("named by cc-switch");
    expect(sourceWords({ ...STALE, set_by: null }, "Claude Code")).toBe("named by another app");
    expect(sourceWords({ ...USUAL, origin: "something-new" }, "Claude Code")).toBe("found on this computer");
  });

  it("each field has its word and source; an unknown confidence never claims more than a guess", () => {
    expect(fieldRows(STALE, { name: "claude-old", used: "Used 200 days ago" })).toEqual([
      { field: "Name", value: "claude-old", confidence: "found", source: "the tool's usual folder" },
      { field: "Sign-in", value: "May have expired", confidence: "likely", source: "its sign-in file is there but empty" },
      { field: "Last used", value: "Used 200 days ago", confidence: "found", source: "its newest conversation" },
    ]);
    const odd = { ...ODD, inference: { ...ODD.inference, purpose_confidence: "certain" } };
    expect(rowConfidence(odd)).toBe("guessed");
  });
});

describe("AJ-E2906 the found step", () => {
  const renderIt = () => render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
  const rowOf = (step: HTMLElement, folder: string) => within(step).getAllByTestId("first-run-row").find((row) => row.textContent?.includes(folder))!;

  it("each row says its word and where it came from; Guessed says it can be renamed; an empty sign-in may have expired", async () => {
    const { findByTestId } = renderIt();
    const step = await findByTestId("first-run-found");
    const usual = rowOf(step, "~/.claude ");
    expect(within(usual).getByTestId("first-run-confidence").textContent).toBe("Found");
    expect(within(usual).getByTestId("first-run-source").textContent).toBe(" · Claude Code's usual folder");
    expect(within(rowOf(step, ".claude-work")).getByTestId("first-run-confidence").textContent).toBe("Likely");
    const odd = rowOf(step, ".claude-x");
    expect(within(odd).getByTestId("first-run-confidence").textContent).toBe("Guessed");
    expect(odd.textContent).toContain("You can rename it after");
    expect(rowOf(step, ".claude-old").textContent).toContain("May have expired");
    for (const id of ["vendor-default", "home-pattern", "env-override", "other-manager", "usual-folder", "sign-in-empty"]) expect(step.textContent).not.toContain(id);
  });

  it("Details lists each field with its word and source, and the scan's notes", async () => {
    const { findByTestId } = renderIt();
    const usual = rowOf(await findByTestId("first-run-found"), "~/.claude ");
    const toggle = within(usual).getByTestId("first-run-details-toggle");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    const details = within(usual).getByTestId("first-run-details");
    expect(within(details).getAllByTestId("first-run-field").map((row) => row.textContent)).toEqual([
      "NameDefaultFoundfrom the tool's usual folder",
      "Sign-inSigned inFoundfrom its sign-in is in its folder",
      "Last usedUsed todayFoundfrom its newest conversation",
    ]);
    expect(within(details).getByTestId("first-run-notes").textContent).toBe("Claude Code's standard location.");
    // Details never ticks or unticks the row.
    expect(within(usual).getByRole("checkbox")).toHaveProperty("checked", true);
  });

  it("Look again is always there and keeps the ticks on setups still found; a new one comes in as the scan suggests", async () => {
    const { findByTestId, getByTestId } = renderIt();
    const step = await findByTestId("first-run-found");
    // Untick the usual one; tick the odd one.
    fireEvent.click(within(rowOf(step, "~/.claude ")).getByRole("checkbox"));
    fireEvent.click(within(rowOf(step, ".claude-x")).getByRole("checkbox"));
    const fresh = setup("/Users/you/.claude-new", "home-pattern", { purpose_confidence: "guessed", purpose_rule: "folder-name" }, { login: "logged_out", last_used_secs: undefined });
    w().__AIJ_SCAN_REPORT__ = report([USUAL, WORK, ODD, STALE, fresh]);
    fireEvent.click(getByTestId("first-run-look-again"));
    await waitFor(() => expect(getByTestId("first-run-found").textContent).toContain(".claude-new"));
    const again = getByTestId("first-run-found");
    expect(within(rowOf(again, "~/.claude ")).getByRole("checkbox")).toHaveProperty("checked", false);
    expect(within(rowOf(again, ".claude-x")).getByRole("checkbox")).toHaveProperty("checked", true);
    expect(within(rowOf(again, ".claude-new")).getByRole("checkbox")).toHaveProperty("checked", false);
  });
});
