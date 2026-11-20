/**
 * AJ-E3816 — one watch on the list: its mark, its echo (the code's words,
 * names in bold), its target in plain words and the echo's other
 * sentences as chips, where it stands, and Edit, Pause and ⋯. The ⋯ menu
 * (and a right-click) carries Edit, Pause or Resume, the watch's own
 * "Even during quiet hours" switch, Go to, Watch something similar and
 * Remove. Every action names the watch by its id.
 */
import { useState, type JSX, type MouseEvent } from "react";
import { MenuItem, MenuSeparator, RowMenu, menuPoint, type MenuAt } from "../RowMenu";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import { EchoHead, StateCell, WatchChip, WatchMarks } from "./WatchParts";
import { lifetimeChip, optionOf, rowState, targetLine, type TargetOption } from "./watches-view";
import { MENU, ROW } from "./watches-words";
import { IconArrowR, IconBin, IconCopyTwo, IconDots, IconMoon, IconPause, IconPencil, IconPlay, LIFETIME_ICONS, type WatchIcon } from "./watch-icons";

export interface WatchRowActions {
  edit: (info: WatchInfo) => void;
  pause: (info: WatchInfo, paused: boolean) => void;
  quiet: (info: WatchInfo, on: boolean) => void;
  goTo: (info: WatchInfo) => void;
  similar: (info: WatchInfo) => void;
  remove: (info: WatchInfo) => void;
}

const GHOST = "inline-flex h-7 items-center gap-1.5 rounded-lg px-2 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60";
const ICON_BUTTON = "inline-flex size-7 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60";

function MenuRow({ icon: Icon, label, hint, danger = false }: { icon: WatchIcon; label: string; hint?: string; danger?: boolean }): JSX.Element {
  return (
    <span className="flex items-center gap-2.5 py-0.5">
      <Icon width={15} height={15} aria-hidden="true" className={"shrink-0 " + (danger ? "text-danger" : "text-muted")} />
      <span className="flex min-w-0 flex-1 flex-col">
        <span data-label className="text-meta">
          {label}
        </span>
        {hint && <span className="text-label text-muted">{hint}</span>}
      </span>
    </span>
  );
}

/** The switch drawn inside a menu line (the line is the control). */
function MenuSwitch({ on }: { on: boolean }): JSX.Element {
  return (
    <span aria-hidden="true" className={"relative h-5 w-[34px] shrink-0 rounded-full " + (on ? "bg-accent-fill" : "bg-surface-3 inset-ring-1 inset-ring-hairline")}>
      <span className={"absolute top-[3px] size-3.5 rounded-full " + (on ? "left-[17px] bg-white" : "left-[3px] bg-faint")} />
    </span>
  );
}

function WatchMenu({ info, at, onClose, actions, targetName, quietTo }: { info: WatchInfo; at: MenuAt; onClose: () => void; actions: WatchRowActions; targetName: string; quietTo: string | null }): JSX.Element {
  const run = (act: () => void) => () => {
    onClose();
    act();
  };
  const paused = info.state === "paused";
  return (
    <RowMenu heading={MENU.heading} ariaLabel={`${MENU.heading}: ${info.words}`} at={at} onClose={onClose} wide testId="watch-menu">
      {paused ? (
        <MenuItem onClick={run(() => actions.pause(info, false))} testId="watch-menu-resume">
          <MenuRow icon={IconPlay} label={MENU.resume} hint={MENU.resumeHint} />
        </MenuItem>
      ) : (
        <MenuItem onClick={run(() => actions.edit(info))} testId="watch-menu-edit">
          <MenuRow icon={IconPencil} label={MENU.edit} />
        </MenuItem>
      )}
      {paused ? (
        <MenuItem onClick={run(() => actions.edit(info))} testId="watch-menu-edit">
          <MenuRow icon={IconPencil} label={MENU.edit} />
        </MenuItem>
      ) : (
        <MenuItem onClick={run(() => actions.pause(info, true))} testId="watch-menu-pause">
          <MenuRow icon={IconPause} label={MENU.pause} hint={MENU.pauseHint} />
        </MenuItem>
      )}
      {/* Serious choices: asked only when it changes what happens (the
          user has quiet hours). */}
      {quietTo !== null && (
        <MenuItem checked={info.watch.evenInQuietHours} multi onClick={run(() => actions.quiet(info, !info.watch.evenInQuietHours))} testId="watch-menu-quiet">
          <span className="flex items-center gap-2.5">
            <span className="min-w-0 flex-1">
              <MenuRow icon={IconMoon} label={MENU.quiet} hint={info.watch.evenInQuietHours ? MENU.quietOn : MENU.quietOff(quietTo)} />
            </span>
            <MenuSwitch on={info.watch.evenInQuietHours} />
          </span>
        </MenuItem>
      )}
      <MenuSeparator />
      <MenuItem onClick={run(() => actions.goTo(info))} testId="watch-menu-goto">
        <MenuRow icon={IconArrowR} label={MENU.goTo(targetName)} />
      </MenuItem>
      <MenuItem onClick={run(() => actions.similar(info))} testId="watch-menu-similar">
        <MenuRow icon={IconCopyTwo} label={MENU.similar} />
      </MenuItem>
      <MenuSeparator />
      <MenuItem danger onClick={run(() => actions.remove(info))} testId="watch-menu-remove">
        <MenuRow icon={IconBin} label={MENU.remove} hint={MENU.removeHint} danger />
      </MenuItem>
    </RowMenu>
  );
}

