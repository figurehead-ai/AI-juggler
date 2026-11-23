/**
 * AJ-E1553 — the window's sign-ins, from the backend's list: a sign-in
 * that finishes leaves the rail by itself (its page still says "✓ signed
 * in" while open), one that ended early stays with its outcome, and the
 * ones kept from before a restart come back (stale ones were cleared).
 * A sign-in opened here before the backend lists it shows at once.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { onTerminalExit } from "../lib/ipc/terminal-io";
import { refreshSignIns, useSignIns } from "../lib/sign-ins-store";
import type { SignInState } from "../types/generated/SignInState";

/** A sign-in's terminal, in the rail until it finishes or is dismissed;
 * `account`: the App (account id) it signs in, so its words name that
 * App's tool; `state` as the backend says. */
export type LoginTerminal = { id: string; title: string; ended: boolean; account?: string; state?: SignInState };

export function useLoginTerminals(onSignedIn: () => void): {
  loginTerminals: LoginTerminal[];
  opened: (terminal: LoginTerminal) => void;
} {
  const signIns = useSignIns();
  // Opened here, before (or without) the backend's list.
  const [local, setLocal] = useState<LoginTerminal[]>([]);
  useEffect(
    () =>
      onTerminalExit((id) => setLocal((current) => current.map((t) => (t.id === id ? { ...t, ended: true } : t)))),
    [],
  );
  // Once the backend lists one, its list is the truth (a dismissed one
  // goes): adjusted as the list changes, while rendering.
  const [prunedFor, setPrunedFor] = useState(signIns);
  if (prunedFor !== signIns) {
    setPrunedFor(signIns);
    const kept = local.filter((one) => !signIns.some((listed) => listed.id === one.id));
    if (kept.length !== local.length) setLocal(kept);
  }
  const loginTerminals = useMemo(() => {
    const listed: LoginTerminal[] = signIns.map((one) => ({
      id: one.id,
      title: one.title,
      ended: one.state !== "running",
      account: one.account_id,
      state: one.state,
    }));
    const known = new Set(listed.map((one) => one.id));
    return [...listed, ...local.filter((one) => !known.has(one.id))];
  }, [signIns, local]);
  // R1: a sign-in that finishes flips its App to ready: the lists are read again.
  const seen = useRef(new Set<string>());
  useEffect(() => {
    let fresh = false;
    for (const one of signIns) {
      if (one.state === "signed_in" && !seen.current.has(one.id)) {
        seen.current.add(one.id);
        fresh = true;
      }
    }
    if (fresh) onSignedIn();
  }, [signIns, onSignedIn]);
  const opened = useCallback((terminal: LoginTerminal) => {
    setLocal((current) => (current.some((t) => t.id === terminal.id) ? current : [...current, terminal]));
    void refreshSignIns();
  }, []);
  return { loginTerminals, opened };
}
