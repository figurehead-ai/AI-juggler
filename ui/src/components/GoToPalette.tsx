/**
 * ⌘K (and the go-to key from any app): search, act or go (AJ-E414,
 * AJ-E2106, AJ-E2901). A real modal dialog — named, Tab kept inside, Esc
 * closes, focus back where it was. With nothing typed: the selected
 * item's actions, Recent, the top actions, the sessions open now. Typed:
 * Selected · Actions · Sessions · Apps · Projects · Accounts · Settings ·
 * Search, loosely matched (`lib/palette-match`, the rail's search too),
 * each capped at 5. → previews a session, app, project or account beside
 * the list; ← goes back. See all results opens History searched for the
 * words. The rows are `palette/rows`, drawn by `palette/RowView`.
 */
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { Button } from "./primitives";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { RemoteMachineInfo } from "../types/generated/RemoteMachineInfo";
import { errorText } from "../lib/bridge";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../lib/hotkey";
import { useActionHost } from "./ItemMenu";
import { paletteSelection, usePaletteActions, type PaletteSelection } from "./PaletteActions";
import { appCommands, type AppDoings } from "../lib/palette-commands";
import { useConversations } from "../lib/conversations";
import { parseQuery } from "../lib/rail-view";
import { canFocusRemote, domId } from "./palette-helpers";
import { keepTabInside } from "./form";
import { openApp, useApps } from "../lib/apps";
import { placesOpen } from "../lib/ipc/places";
import { accountDisplay, accountName, useSubscriptions } from "../lib/subscriptions";
import { useQuotaDisplays } from "../lib/kept-reads";
import { runwayOf } from "./AccountRunway";
import { providerDisplayName } from "../lib/providers";
import { useCleanupCount } from "../lib/cleanup-count";
import { useNoticeSummary } from "../lib/notices-store";
import { useBackendHealth } from "../lib/backend-health";
import { keepRecents, loadRecents, rememberRecent, type Recent } from "../lib/palette-recents";
import { searchSettings } from "../screens/settings/settings-index";
import { PANES } from "../screens/settings/panes";
import { buildGroups, recentOf, typedAnything, type AccountRow, type PaletteRow } from "./palette/rows";
import { RowView, rowTitle } from "./palette/RowView";
import { canPreview, Preview, type Previewable } from "./palette/Preview";
export { paletteSelection } from "./PaletteActions";

/** Builds a Tauri accelerator string from a KeyboardEvent (lib/hotkey). */
export { toAccelerator };

const PANE_LABELS = Object.fromEntries(PANES.map((pane) => [pane.id, pane.label]));
const LISTBOX = "goto-listbox";
const PREVIEW = "goto-preview";

