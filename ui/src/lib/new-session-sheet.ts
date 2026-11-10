/**
 * The New session sheet's state: whether it's open, and the folder and
 * account it opens on. `aij:new-session` opens it from anywhere; 0.3.4: its
 * `detail.accountId` names the account (a switch target's "Start a session
 * on it", an inactive CLI app's New session). D-96 F: opened from "Start
 * another", it also holds the starts just before (`after`).
 */
import { useLayoutEffect, useState } from "react";

/** The starts just before this one, in one run of Start another. */
export interface StartedBefore {
  /** Their folders: never offered again in this run. */
  roots: string[];
  /** The last one's folder name ("as you chose for billing"). */
  name: string;
  /** The App the user chose for it, kept for this one; `null`: none. */
  app: string | null;
}

interface SheetState {
  open: boolean;
  dir: string | null;
  account: string | null;
  after: StartedBefore | null;
}

export interface NewSessionSheet extends SheetState {
  show: (at?: { dir?: string | null; account?: string | null; after?: StartedBefore | null }) => void;
  close: () => void;
}

const CLOSED: SheetState = { open: false, dir: null, account: null, after: null };

export function useNewSessionSheet(): NewSessionSheet {
  const [state, setState] = useState<SheetState>(CLOSED);
  const show = (at: { dir?: string | null; account?: string | null; after?: StartedBefore | null } = {}) =>
    setState({ open: true, dir: at.dir ?? null, account: at.account ?? null, after: at.after ?? null });
  useLayoutEffect(() => {
    const on = (event: Event) => {
      const accountId = (event as CustomEvent<{ accountId?: unknown } | null>).detail?.accountId;
      setState({ open: true, dir: null, account: typeof accountId === "string" ? accountId : null, after: null });
    };
    window.addEventListener("aij:new-session", on);
    return () => window.removeEventListener("aij:new-session", on);
  }, []);
  return { ...state, show, close: () => setState(CLOSED) };
}
