/**
 * D-62 (AJ-E3205): an Account's page. One subscription with its limits
 * first, the Apps that share it (a CLI App and a Claude app profile on
 * one sign-in, A2), and its own name, set once from its Settings.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SubscriptionPage } from "./SubscriptionPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import { staleSubscriptions } from "../../lib/subscriptions";
import type { AccountInfo } from "../../lib/bridge";

const NOW = Date.now();
const ID = "acct:anthropic:work";
const seams = () => window as unknown as Record<string, unknown>;
const folders = [
  { id: "claude-work", label: "Work Max", provider: "claude-code", plan: "max-20x", login_state: "logged_in" } as AccountInfo,
  { id: "claude-personal", label: "Personal", provider: "claude-code", plan: null, login_state: "logged_in" } as AccountInfo,
];
const member = (id: string, label: string) => ({ id, label, folder: `~/.claude-${id}`, plan: null, renewal_date: null, fee_usd: null });
const subscriptions = [
  { key: "k-work", account_id: ID, name: null, tool: "claude-code", state: "signed_in", email: "w…@example.com", organisation: null, reason: null, profiles: [member("claude-work", "Work Max")], plan: "max-20x", renewal_date: null, fee_usd: null, differs: [] },
  { key: "k-me", account_id: "acct:anthropic:me", name: null, tool: "claude-code", state: "signed_in", email: "p…@example.com", organisation: null, reason: null, profiles: [member("claude-personal", "Personal")], plan: null, renewal_date: null, fee_usd: null, differs: [] },
];

afterEach(() => {
  cleanup();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
  staleSubscriptions();
  localStorage.clear();
});

function page(id: string) {
  staleSubscriptions();
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW), subscriptions };
  seams().__AIJ_PLACES__ = knownPlacesPreview(NOW);
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <SubscriptionPage id={id} sessions={[]} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("D-62 the Account page", () => {
  it("named its own way, with the Apps that share it and their sessions (A2)", async () => {
    page(ID);
    expect((await screen.findByTestId("page-name")).textContent).toBe("Claude Max 20x");
    const apps = await screen.findByTestId("account-apps-top");
    await waitFor(() => expect(within(apps).getAllByTestId("account-app").map((row) => row.getAttribute("data-app-id"))).toEqual(expect.arrayContaining(["cli:claude-work", "app:claude-app:/Users/you/Profiles/Work"])));
    // Another account's App isn't here.
    expect(within(apps).queryByText("Personal")).toBeNull();
    const tabs = screen.getAllByRole("tab").map((tab) => tab.textContent);
    expect(tabs.slice(0, 2)).toEqual(["Overview", expect.stringMatching(/^Apps/)]);
    expect(screen.getByTestId("account-new-session")).toBeTruthy();
  });

  it("renamed in its Settings, the name shows; the default comes back", async () => {
    page(ID);
    fireEvent.click((await screen.findAllByRole("tab")).find((tab) => tab.textContent === "Settings")!);
    fireEvent.click(await screen.findByTestId("account-rename"));
    fireEvent.change(screen.getByTestId("account-rename-sheet-name"), { target: { value: "Day job" } });
    fireEvent.click(screen.getByTestId("account-rename-sheet-save"));
    await waitFor(() => expect(screen.getByTestId("page-name").textContent).toBe("Day job"));
    fireEvent.click(await screen.findByTestId("account-name-reset"));
    await waitFor(() => expect(screen.getByTestId("page-name").textContent).toBe("Claude Max 20x"));
  });

  it("one AI Juggler doesn't know now says so", async () => {
    page("acct:anthropic:gone");
    expect((await screen.findByTestId("account-page-gone")).textContent).toContain("doesn't know this account");
  });
});
