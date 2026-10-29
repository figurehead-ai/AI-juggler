/**
 * E18 vitest — messages in AI Juggler (AJ-E1802..E1804).
 *
 * - A pool's threads, newest first; the open one as a conversation: an
 *   agent's message on the left, yours on the right, its kind and where
 *   it is for each recipient. Deliver only where it can go, one click.
 * - A paused pool says why, with Resume; a thread pauses, stops, resumes.
 * - You write to a member or the pool, with Deliver (on by default).
 * - Needs you: an agent's held message as the same approval card (keys),
 *   "Always allow in this pool" only from the second time, never for a
 *   hop-limit delivery; messages waiting for a session, one Deliver.
 * - Shared docs say what was withheld, as packs do.
 * - Nothing in these views is smaller than 12 px.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import MessagesViewSource from "./MessagesView.tsx?raw";
import ThreadViewSource from "./ThreadView.tsx?raw";
import ComposeSource from "./Compose.tsx?raw";
import SharedDocsSource from "./SharedDocs.tsx?raw";
import PoolLinksSource from "./PoolLinks.tsx?raw";
import MessageNeedsSource from "../../components/MessageNeeds.tsx?raw";
import { MessagesView } from "./MessagesView";
import { SharedDocs } from "./SharedDocs";
import { MessageNeeds } from "../../components/MessageNeeds";
import { resetPreviewMessages, type MessageInfo, type MessageNeedInfo, type MessagesSeed, type ThreadInfo } from "../../lib/ipc/messages";

type Seams = { __AIJ_MESSAGES__?: MessagesSeed; __AIJ_MESSAGES_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_MESSAGES_CALLS__ ?? [];
const NOW = 1_728_800_000_000;

const thread = (over: Partial<ThreadInfo>): ThreadInfo => ({
  id: "thr-1",
  pool_id: "pool-team",
  pool_name: "Checkout redesign",
  subject: "Start on the API",
  people: ["Planner", "Builder", "You"],
  count: 2,
  hop: 1,
  hop_cap: 6,
  state: "open",
  state_reason: null,
  updated_at_ms: NOW - 120_000,
  last_from: "Planner",
  last_line: "The schema is in the shared doc.",
  unread: 0,
  ...over,
});

const message = (over: Partial<MessageInfo>): MessageInfo => ({
  id: "msg-1",
  thread_id: "thr-1",
  from_name: "Planner",
  from_kind: "agent",
  from_session_id: "sess-live-1",
  kind: "handoff",
  subject: "Start on the API",
  body: "Start on the API; the schema is in the shared doc.",
  hop: 1,
  redacted: 0,
  to_all: false,
  at_ms: NOW - 300_000,
  recipients: [{ id: "session:sess-done-1", name: "Builder", session_id: "sess-done-1", state: "unread", via: null, deliverable: true, how: "Deliver puts it in as its next prompt, between turns" }],
  ...over,
});

const need = (over: Partial<MessageNeedInfo>): MessageNeedInfo => ({
  key: "ask:ask-1",
  kind: "ask",
  pool_id: "pool-team",
  pool_name: "Checkout redesign",
  thread_id: "thr-1",
  message_id: "msg-1",
  session_id: null,
  ask_id: "ask-1",
  offer_always: false,
  title: "«Planner» wants to message Builder",
  detail: "«Start on the API» · above what the pool lets agents do",
  preview: "Can I start?",
  at_ms: NOW,
  ...over,
});

beforeEach(() => {
  resetPreviewMessages();
  seams().__AIJ_MESSAGES_CALLS__ = [];
  seams().__AIJ_MESSAGES__ = {
    threads: [thread({}), thread({ id: "thr-2", subject: "Review the cart", updated_at_ms: NOW - 3_600_000, unread: 1, last_from: "Reviewer", last_line: "Can you look?" })],
    messages: {
      "thr-1": [message({}), message({ id: "msg-2", from_name: "You", from_kind: "user", from_session_id: null, kind: "note", body: "Go ahead.", at_ms: NOW - 120_000, recipients: [{ id: "session:sess-live-1", name: "Planner", session_id: "sess-live-1", state: "delivered", via: "turn", deliverable: false, how: "Started its turn" }] })],
      "thr-2": [message({ id: "msg-3", thread_id: "thr-2", from_name: "Reviewer", kind: "question", body: "Can you look?", recipients: [{ id: "user", name: "You", session_id: null, state: "unread", via: null, deliverable: false, how: "To you" }] })],
    },
  };
});
afterEach(() => cleanup());

const members = [
  { session_id: "sess-live-1", name: "Planner" },
  { session_id: "sess-done-1", name: "Builder" },
];

describe("a pool's messages", () => {
  it("lists the threads and shows the open one as a conversation", async () => {
    const { findAllByTestId, getAllByTestId, getByTestId } = render(<MessagesView poolId="pool-team" members={members} now={NOW} />);
    expect(await findAllByTestId("thread-row")).toHaveLength(2);
    const bubbles = await findAllByTestId("message-bubble");
    expect(bubbles.map((b) => b.dataset.from)).toEqual(["agent", "user"]);
    expect(bubbles[0]!.textContent).toContain("Handoff");
    expect(bubbles[0]!.textContent).toContain("an agent");
    const rows = getAllByTestId("message-recipient");
    expect(rows[0]!.textContent).toContain("Builder · Waiting in its inbox");
    expect(rows[1]!.textContent).toContain("Started its turn");
    expect(getByTestId("thread-view").textContent).toContain("can't approve anything or change settings");
    fireEvent.click(getByTestId("message-deliver"));
    await waitFor(() => expect(calls()).toContain("deliver:msg-1:session:sess-done-1"));
  });

  it("opens another thread, and its unread question is marked for you", async () => {
    const { findAllByTestId, findByText } = render(<MessagesView poolId="pool-team" members={members} now={NOW} />);
    const rows = await findAllByTestId("thread-row");
    expect(within(rows[1]!).getByLabelText("1 new for you")).toBeTruthy();
    fireEvent.click(rows[1]!);
    expect(await findByText("Can you look?", { selector: "p" })).toBeTruthy();
  });

  it("says when the pool is paused, and why, with Resume", async () => {
    seams().__AIJ_MESSAGES__!.paused = [{ pool_id: "pool-team", pool_name: "Checkout redesign", at_ms: NOW, reason: "«Alpha» and «Beta» kept answering each other (8 messages without you)" }];
    const { findByTestId } = render(<MessagesView poolId="pool-team" members={members} now={NOW} />);
    const banner = await findByTestId("messages-paused");
    expect(banner.textContent).toContain("kept answering each other");
    fireEvent.click(within(banner).getByRole("button", { name: "Resume messages" }));
    await waitFor(() => expect(calls()).toContain("pool:pool-team:false"));
  });

  it("pauses and resumes a thread", async () => {
    const { findByTestId } = render(<MessagesView poolId="pool-team" members={members} now={NOW} />);
    fireEvent.click(await findByTestId("thread-pause"));
    await waitFor(() => expect(calls()).toContain("thread:thr-1:paused"));
    fireEvent.click(await findByTestId("thread-resume"));
    await waitFor(() => expect(calls()).toContain("thread:thr-1:open"));
  });

  it("writes to the whole pool with Deliver on, from the keyboard", async () => {
    const { findByTestId, getAllByTestId } = render(<MessagesView poolId="pool-team" members={members} now={NOW} />);
    fireEvent.click(await findByTestId("messages-new"));
    const form = getAllByTestId("message-compose").find((f) => f.getAttribute("aria-label") === "New message")!;
    expect(within(form).getByTestId("compose-to").getAttribute("data-value")).toBe("pool");
    expect((within(form).getByLabelText("Deliver to their sessions") as HTMLInputElement).checked).toBe(true);
    fireEvent.change(within(form).getByTestId("compose-body"), { target: { value: "Pause; review first." } });
    fireEvent.keyDown(within(form).getByTestId("compose-body"), { key: "Enter", metaKey: true });
    await waitFor(() => expect(calls()).toContain("send:pool:true"));
  });

  it("a session's page shows its own threads only", async () => {
    const { findAllByTestId } = render(<MessagesView poolId="pool-team" sessionId="sess-done-1" members={members.slice(1)} composeTo="sess-done-1" now={NOW} />);
    const rows = await findAllByTestId("thread-row");
    expect(rows).toHaveLength(1);
    expect(rows[0]!.textContent).toContain("Start on the API");
  });
});

describe("Needs you", () => {
  it("an agent's held message is the same approval card, answered from the keyboard", async () => {
    seams().__AIJ_MESSAGES__!.needs = [need({})];
    const { getByTestId, queryByText } = render(<MessageNeeds needs={[need({})]} />);
    const card = getByTestId("ask-card");
    expect(card.textContent).toContain("An agent asks to send");
    expect(card.textContent).toContain("Can I start?");
    expect(queryByText("Always allow in this pool")).toBeNull();
    fireEvent.keyDown(card, { key: "1" });
    await waitFor(() => expect(calls()).toContain("answer:ask-1:once"));
  });

  it("offers the rule from the second time, never for the hop limit", () => {
    const { getAllByTestId, rerender } = render(<MessageNeeds needs={[need({ offer_always: true })]} />);
    expect(getAllByTestId("ask-option-always")).toHaveLength(1);
    rerender(<MessageNeeds needs={[need({ kind: "deliver", key: "ask:2", ask_id: "ask-2", offer_always: false, title: "«Planner»'s message would start another session's turn" })]} />);
    expect(document.querySelector("[data-testid='ask-option-always']")).toBeNull();
    expect(getAllByTestId("ask-card")[0]!.textContent).toContain("Don't deliver");
  });

  it("messages waiting for a session are one Deliver", async () => {
    const waiting = need({ key: "waiting:sess-done-1", kind: "waiting", ask_id: null, session_id: "sess-done-1", title: "Builder has 2 messages waiting", detail: "from Planner · Checkout redesign" });
    const { getByTestId } = render(<MessageNeeds needs={[waiting]} />);
    fireEvent.click(within(getByTestId("message-need-waiting")).getByRole("button", { name: /^Deliver/ }));
    await waitFor(() => expect(calls()).toContain("deliver-waiting:sess-done-1"));
  });
});

describe("shared docs", () => {
  it("say what was withheld, as packs do, and open read-only", async () => {
    seams().__AIJ_MESSAGES__!.docs = [
      { id: "doc-1", pool_id: "pool-team", name: "api-schema.md", mime: "text/markdown", size_bytes: 2048, shared_by: "Planner", at_ms: NOW - 600_000, policy: "strip", policy_words: "Secrets replaced with •••", withheld: [{ what: "API keys", count: 1 }] },
    ];
    seams().__AIJ_MESSAGES__!.docText = { "doc-1": "GET /orders\nOPENAI_API_KEY=•••" };
    const { findByTestId, getByTestId } = render(<SharedDocs poolId="pool-team" now={NOW} />);
    expect((await findByTestId("shared-doc-withheld")).textContent).toBe("1 API key withheld");
    fireEvent.click(within(getByTestId("shared-doc")).getByRole("button", { name: /Read/ }));
    expect((await findByTestId("shared-doc-text")).textContent).toContain("OPENAI_API_KEY=•••");
  });
});

describe("readability", () => {
  it("no text smaller than 12 px in the message views", () => {
    const files: Record<string, string> = { MessagesViewSource, ThreadViewSource, ComposeSource, SharedDocsSource, PoolLinksSource, MessageNeedsSource };
    for (const [file, text] of Object.entries(files)) {
      const small = (text.match(/text-\[(\d+(?:\.\d+)?)px\]/g) ?? []).filter((cls: string) => Number(cls.slice(6, -3)) < 12);
      expect(small, file).toEqual([]);
    }
  });
});
