import { useEffect, useMemo, useRef, useState } from "react";
import { Button, Card, Chip } from "./primitives";
import type { InstanceInfo } from "../lib/bridge";
import { errorText } from "../lib/bridge";

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

/** Builds a Tauri accelerator string ("Alt+Space") from a KeyboardEvent;
 * null while only modifiers are held (the capture waits for a real key). */
export function toAccelerator(event: {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}): string | null {
  if (["Control", "Alt", "AltGraph", "Shift", "Meta", "Dead", "CapsLock"].includes(event.key)) {
    return null;
  }
  if (event.key === "Escape") {
    return null;
  }
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Super");
  if (event.shiftKey) parts.push("Shift");
  parts.push(event.key === " " ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key);
  return parts.join("+");
}

export function GoToPalette({
  open,
  onClose,
  sessions,
  onFocusWindow,
  onOpenSession,
  hotkey,
  onHotkeyChange,
}: {
  open: boolean;
  onClose: () => void;
  sessions: InstanceInfo[];
  onFocusWindow: (session: InstanceInfo) => void;
  onOpenSession: (id: string) => void;
  hotkey: string;
  onHotkeyChange: (combo: string) => Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  const rows = useMemo(() => {
    const live = sessions.filter((s) => isLive(s.status));
    const haystack = (s: InstanceInfo) =>
      `${s.account_label} ${s.provider} ${s.kind}`.toLowerCase();
    const filtered = query.trim()
      ? live.filter((s) => haystack(s).includes(query.trim().toLowerCase()))
      : live;
    // Desktop first (they own windows — the hotkey's reason to exist).
    return [
      ...filtered.filter((s) => s.kind === "desktop"),
      ...filtered.filter((s) => s.kind !== "desktop"),
    ];
  }, [sessions, query]);

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

  if (!open) {
    return null;
  }

  const choose = (row: InstanceInfo) => {
    if (row.kind === "desktop") {
      onFocusWindow(row);
    } else {
      onOpenSession(row.id);
    }
    onClose();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (capturing) {
      event.preventDefault();
      if (event.key === "Escape") {
        setCapturing(false);
        return;
      }
      const combo = toAccelerator(event.nativeEvent);
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
            aria-activedescendant={rows[active] ? `goto-option-${rows[active]!.id}` : undefined}
            aria-label="Go to instance"
            value={query}
            placeholder={capturing ? "Press the new combo (Esc cancels)…" : "Go to instance…"}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            className="w-full bg-transparent text-[13px] text-ink outline-none placeholder:text-faint"
          />
        </div>
        <ul
          id="goto-listbox"
          role="listbox"
          aria-label="Live instances"
          className="max-h-72 overflow-y-auto p-1.5"
        >
          {rows.length === 0 && (
            <li className="px-2.5 py-3 text-[11.5px] text-faint">
              No live instances — start one from the rail.
            </li>
          )}
          {rows.map((row, index) => (
            <li key={row.id} id={`goto-option-${row.id}`} role="option" aria-selected={index === active}>
              <button
                type="button"
                data-testid={`goto-row-${row.id}`}
                onClick={() => choose(row)}
                onMouseEnter={() => setActive(index)}
                className={
                  "flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-[11.5px] transition-colors " +
                  (index === active ? "bg-accent-soft text-accent-hi" : "text-ink hover:bg-surface")
                }
              >
                {row.kind === "desktop" ? (
                  <Chip tone="accent">app</Chip>
                ) : (
                  <Chip tone="neutral">term</Chip>
                )}
                <span className="min-w-0 shrink truncate">{row.account_label}</span>
                <span className="min-w-0 shrink truncate text-faint">{row.provider}</span>
                <span className="ml-auto shrink-0 text-faint">
                  {row.kind === "desktop" ? "focus window" : "open panel"}
                </span>
              </button>
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-3 border-t border-hairline px-3 py-2 text-[10.5px] text-faint">
          <span>↑↓ navigate · Enter go · Esc close</span>
          <Button
            size="sm"
            variant="ghost"
            data-testid="goto-change-hotkey"
            onClick={() => setCapturing(true)}
          >
            {capturing ? "press a combo…" : `${hotkey} — change…`}
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
