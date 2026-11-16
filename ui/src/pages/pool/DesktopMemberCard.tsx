/**
 * AJ-E3013 (D-116, D-119): a session another app runs, named in a pool,
 * on the canvas's column. With AI Juggler's connector in its app it's a
 * member: its role and how a new message reaches it ("Wakes on new
 * messages", "Reads messages on its next turn"; a Codex app thread can't
 * be woken, and says so). Without one it's Watched, and says why ("Its
 * app has no AI Juggler connector") with Add, which opens it in the
 * inspector on the consent card. Its menu keeps Continue as CLI (a new
 * process here that resumes it, with the pool's tools); the card has no
 * Continue button of its own any more.
 */
import { useState, type JSX, type MouseEvent } from "react";

import { ACTION_ICONS } from "../../components/ActionIcon";
import { useActionHost } from "../../components/ItemMenu";
import { Button, Chip } from "../../components/primitives";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "../../components/RowMenu";
import { reachOf } from "../../lib/connector";
import { useConversations } from "../../lib/conversations";
import type { DesktopMemberInfo } from "../../lib/ipc/connector";
import type { WatchedMemberInfo } from "../../lib/ipc/pool-model";
import { CONTINUE_WATCHED, roleLabel } from "../../lib/pools";
import { continueWhy } from "../../lib/resume-refusal";
import { FOCUS_RING, SELECTED } from "./selection";

/** The member menu's Continue as CLI, and its line. */
export const CONTINUE_AS_CLI = "Continue as CLI";

export function DesktopMemberCard({ one, desktop, selected, onSelect }: { one: WatchedMemberInfo; desktop: DesktopMemberInfo | undefined; selected: boolean; onSelect: () => void }): JSX.Element {
  const host = useActionHost();
  const { all } = useConversations();
  const [menuAt, setMenuAt] = useState<MenuAt | null>(null);
  const conversation = all.find((row) => row.account_id === one.folder && row.id === one.conversation_id);
  const reach = reachOf(desktop);
  const title = one.title ?? "A session in another app";
  const watched = reach.kind === "watched";
  const blocked = conversation ? continueWhy(conversation) : null;
  const pick = (run: () => void) => () => {
    setMenuAt(null);
    run();
  };
  return (
    <div className={"pointer-events-auto flex flex-col gap-1 rounded-xl px-3 py-2 " + (selected ? SELECTED : "bg-surface shadow-card")} data-testid="canvas-watched-node" data-reach={reach.kind} data-rule-ref={one.member}>
      <div className="flex items-start gap-1">
        <button type="button" aria-pressed={selected} onClick={onSelect} className={"flex min-w-0 flex-1 flex-col gap-1 rounded-lg text-left " + FOCUS_RING}>
          <span className="flex items-center gap-1.5">
            {watched && <Chip tone="neutral">Watched</Chip>}
            <span className="text-label text-muted">{roleLabel(one.role)}</span>
          </span>
          <span className="line-clamp-2 text-body font-semibold text-ink">{title}</span>
          <span className="text-meta text-ink-2" data-testid="canvas-watched-reach">
            {[one.tool, reach.line].filter(Boolean).join(" · ")}
          </span>
        </button>
        {conversation && (
          <Button size="sm" variant="ghost" aria-label={`More for ${title}`} aria-haspopup="menu" aria-expanded={menuAt !== null} onClick={(event: MouseEvent<HTMLButtonElement>) => setMenuAt(menuPoint(event))} data-testid="canvas-watched-more">
            ⋯
          </Button>
        )}
      </div>
      {watched && reach.fix && (
        <Button size="sm" variant="accent" className="self-start" aria-label={`${reach.fix === "add" ? "Add" : "Fix"} AI Juggler's connector for ${title}`} onClick={onSelect} data-testid="canvas-watched-add">
          {reach.fix === "add" ? "Add" : "Fix"}
        </Button>
      )}
      {menuAt && conversation && (
        <RowMenu heading={title} ariaLabel={`More for ${title}`} at={menuAt} onClose={() => setMenuAt(null)}>
          {conversation.live && host.goToConversation && (
            <MenuItem onClick={pick(() => void host.goToConversation?.(conversation))} testId="canvas-watched-go-to">
              <MenuLine icon={ACTION_ICONS["go-to"]} label="Go to it" />
            </MenuItem>
          )}
          {host.openConversation && (
            <MenuItem onClick={pick(() => host.openConversation?.(conversation))} testId="canvas-watched-open">
              <MenuLine icon={ACTION_ICONS.page} label="Open its page" />
            </MenuItem>
          )}
          <MenuItem onClick={pick(() => void host.continueConversation?.(conversation))} disabled={blocked !== null || !host.continueConversation} title={blocked ?? undefined} testId="canvas-watched-continue">
            <MenuLine icon={ACTION_ICONS["carry-in"]} label={CONTINUE_AS_CLI} hint={blocked ?? CONTINUE_WATCHED} />
          </MenuItem>
        </RowMenu>
      )}
    </div>
  );
}
