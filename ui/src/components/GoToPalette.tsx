import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { Button, Card } from "./primitives";
import type { InstanceInfo, ProjectInfo, RemoteInstanceInfo, RemoteMachineInfo } from "../lib/bridge";
import { errorText } from "../lib/bridge";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../lib/hotkey";
import { ProjectMark } from "./ProjectMark";
import { homeRelative } from "../lib/project-words";
import { AppIcon, OWN_APP_ID } from "./AppIcon";
import { appOfConversation, appOfSession, type AppRef } from "../lib/app-of";
import { OWN_APP } from "../lib/rail-items";
import { useActionHost } from "./ItemMenu";
import { ActionOption, CommandOption, paletteSelection, usePaletteActions, type PaletteSelection } from "./PaletteActions";
import { appCommands, commandsFor, type AppDoings, type PaletteCommand } from "../lib/palette-commands";
export { paletteSelection } from "./PaletteActions";
import { conversationTitle, useConversations } from "../lib/conversations";
import { sessionTitle } from "./ManageWork";
import { providerDisplayName } from "../lib/providers";
import { paletteMatches, parseQuery } from "../lib/rail-view";
import type { ResolvedAction } from "../lib/actions";
import type { ConversationInfo } from "../lib/ipc/conversations";

/**
 * "Go to instance…" (AJ-E414 Req.4): summoned by the global hotkey
 * (app://go-to) or ⌘K, listing every LIVE instance — desktop entries
 * focus their window (the tiered service), terminal entries select
 * their panel. Combobox pattern (aria-expanded/activedescendant),
 * full keyboard traversal; the footer names the binding and offers
 * the rebind capture. deep/11 §4.17 (the 0.2 walk, F7): actions come
 * first — the selected item's (a session another app runs too), then
 * App's own (screens, New session…, notifications; `lib/palette-commands`)
 * — filtered by what's typed, then sessions and projects, and a search
 * of History for the words, last.
 */

const isLive = (status: string) => status === "running" || status === "starting";

/** Builds a Tauri accelerator string from a KeyboardEvent (lib/hotkey,
 * shared with Settings › Desktop apps). */
