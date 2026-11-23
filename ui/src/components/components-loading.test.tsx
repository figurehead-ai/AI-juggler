/**
 * AJ-E1552 — the shared components' skeletons draw only while their read
 * loads: each shows its skeleton while loading (Still loading after 3 s),
 * its own empty words, and an error with Retry at its deadline or when
 * its read fails. The rail waits on the shell's first read, whose own
 * deadline and Retry take over ("stopped answering"), so it is tested for
 * loading only; an icon gives up its pulse for its letters at the
 * deadline (no room for words in an icon's slot).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEADLINE_MS } from "../lib/loading";
import * as conversationsIpc from "../lib/ipc/conversations";
import * as feedIpc from "../lib/ipc/feed";
import * as noticesIpc from "../lib/ipc/notices";
import { ConversationsProvider } from "../lib/conversations";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../test/loading-kit";
import { AccountAppRows } from "../pages/subscription/AccountApps";
import { StatTile } from "./charts/StatTile";
import { AppChips } from "./usage/RunwayCards";
import { RailLoading } from "./RailLoading";
import { HomeFeed } from "./HomeFeed";
import { NotificationCentre } from "./NotificationCentre";
import { ConversationPage } from "./ConversationPage";
import { ActionHostProvider } from "./ItemMenu";

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const pastDeadline = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(DEADLINE_MS);
  });

describe("an Account's app rows", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    render(<AccountAppRows apps={null} now={0} />);
    await expectSlowSkeleton("account-apps-pending");
  });
  it("empty", async () => {
    render(<AccountAppRows apps={[]} now={0} />);
    await expectEmpty("account-apps-empty", "No app uses this account yet.", "account-apps-pending");
  });
  it("error at the deadline, with Retry", async () => {
    vi.useFakeTimers();
    const onRetry = vi.fn();
    render(<AccountAppRows apps={null} now={0} onRetry={onRetry} />);
    await pastDeadline();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load its apps.");
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("account-apps-pending")).toBeTruthy();
  });
});

describe("a usage card's app chips", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    render(<AppChips chips={null} label="Work" />);
    await expectSlowSkeleton("usage-account-apps-pending");
  });
  it("empty", async () => {
    render(<AppChips chips={[]} label="Work" />);
    await expectEmpty("usage-account-apps-empty", "No app uses this account yet.", "usage-account-apps-pending");
  });
  it("error at the deadline; Retry waits again", async () => {
    vi.useFakeTimers();
    render(<AppChips chips={null} label="Work" />);
    await pastDeadline();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load its apps.");
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(screen.getByTestId("usage-account-apps-pending")).toBeTruthy();
  });
});

describe("a stat tile's held space", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    render(<StatTile label="Tokens" value={null} unit="tokens" holdSpace />);
    await expectSlowSkeleton("stat-hold");
  });
  it("empty: past the deadline with no Retry, the held space ends", async () => {
    vi.useFakeTimers();
    render(<StatTile label="Tokens" value={null} unit="tokens" holdSpace />);
    await pastDeadline();
    expect(screen.queryByTestId("stat-hold")).toBeNull();
    expect(screen.getByTestId("stat-value").textContent).toBe("—");
  });
  it("error with Retry", async () => {
    vi.useFakeTimers();
    const onRetry = vi.fn();
    render(<StatTile label="Tokens" value={null} unit="tokens" holdSpace onRetry={onRetry} />);
    await pastDeadline();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load tokens.");
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});

describe("the rail", () => {
  it("loading: its rows, Still loading after 3 s", async () => {
    vi.useFakeTimers();
    render(<RailLoading />);
    await expectSlowSkeleton("placeholder-rail");
  });
});

describe("Home's feed", () => {
  const feed = () =>
    render(
      <HomeFeed stacked={false} now={0} nameOf={() => null} quiet onNewSession={vi.fn()} onOpenApp={vi.fn()} toReview={0} onReviewAll={vi.fn()} scroller={() => null}
        facets={{ projects: [], accounts: [], subscriptions: [], pools: null }} actions={{ open: vi.fn(() => true), review: vi.fn(), railOf: () => null }} />,
    );
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(feedIpc, "feedPage").mockReturnValue(never());
    feed();
    await expectSlowSkeleton("feed-skeleton");
  });
  it("empty: what to do first", async () => {
    vi.spyOn(feedIpc, "feedPage").mockResolvedValue({ items: [], next_cursor: null, total: 0 });
    feed();
    await expectEmpty("feed-empty", /\S/, "feed-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(feedIpc, "feedPage").mockRejectedValue(new Error("down"));
    feed();
    expect((await screen.findByTestId("feed-error")).textContent).toContain("The feed couldn't be read.");
    const before = read.mock.calls.length;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
  });
});

describe("the notification centre's Needs action", () => {
  const centre = () => render(<NotificationCentre summary={null} sessions={[]} onClose={() => {}} />);
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(noticesIpc, "notificationsPage").mockReturnValue(never());
    centre();
    await expectSlowSkeleton("placeholder-home");
  });
  it("empty: all caught up", async () => {
    vi.spyOn(noticesIpc, "notificationsPage").mockResolvedValue({ items: [], next_cursor: null, total: 0 });
    centre();
    await expectEmpty("notices-empty", "You're all caught up.", "placeholder-home");
  });
  it("error with Retry, never all caught up", async () => {
    const read = vi.spyOn(noticesIpc, "notificationsPage").mockRejectedValue(new Error("down"));
    centre();
    await expectErrorRetry("what needs you", read, "placeholder-home");
    expect(screen.queryByTestId("notices-empty")).toBeNull();
  });
});

describe("a conversation's page", () => {
  const page = () =>
    render(
      <ConversationsProvider>
        <ActionHostProvider host={{}}>
          <ConversationPage accountId="a1" id="c" now={0} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(conversationsIpc, "conversationsList").mockReturnValue(never());
    page();
    await expectSlowSkeleton("conversation-loading");
  });
  it("empty: gone, once the list answered without it", async () => {
    vi.useFakeTimers();
    vi.spyOn(conversationsIpc, "conversationsList").mockResolvedValue([]);
    page();
    await pastDeadline();
    expect(screen.getByTestId("conversation-gone").textContent).toContain("isn't there any more");
  });
  it("error at the deadline with Retry, which reads the list again", async () => {
    vi.useFakeTimers();
    const read = vi.spyOn(conversationsIpc, "conversationsList").mockReturnValue(never());
    page();
    await pastDeadline();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load this session.");
    const before = read.mock.calls.length;
    fireEvent.click(screen.getByTestId("loading-retry"));
    expect(read.mock.calls.length).toBeGreaterThan(before);
  });
});