export function GoToPalette({
  open,
  onClose,
  sessions,
  onFocusWindow,
  onOpenSession,
  hotkey,
  onHotkeyChange,
  remoteMachines = [],
  onFocusRemote,
  onOpenRemote,
  selected = null,
  projects = [],
  accounts = [],
  onChanged = () => {},
  selectedConversation = null,
  app,
}: {
  open: boolean;
  onClose: () => void;
  sessions: InstanceInfo[];
  onFocusWindow: (session: InstanceInfo) => void;
  onOpenSession: (id: string) => void;
  hotkey: string;
  onHotkeyChange: (combo: string) => Promise<void>;
  /** AJ-E711: paired machines' live Instances (from cached Fleet state). */
  remoteMachines?: RemoteMachineInfo[];
  onFocusRemote?: (machineId: string, instanceId: string) => void;
  onOpenRemote?: (machineId: string, instanceId: string) => void;
  /** AJ-E2106: what is selected now; its actions come first. */
  selected?: PaletteSelection | null;
  projects?: ProjectInfo[];
  /** The folders (Apps' and Accounts' sign-ins), for Apps and New session on an Account. */
  accounts?: AccountInfo[];
  /** Something an action changed (a rename, a pin). */
  onChanged?: () => void;
  /** The page open on a session another app runs (its actions come first). */
  selectedConversation?: { accountId: string; id: string } | null;
  /** App's own actions (screens, New session, notifications…), F7. */
  app?: AppDoings;
}) {
  const [typed, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [previewing, setPreviewing] = useState(false);
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [recents, setRecents] = useState<Recent[]>([]);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const invoker = useRef<HTMLElement | null>(null);

  const { all: conversations } = useConversations();
  const host = useActionHost();
  const apps = useApps(accounts) ?? [];
  const subscriptions = useSubscriptions(0, open);
  const { displays } = useQuotaDisplays({ pollMs: 0, maxAgeMs: 10 * 60_000 });
  const leftovers = useCleanupCount();
  const unread = useNoticeSummary()?.unread ?? 0;
  const offline = useBackendHealth().downSince !== null;
  const query = parseQuery(typed);
  const needle = query.text;
  const conversation = selectedConversation ? (conversations.find((c) => c.account_id === selectedConversation.accountId && c.id === selectedConversation.id) ?? null) : null;
  const chosen = selected ?? paletteSelection(null, null, null, null, conversation);
  const panel = usePaletteActions(chosen, needle, onChanged);
  const commands = useMemo(() => (app ? appCommands(app) : []), [app]);
  const accountRows = useMemo<AccountRow[]>(
    () =>
      (subscriptions ?? []).map((subscription) => {
        const run = runwayOf(accountDisplay(subscription, displays));
        const folder = accounts.find((folder) => subscription.profiles.some((profile) => profile.id === folder.id) && folder.login_state === "logged_in") ?? null;
        return { key: subscription.key, accountId: subscription.account_id ?? null, name: accountName(subscription), tool: providerDisplayName(subscription.tool), room: run.words, folder, subscription };
      }),
    [subscriptions, displays, accounts],
  );
  const groups = useMemo(
    () =>
      buildGroups({
        typed, query, actions: panel.actions, selectedHeading: chosen?.heading ?? null, commands, app, sessions, conversations, remoteMachines, projects, apps, accounts: accountRows, recents, leftovers, unread,
        settings: needle ? searchSettings(needle, PANE_LABELS).filter((hit) => hit.pane !== "macs") : [],
      }),
    [typed, panel.actions, chosen?.heading, commands, app, sessions, conversations, remoteMachines, projects, apps, accountRows, recents, leftovers, unread],
  );
  const rows = useMemo(() => groups.flatMap((group) => group.rows), [groups]);
  const current = rows[active];

  // Open: the recents, focus in the field; closed: focus back where it was.
  useEffect(() => {
    if (!open) return;
    invoker.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setQuery("");
    setActive(0);
    setPreviewing(false);
    setCapturing(false);
    setCaptureError(null);
    setRecents(loadRecents());
    inputRef.current?.focus();
    return () => {
      const back = invoker.current;
      if (back && back.isConnected) back.focus();
    };
  }, [open]);

  useEffect(() => {
    setActive((at) => Math.min(at, Math.max(rows.length - 1, 0)));
  }, [rows.length]);
  // A row named by what's typed starts active, so ↩ opens it (typing a
  // project's name and pressing Enter opens it, the review of 0.1.4), even
  // when a looser match sits in an earlier group.
  useEffect(() => {
    const words = typed.trim().toLowerCase();
    const named = words ? rows.findIndex((row) => row.where !== "action" && rowTitle(row).toLowerCase().startsWith(words)) : -1;
    setActive(Math.max(named, 0));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [typed]);
  // The recents whose item is gone are dropped — judged only by lists
  // already read, so one still loading never loses its recents.
  useEffect(() => {
    if (!open) return;
    const known: Partial<Record<Recent["kind"], (id: string) => boolean>> = {};
    if (commands.length > 0) known.command = (id) => commands.some((command) => command.id === id);
    if (sessions.length > 0) known.session = (id) => sessions.some((session) => session.id === id);
    if (conversations.length > 0) known.conversation = (id) => conversations.some((entry) => `${entry.account_id}::${entry.id}` === id);
    if (apps.length > 0) known.app = (id) => apps.some((entry) => entry.id === id);
    if (projects.length > 0) known.project = (id) => projects.some((project) => project.id === id);
    if (subscriptions) known.account = (id) => subscriptions.some((subscription) => subscription.key === id);
    keepRecents((recent) => known[recent.kind]?.(recent.id) ?? true);
  }, [open, commands, sessions, conversations, apps, projects, subscriptions]);

  // A sheet an action opened (a rename) outlives the palette.
  if (!open) return <>{panel.element}</>;

  const preview = previewing && canPreview(current) ? current : null;

  /** ↩ opens its page here; ⌘↩ goes to its app (or New session on an account). */
  const choose = (entry: PaletteRow, toApp = false) => {
    if (entry.where === "action") {
      if (entry.action.reason) return; // greyed: it says why, and stays
      onClose();
      void entry.action.run();
      return;
    }
    const recent = recentOf(entry);
    if (recent) rememberRecent(recent);
    onClose();
    switch (entry.where) {
      case "command":
        entry.command.run();
        return;
      case "setting":
        window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: entry.hit.anchor } }));
        return;
      case "project":
        host.openProjectById?.(entry.project.id);
        return;
      case "outside":
        if (toApp && host.goToConversation) void host.goToConversation(entry.conversation);
        else host.openConversation?.(entry.conversation);
        return;
      case "local":
        if (toApp && entry.desktop) onFocusWindow(entry.row);
        else onOpenSession(entry.row.id);
        return;
      case "app":
        if (toApp && entry.app.place) void placesOpen(entry.app.place.id);
        else openApp(entry.app.id);
        return;
      case "account":
        if (toApp && entry.account.folder && host.startSessionOn) host.startSessionOn(entry.account.folder);
        else if (entry.account.accountId) host.openAccountById?.(entry.account.accountId);
        return;
      case "remote":
        if (toApp && entry.desktop && canFocusRemote(entry.machine)) onFocusRemote?.(entry.machine.machine_id, entry.row.id);
        else onOpenRemote?.(entry.machine.machine_id, entry.row.id);
    }
  };

  const capture = (event: KeyboardEvent) => {
    event.preventDefault();
    if (event.key === "Escape") return setCapturing(false);
    const combo = toAccelerator(event.nativeEvent);
    const refusal = combo === null ? null : hotkeyRefusal(combo);
    if (refusal !== null) return setCaptureError(refusal);
    if (combo === null) return;
    setCaptureError(null);
    onHotkeyChange(combo)
      .then(() => setCapturing(false))
      .catch((cause: unknown) => setCaptureError(errorText(cause)));
  };

  const onInputKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (capturing) return capture(event);
    const move = (by: number) => {
      event.preventDefault();
      setActive((at) => (rows.length === 0 ? 0 : (at + by + rows.length) % rows.length));
    };
    if (event.key === "ArrowDown") move(1);
    else if (event.key === "ArrowUp") move(-1);
    else if (event.key === "Enter") {
      event.preventDefault();
      if (current) choose(current, event.metaKey || event.ctrlKey);
    } else if (event.key === "ArrowRight" && canPreview(current) && event.currentTarget.selectionStart === typed.length) {
      event.preventDefault();
      setPreviewing(true);
    } else if (event.key === "ArrowLeft" && previewing) {
      event.preventDefault();
      setPreviewing(false);
    }
  };

  const onDialogKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (capturing) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (event.key === "ArrowLeft" && previewing && event.target !== inputRef.current) {
      event.preventDefault();
      setPreviewing(false);
      inputRef.current?.focus();
    }
    keepTabInside(dialogRef.current, event);
  };

  const typedAny = typedAnything(query);
  const onlySearch = typedAny && rows.every((row) => row.where === "command" && row.command.id === "search-history");
  let index = -1;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-24 backdrop-blur-sm" onClick={onClose} data-testid="goto-overlay">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label="Search, act or go"
        className={"sheen flex max-w-[92vw] flex-col overflow-hidden rounded-card border border-hairline bg-surface shadow-lg " + (preview ? "w-[920px]" : "w-[640px]")}
        data-testid="goto-palette"
        onClick={(event: MouseEvent) => event.stopPropagation()}
        onKeyDown={onDialogKey}
      >
        <div className="border-b border-hairline p-3">
          <input
            ref={inputRef}
            role="combobox"
            aria-expanded="true"
            aria-controls={preview ? `${LISTBOX} ${PREVIEW}` : LISTBOX}
            aria-activedescendant={current ? `goto-option-${domId(current.id)}` : undefined}
            aria-label={chosen ? `Actions for ${chosen.heading}, or search, act or go` : "Search, act or go"}
            value={typed}
            placeholder={capturing ? "Press the new combo (Esc cancels)…" : chosen ? `Act on ${chosen.heading}, or search…` : "Search sessions, apps, projects and accounts, or type what to do…"}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onInputKey}
            className="w-full bg-transparent text-meta text-ink outline-none placeholder:text-muted"
          />
        </div>
        {offline && (
          <p className="border-b border-hairline bg-warning-soft px-3 py-1.5 text-label text-ink" role="status" data-testid="goto-offline">
            AI Juggler's service isn't answering — showing what was open last.
          </p>
        )}
        <div className="flex min-h-0">
          <ul id={LISTBOX} role="listbox" aria-label="Results" className="max-h-96 min-w-0 flex-1 overflow-y-auto p-1.5">
            {onlySearch && (
              <li role="presentation" className="px-2.5 py-2 text-meta text-muted" data-testid="goto-no-matches">
                No matches for “{typed.trim()}”.
              </li>
            )}
            {rows.length === 0 && (
              <li role="presentation" className="px-2.5 py-3 text-meta text-muted">
                Nothing open yet — start one with New session.
              </li>
            )}
            {groups.map((group) => (
              <li key={group.key} role="presentation">
                <div id={`goto-group-${group.key}`} aria-hidden="true" className="flex items-baseline gap-1.5 px-2.5 pb-1 pt-2 text-label font-semibold text-muted">
                  {group.label}
                  {group.more > 0 && <span className="font-normal" data-testid={`goto-more-${group.key}`}>· {group.more} more</span>}
                </div>
                <ul role="group" aria-label={group.label} data-testid={`goto-group-${group.key}`}>
                  {group.rows.map((entry) => {
                    index += 1;
                    const at = index;
                    return (
                        <li
                          key={entry.id}
                          id={`goto-option-${domId(entry.id)}`}
                          role="option"
                          aria-selected={at === active}
                          aria-disabled={entry.where === "action" && entry.action.reason ? true : undefined}
                          data-testid={`goto-row-${entry.where === "remote" ? `remote-${entry.row.id}` : entry.id}`}
                          onClick={(event) => choose(entry, event.metaKey || event.ctrlKey)}
                          onMouseEnter={() => setActive(at)}
                          className={"flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-left text-meta transition-colors " + (at === active ? "bg-accent-soft text-accent-hi" : "text-ink hover:bg-surface-2")}
                        >
                          <RowView entry={entry} />
                        </li>
                    );
                  })}
                </ul>
              </li>
            ))}
          </ul>
          {preview && (
            <Preview
              entry={preview}
              id={PREVIEW}
              doings={{
                open: (entry: Previewable) => choose(entry),
                goTo: (entry: Previewable) => choose(entry, true),
                continueElsewhere: (entry: Previewable) => {
                  onClose();
                  if (entry.where === "local") host.continueSession?.(entry.row);
                  else if (entry.where === "outside") host.continueConversationElsewhere?.(entry.conversation);
                },
              }}
            />
          )}
        </div>
        <div className="flex flex-col gap-1 border-t border-hairline px-3 py-2 text-label text-muted">
          <span className="truncate" data-testid="goto-keys">↑↓ move · ↩ open · ⌘↩ go to its app · → preview · Esc close</span>
          <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
            {app ? (
              <Button size="sm" variant="ghost" className="-ml-2" onClick={() => (onClose(), app.searchHistory(needle))} data-testid="goto-see-all">
                See all results →
              </Button>
            ) : (
              <span />
            )}
            <Button size="sm" variant="ghost" className="-mr-2" data-testid="goto-change-hotkey" onClick={() => setCapturing(true)}>
              {capturing ? "press a combo…" : hotkey ? `From any app: ${hotkeyWords(hotkey)} — change…` : "Choose a key to open this from any app…"}
            </Button>
          </div>
          {captureError !== null && (
            <span className="text-danger" data-testid="goto-hotkey-error">
              {captureError}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
