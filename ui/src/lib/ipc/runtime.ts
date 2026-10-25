/**
 * AJ-E2302..E2307 (D-40) — a session's runtime: how it runs, its exact
 * state, its open asks, and the user's answers (ENGINEERING-GUIDE U3: one
 * IPC module per feature). Only the app's own window calls these.
 *
 * An answer is only ever sent from the user's click (D-28 as amended):
 * nothing here answers on a timer or by default.
 *
 * The preview serves views seeded by `__AIJ_RUNTIME__` (session id →
 * {@link RuntimeViewInfo}) and records calls in `__AIJ_RUNTIME_CALLS__`.
 */
import { useEffect, useState } from "react";
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { AnswerInput } from "../../types/generated/AnswerInput";
import type { AskInfo } from "../../types/generated/AskInfo";
import type { RuntimeChoicesInfo } from "../../types/generated/RuntimeChoicesInfo";
import type { RuntimeInfo } from "../../types/generated/RuntimeInfo";
import type { RuntimeViewInfo } from "../../types/generated/RuntimeViewInfo";
import type { HandoverInfo } from "../../types/generated/HandoverInfo";
import type { FolderTrustInfo } from "../../types/generated/FolderTrustInfo";

export type { AnswerInput, AskInfo, RuntimeChoicesInfo, RuntimeInfo, RuntimeViewInfo, HandoverInfo, FolderTrustInfo };

/** The event the backend sends when driven sessions change (their ids). */
export const RUNTIME_CHANGED = "runtime://changed";

interface Seams {
  __AIJ_RUNTIME__?: Record<string, RuntimeViewInfo>;
  __AIJ_RUNTIME_CALLS__?: string[];
  /** The folder-trust question the preview asks (none when unset). */
  __AIJ_TRUST__?: FolderTrustInfo;
}
const seams = () => window as unknown as Seams;
const record = (call: string) => (seams().__AIJ_RUNTIME_CALLS__ ??= []).push(call);
const previewViews = () => seams().__AIJ_RUNTIME__ ?? {};

/** A driven session's page; `null` for a session in a terminal. */
export async function runtimeGet(sessionId: string): Promise<RuntimeViewInfo | null> {
  if (!hasLiveBackend()) return previewViews()[sessionId] ?? null;
  return invoke<RuntimeViewInfo | null>("runtime_get", { sessionId });
}

/** The open asks across driven sessions, oldest first (one page). */
export async function runtimeAsks(offset = 0, limit = 50): Promise<AskInfo[]> {
  if (!hasLiveBackend()) {
    return Object.values(previewViews())
      .flatMap((view) => view.runtime.asks)
      .sort((a, b) => a.asked_at_ms - b.asked_at_ms)
      .slice(offset, offset + limit);
  }
  return invoke<AskInfo[]>("runtime_asks", { offset, limit });
}

/** The user's own answer to an ask: one of its options, or replies. */
export async function runtimeAnswer(sessionId: string, askId: string, answer: AnswerInput): Promise<void> {
  if (!hasLiveBackend()) {
    record(`answer:${sessionId}:${askId}:${answer.option ?? "reply"}`);
    const view = previewViews()[sessionId];
    if (view) view.runtime.asks = view.runtime.asks.filter((ask) => ask.id !== askId);
    return;
  }
  return invoke<void>("runtime_answer", { sessionId, askId, answer });
}

/** The user's message: a new turn, or into the running one where the tool takes that. */
export async function runtimeSend(sessionId: string, text: string): Promise<void> {
  if (!hasLiveBackend()) {
    record(`send:${sessionId}`);
    return;
  }
  return invoke<void>("runtime_send", { sessionId, text });
}

/** Interrupts the running turn. */
export async function runtimeInterrupt(sessionId: string): Promise<void> {
  if (!hasLiveBackend()) {
    record(`interrupt:${sessionId}`);
    return;
  }
  return invoke<void>("runtime_interrupt", { sessionId });
}

/** Hands the conversation over at a turn boundary: to the tool's own
 * terminal ("Open in terminal") or back to AI Juggler ("Run in AI Juggler"). */
export async function runtimeHandover(sessionId: string, toward: "terminal" | "driven"): Promise<HandoverInfo> {
  if (!hasLiveBackend()) {
    record(`handover:${sessionId}:${toward}`);
    return { session_id: sessionId };
  }
  return invoke<HandoverInfo>("runtime_handover", { sessionId, toward });
}

