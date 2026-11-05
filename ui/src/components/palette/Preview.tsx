/**
 * AJ-E2901: ⌘K's preview (→ on a session, app, project or account; ←
 * goes back): what it is now, where it runs, its account and when it was
 * last active, and its main actions. It reads only what is already
 * cached — never a conversation's text (that comes only through the
 * session page's last-messages reader, AJ-E2108).
 */
import type { JSX } from "react";
import { Button } from "../primitives";
import { agoWords } from "../../lib/time-words";
import { conversationItem, sessionItem, stateWord } from "../../lib/rail-items";
import { homeRelative } from "../../lib/project-words";
import { placeLine, rowTitle } from "./RowView";
import type { PaletteRow } from "./rows";

export type Previewable = Extract<PaletteRow, { where: "local" | "outside" | "app" | "project" | "account" }>;

export const canPreview = (entry: PaletteRow | undefined): entry is Previewable =>
  entry !== undefined && ["local", "outside", "app", "project", "account"].includes(entry.where);

export interface PreviewDoings {
  open: (entry: Previewable) => void;
  /** ⌘↩'s: its app, or New session on an account. */
  goTo: (entry: Previewable) => void;
  /** Continue on another account… (a session). */
  continueElsewhere?: (entry: Previewable) => void;
}

function facts(entry: Previewable, now: number): [string, string][] {
  switch (entry.where) {
    case "local": {
      const item = sessionItem(entry.row, () => null, false);
      return [
        ["Now", stateWord(item.state, item.probably)],
        ["Where", placeLine(entry) || "AI Juggler's terminal"],
        ["Account", entry.row.account_label],
        ["Last active", entry.row.last_activity_ms ? agoWords(entry.row.last_activity_ms, now) : "not yet"],
      ];
    }
    case "outside": {
      const item = conversationItem(entry.conversation, () => null, null);
      return [
        ["Now", stateWord(item.state, item.probably)],
        ["Where", placeLine(entry)],
        ["Account", item.accountLabel],
        ["Last active", entry.conversation.last_activity_ms ? agoWords(entry.conversation.last_activity_ms, now) : "not known"],
      ];
    }
    case "app":
      return [
        ["Now", entry.open ? "Open now" : "Not running"],
        ["App", entry.app.defaultName],
        ...(entry.app.folder ? ([["Signed in as", entry.app.folder.label]] as [string, string][]) : []),
      ].filter(([, value]) => value !== "") as [string, string][];
    case "project":
      return [
        ["Folder", homeRelative(entry.project.root)],
        ["Last active", entry.project.last_activity_ms ? agoWords(entry.project.last_activity_ms, now) : "not yet"],
      ];
    case "account":
      return [
        ["Tool", entry.account.tool],
        ["Room", entry.account.room],
        ...(entry.account.subscription.plan ? ([["Plan", entry.account.subscription.plan]] as [string, string][]) : []),
      ];
  }
}

/** Its main actions' words. */
function actionWords(entry: Previewable): { open: string; goTo: string | null; continue: string | null } {
  switch (entry.where) {
    case "local":
      return { open: "Open", goTo: entry.desktop ? "Go to its app" : null, continue: "Continue on another account…" };
    case "outside":
      return { open: "Open", goTo: "Go to its app", continue: "Continue on another account…" };
    case "app":
      return { open: "Open its page", goTo: entry.app.place ? (entry.open ? "Bring it forward" : "Start it") : null, continue: null };
    case "project":
      return { open: "Open", goTo: null, continue: null };
    case "account":
      return { open: "Open its page", goTo: entry.account.folder ? "New session on it…" : null, continue: null };
  }
}

export function Preview({ entry, id, doings, now = Date.now() }: { entry: Previewable; id: string; doings: PreviewDoings; now?: number }): JSX.Element {
  const words = actionWords(entry);
  return (
    <aside id={id} aria-label={`Preview of ${rowTitle(entry)}`} className="flex w-72 shrink-0 flex-col gap-3 border-l border-hairline p-4" data-testid="goto-preview">
      <h3 className="text-section font-semibold leading-snug text-ink">{rowTitle(entry)}</h3>
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-meta">
        {facts(entry, now).map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-muted">{label}</dt>
            <dd className="min-w-0 truncate text-ink" title={value}>
              {value}
            </dd>
          </div>
        ))}
      </dl>
      <div className="mt-auto flex flex-col gap-1.5">
        <Button size="sm" variant="primary" onClick={() => doings.open(entry)} data-testid="goto-preview-open">
          {words.open}
        </Button>
        {words.goTo && (
          <Button size="sm" variant="secondary" onClick={() => doings.goTo(entry)} data-testid="goto-preview-goto">
            {words.goTo}
          </Button>
        )}
        {words.continue && doings.continueElsewhere && (
          <Button size="sm" variant="ghost" onClick={() => doings.continueElsewhere!(entry)} data-testid="goto-preview-continue">
            {words.continue}
          </Button>
        )}
      </div>
      <p className="text-label text-muted">← back to the list</p>
    </aside>
  );
}
