/**
 * The window's top bar (moved out of App.tsx, its size budget). 0.3 (the
 * redesign, §1): the main navigation first — Home · Projects · Pools ·
 * Tasks · Accounts · Usage · History — its lit tab the page's title (the
 * title itself is said once, for screen readers); then This Mac's line,
 * the cleanup pill, New session, the bell (AJ-E2502) and your menu
 * (AJ-E2507). Settings, Processes and This Mac live in your menu; ⌘,
 * still opens Settings. While AI Juggler's own service doesn't answer,
 * New session says why it waits.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { IconTerminal } from "./icons";
import { CleanupPill, MainNav } from "./MainNav";
import { MachineStatusLine } from "./MachineStatusLine";
import { NotificationBell } from "./NotificationBell";
import { UserMenu } from "./UserMenu";
import { PRIMARY_NAV, type GlobalId } from "../lib/nav";
import { useBackendHealth } from "../lib/backend-health";
import type { InstanceInfo } from "../lib/bridge";

export function AppHeader({
  title,
  global,
  page,
  pairedCount,
  cleanupCount,
  sessions,
  onPick,
  onOpenThisMac,
  onNewSession,
}: {
  /** The page's title: the lit tab says it on screen; this is for
   * screen readers (the window's one h1). */
  title: string;
  global: GlobalId | null;
  page: string;
  pairedCount: number;
  cleanupCount: number;
  sessions: InstanceInfo[];
  onPick: (id: GlobalId | "home") => void;
  onOpenThisMac: () => void;
  onNewSession: () => void;
}): JSX.Element {
  const { downSince } = useBackendHealth();
  return (
    <header
      data-tauri-drag-region
      // AJ-E804 (WCAG 1.4.10): at narrow widths and 200 % zoom the right
      // cluster wraps onto a second row instead of being clipped; at 1024
      // and 1280 px with the list open it fits one row (0.3).
      className="relative z-30 flex min-h-14 shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline bg-surface/50 px-5 py-2 backdrop-blur-xl"
    >
      <h1 className="sr-only">{title}</h1>
      <MainNav primary={PRIMARY_NAV} global={global} page={page} pairedCount={pairedCount} onPick={onPick} />
      <div className="ml-auto flex min-w-0 flex-wrap items-center justify-end gap-1 xl:gap-1.5" data-tauri-drag-region={false}>
        {/* AJ-E2105: This Mac's line, in the top bar (it gives way first). */}
        <span className="mr-1 hidden min-w-0 text-muted xl:flex">
          <MachineStatusLine minimal={false} onOpen={onOpenThisMac} />
        </span>
        <CleanupPill count={cleanupCount} shown={global !== "cleanup"} onOpen={() => onPick("cleanup")} />
        <Button size="sm" variant="primary" onClick={onNewSession} disabled={downSince !== null} title={downSince !== null ? "Waits until AI Juggler's service answers again" : undefined} className="ml-1">
          <IconTerminal width={14} height={14} />
          New session
        </Button>
        <span className="ml-1.5 flex items-center gap-2">
          <NotificationBell sessions={sessions} />
          <UserMenu />
        </span>
      </div>
    </header>
  );
}
