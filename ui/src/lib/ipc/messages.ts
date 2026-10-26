/**
 * E18 — messages between the agents of a pool, and the user's (one IPC
 * module per feature, ENGINEERING-GUIDE U3). Only the app's window calls
 * these: an agent reaches messages through its own tools, never here.
 *
 * Every write is the user's: their message, their Deliver (it puts a
 * message into a session run in AI Juggler as its next prompt, between
 * turns), pausing a thread or a pool, answering a held ask, linking pools.
 *
 * The preview keeps its own messages, seeded by `__AIJ_MESSAGES__`, and
 * records the calls that change them in `__AIJ_MESSAGES_CALLS__`.
 */
import { useEffect, useState } from "react";
import { invoke, listen } from "./transport";
import { hasLiveBackend, refused } from "../bridge";
import type { ThreadsPage } from "../../types/generated/ThreadsPage";
import type { ThreadInfo } from "../../types/generated/ThreadInfo";
import type { ThreadPage } from "../../types/generated/ThreadPage";
import type { MessageInfo } from "../../types/generated/MessageInfo";
import type { RecipientInfo } from "../../types/generated/RecipientInfo";
import type { ComposeInput } from "../../types/generated/ComposeInput";
import type { DocInfo } from "../../types/generated/DocInfo";
import type { DocsPage } from "../../types/generated/DocsPage";
import type { DocContent } from "../../types/generated/DocContent";
import type { MessageNeedInfo } from "../../types/generated/MessageNeedInfo";
import type { PauseInfo } from "../../types/generated/PauseInfo";

export type { ThreadsPage, ThreadInfo, ThreadPage, MessageInfo, RecipientInfo, ComposeInput, DocInfo, DocsPage, DocContent, MessageNeedInfo, PauseInfo };

/** The event the backend sends when messages change (the pools' ids). */
export const MESSAGES_CHANGED = "messages://changed";

/** What the preview starts with. */
export interface MessagesSeed {
  threads?: ThreadInfo[];
  /** Each thread's messages, oldest first. */
  messages?: Record<string, MessageInfo[]>;
  docs?: DocInfo[];
  docText?: Record<string, string>;
  needs?: MessageNeedInfo[];
  paused?: PauseInfo[];
  links?: Record<string, string[]>;
}
interface Seams {
  __AIJ_MESSAGES__?: MessagesSeed;
  __AIJ_MESSAGES_CALLS__?: string[];
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_MESSAGES_CALLS__ ??= []).push(call);
const PAGE = 30;

let store: Required<MessagesSeed> | null = null;
const preview = (): Required<MessagesSeed> => {
  if (store === null) {
    const seed = seams().__AIJ_MESSAGES__ ?? {};
    store = {
      threads: [...(seed.threads ?? [])],
      messages: Object.fromEntries(Object.entries(seed.messages ?? {}).map(([id, list]) => [id, [...list]])),
      docs: [...(seed.docs ?? [])],
      docText: { ...(seed.docText ?? {}) },
      needs: [...(seed.needs ?? [])],
      paused: [...(seed.paused ?? [])],
      links: { ...(seed.links ?? {}) },
    };
  }
  return store;
};
const listeners = new Set<() => void>();
const changed = () => listeners.forEach((fn) => fn());

/** Tests: forget the preview's messages between cases. */
export function resetPreviewMessages(): void {
  store = null;
}

function involves(thread: ThreadInfo, sessionId: string): boolean {
  return (preview().messages[thread.id] ?? []).some(
    (m) => m.from_session_id === sessionId || m.recipients.some((r) => r.session_id === sessionId),
  );
}

function previewPage(threadId: string): ThreadPage {
  const data = preview();
  const thread = data.threads.find((t) => t.id === threadId);
  if (!thread) throw refused("That thread isn't there any more");
  const all = data.messages[threadId] ?? [];
  for (const m of all) for (const r of m.recipients) if (r.id === "user" && r.state === "unread") r.state = "read";
  thread.unread = 0;
  return { thread: { ...thread }, messages: all.slice(-PAGE), earlier: all.length > PAGE ? `m${all.length - PAGE}` : null };
}

/** One page of a pool's threads, or a session's, newest first. */
export async function messagesThreads(scope: { poolId?: string; sessionId?: string }, cursor: string | null = null): Promise<ThreadsPage> {
  if (!hasLiveBackend()) {
    const data = preview();
    const rows = data.threads
      .filter((t) => (!scope.poolId || t.pool_id === scope.poolId) && (!scope.sessionId || involves(t, scope.sessionId)))
      .sort((a, b) => b.updated_at_ms - a.updated_at_ms);
    const from = cursor ? Number(cursor.slice(1)) : 0;
    return {
      threads: rows.slice(from, from + PAGE).map((t) => ({ ...t })),
      next: rows.length > from + PAGE ? `t${from + PAGE}` : null,
      paused: data.paused.find((p) => p.pool_id === scope.poolId) ?? null,
    };
  }
  return invoke<ThreadsPage>("messages_threads", { poolId: scope.poolId ?? null, sessionId: scope.sessionId ?? null, cursor });
}