export { toAccelerator };

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
  /** Projects to go to, listed once something is typed. */
  projects?: ProjectInfo[];
  /** Something an action changed (a rename, a pin). */
  onChanged?: () => void;
  /** The page open on a session another app runs (its actions come first). */
  selectedConversation?: { accountId: string; id: string } | null;
  /** App's own actions (screens, New session, notifications…), F7. */
  app?: AppDoings;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // AJ-E1529: sessions started outside AI Juggler, running now, too.
  const { all: conversations } = useConversations();
  const host = useActionHost();
  const needle = parseQuery(query).text;
  const conversation = selectedConversation ? (conversations.find((c) => c.account_id === selectedConversation.accountId && c.id === selectedConversation.id) ?? null) : null;
  const chosen = selected ?? paletteSelection(null, null, null, null, conversation);
  // AJ-E2106: the selected item's actions first.
  const panel = usePaletteActions(chosen, needle, onChanged);
  const actions = panel.actions;
  // F7: App's own actions, by what's typed.
  const commands = useMemo(() => (app ? appCommands(app) : []), [app]);
  const rows = useMemo<PaletteRow[]>(() => {
    const doing: PaletteRow[] = actions.map((action) => ({ where: "action", action, id: `action::${action.id}`, desktop: false }));
    const shown = app ? commandsFor(commands, needle, app) : [];
    const search = shown.filter((command) => command.id === "search-history");
    const commanding: PaletteRow[] = shown.filter((command) => command.id !== "search-history").map((command) => ({ where: "command", command, id: `command::${command.id}`, desktop: false }));
    const searching: PaletteRow[] = search.map((command) => ({ where: "command", command, id: `command::${command.id}`, desktop: false }));
    // Projects once something is typed: going to one is a search.
    const places: PaletteRow[] = needle
      ? projects
          .filter((p) => `${p.name} ${p.root}`.toLowerCase().includes(needle))
          .slice(0, 8)
          .map((project) => ({ where: "project", project, id: `project::${project.id}`, desktop: false }))
      : [];
    // AJ-E2104: the rail's words work here too (is:ended app:claude …).
    const found = paletteMatches(sessions, conversations, projects, parseQuery(query));
    const elsewhere: PaletteRow[] = found.conversations.map((conversation) => ({ where: "outside", conversation, id: `outside::${conversation.account_id}::${conversation.id}`, desktop: false }));
    const local: PaletteRow[] = found.sessions.map((row) => ({ where: "local", row, id: row.id, desktop: row.kind === "desktop" }));
    const remote: PaletteRow[] = remoteMachines.flatMap((machine) =>
      machine.instances
        .filter((row) => isLive(row.status) || row.status === "waiting_user")
        .filter(
          (row) =>
            !needle ||
            `${row.account_label} ${row.provider} ${row.kind} ${machine.label}`.toLowerCase().includes(needle),
        )
        .map((row) => ({
          where: "remote" as const,
          machine,
          row,
          id: `${machine.machine_id}::${row.id}`,
          desktop: row.kind === "desktop",
        })),
    );
    // Desktop first (they own windows — the hotkey's reason to exist),
    // this Mac before paired machines.
    // A project named by what's typed comes first: typing its name and
    // pressing Enter opens it (the review of 0.1.4).
    const named = places.length > 0 && projects.some((p) => p.name.toLowerCase().startsWith(needle));
    const sessionsFound = [
      ...local.filter((r) => r.desktop),
      ...remote.filter((r) => r.desktop),
      ...local.filter((r) => !r.desktop),
      ...elsewhere,
      ...remote.filter((r) => !r.desktop),
    ];
    // The selected item's actions, a project named by what's typed, App's
    // actions, sessions, projects, and last a search of History.
    return named ? [...doing, ...places, ...commanding, ...sessionsFound, ...searching] : [...doing, ...commanding, ...sessionsFound, ...places, ...searching];
  }, [sessions, remoteMachines, needle, conversations, actions, projects, app, commands]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActive(0);
      setCapturing(false);
      setCaptureError(null);
      inputRef.current?.focus();
    }
  }, [open]);

  useEffect(() => {
    setActive((current) => Math.min(current, Math.max(rows.length - 1, 0)));
  }, [rows.length]);

  // A sheet an action opened (a rename) outlives the palette.
  if (!open) {
    return <>{panel.element}</>;
  }

  /** ↩ (or a click) opens its page here; ⌘↩ goes to the app it runs in
   * (the review of 0.1.4: Enter on a project brought a Claude window
   * forward). A project and AI Juggler's own terminal have only a page. */
  const choose = (entry: PaletteRow, toApp = false) => {
    if (entry.where === "action") {
      // Greyed: it says why, and stays.
      if (entry.action.reason) return;
      onClose();
      void entry.action.run();
      return;
    }
    if (entry.where === "command") {
      onClose();
      entry.command.run();
      return;
    }
    if (entry.where === "project") {
      host.openProjectById?.(entry.project.id);
    } else if (entry.where === "outside") {
      if (toApp && host.goToConversation) void host.goToConversation(entry.conversation);
      else host.openConversation?.(entry.conversation);
    } else if (entry.where === "local") {
      if (toApp && entry.desktop) onFocusWindow(entry.row);
      else onOpenSession(entry.row.id);
    } else if (toApp && entry.desktop && canFocusRemote(entry.machine)) {
      onFocusRemote?.(entry.machine.machine_id, entry.row.id);
    } else {
      onOpenRemote?.(entry.machine.machine_id, entry.row.id);
    }
    onClose();
  };

  const sectioned = new Set(rows.map(sectionOf)).size > 1;

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (capturing) {
      event.preventDefault();
      if (event.key === "Escape") {
        setCapturing(false);
        return;
      }
      const combo = toAccelerator(event.nativeEvent);
      const refusal = combo === null ? null : hotkeyRefusal(combo);
      if (refusal !== null) {
        setCaptureError(refusal);
        return;
      }
      if (combo !== null) {
        setCaptureError(null);
        onHotkeyChange(combo)
          .then(() => setCapturing(false))
          .catch((cause: unknown) => setCaptureError(errorText(cause)));
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((current) => (rows.length === 0 ? 0 : (current + 1) % rows.length));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) => (rows.length === 0 ? 0 : (current - 1 + rows.length) % rows.length));
    } else if (event.key === "Enter") {
      event.preventDefault();
      const row = rows[active];
      if (row) choose(row, event.metaKey || event.ctrlKey);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-24 backdrop-blur-sm"
      onClick={onClose}
      data-testid="goto-overlay"
    >
      <Card
        className="w-[560px] max-w-[90vw] gap-0 overflow-hidden p-0"
        data-testid="goto-palette"
        onClick={(event: React.MouseEvent) => event.stopPropagation()}
      >
        <div className="border-b border-hairline p-3">
          <input
            ref={inputRef}
            role="combobox"
            aria-expanded="true"
            aria-controls="goto-listbox"
            aria-activedescendant={rows[active] ? `goto-option-${domId(rows[active]!.id)}` : undefined}
            aria-label={chosen ? `Actions for ${chosen.heading}, or search and go` : "Search, act or go to a session"}
            value={query}
            placeholder={
              capturing ? "Press the new combo (Esc cancels)…" : chosen ? `Act on ${chosen.heading}, or search…` : "Go to a session or screen, or type what to do…"
            }
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            className="w-full bg-transparent text-meta text-ink outline-none placeholder:text-muted"
          />
        </div>
        <ul
          id="goto-listbox"
          role="listbox"
          aria-label="Results"
          className="max-h-80 overflow-y-auto p-1.5"
        >
          {rows.length === 0 && (
            <li className="px-2.5 py-3 text-meta text-muted">
              {needle ? `No matches for “${needle}”.` : "Nothing open yet — start one with New session."}
            </li>
          )}
          {rows.map((entry, index) => (
            <Fragment key={entry.id}>
              {/* Section names, when there's more than one kind of row:
                  the options name themselves, so these are for the eye. */}
              {sectioned && sectionOf(entry) !== (index > 0 ? sectionOf(rows[index - 1]!) : null) && (
                <li role="none" aria-hidden="true" className="px-2.5 pb-1 pt-2 text-label font-semibold text-muted first:pt-0.5">
                  {entry.where === "action" ? `Actions for ${chosen?.heading ?? ""}` : sectionOf(entry)}
                </li>
              )}
            {/* The option itself is the target: keyboard goes through the
                combobox (aria-activedescendant), so no nested button. */}
            <li
              id={`goto-option-${domId(entry.id)}`}
              role="option"
              aria-selected={index === active}
              aria-disabled={entry.where === "action" && entry.action.reason ? true : undefined}
              data-testid={`goto-row-${entry.where === "remote" ? `remote-${entry.row.id}` : entry.id}`}
              onClick={(event) => choose(entry, event.metaKey || event.ctrlKey)}
              onMouseEnter={() => setActive(index)}
              className={
                "flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-left text-meta transition-colors " +
                (index === active ? "bg-accent-soft text-accent-hi" : "text-ink hover:bg-surface")
              }
            >
                {entry.where === "action" ? (
                  <ActionOption action={entry.action} />
                ) : entry.where === "command" ? (
                  <CommandOption command={entry.command} />
                ) : entry.where === "project" ? (
                  <>
                    {/* The project's own mark, its folder under your home (P24). */}
                    <ProjectMark projectId={entry.project.id} size={16} />
                    <span className="min-w-0 shrink truncate">{entry.project.name}</span>
                    <span className="min-w-0 shrink truncate text-muted">{homeRelative(entry.project.root)}</span>
                    <span className="ml-auto shrink-0 text-muted">open</span>
                  </>
                ) : (
                <>
                {/* D-38: the app it is in, by its own icon (AI Juggler's
                    for a session in its terminal); the words at the end
                    say what choosing it does. */}
                {/* Its app, by its icon (decoration: the words say where). */}
                <AppIcon {...paletteApp(entry)} size={16} />
                {entry.where === "outside" ? (
                  <>
                    <span className="min-w-0 shrink truncate">{conversationTitle(entry.conversation)}</span>
                    <span className="min-w-0 shrink truncate text-muted">{entry.conversation.account_label} · in {paletteApp(entry).name}</span>
                  </>
                ) : (
                  <>
                    <span className="min-w-0 shrink truncate">{entry.where === "local" ? sessionTitle(entry.row) : entry.row.account_label}</span>
                    <span className="min-w-0 shrink truncate text-muted">{entry.where === "local" ? `${entry.row.account_label} · ${providerDisplayName(entry.row.provider)}` : providerDisplayName(entry.row.provider)}</span>
                  </>
                )}
                {entry.where === "remote" && (
                  <span className="shrink-0 text-muted" data-testid="goto-remote-machine">
                    on {entry.machine.label} ·{" "}
                    {entry.machine.freshness === "live" ? "● live" : entry.machine.freshness === "stale" ? "◐ stale" : "○ offline"}
                  </span>
                )}
                <span className="ml-auto shrink-0 text-muted">{entry.where === "remote" ? "inspect" : "open"}</span>
                </>
                )}
            </li>
            </Fragment>
          ))}
        </ul>
        <div className="flex items-center gap-3 border-t border-hairline px-3 py-2 text-label text-muted">
          <span className="min-w-0 flex-1">↩ open its page · ⌘↩ go to its app</span>
          <Button
            size="sm"
            variant="ghost"
            data-testid="goto-change-hotkey"
            onClick={() => setCapturing(true)}
          >
            {capturing ? "press a combo…" : hotkey ? `${hotkeyWords(hotkey)} — change…` : "Choose a key to open from anywhere…"}
          </Button>
          {captureError !== null && (
            <span className="text-danger" data-testid="goto-hotkey-error">
              {captureError}
            </span>
          )}
        </div>
      </Card>
    </div>
  );
}

