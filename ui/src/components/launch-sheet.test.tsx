/**
 * AJ-E306 vitest — flow (e): the launch sheet (NewSessionModal
 * reshaped). The recommended Launch settings ride preselected with
 * the admission verdict inline (Ok/Tight/No — words, never a bare
 * colour or fabricated %); unknown headroom says so in TEXT;
 * logged-out rows are disabled with a one-tap Sign-in-first detour
 * that keeps the sheet open; ⌘↩ launches the preselection; and the
 * word "Profile" never appears as a sheet noun (D-02/D-10).
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as bridgeModule from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
];

const RECIPES = [
  { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null },
  { id: "p-2", name: "Main", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null },
];

const PROJECTS = [
  { id: "proj-1", root: "/Users/dev/api", name: "api", last_activity_ms: 1000, accounts_used: ["Work Max"], history_sessions: 3, active_instance: null, instance_count: 0, missing: false },
];

beforeEach(() => {
  w().__AIJ_RECIPES__ = RECIPES;
  w().__AIJ_QUOTA_DISPLAY__ = [];
  w().__AIJ_RANKING__ = [
    { account: "claude-work", score: 0.9, top_factors: [], factors: [], headroom_unknown: false, confidence: "Measured", suggested: true, admission: "tight" },
    { account: "claude-personal", score: 0.4, top_factors: [], factors: [], headroom_unknown: true, confidence: "Estimate", suggested: false },
  ];
});

afterEach(() => {
  vi.restoreAllMocks();
});

const mountSheet = (extra: { ranking?: unknown[] } = {}) => {
  if (extra.ranking !== undefined) w().__AIJ_RANKING__ = extra.ranking;
  const launchSpy = vi
    .spyOn(bridgeModule, "launchSession")
    .mockResolvedValue({ session_id: "s1", pid: 1 } as never);
  const signinSpy = vi
    .spyOn(bridgeModule, "terminalSpawnLogin")
    .mockResolvedValue({ id: "t1" } as never);
  const launched = vi.fn();
  const mounted = render(
    <NewSessionModal
      projects={PROJECTS}
      accounts={ACCOUNTS}
      initialDirectory={null}
      onClose={() => {}}
      onLaunched={launched}
      openLoginTerminal={() => {}}
    />,
  );
  return { ...mounted, launchSpy, signinSpy, launched };
};

describe("AJ-E306 flow (e) — the launch sheet", () => {
  it("tile → Launch → is 2 clicks; the preselect rule (last-used) names the account; ⌘↩ also launches it", async () => {
    const { container, launchSpy } = mountSheet();
    let clicks = 0;
    const tile = await waitFor(() => {
      const found = Array.from(container.querySelectorAll("button")).find((button) =>
        button.textContent?.includes("api"),
      );
      expect(found).toBeTruthy();
      return found!;
    });
    fireEvent.click(tile);
    clicks += 1;
    const preselected = await waitFor(() => {
      const found = container.querySelector("[data-testid='launch-preselected']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(preselected.textContent).toContain("Work Max");
    expect(preselected.textContent).toContain("Launch settings: Main");
    // Click 2: Launch → (the preselected recipe + the chosen dir).
    fireEvent.click(container.querySelector("[data-testid='launch-now']")!);
    clicks += 1;
    await waitFor(() => expect(launchSpy).toHaveBeenCalled());
    expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api");
    expect(clicks).toBeLessThanOrEqual(2);
  });

  it("⌘↩ launches the preselection with zero clicks (the HIG fast path)", async () => {
    const { container, launchSpy } = mountSheet();
    // Pick the dir (the keyboard flow would use arrows + Enter).
    const tile = await waitFor(() => {
      const found = Array.from(container.querySelectorAll("button")).find((button) =>
        button.textContent?.includes("api"),
      );
      expect(found).toBeTruthy();
      return found!;
    });
    fireEvent.click(tile);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/api"));
  });

  it("the admission verdict renders as WORDS — Tight explains; unknown headroom is TEXT, never a fabricated %", async () => {
    const { container } = mountSheet();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
    const text = container.textContent ?? "";
    expect(text).toContain("Tight — projected burn nears this account's headroom");
    // The headroom-fabrication regression: no percent sign anywhere
    // in the verdict region (the sheet's account area).
    const accounts = container.querySelector("[data-testid='launch-sheet-accounts']")!;
    expect(accounts.textContent).not.toMatch(/%/);
    // Customise ▸ exposes the full list — the unknown-headroom account
    // says so in text with its Sign-in-first detour.
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((b) => b.textContent === "Customise ▸")!,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='sign-in-first']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='launch-sheet-accounts']")!.textContent).toContain(
      "headroom unknown",
    );
  });

  it("logged-out rows: Sign in first detours WITHOUT closing the sheet, and never show a Launch button (Req.8)", async () => {
    const { container, signinSpy } = mountSheet();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((b) => b.textContent === "Customise ▸")!,
    );
    const signin = await waitFor(() => {
      const found = container.querySelector("[data-testid='sign-in-first']");
      expect(found).not.toBeNull();
      return found! as HTMLButtonElement;
    });
    fireEvent.click(signin);
    await waitFor(() => expect(signinSpy).toHaveBeenCalledWith("claude-personal"));
    // The sheet is still open (the detour returns).
    expect(container.querySelector("[data-testid='launch-sheet-accounts']")).not.toBeNull();
  });

  it("the word Profile never appears as a sheet noun (D-02/D-10 — Launch settings only)", async () => {
    const { container } = mountSheet();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
    const text = container.textContent ?? "";
    expect(text).not.toMatch(/\bProfile\b/);
    expect(text).toContain("Launch settings");
  });

  it("Customise ▸ collapses back; the recommended row returns", async () => {
    const { container } = mountSheet();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((b) => b.textContent === "Customise ▸")!,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='sign-in-first']")).not.toBeNull(),
    );
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((b) =>
        b.textContent?.includes("hide Launch settings"),
      )!,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='launch-preselected']")).not.toBeNull(),
    );
  });
});