/** One page of a thread, oldest message first (yours are marked read). */
export async function messagesThread(threadId: string, cursor: string | null = null): Promise<ThreadPage> {
  if (!hasLiveBackend()) return previewPage(threadId);
  return invoke<ThreadPage>("messages_thread", { threadId, cursor });
}

/** You write to a member or the whole pool (Deliver: also into their
 * sessions run in AI Juggler, at their next turn boundary). */
export async function messagesSend(input: ComposeInput): Promise<ThreadPage> {
  if (!hasLiveBackend()) {
    const body = input.body.trim();
    if (!body) throw refused("A message needs a body.");
    record(`send:${input.to}:${input.deliver}`);
    const data = preview();
    let thread = input.thread_id ? data.threads.find((t) => t.id === input.thread_id) : undefined;
    const now = Date.now();
    if (!thread) {
      thread = { id: `thr-preview-${data.threads.length + 1}`, pool_id: input.pool_id, pool_name: "", subject: input.subject?.trim() || body.split("\n")[0]!.slice(0, 60), people: ["You"], count: 0, hop: 0, hop_cap: 6, state: "open", state_reason: null, updated_at_ms: now, last_from: "You", last_line: "", unread: 0 };
      data.threads.push(thread);
    }
    Object.assign(thread, { state: "open", state_reason: null, hop: 0, updated_at_ms: now, last_from: "You", last_line: body.split("\n")[0]!, count: thread.count + 1 });
    const message: MessageInfo = { id: `msg-preview-${now}`, thread_id: thread.id, from_name: "You", from_kind: "user", from_session_id: null, kind: input.kind ?? "note", subject: thread.subject, body, hop: 0, redacted: 0, to_all: input.to === "pool", at_ms: now, recipients: [] };
    (data.messages[thread.id] ??= []).push(message);
    changed();
    return previewPage(thread.id);
  }
  return invoke<ThreadPage>("messages_send", { input });
}

/** Your Deliver for one recipient of a message. */
export async function messagesDeliver(messageId: string, recipient: string): Promise<ThreadPage> {
  if (!hasLiveBackend()) {
    record(`deliver:${messageId}:${recipient}`);
    const data = preview();
    const threadId = Object.keys(data.messages).find((id) => data.messages[id]!.some((m) => m.id === messageId));
    const row = threadId && data.messages[threadId]!.find((m) => m.id === messageId)!.recipients.find((r) => r.id === recipient);
    if (!threadId || !row) throw refused("It wasn't sent to them");
    Object.assign(row, { state: "queued", deliverable: false, how: "Goes in at its next turn" });
    changed();
    return previewPage(threadId);
  }
  return invoke<ThreadPage>("messages_deliver", { messageId, recipient });
}

/** Your Deliver for everything waiting for a session run in AI Juggler. */
export async function messagesDeliverWaiting(sessionId: string): Promise<number> {
  if (!hasLiveBackend()) {
    record(`deliver-waiting:${sessionId}`);
    preview().needs = preview().needs.filter((n) => !(n.kind === "waiting" && n.session_id === sessionId));
    changed();
    return 1;
  }
  return invoke<number>("messages_deliver_waiting", { sessionId });
}

let focused: string | null = null;
/** The thread a pool's Messages tab opens on next (from Needs you). */
export function focusThread(threadId: string | null): void {
  focused = threadId;
  // E19: a Messages tab already open (a task's thread) takes it now.
  window.dispatchEvent(new Event(FOCUS_THREAD));
}
/** The event a Messages tab already open listens for. */
export const FOCUS_THREAD = "aij:focus-thread";
/** …taken once. */
export function takeFocusedThread(): string | null {
  const thread = focused;
  focused = null;
  return thread;
}

/** You pause, stop or resume (`open`) a thread. */
export async function messagesSetThread(threadId: string, to: "open" | "paused" | "stopped"): Promise<ThreadPage> {
  if (!hasLiveBackend()) {
    record(`thread:${threadId}:${to}`);
    const thread = preview().threads.find((t) => t.id === threadId);
    if (thread) Object.assign(thread, { state: to, state_reason: to === "open" ? null : `you ${to} it`, hop: to === "open" ? 0 : thread.hop });
    changed();
    return previewPage(threadId);
  }
  return invoke<ThreadPage>("messages_set_thread", { threadId, to });
}