/** AJ-E2310: the user's Resume on a session whose turn was cut off when
 * AI Juggler stopped: its conversation is resumed, and nothing is sent. */
export async function runtimeResume(sessionId: string): Promise<HandoverInfo> {
  if (!hasLiveBackend()) {
    record(`resume:${sessionId}`);
    return { session_id: sessionId };
  }
  return invoke<HandoverInfo>("runtime_resume", { sessionId });
}

/** AJ-E2310: the user's End on a session whose turn was cut off. */
export async function runtimeEnd(sessionId: string): Promise<void> {
  if (!hasLiveBackend()) {
    record(`end:${sessionId}`);
    return;
  }
  return invoke<void>("runtime_end", { sessionId });
}

/** Lab 2: the folder-trust question a launch of these settings in `cwd`
 * needs (the tool's own words), with the answer so far; `null` when its
 * tool asks nothing. */
export async function runtimeTrust(profileId: string, cwd: string | null): Promise<FolderTrustInfo | null> {
  if (!hasLiveBackend()) return seams().__AIJ_TRUST__ ?? null;
  return invoke<FolderTrustInfo | null>("runtime_trust", { profileId, cwd });
}

/** The user's answer to it (their click), kept in AI Juggler and passed to
 * each launch in memory: the tool's own config is never written. */
export async function runtimeTrustChoose(profileId: string, cwd: string | null, trusted: boolean): Promise<FolderTrustInfo | null> {
  if (!hasLiveBackend()) {
    record(`trust:${profileId}:${trusted ? "trusted" : "untrusted"}`);
    const asked = seams().__AIJ_TRUST__;
    return asked ? { ...asked, choice: trusted ? "trusted" : "untrusted" } : null;
  }
  return invoke<FolderTrustInfo | null>("runtime_trust_choose", { profileId, cwd, trusted });
}

/** The profiles' runtime choices and the Beta switch. */
export async function runtimeChoices(): Promise<RuntimeChoicesInfo> {
  if (!hasLiveBackend()) return { beta_on: false, profiles: [] };
  return invoke<RuntimeChoicesInfo>("runtime_choices");
}

/** A profile's runtime: `terminal`, `driven`, or `null` for the tool's default. */
export async function runtimeChoose(profileId: string, runtime: "terminal" | "driven" | null): Promise<RuntimeChoicesInfo> {
  if (!hasLiveBackend()) {
    record(`choose:${profileId}:${runtime ?? "default"}`);
    return { beta_on: false, profiles: [] };
  }
  return invoke<RuntimeChoicesInfo>("runtime_choose", { profileId, runtime });
}

/** "Run without a terminal (Beta)" in Settings › Sessions. */
export async function runtimeBeta(on: boolean): Promise<RuntimeChoicesInfo> {
  if (!hasLiveBackend()) {
    record(`beta:${on}`);
    return { beta_on: on, profiles: [] };
  }
  return invoke<RuntimeChoicesInfo>("runtime_beta", { on });
}

/** Calls `handler` with the ids of driven sessions that changed. */
export function onRuntimeChanged(handler: (ids: string[]) => void): () => void {
  if (!hasLiveBackend()) return () => undefined;
  let unlisten: (() => void) | null = null;
  let gone = false;
  void listen<string[]>(RUNTIME_CHANGED, (event) => handler(event.payload)).then((off) => {
    if (gone) off();
    else unlisten = off;
  });
  return () => {
    gone = true;
    unlisten?.();
  };
}

/** A driven session's page, kept current; `null` while it loads or when
 * the session runs in a terminal. */
export function useRuntimeView(sessionId: string | null): RuntimeViewInfo | null {
  const [view, setView] = useState<RuntimeViewInfo | null>(null);
  useEffect(() => {
    if (!sessionId) return;
    let alive = true;
    const read = () => void runtimeGet(sessionId).then((next) => alive && setView(next), () => undefined);
    read();
    const off = onRuntimeChanged((ids) => {
      if (ids.includes(sessionId)) read();
    });
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  return sessionId ? view : null;
}

/** Every open ask across driven sessions, kept current. */
export function useOpenAsks(): AskInfo[] {
  const [asks, setAsks] = useState<AskInfo[]>([]);
  useEffect(() => {
    let alive = true;
    const read = () => void runtimeAsks().then((next) => alive && setAsks(next), () => undefined);
    read();
    const off = onRuntimeChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, []);
  return asks;
}
