/**
 * D-62 (AJ-E3204): an App's page. A Claude app profile's page has only
 * its own sessions (A1), says whether it runs with the one thing to do
 * there, counts its tabs, and is renamed from its Settings.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AppPage } from "./AppPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import type { AccountInfo } from "../../lib/bridge";

const NOW = Date.now();
const WORK = "app:claude-app:/Users/you/Profiles/Work";
const seams = () => window as unknown as Record<string, unknown>;
const folders = [
  { id: "claude-work", label: "Work Max", provider: "claude-code", login_state: "logged_in", renewal_date: null, cleanup_opt_in: false, preview_opt_in: false } as AccountInfo,
  { id: "z-out", label: "Old laptop", provider: "claude-code", login_state: "logged_out", renewal_date: null, cleanup_opt_in: false, preview_opt_in: false } as AccountInfo,
];

afterEach(() => {
  cleanup();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
  localStorage.clear();
});

function page(id: string) {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  seams().__AIJ_PLACES__ = knownPlacesPreview(NOW);
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <AppPage id={id} sessions={[]} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
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
    expect(screen.getByTestId("app-new-session")).toBeTruthy();
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

  it("one AI Juggler doesn't know now says so", async () => {
    page("app:claude-app:/nowhere");
    expect((await screen.findByTestId("app-page-gone")).textContent).toContain("doesn't know this app");
  });
});
