/**
 * AJ-E1552 — Home's, Accounts' and This Mac's skeletons draw only while
 * their read loads (Still loading after 3 s), then their own empty state,
 * or an error with Retry that reads again.
 *
 * Home's skeleton waits on the shell's first read, which has its own
 * deadline (shell/use-app-data): when it misses, the read counts as done
 * and the "service stopped answering" banner, with its Retry, takes over.
 * So Home is tested for loading and empty here; its error is the shell's.
 */
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as accountsIpc from "../lib/ipc/accounts";
import * as launchIpc from "../lib/ipc/launch";
import * as providersIpc from "../lib/ipc/providers";
import * as quotaIpc from "../lib/ipc/quota";
import * as subsIpc from "../lib/ipc/subscriptions";
import * as thisMacIpc from "../lib/ipc/this-mac";
import { DEADLINE_MS } from "../lib/loading";
import { ThisMacContext, type ThisMacInfo } from "../lib/machine";
import { mockMachineHealth } from "../preview/machine";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../test/loading-kit";
import { HomeScreen } from "./Home";
import { ProfilesScreen } from "./Profiles";
import { ThisMacScreen } from "./ThisMacScreen";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const NOW = 1_728_800_000_000;

describe("Home", () => {
  const home = (loading: boolean) =>
    render(<HomeScreen loading={loading} sessions={[]} accounts={[]} projects={[]} now={NOW} onOpenSession={() => {}} onNewSession={() => {}} onAddAccounts={() => {}} />);
  it("loading", async () => {
    vi.useFakeTimers();
    home(true);
    await expectSlowSkeleton("home-loading");
  });
  it("empty: the welcome, no skeleton", async () => {
    home(false);
    await expectEmpty("home-empty-accounts", /./, "home-loading");
  });
});

describe("Accounts", () => {
  beforeEach(() => {
    vi.spyOn(launchIpc, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(quotaIpc, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(providersIpc, "providersCapabilities").mockResolvedValue([]);
    vi.spyOn(subsIpc, "subscriptionsList").mockResolvedValue([]);
  });
  const accounts = () => render(<ProfilesScreen knownAccounts={null} knownProfiles={null} openTerminal={() => {}} onAddAccounts={() => {}} />);
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(accountsIpc, "accountsList").mockReturnValue(never());
    accounts();
    await expectSlowSkeleton("profiles-loading");
  });
  it("empty", async () => {
    vi.spyOn(accountsIpc, "accountsList").mockResolvedValue([]);
    accounts();
    await expectEmpty("accounts-empty", "No accounts yet", "profiles-loading");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(accountsIpc, "accountsList").mockRejectedValue(new Error("down"));
    accounts();
    await expectErrorRetry("your accounts", read, "profiles-loading");
  });
});

/** A measured Mac with no AI tool found yet (`scanned` says whether it looked). */
function measured(scanned: boolean): ThisMacInfo {
  const none = { count: 0, cpu_pct: null, rss_bytes: 0, memory_estimate: false };
  return {
    machine: mockMachineHealth("plenty", NOW),
    tools: none,
    ours: none,
    theirs: none,
    by_tool: [],
    top: [],
    others: [],
    heavy: [],
    other_apps: [],
    leftover_processes: 0,
    leftover_ports: 0,
    scanned,
    alerts: [],
    recommendations: [],
    awake: { working: 0, kept_awake: false, on_battery: false },
    own: { rss_bytes: 214_000_000, watchdog: true, opens_at_login: false },
  };
}

describe("This Mac", () => {
  const page = (info: ThisMacInfo | null) =>
    render(
      <ThisMacContext.Provider value={info}>
        <ThisMacScreen nameOf={(id) => id} onOpenSession={() => {}} />
      </ThisMacContext.Provider>,
    );
  it("loading: measuring", async () => {
    vi.useFakeTimers();
    page(null);
    await expectSlowSkeleton("this-mac-loading");
  });
  it("loading: looking for running AI tools", async () => {
    vi.useFakeTimers();
    page(measured(false));
    await expectSlowSkeleton("this-mac-tools-loading");
  });
  it("empty: no AI tool running", async () => {
    page(measured(true));
    await expectEmpty("this-mac-tools-empty", "No AI tools are running. Each one shows up here when it starts.", "this-mac-tools-loading");
  });
  it("error with Retry when no measurement comes by the deadline", async () => {
    const read = vi.spyOn(thisMacIpc, "thisMac").mockReturnValue(never());
    vi.useFakeTimers();
    page(null);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(DEADLINE_MS);
    });
    vi.useRealTimers();
    await expectErrorRetry("this Mac's measurements", read, "this-mac-loading");
    expect(screen.queryByTestId("this-mac-loading")).toBeTruthy();
  });
});
