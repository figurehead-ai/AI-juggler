/**
 * D-136 — where Bot lives in the window: the docked panel beside the
 * page while it is open, loaded on first use (lazy-load-everywhere), and
 * its links' targets, by id, from the shell.
 */
import { useMemo, type JSX, type ReactNode } from "react";
import { lazyNamed } from "../../lib/lazy";
import { useBot } from "../../lib/bot-store";
import { useActionHost } from "../ItemMenu";
import { BotNavContext, type BotNav } from "./bot-nav";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const BotDockLazy = lazyNamed(() => import("./BotDock"), "BotDock");

export function BotNavProvider({
  sessions,
  openSession,
  openPool,
  openAccount,
  openHome,
  openUsage,
  children,
}: {
  sessions: readonly InstanceInfo[];
  openSession: (id: string) => void;
  openPool: (id: string) => void;
  openAccount: (id: string) => void;
  openHome: () => void;
  openUsage: () => void;
  children: ReactNode;
}): JSX.Element {
  const host = useActionHost();
  const nav = useMemo<BotNav>(
    () => ({
      openRow: (row) => {
        if (row.kind === "session") openSession(row.id);
        else if (row.kind === "task" && row.pool_id) openPool(row.pool_id);
        else if (row.kind === "account") openAccount(row.id);
      },
      goTo: host.focusSession
        ? (row) => {
            const session = sessions.find((s) => s.id === row.id);
            if (session) void host.focusSession?.(session);
          }
        : null,
      openShortcut: (shortcut) => (shortcut === "usage" ? openUsage() : openHome()),
      openSettings: (pane) => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: pane === "ai-models" ? "ai-models" : "ai-jobs" } })),
    }),
    [host, sessions, openSession, openPool, openAccount, openHome, openUsage],
  );
  return <BotNavContext.Provider value={nav}>{children}</BotNavContext.Provider>;
}

/** The docked panel, once Bot is open. */
export function BotDockHost(): JSX.Element | null {
  const bot = useBot();
  return bot.mode === "panel" ? <BotDockLazy /> : null;
}
