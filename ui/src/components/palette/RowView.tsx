/**
 * AJ-E2901: what one ⌘K option shows — its mark, its title with the
 * matched letters bold, a line of where or what, and at the end what
 * ↩ does, on the active row only ("open" on every row was noise, and read
 * beside "open now"; the 0.4 design review, 52 and 72). The option itself
 * is the target (the combobox moves through them), so nothing inside is a
 * button.
 */
import type { JSX } from "react";
import { highlight, type Range } from "../../lib/palette-match";
import { homeRelative } from "../../lib/project-words";
import { appOfConversation, appOfSession, type AppRef } from "../../lib/app-of";
import { conversationItem, placeWords, sessionItem } from "../../lib/rail-items";
import { conversationTitle } from "../../lib/conversations";
import { providerDisplayName } from "../../lib/providers";
import { AppIcon } from "../AppIcon";
import { ProjectMark } from "../ProjectMark";
import { PlaceMark } from "../PlaceMarks";
import { AppKindBadge } from "../AppKindBadge";
import { appKindWords } from "../../lib/app-kind";
import { ToolMark } from "../ToolMark";
import { sessionTitle } from "../ManageWork";
import { ActionOption, CommandOption } from "../PaletteActions";
import { IconGear } from "../icons-menu";
import type { PaletteRow } from "./rows";

/** `text` with `ranges` bold. */
export function Highlighted({ text, ranges }: { text: string; ranges: readonly Range[] }): JSX.Element {
  if (ranges.length === 0) return <>{text}</>;
  return (
    <>
      {highlight(text, ranges).map((part, index) =>
        part.hit ? (
          <b key={index} className="font-semibold text-ink" data-hit="">
            {part.text}
          </b>
        ) : (
          <span key={index}>{part.text}</span>
        ),
      )}
    </>
  );
}

/** D-38: the app a session row shows: its own App's, never AI Juggler's
 * mark (the 0.4 design review, 72). */
export function rowApp(entry: Extract<PaletteRow, { where: "local" | "outside" | "remote" }>): AppRef {
  if (entry.where === "outside") return appOfConversation(entry.conversation);
  return appOfSession(entry.row);
}

/** D-62: a session's App first, then where its window is, as the rail says it. */
export function placeLine(entry: Extract<PaletteRow, { where: "local" | "outside" }>): string {
  const item = entry.where === "outside" ? conversationItem(entry.conversation, () => null, null) : sessionItem(entry.row, () => null, false);
  return placeWords(item.place).join(" · ");
}

/** A row's title, as the preview and the option name it. */
export function rowTitle(entry: PaletteRow): string {
  switch (entry.where) {
    case "action":
      return entry.action.label;
    case "command":
      return entry.command.label;
    case "project":
      return entry.project.name;
    case "local":
      return sessionTitle(entry.row);
    case "outside":
      return conversationTitle(entry.conversation);
    case "remote":
      return entry.row.account_label;
    case "app":
      return entry.app.name;
    case "account":
      return entry.account.name;
    case "setting":
      return `${entry.hit.paneLabel} › ${entry.hit.title}`;
  }
}

const Muted = ({ children, testId }: { children: string; testId?: string }) => (
  <span className="min-w-0 shrink truncate text-muted" data-testid={testId}>
    {children}
  </span>
);
/** What ↩ does, shown on the active row only (`aria-selected`). */
const End = ({ children }: { children: string }) => (
  <span className="ml-auto hidden shrink-0 pl-2 text-muted in-aria-selected:inline" data-testid="goto-row-hint">
    ↩ {children}
  </span>
);

export function RowView({ entry }: { entry: PaletteRow }): JSX.Element {
  switch (entry.where) {
    case "action":
      return <ActionOption action={entry.action} />;
    case "command":
      return <CommandOption command={entry.command} />;
    case "setting":
      return (
        <>
          <IconGear width={14} height={14} aria-hidden="true" focusable="false" className="shrink-0 text-muted" />
          <span className="min-w-0 shrink truncate">{entry.hit.title}</span>
          <Muted>{entry.hit.paneLabel}</Muted>
          <End>Open</End>
        </>
      );
    case "project":
      return (
        <>
          <ProjectMark projectId={entry.project.id} size={16} />
          <span className="min-w-0 shrink truncate">
            <Highlighted text={entry.project.name} ranges={entry.ranges} />
          </span>
          <Muted>{homeRelative(entry.project.root)}</Muted>
          <End>Open</End>
        </>
      );
    case "app": {
      const app = entry.app;
      return (
        <>
          <PlaceMark place={{ key: app.railKey, app: app.app, appId: app.appId, kind: app.kind === "desktop" ? "desktop" : null, profile: app.profile, appPid: app.place?.app_pid ?? null, host: null }} size={16} ring="var(--color-surface)" />
          {/* D-100: Desktop or CLI, by icon beside its icon and by word. */}
          <AppKindBadge kind={app.kind} decorative />
          <span className="min-w-0 shrink truncate">
            <Highlighted text={app.name} ranges={entry.ranges} />
          </span>
          <Muted testId="goto-app-state">{`${appKindWords(app.kind).word} · ${entry.open ? "open now" : "not running"}`}</Muted>
          <End>Open</End>
        </>
      );
    }
    case "account":
      return (
        <>
          <ToolMark tool={entry.account.subscription.tool} size={16} className="shrink-0 text-muted" />
          <span className="min-w-0 shrink truncate">
            <Highlighted text={entry.account.name} ranges={entry.ranges} />
          </span>
          <Muted testId="goto-account-room">{`${entry.account.tool} · ${entry.account.room}`}</Muted>
          <End>Open</End>
        </>
      );
    default:
      return (
        <>
          <AppIcon {...rowApp(entry)} size={16} />
          {entry.where === "remote" ? (
            <>
              <span className="min-w-0 shrink truncate">{entry.row.account_label}</span>
              <Muted>{providerDisplayName(entry.row.provider)}</Muted>
              <span className="shrink-0 text-muted" data-testid="goto-remote-machine">
                on {entry.machine.label} · {entry.machine.freshness === "live" ? "● live" : entry.machine.freshness === "stale" ? "◐ stale" : "○ offline"}
              </span>
            </>
          ) : (
            <>
              <span className="min-w-0 shrink truncate">
                <Highlighted text={rowTitle(entry)} ranges={entry.ranges} />
              </span>
              <Muted>{placeLine(entry)}</Muted>
            </>
          )}
          <End>{entry.where === "remote" ? "Inspect" : "Open"}</End>
        </>
      );
  }
}