type PaletteRow =
  | { where: "action"; action: ResolvedAction; id: string; desktop: false }
  | { where: "command"; command: PaletteCommand; id: string; desktop: false }
  | { where: "project"; project: ProjectInfo; id: string; desktop: false }
  | { where: "local"; row: InstanceInfo; id: string; desktop: boolean }
  | { where: "outside"; conversation: ConversationInfo; id: string; desktop: false }
  | { where: "remote"; machine: RemoteMachineInfo; row: RemoteInstanceInfo; id: string; desktop: boolean };

/** D-38: the app a session row shows. */
function paletteApp(entry: Extract<PaletteRow, { where: "local" | "outside" | "remote" }>): AppRef {
  if (entry.where === "outside") return appOfConversation(entry.conversation);
  if (entry.where === "local" && !entry.desktop) return { appId: OWN_APP_ID, appPid: null, name: OWN_APP };
  return appOfSession(entry.row);
}

/** AJ-E2106: the palette's sections — the selected item's actions, then
 * sessions, then projects. */
function sectionOf(entry: PaletteRow): string {
  return entry.where === "action" ? "Selected" : entry.where === "command" ? (entry.command.id === "search-history" ? "Search" : "Actions") : entry.where === "project" ? "Projects" : "Sessions";
}

/** A remote focus is offered only where that machine allows control and
 * is connected now (it decides again when the request arrives). */
function canFocusRemote(machine: RemoteMachineInfo): boolean {
  return machine.permission === "control" && machine.connection === "connected";
}

/** Option ids must be valid DOM ids. */
function domId(id: string): string {
  return id.replace(/[^A-Za-z0-9_-]/g, "_");
}
