/**
 * The window's top bar (moved out of App.tsx, its size budget): the
 * context's title and tool chip on the left; This Mac's line, the main
 * navigation (Home · Accounts · Usage · History, D-25), New session, the
 * bell (AJ-E2502) and your menu (AJ-E2507) on the right. Settings and the
 * old More now live in your menu; ⌘, still opens Settings. While AI
 * Juggler's own service doesn't answer, New session says why it waits.
 */
import type { JSX } from "react";
import { Button } from "./primitives";
import { IconTerminal } from "./icons";
import { MainNav } from "./MainNav";
import { MachineStatusLine } from "./MachineStatusLine";
import { NotificationBell } from "./NotificationBell";
import { ToolChip } from "./ToolMark";
import { UserMenu } from "./UserMenu";
import { PRIMARY_NAV, type GlobalId } from "../lib/nav";
import { useBackendHealth } from "../lib/backend-health";
import type { InstanceInfo } from "../lib/bridge";

export function AppHeader({
  title,
  tool,
  global,
  page,
  pairedCount,
  cleanupCount,
  sessions,
  onPick,
  onOpenTool,
  onOpenThisMac,
  onNewSession,
}: {
  title: string;
  /** The context's tool (its chip opens its details), when there is one. */
  tool: { id: string; name: string } | null;
  global: GlobalId | null;
  page: string;
  pairedCount: number;
  cleanupCount: number;
  sessions: InstanceInfo[];
  onPick: (id: GlobalId | "home") => void;
  onOpenTool: () => void;
  onOpenThisMac: () => void;
  onNewSession: () => void;
}): JSX.Element {
  const { downSince } = useBackendHealth();
  return (
    <header
      data-tauri-drag-region
      // AJ-E804 (WCAG 1.4.10): at narrow widths and 200 % zoom the
      // menus wrap onto a second row instead of being clipped. The
      // title has a zero basis, so it truncates first and a wide
      // window keeps the single row.
      className="relative z-30 flex min-h-14 shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline bg-surface/50 px-5 py-2 backdrop-blur-xl"
    >
      {/* Room for the title and its tool chip, so the nav beside it
          wraps rather than covering the chip at 1024 px (0.1.5). */}
      <div className="flex min-w-[12rem] flex-1 basis-0 items-center gap-3">
        <h1 className="min-w-0 truncate text-body font-semibold">{title}</h1>
        {/* The tool's own view: one click from a session that runs on it. */}
        {tool && (
          <button type="button" onClick={onOpenTool} aria-label={`About ${tool.name}`} title={`About ${tool.name}`} className="rounded-full outline-none focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="context-tool">
            <ToolChip tool={tool.id} tone="accent" />
          </button>
        )}
      </div>

      <div className="ml-auto flex min-w-0 max-w-full flex-wrap items-center justify-end gap-0.5 xl:gap-1.5" data-tauri-drag-region={false}>
        <span className="mr-2 flex min-w-0 text-muted">{/* AJ-E2105: This Mac's line, in the top bar */}<MachineStatusLine minimal={false} onOpen={onOpenThisMac} /></span>
        <MainNav primary={PRIMARY_NAV} global={global} page={page} pairedCount={pairedCount} cleanupCount={cleanupCount} onPick={onPick} />
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