/** You pause or resume a pool's messages. */
export async function messagesSetPool(poolId: string, paused: boolean): Promise<ThreadsPage> {
  if (!hasLiveBackend()) {
    record(`pool:${poolId}:${paused}`);
    const data = preview();
    data.paused = data.paused.filter((p) => p.pool_id !== poolId);
    if (paused) data.paused.push({ pool_id: poolId, pool_name: "", at_ms: Date.now(), reason: "you paused them" });
    data.needs = data.needs.filter((n) => !(n.kind === "pool_paused" && n.pool_id === poolId));
    changed();
    return messagesThreads({ poolId });
  }
  return invoke<ThreadsPage>("messages_set_pool", { poolId, paused });
}

/** What about messages waits on you (Needs you). */
export async function messagesNeeds(): Promise<MessageNeedInfo[]> {
  if (!hasLiveBackend()) return preview().needs.map((n) => ({ ...n }));
  return invoke<MessageNeedInfo[]>("messages_needs");
}

/** Your answer to a held ask: once, always (a rule in this pool) or deny. */
export async function messagesAnswer(askId: string, answer: "once" | "always" | "deny"): Promise<void> {
  if (!hasLiveBackend()) {
    record(`answer:${askId}:${answer}`);
    preview().needs = preview().needs.filter((n) => n.ask_id !== askId);
    changed();
    return;
  }
  return invoke<void>("messages_answer", { askId, answer });
}

/** One page of a pool's shared documents. */
export async function messagesDocs(poolId: string, cursor: string | null = null): Promise<DocsPage> {
  if (!hasLiveBackend()) {
    const docs = preview().docs.filter((d) => d.pool_id === poolId).sort((a, b) => b.at_ms - a.at_ms);
    const from = cursor ? Number(cursor.slice(1)) : 0;
    return { docs: docs.slice(from, from + 20), next: docs.length > from + 20 ? `d${from + 20}` : null };
  }
  return invoke<DocsPage>("messages_docs", { poolId, cursor });
}

/** One shared document's text. */
export async function messagesDoc(poolId: string, docId: string): Promise<DocContent> {
  if (!hasLiveBackend()) {
    const doc = preview().docs.find((d) => d.id === docId && d.pool_id === poolId);
    if (!doc) throw refused("That document isn't in this pool");
    return { doc, text: preview().docText[docId] ?? "" };
  }
  return invoke<DocContent>("messages_doc", { poolId, docId });
}

/** You're typing in a session's message box, or stopped (a delivery
 * waits while you type). */
export async function messagesTyping(sessionId: string, typing: boolean): Promise<void> {
  if (!hasLiveBackend()) {
    record(`typing:${sessionId}:${typing}`);
    return;
  }
  return invoke<void>("messages_typing", { sessionId, typing });
}

/** A pool's messages with every word of `query`, newest first. */
export async function messagesSearch(poolId: string, query: string): Promise<MessageInfo[]> {
  if (!hasLiveBackend()) {
    const words = query.toLowerCase().split(/\W+/).filter(Boolean);
    const data = preview();
    return data.threads
      .filter((t) => t.pool_id === poolId)
      .flatMap((t) => data.messages[t.id] ?? [])
      .filter((m) => words.length > 0 && words.every((w) => `${m.subject} ${m.body}`.toLowerCase().includes(w)))
      .reverse();
  }
  return invoke<MessageInfo[]>("messages_search", { poolId, query });
}

/** The pools linked with a pool (their members may message each other). */
export async function messagesLinks(poolId: string): Promise<string[]> {
  if (!hasLiveBackend()) return [...(preview().links[poolId] ?? [])];
  return invoke<string[]>("messages_links", { poolId });
}

/** You link two pools, or take the link back. */
export async function messagesLink(poolId: string, other: string, on: boolean): Promise<string[]> {
  if (!hasLiveBackend()) {
    record(`link:${poolId}:${other}:${on}`);
    const links = preview().links;
    const set = (a: string, b: string) => {
      const list = (links[a] ??= []).filter((x) => x !== b);
      links[a] = on ? [...list, b] : list;
    };
    set(poolId, other);
    set(other, poolId);
    return [...links[poolId]!];
  }
  return invoke<string[]>("messages_link", { poolId, other, on });
}

/** Calls `handler` when messages change (the backend's event, or the
 * preview's own writes). */
export function onMessagesChanged(handler: () => void): () => void {
  if (!hasLiveBackend()) {
    listeners.add(handler);
    return () => listeners.delete(handler);
  }
  let unlisten: (() => void) | null = null;
  let gone = false;
  void listen<string[]>(MESSAGES_CHANGED, () => handler()).then((off) => {
    if (gone) off();
    else unlisten = off;
  });
  return () => {
    gone = true;
    unlisten?.();
  };
}

/** What waits on you about messages, kept current. */
export function useMessageNeeds(): MessageNeedInfo[] {
  const [needs, setNeeds] = useState<MessageNeedInfo[]>([]);
  useEffect(() => {
    let alive = true;
    const read = () => void messagesNeeds().then((next) => alive && setNeeds(next), () => undefined);
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, []);
  return needs;
}
