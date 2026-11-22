/**
 * AJ-E3816 — the Watches screen in the shell: the page given this Mac's
 * targets (sessions, Apps, projects, pools, accounts), where Go to leads
 * for each kind, Bot's panel as the second way in (only when Bot has a
 * route, D-127), and Settings › Notifications for quiet hours. Its own chunk,
 * loaded when the page is first opened.
 */
import { useMemo, type JSX } from "react";
import { WatchesPage } from "../components/watches/WatchesPage";
import { isShown, useAiJob } from "../lib/ai-availability";
import { openAccount } from "../lib/accounts-page";
import { openBot } from "../lib/bot-store";
import { usePools } from "../lib/pools";
import { useApps, type AppModel } from "../lib/apps";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { WatchTarget } from "../types/generated/WatchTarget";

const NO_APPS: AppModel[] = [];

export function WatchesScreen({
  sessions,
  projects,
  accounts,
  now,
  onOpenSession,
  onOpenProject,
  onOpenPage,
  onOpenHome,
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
  onOpenPage: (page: { kind: "app" | "pool"; id: string }) => void;
  onOpenHome: () => void;
}): JSX.Element {
  const pools = usePools();
  // Apps by their App id, as the rail and the App pages have them.
  const apps = useApps(accounts) ?? NO_APPS;
  const bot = useAiJob("chat");
  // A project's own pool is the project itself: watch the project.
  const own = useMemo(() => (pools?.pools ?? []).filter((pool) => pool.kind !== "default"), [pools]);
  const sources = useMemo(() => ({ sessions, apps, projects, pools: own, accounts, now }), [sessions, apps, projects, own, accounts, now]);
  const goTo = (target: WatchTarget) => {
    if (target.kind === "session") onOpenSession(target.id);
    else if (target.kind === "project") onOpenProject(target.id);
    else if (target.kind === "app" || target.kind === "pool") onOpenPage({ kind: target.kind, id: target.id });
    else if (target.kind === "account") openAccount(target.id);
    else onOpenHome();
  };
  return (
    <WatchesPage
      sources={sources}
      onGoTo={goTo}
      onAskBot={isShown(bot) ? () => openBot() : null}
      onQuietHours={() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "notifications" } }))}
    />
  );
}
