import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { Button, Card, Chip } from "./primitives";
import type { InstanceInfo, ProjectInfo, RemoteInstanceInfo, RemoteMachineInfo } from "../lib/bridge";
import { errorText } from "../lib/bridge";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../lib/hotkey";
import { ActionIcon } from "./ActionIcon";
import { useActionHost } from "./ItemMenu";
import { ActionOption, usePaletteActions, type PaletteSelection } from "./PaletteActions";
export { paletteSelection } from "./PaletteActions";
import { conversationTitle, liveOutside, useConversations } from "../lib/conversations";
import type { ResolvedAction } from "../lib/actions";
import type { ConversationInfo } from "../lib/ipc/conversations";

/**
 * "Go to instance…" (AJ-E414 Req.4): summoned by the global hotkey
 * (app://go-to) or ⌘K, listing every LIVE instance — desktop entries
 * focus their window (the tiered service), terminal entries select
 * their panel. Combobox pattern (aria-expanded/activedescendant),
 * full keyboard traversal; the footer names the binding and offers
 * the rebind capture. The full command palette (actions/search/
 * recents) is deep-review §4.17's own surface — this is its Go-to
 * slice.
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
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // AJ-E1529: sessions started outside AI Juggler, running now, too.
  const { all: conversations } = useConversations();
  const host = useActionHost();
  const needle = query.trim().toLowerCase();
  // AJ-E2106: the selected item's actions first.
  const panel = usePaletteActions(selected, needle, onChanged);
  const actions = panel.actions;
  const rows = useMemo<PaletteRow[]>(() => {
    const doing: PaletteRow[] = actions.map((action) => ({ where: "action", action, id: `action::${action.id}`, desktop: false }));
    // Projects once something is typed: going to one is a search.
    const places: PaletteRow[] = needle
      ? projects
          .filter((p) => `${p.name} ${p.root}`.toLowerCase().includes(needle))
          .slice(0, 8)
          .map((project) => ({ where: "project", project, id: `project::${project.id}`, desktop: false }))
      : [];
    const elsewhere: PaletteRow[] = liveOutside(conversations)
      .filter((c) => !needle || `${c.account_label} ${c.tool} ${conversationTitle(c)}`.toLowerCase().includes(needle))
      .map((conversation) => ({ where: "outside", conversation, id: `outside::${conversation.account_id}::${conversation.id}`, desktop: false }));
    const local: PaletteRow[] = sessions
      .filter((s) => isLive(s.status))
      .filter((s) => !needle || `${s.account_label} ${s.provider} ${s.kind}`.toLowerCase().includes(needle))
      .map((row) => ({ where: "local", row, id: row.id, desktop: row.kind === "desktop" }));
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
    return [
      ...doing,
      ...local.filter((r) => r.desktop),
      ...remote.filter((r) => r.desktop),
      ...local.filter((r) => !r.desktop),
      ...elsewhere,
      ...remote.filter((r) => !r.desktop),
      ...places,
    ];
  }, [sessions, remoteMachines, needle, conversations, actions, projects]);

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

  const choose = (entry: PaletteRow) => {
    if (entry.where === "action") {
      // Greyed: it says why, and stays.
      if (entry.action.reason) return;
      onClose();
      void entry.action.run();
      return;
    }
    if (entry.where === "project") {
      host.openProjectById?.(entry.project.id);
    } else if (entry.where === "outside") {
      // AJ-E2102: the palette goes to it; its page when that can't be done.
      if (host.goToConversation) void host.goToConversation(entry.conversation);
      else host.openConversation?.(entry.conversation);
    } else if (entry.where === "local") {
      if (entry.desktop) onFocusWindow(entry.row);
      else onOpenSession(entry.row.id);
    } else if (entry.desktop && canFocusRemote(entry.machine)) {
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
      if (row) choose(row);
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
            aria-label={selected ? `Actions for ${selected.heading}, or go to a session` : "Go to session"}
            value={query}
            placeholder={
              capturing ? "Press the new combo (Esc cancels)…" : selected ? `Act on ${selected.heading}, or go to a session…` : "Go to session…"
            }
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            className="w-full bg-transparent text-[13px] text-ink outline-none placeholder:text-faint"
          />
        </div>
        <ul
          id="goto-listbox"
          role="listbox"
          aria-label={selected ? "Actions and live sessions" : "Live sessions"}
          className="max-h-80 overflow-y-auto p-1.5"
        >
          {rows.length === 0 && (
            <li className="px-2.5 py-3 text-[11.5px] text-muted">
              No live sessions — start one from the rail.
            </li>
          )}
          {rows.map((entry, index) => (
            <Fragment key={entry.id}>
              {/* Section names, when there's more than one kind of row:
                  the options name themselves, so these are for the eye. */}
              {sectioned && sectionOf(entry) !== (index > 0 ? sectionOf(rows[index - 1]!) : null) && (
                <li role="none" aria-hidden="true" className="px-2.5 pb-1 pt-2 text-[9.5px] font-medium uppercase tracking-wide text-faint first:pt-0.5">
                  {entry.where === "action" ? `Actions for ${selected?.heading ?? ""}` : sectionOf(entry)}
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
              onClick={() => choose(entry)}
              onMouseEnter={() => setActive(index)}
              className={
                "flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-left text-[11.5px] transition-colors " +
                (index === active ? "bg-accent-soft text-accent-hi" : "text-ink hover:bg-surface")
              }
            >
                {entry.where === "action" ? (
                  <ActionOption action={entry.action} />
                ) : entry.where === "project" ? (
                  <>
                    <ActionIcon name="project" className="text-muted" />
                    <span className="min-w-0 shrink truncate">{entry.project.name}</span>
                    <span className="min-w-0 shrink truncate text-muted">{entry.project.root}</span>
                    <span className="ml-auto shrink-0 text-muted">open</span>
                  </>
                ) : (
                <>
                {entry.desktop ? <Chip tone="accent">app</Chip> : <Chip tone="neutral">term</Chip>}
                {entry.where === "outside" ? (
                  <>
                    <span className="min-w-0 shrink truncate">{conversationTitle(entry.conversation)}</span>
                    <span className="min-w-0 shrink truncate text-muted">{entry.conversation.account_label} · outside AI Juggler</span>
                  </>
                ) : (
                  <>
                    <span className="min-w-0 shrink truncate">{entry.row.account_label}</span>
                    <span className="min-w-0 shrink truncate text-muted">{entry.row.provider}</span>
                  </>
                )}
                {entry.where === "remote" && (
                  <span className="shrink-0 text-muted" data-testid="goto-remote-machine">
                    on {entry.machine.label} ·{" "}
                    {entry.machine.freshness === "live" ? "● live" : entry.machine.freshness === "stale" ? "◐ stale" : "○ offline"}
                  </span>
                )}
                <span className="ml-auto shrink-0 text-muted">
                  {entry.where === "outside"
                    ? "open"
                    : entry.where === "local"
                    ? entry.desktop
                      ? "focus window"
                      : "open panel"
                    : entry.desktop && canFocusRemote(entry.machine)
                      ? `Focus on ${entry.machine.label}`
                      : "inspect"}
                </span>
                </>
                )}
            </li>
            </Fragment>
          ))}
        </ul>
        <div className="flex items-center gap-3 border-t border-hairline px-3 py-2 text-[10.5px] text-muted">
          <span>↑↓ navigate · Enter go · Esc close</span>
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
  | { where: "project"; project: ProjectInfo; id: string; desktop: false }
  | { where: "local"; row: InstanceInfo; id: string; desktop: boolean }
  | { where: "outside"; conversation: ConversationInfo; id: string; desktop: false }
  | { where: "remote"; machine: RemoteMachineInfo; row: RemoteInstanceInfo; id: string; desktop: boolean };

/** AJ-E2106: the palette's sections — the selected item's actions, then
 * sessions, then projects. */
function sectionOf(entry: PaletteRow): string {
  return entry.where === "action" ? "Actions" : entry.where === "project" ? "Projects" : "Sessions";
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
