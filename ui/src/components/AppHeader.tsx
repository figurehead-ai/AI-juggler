/**
 * The window's top bar (moved out of App.tsx, its size budget). 0.3 (the
 * redesign, §1): the main navigation first — Home · Projects · Pools ·
 * Tasks · Accounts · Usage · History — its lit tab the page's title (the
 * title itself is said once, for screen readers); then This Mac's line,
 * the cleanup pill, New session, the bell (AJ-E2502), the Floor (AJ-E3501)
 * and your menu (AJ-E2507). Settings, Processes and This Mac live in your menu; ⌘,
 * still opens Settings. While AI Juggler's own service doesn't answer,
 * New session says why it waits.
 */
import { RoomCount } from "./RoomChip";
import type { JSX } from "react";
import { Button } from "./primitives";
import { IconTerminal } from "./icons";
import { IconChevronRight } from "./icons-controls";
import { keysOf } from "../lib/shortcuts";
import { FloorButton } from "./FloorButton";
import { CleanupPill, MainNav } from "./MainNav";
import { MachineStatusLine } from "./MachineStatusLine";
import { NotificationBell } from "./NotificationBell";
import { UserMenu } from "./UserMenu";
import { PRIMARY_NAV, type GlobalId } from "../lib/nav";
import { useBackendHealth } from "../lib/backend-health";
import { CLEAN_UP_VIEW, openScreenOn } from "../lib/landing";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

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
  back,
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
  /** Walk A3: Back and Forward through the pages opened (⌘[ ⌘]). */
  back?: { canBack: boolean; canForward: boolean; onBack: () => void; onForward: () => void };
}): JSX.Element {
  const { downSince } = useBackendHealth();
  return (
    <header
      data-tauri-drag-region
      // AJ-E804 (WCAG 1.4.10): at narrow widths and 200 % zoom the right
      // cluster wraps onto a second row instead of being clipped; at 1024
      // and 1280 px with the list open it fits one row (0.3).
      className="@container/header relative z-30 flex min-h-14 shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline bg-surface/50 px-5 py-2 backdrop-blur-xl"
    >
      <h1 className="sr-only">{title}</h1>
      {back && <BackForward {...back} />}
      <MainNav primary={PRIMARY_NAV} global={global} page={page} pairedCount={pairedCount} onPick={onPick} />
      {/* From 960 px the rest always fits one row beside the tabs, so the
          cluster takes the room left and This Mac's line shortens (…)
          instead of the bar wrapping (0.3.2, a longer status word). */}
      <div className="ml-auto flex min-w-0 flex-wrap items-center justify-end gap-1 xl:gap-1.5 @min-[960px]/header:grow @min-[960px]/header:basis-0 @min-[960px]/header:flex-nowrap" data-tauri-drag-region={false}>
        {/* AJ-E2105: This Mac's line, in the top bar (it gives way first):
            by the bar's own width, not the window's, so the list open at
            1280 px with Fleet's tab too (a paired Mac) still fits one row
            (the 0.3 re-render found it wrapping). */}
        <span className={"mr-1 hidden min-w-0 text-muted " + (pairedCount > 0 ? "@min-[1060px]/header:flex" : "@min-[960px]/header:flex")} data-testid="header-this-mac">
          <MachineStatusLine minimal={false} short onOpen={onOpenThisMac} />
        </span>
        {/* D-87: accounts out of room or low, a calm count (the bell holds the notices). */}
        <RoomCount />
        {/* 0.4.2: "3 to clean up" lands on Needs attention (lib/landing). */}
        <CleanupPill count={cleanupCount} shown={global !== "cleanup"} onOpen={() => openScreenOn("cleanup", CLEAN_UP_VIEW)} />
        <Button size="sm" variant="accent" onClick={onNewSession} disabled={downSince !== null} title={downSince !== null ? "Waits until AI Juggler's service answers again" : undefined} className="ml-1 shrink-0">
          <IconTerminal width={14} height={14} />
          New session
        </Button>
        <span className="ml-1.5 flex shrink-0 items-center gap-2">
          <NotificationBell sessions={sessions} />
          <FloorButton />
          {/* A clear gap: the avatar is its own thing, never the Floor's. */}
          <span className="ml-2 flex">
            <UserMenu />
          </span>
        </span>
      </div>
    </header>
  );
}

/** ‹ › before the tabs, as in Finder and a browser: greyed with nowhere
 * to go, each saying its key. */
function BackForward({ canBack, canForward, onBack, onForward }: { canBack: boolean; canForward: boolean; onBack: () => void; onForward: () => void }): JSX.Element {
  const step = "inline-flex size-7 items-center justify-center rounded-md text-ink-2 outline-none transition-colors hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60 disabled:pointer-events-none disabled:opacity-35";
  return (
    <span className="-ml-1 flex shrink-0 items-center" data-tauri-drag-region={false} data-testid="header-back-forward">
      <button type="button" className={step} onClick={onBack} disabled={!canBack} aria-label="Back" title={`Back (${keysOf("back")[0]})`} data-testid="header-back">
        <IconChevronRight width={15} height={15} aria-hidden="true" className="rotate-180" />
      </button>
      <button type="button" className={step} onClick={onForward} disabled={!canForward} aria-label="Forward" title={`Forward (${keysOf("forward")[0]})`} data-testid="header-forward">
        <IconChevronRight width={15} height={15} aria-hidden="true" />
      </button>
    </span>
  );
}
