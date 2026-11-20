/**
 * Walk K1 (0.4.6 RC): an Account's "Its apps" said "Claude Code CLI · Work
 * · work, 67 sessions" where its Sessions tab, the App's page and the
 * database said 333: it counted the newest few hundred sessions the app
 * holds. Each App's count is now the backend's (the relation cache, as the
 * Sessions tab counts), and an App the loaded window missed isn't "quiet".
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SubscriptionPage } from "./SubscriptionPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import { staleSubscriptions } from "../../lib/subscriptions";
import type { AccountInfo } from "../../types/generated/AccountInfo";

const ID = "acct:anthropic:work";
const CLI = "cli:claude-work";
const DESKTOP = "app:claude-app:/Users/you/Profiles/Work";

vi.mock("../../lib/relation-counts", async (original) => {
  const real = await original<typeof import("../../lib/relation-counts")>();
  return {
    ...real,
    // The backend's cache: the Account's sessions in each of its Apps.
    useRelatedCounts: (...args: Parameters<typeof real.useRelatedCounts>) =>
      args[0] === "subscription" && args[1] === ID && args[2] === "app" ? new Map([[CLI, 333], [DESKTOP, 12]]) : real.useRelatedCounts(...args),
  };
});

const NOW = Date.now();
const folders: AccountInfo[] = [
  { id: "claude-work", label: "Work Max", provider: "claude-code", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const member = (id: string, label: string) => ({ id, label, folder: `~/.claude-${id}`, plan: null, renewal_date: null, fee_usd: null });
const subscriptions = [
  { key: "k-work", account_id: ID, name: null, tool: "claude-code", state: "signed_in", email: "w…@example.com", organisation: null, reason: null, profiles: [member("claude-work", "Work Max")], plan: "max-20x", renewal_date: null, fee_usd: null, differs: [] },
];

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
  staleSubscriptions();
  localStorage.clear();
});

describe("an Account's Apps count all their sessions (walk K1)", () => {
  it("each App's row says the backend's count, not the loaded window's", async () => {
    staleSubscriptions();
    Reflect.set(window, "__AIJ_FIXTURE__", { conversations: placesConversations(NOW), subscriptions });
    Reflect.set(window, "__AIJ_PLACES__", knownPlacesPreview(NOW));
    // No sessions loaded at all: every count below is the cache's.
    render(
      <ConversationsProvider>
        <ActionHostProvider host={{}}>
          <SubscriptionPage id={ID} sessions={[]} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    const apps = await screen.findByTestId("account-apps-top");
    const row = (id: string) => within(apps).getAllByTestId("account-app").find((one) => one.getAttribute("data-app-id") === id);
    // "333 sessions · …", or "3 open · 333 in all" with some open.
    await waitFor(() => expect(row(CLI)?.textContent).toMatch(/\b333 (sessions|in all)\b/));
    expect(row(DESKTOP)?.textContent).toMatch(/\b12 (sessions|in all)\b/);
    // Both counted, so neither is in the quiet group; the busier one first.
    expect(within(apps).queryByTestId("account-apps-quiet")).toBeNull();
    const order = within(apps).getAllByTestId("account-app").map((one) => one.getAttribute("data-app-id"));
    expect(order.indexOf(CLI)).toBeLessThan(order.indexOf(DESKTOP));
  });
});
