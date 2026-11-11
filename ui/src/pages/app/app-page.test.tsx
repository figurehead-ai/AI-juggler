/**
 * D-62 (AJ-E3204): an App's page. A Claude app profile's page has only
 * its own sessions (A1), says whether it runs with the one thing to do
 * there, counts its tabs, and is renamed from its Settings.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppPage } from "./AppPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import * as subs from "../../lib/ipc/subscriptions";
import { loadSubscriptions, staleSubscriptions } from "../../lib/subscriptions";

const NOW = Date.now();
const WORK = "app:claude-app:/Users/you/Profiles/Work";
const seams = () => window as unknown as Record<string, unknown>;
const folders = [
  { id: "claude-work", label: "Work Max", provider: "claude-code", login_state: "logged_in", renewal_date: null, cleanup_opt_in: false, preview_opt_in: false } as AccountInfo,
  { id: "z-out", label: "Old laptop", provider: "claude-code", login_state: "logged_out", renewal_date: null, cleanup_opt_in: false, preview_opt_in: false } as AccountInfo,
];

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
  localStorage.clear();
});

function page(id: string, accounts: AccountInfo[] = folders) {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  seams().__AIJ_PLACES__ = knownPlacesPreview(NOW);
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <AppPage id={id} sessions={[]} projects={[]} accounts={accounts} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("D-62 the App page", () => {
  it("a running Claude app profile: its name, Go to it, and only its own sessions (A1)", async () => {
    page(WORK);
    expect(await screen.findByTestId("page-name")).toHaveProperty("textContent", "Claude app · Work Max");
    expect(screen.getByTestId("app-go").textContent).toBe("Go to it");
    const open = await screen.findByTestId("app-open");
    await waitFor(() => expect(within(open).getAllByTestId("page-session-item").length).toBeGreaterThan(0));
    // Its sessions are the ones last seen in Work; Personal's aren't here.
    const keys = [...document.querySelectorAll("[data-testid='page-session-item']")].map((row) => row.getAttribute("data-key"));
    expect(keys.some((key) => key?.endsWith("/outside-1"))).toBe(true);
    expect(keys.some((key) => key?.endsWith("/outside-5") || key?.endsWith("/outside-2"))).toBe(false);
    const tabs = screen.getAllByRole("tab").map((tab) => tab.textContent);
    expect(tabs[0]).toBe("Overview");
    expect(tabs.some((tab) => tab?.startsWith("Sessions"))).toBe(true);
    expect(tabs).toContain("Settings");
  });

  it("its facts start at its state, counted nowhere else; Last used shows only once something has ended (the 0.4 review, 47 and 78)", async () => {
    page(WORK);
    const open = await screen.findByTestId("app-open");
    await waitFor(() => expect(within(open).getAllByTestId("page-session-item").length).toBeGreaterThan(0));
    const facts = screen.getByTestId("page-facts").textContent!;
    expect(facts.startsWith("Running · kept alive")).toBe(true);
    expect(facts).not.toMatch(/\bsessions?\b|\bopen\b/);
    // Open now's rows are its count: no "2 open" sub-line over them.
    expect(open.querySelector("h2, h3")?.parentElement?.textContent ?? "").not.toMatch(/\d+ open/);
    expect(await screen.findByTestId("app-recent")).toBeTruthy();
    cleanup();
    // Everything in the Codex app is open: no Last used card at all.
    page("app:codex-app:");
    await waitFor(() => expect(within(screen.getByTestId("app-open")).getAllByTestId("page-session-item").length).toBeGreaterThan(0));
    expect(screen.queryByTestId("app-recent")).toBeNull();
    expect(screen.getByTestId("app-overview").textContent).not.toContain("Everything here is open now.");
  });

  it("one that isn't running offers Start; renamed, its new name shows", async () => {
    page("app:claude-app:/Users/you/Profiles/Lab");
    expect((await screen.findByTestId("app-go")).textContent).toBe("Start");
    fireEvent.click(screen.getAllByRole("tab").find((tab) => tab.textContent === "Settings")!);
    fireEvent.click(await screen.findByTestId("app-rename"));
    fireEvent.change(screen.getByTestId("app-rename-sheet-name"), { target: { value: "Experiments" } });
    fireEvent.click(screen.getByTestId("app-rename-sheet-save"));
    await waitFor(() => expect(screen.getByTestId("page-name").textContent).toBe("Experiments"));
    // Back to the default, from the same place.
    fireEvent.click(await screen.findByTestId("app-name-reset"));
    await waitFor(() => expect(screen.getByTestId("page-name").textContent).toBe("Claude app · Lab"));
  });

  it("a CLI App: New session, tidy-up in Settings, and its plan, renewal and fee shown as its Account's, set there (AJ-E3202 req. 4)", async () => {
    page("cli:claude-work");
    expect((await screen.findByTestId("page-name")).textContent).toBe("Claude Code · Work Max");
    // AJ-E3204 R1: a CLI App's main button, as the project page says it.
    expect(screen.getByTestId("app-new-session").textContent).toBe("New session here");
    fireEvent.click(screen.getAllByRole("tab").find((tab) => tab.textContent === "Settings")!);
    expect((await screen.findByTestId("app-terms-account")).textContent).toContain("They're its account's");
    expect(screen.getByTestId("app-terms-renewal").textContent).toContain("Renewal date");
    // Read-only here: no editors of its own.
    expect(screen.queryByTestId("renewal-editor")).toBeNull();
    expect(screen.queryByTestId("fee-editor")).toBeNull();
    expect(screen.getByTestId("account-cleanup-toggle")).toBeTruthy();
    // "Set on its account" opens the Account's Settings.
    const opened: string[] = [];
    const on = (event: Event) => opened.push((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener("aij:open-account", on);
    fireEvent.click(screen.getByTestId("app-terms-open"));
    window.removeEventListener("aij:open-account", on);
    expect(opened).toEqual([expect.stringMatching(/^acct:/)]);
    expect(localStorage.getItem("aij.page.tab.subscription")).toBe("settings");
    // Its ⋯: the folder's own actions.
    expect(screen.getByTestId("header-menu")).toBeTruthy();
  });

  it("an App whose sign-in couldn't be checked stays on its Account and says so (0.4.1)", async () => {
    const read = await loadSubscriptions(true);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue(
      read.map((row) => ({ ...row, profiles: row.profiles.map((p) => (p.id === "claude-work" ? { ...p, unchecked: "its state file was being written" } : p)) })),
    );
    staleSubscriptions();
    page("cli:claude-work");
    const said = await screen.findByTestId("app-sign-in-unchecked");
    expect(said.textContent).toBe("Couldn't check its sign-in · its state file was being written");
    // Still its Account's: the door to it stays.
    expect(screen.getByTestId("page-facts").textContent).toContain("Couldn't check its sign-in");
    cleanup();
    vi.restoreAllMocks();
    staleSubscriptions();
    page("cli:claude-work");
    await screen.findByTestId("page-name");
    await waitFor(() => expect(screen.queryByTestId("app-sign-in-unchecked")).toBeNull());
  });

  it("its header names the Mac it runs on, a door to This Mac, and says when its room is shared (AJ-E3204 R1)", async () => {
    page(WORK);
    const mac = await screen.findByTestId("door-mac");
    expect(mac.textContent).toBe("This Mac");
    let opened: unknown = null;
    const on = (event: Event) => (opened = event instanceof CustomEvent ? (event.detail as unknown) : null);
    window.addEventListener("aij:open-screen", on);
    fireEvent.click(mac);
    window.removeEventListener("aij:open-screen", on);
    expect(opened).toEqual({ id: "thismac" });
    // Work Max's Claude app profile and its terminal share one Account.
    const shared = await screen.findByTestId("app-header-shared");
    expect(shared.textContent).toBe("room shared with 1 App");
    expect(shared.getAttribute("title")).toContain("Claude Code · Work Max");
    cleanup();
    // Alone on its Account: nothing said.
    page("app:codex-app:");
    await screen.findByTestId("door-mac");
    expect(screen.queryByTestId("app-header-shared")).toBeNull();
  });

  it("a CLI App has its launch settings as a tab of its own, counted; a desktop App doesn't (AJ-E3204 R2, AJ-E3104 R1)", async () => {
    seams().__AIJ_RECIPES__ = [
      { id: "rec-deep", name: "Deep work", account_id: "claude-work", model: "opus", default_cwd: "/Users/you/Dev/shop", settings_overlay: {} },
      { id: "rec-other", name: "Elsewhere", account_id: "z-out", model: null, default_cwd: null, settings_overlay: {} },
    ];
    page("cli:claude-work");
    await screen.findByTestId("page-name");
    // AJ-E3104: its count is its own sets only (1, not the other folder's).
    await waitFor(() => expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toContain("Launch settings 1"));
    const tabs = screen.getAllByRole("tab").map((tab) => tab.textContent);
    // In the requirement's order: after Pools, before Usage.
    expect(tabs.findIndex((tab) => tab?.startsWith("Launch settings"))).toBe(tabs.findIndex((tab) => tab?.startsWith("Pools")) + 1);
    fireEvent.click(screen.getAllByRole("tab").find((tab) => tab.textContent?.startsWith("Launch settings"))!);
    const list = await screen.findByTestId("profile-launch-settings");
    // Its own sets only, each with Launch and its bin.
    expect(list.textContent).toContain("Deep work");
    expect(list.textContent).not.toContain("Elsewhere");
    expect(within(list).getByRole("button", { name: /Launch/ })).toBeTruthy();
    expect(within(list).getByTestId("recipe-delete")).toBeTruthy();
    cleanup();
    delete seams().__AIJ_RECIPES__;
    page(WORK);
    await screen.findByTestId("page-name");
    expect(screen.getAllByRole("tab").some((tab) => tab.textContent?.startsWith("Launch settings"))).toBe(false);
  });

  it("a CLI App that's signed out says so, and where to sign in; with no Account, its folder keeps its own renewal", async () => {
    page("cli:z-out");
    expect((await screen.findAllByTestId("app-signed-out")).length).toBeGreaterThan(0);
    expect(screen.queryByTestId("account-limits")).toBeNull();
    // Its notice's Sign in is the one way on: no New session beside it.
    expect(screen.queryByTestId("app-new-session")).toBeNull();
    fireEvent.click(screen.getAllByRole("tab").find((tab) => tab.textContent === "Settings")!);
    expect(await screen.findByTestId("renewal-editor")).toBeTruthy();
    expect(screen.queryByTestId("app-terms-account")).toBeNull();
  });

  it("a Claude app profile whose folder's terminal isn't signed in isn't called signed out (walk A1)", async () => {
    page(WORK, folders.map((folder) => (folder.id === "claude-work" ? { ...folder, login_state: "logged_out" } : folder)));
    expect(await screen.findByTestId("page-name")).toHaveProperty("textContent", "Claude app · Work Max");
    await screen.findByTestId("app-overview");
    expect(screen.queryByTestId("app-signed-out")).toBeNull();
    expect(screen.queryByTestId("account-limits")).toBeNull();
  });

  it("one AI Juggler doesn't know now says so", async () => {
    page("app:claude-app:/nowhere");
    expect((await screen.findByTestId("app-page-gone")).textContent).toContain("doesn't know this app");
  });
});