/** The name "Go to" uses: the one target's, quoted, else the first's. */
function goToName(info: WatchInfo, book: Map<string, TargetOption>): string {
  const first = info.echo.head.find((span) => span.name)?.text;
  if (first) return first;
  return optionOf(book, info.watch.targets[0]!).name;
}

export function WatchRow({ info, book, now, compact, actions, quietTo }: { info: WatchInfo; book: Map<string, TargetOption>; now: number; compact: boolean; actions: WatchRowActions; quietTo: string | null }): JSX.Element {
  const [menuAt, setMenuAt] = useState<MenuAt | null>(null);
  const paused = info.state === "paused";
  const state = rowState(info, now);
  const options = info.watch.targets.map((target) => optionOf(book, target));
  const LifeIcon = LIFETIME_ICONS[info.watch.lifetime.kind];
  const open = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    setMenuAt(menuPoint(event));
  };
  return (
    <li
      className={
        "relative grid items-center gap-x-3.5 rounded-[10px] px-3 [&+&]:rounded-none [&+&]:border-t [&+&]:border-divider " +
        (compact ? "grid-cols-[32px_minmax(0,1fr)_minmax(150px,200px)_auto] py-[9px]" : "grid-cols-[44px_minmax(0,1fr)_minmax(150px,200px)_auto] py-3") +
        (menuAt ? " bg-selected" : "")
      }
      data-testid="watch-row"
      data-watch-id={info.watch.id}
      data-state={info.state}
    >
      <WatchMarks options={options} compact={compact} dim={paused} />
      <div className="min-w-0">
        <p className={"text-body " + (paused ? "text-muted" : "text-ink-2")} data-testid="watch-echo">
          <EchoHead spans={info.echo.head} tone={paused ? "quiet" : "live"} />
        </p>
        <p className={"flex flex-wrap items-center gap-x-2.5 gap-y-1.5 text-meta text-muted " + (compact ? "mt-px" : "mt-1")}>
          <span data-testid="watch-target-line">{targetLine(info.watch, book)}</span>
          <WatchChip icon={LifeIcon} testId="watch-lifetime">
            {lifetimeChip(info)}
          </WatchChip>
          {info.echo.quiet && (
            <WatchChip icon={IconMoon} accent testId="watch-quiet">
              {info.echo.quiet.replace(/\.$/, "")}
            </WatchChip>
          )}
        </p>
      </div>
      <StateCell {...state} />
      <div className="flex items-center justify-end gap-0.5">
        {paused ? (
          <button type="button" className={GHOST} onClick={() => actions.pause(info, false)} data-testid="watch-resume">
            <IconPlay width={14} height={14} aria-hidden="true" />
            {ROW.resume}
            <span className="sr-only">: {info.words}</span>
          </button>
        ) : (
          <>
            <button type="button" className={GHOST} onClick={() => actions.edit(info)} data-testid="watch-edit">
              <IconPencil width={14} height={14} aria-hidden="true" />
              {ROW.edit}
              <span className="sr-only">: {info.words}</span>
            </button>
            <button type="button" className={ICON_BUTTON} onClick={() => actions.pause(info, true)} aria-label={`${ROW.pause}: ${info.words}`} title={ROW.pause} data-testid="watch-pause">
              <IconPause width={14} height={14} aria-hidden="true" />
            </button>
          </>
        )}
        <button type="button" className={ICON_BUTTON} aria-label={ROW.more(info.words)} aria-haspopup="menu" aria-expanded={menuAt !== null} onClick={open} data-testid="watch-more">
          <IconDots width={16} height={16} aria-hidden="true" />
        </button>
      </div>
      {menuAt && <WatchMenu info={info} at={menuAt} onClose={() => setMenuAt(null)} actions={actions} targetName={goToName(info, book)} quietTo={quietTo} />}
    </li>
  );
}
