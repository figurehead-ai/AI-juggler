/**
 * The New session sheet's state: whether it's open, and the folder and
 * account it opens on. `aij:new-session` opens it from anywhere; 0.3.4: its
 * `detail.accountId` names the account (a switch target's "Start a session
 * on it", an inactive CLI app's New session).
 */
import { useEffect, useState } from "react";

export interface NewSessionSheet {
  open: boolean;
  dir: string | null;
  account: string | null;
  show: (at?: { dir?: string | null; account?: string | null }) => void;
  close: () => void;
}

export function useNewSessionSheet(): NewSessionSheet {
  const [state, setState] = useState<{ open: boolean; dir: string | null; account: string | null }>({ open: false, dir: null, account: null });
  const show = (at: { dir?: string | null; account?: string | null } = {}) => setState({ open: true, dir: at.dir ?? null, account: at.account ?? null });
  useEffect(() => {
    const on = (event: Event) => {
      const accountId = (event as CustomEvent<{ accountId?: unknown } | null>).detail?.accountId;
      setState({ open: true, dir: null, account: typeof accountId === "string" ? accountId : null });
    };
    window.addEventListener("aij:new-session", on);
    return () => window.removeEventListener("aij:new-session", on);
  }, []);
  return { ...state, show, close: () => setState({ open: false, dir: null, account: null }) };
}
