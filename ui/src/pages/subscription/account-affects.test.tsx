/**
 * A5 (the AJ-E3306 rc22 walk): an Account out of room says on its own
 * page what that affects: each session open on it, with its own switch
 * (the one ⌘⇧H and its row menu run, opening that session's switch
 * sheet), or plainly that nothing is open, with when it resets.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SubscriptionPage } from "./SubscriptionPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import { staleSubscriptions } from "../../lib/subscriptions";
import { resetNoticeStandIn } from "../../lib/ipc/notices";
import { forecastDisplay, quotaDisplay, windowDisplay } from "../../lib/wire";
import type { ActionHost } from "../../lib/actions";
import { account, clearSeams, instance, setSeam, subscription } from "../../test/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ConversationInfo } from "../../lib/ipc/conversations";

const NOW = Date.now();
const RESETS = NOW + 2 * 3_600_000;
const ID = "acct:anthropic:work";
const folders = [account({ id: "claude-work", label: "Work Max", plan: "max-20x" })];
const subscriptions = [subscription({ folder: "claude-work", account_id: ID, name: "Work", plan: "max-20x" })];
// Work's 5-hour window used up, resetting in two hours.
const out = {
  ...quotaDisplay("claude-work"),
  five_hour: windowDisplay("exhausted", { percentage: 100, resets_at_ms: RESETS }),
  weekly: null,
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: forecastDisplay("learning", { badge: "" }),
};
// One AI Juggler runs, and one another app runs, both on Work.
const session = instance({ id: "s-work", account_id: "claude-work", title: "Fix the flaky login test", cwd: "/Users/you/Dev/web", conversation_id: "conv-s-work" });
const other = (): ConversationInfo => {
  const one = placesConversations(NOW).find((c) => c.account_id === "claude-work" && c.live)!;
  return { ...one, id: "conv-other", title: "Write the onboarding copy", project_id: null, session_id: null };
};

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_FIXTURE__", "__AIJ_PLACES__", "__AIJ_QUOTA_DISPLAY__", "__AIJ_NOTICES__");
  resetNoticeStandIn();
  staleSubscriptions();
  localStorage.clear();
});

function page({ sessions, conversations, host, quota = [out] }: { sessions: InstanceInfo[]; conversations: ConversationInfo[]; host: ActionHost; quota?: unknown[] }) {
  staleSubscriptions();
  setSeam("__AIJ_FIXTURE__", { conversations, subscriptions });
  setSeam("__AIJ_PLACES__", knownPlacesPreview(NOW));
  setSeam("__AIJ_QUOTA_DISPLAY__", quota);
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={host}>
        <SubscriptionPage id={ID} sessions={sessions} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("A5 an out-of-room Account's page: what it affects", () => {
  it("lists both open sessions, each with its own switch, which opens that session's switch sheet", async () => {
    const continueSession = vi.fn();
    const continueConversationElsewhere = vi.fn();
    const conversation = other();
    page({ sessions: [session], conversations: [conversation], host: { continueSession, continueConversationElsewhere } });
    const block = await screen.findByTestId("account-affects");
    expect(within(block).getByRole("heading").textContent).toBe("What this affects");
    expect(within(block).getByTestId("account-affects-mark").dataset.kind).toBe("out");
    const items = await within(block).findAllByTestId("account-affects-item");
    expect(items).toHaveLength(2);
    const byTitle = (words: string) => items.find((item) => item.textContent?.includes(words))!;
    const mine = within(byTitle("Fix the flaky login test")).getByTestId("account-affects-switch");
    const theirs = within(byTitle("Write the onboarding copy")).getByTestId("account-affects-switch");
    expect(mine.textContent).toContain("Continue in another app…");
    expect(theirs.textContent).toContain("Continue in another app…");
    fireEvent.click(mine);
    expect(continueSession).toHaveBeenCalledTimes(1);
    expect(continueSession.mock.calls[0]![0].id).toBe("s-work");
    expect(continueConversationElsewhere).not.toHaveBeenCalled();
    fireEvent.click(theirs);
    expect(continueConversationElsewhere).toHaveBeenCalledTimes(1);
    expect(continueConversationElsewhere.mock.calls[0]![0].id).toBe("conv-other");
    // Each session once on the page: the block holds what's open.
    expect(screen.queryByTestId("account-open")).toBeNull();
  });

  it("with nothing open, says so plainly, with when it resets", async () => {
    page({ sessions: [], conversations: [], host: { continueSession: vi.fn() } });
    const none = await screen.findByTestId("account-affects-none");
    expect(none.textContent).toMatch(/^Nothing open on this account now · resets at .+\.$/);
    expect(screen.queryByTestId("account-affects-switch")).toBeNull();
  });

  it("with room, there's no block and Open now is as before", async () => {
    page({ sessions: [session], conversations: [], host: {}, quota: [] });
    expect(await screen.findByTestId("account-open")).toBeTruthy();
    expect(screen.queryByTestId("account-affects")).toBeNull();
  });

  it("keeps the account notice's Remind me and Snooze 1 h; snoozed, they go", async () => {
    resetNoticeStandIn();
    const target = { session_id: null, conversation_id: null, account_id: "claude-work", project_id: null, page: null, task_id: null, pool_id: null };
    const facts = { window: "5-hour", resets_at_ms: RESETS, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Work" };
    setSeam("__AIJ_NOTICES__", {
      app: [{ id: 7, kind: "quota", needs: true, title: "Work has run out of room", detail: "0% of its 5-hour window left", project_id: null, project: null, target, facts, action: "hand-over", meter: 0, created_ms: NOW, unread: true, subject: "n:7", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false }],
    });
    page({ sessions: [session], conversations: [], host: { continueSession: vi.fn() } });
    const remind = await screen.findByTestId("account-affects-remind");
    expect(remind.textContent).toMatch(/^Remind me at /);
    fireEvent.click(screen.getByTestId("account-affects-snooze"));
    await waitFor(() => expect(screen.queryByTestId("account-affects-snooze")).toBeNull());
    expect(screen.queryByTestId("account-affects-remind")).toBeNull();
  });
});
