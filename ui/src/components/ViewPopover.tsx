/**
 * AJ-E2104 — the rail's View (⇧V, or its button; a dot on it when the
 * view isn't the default): Group by, Show and Only, and Back to the
 * defaults. Linear's display options, in the rail's words.
 */
import { useEffect, useId, useRef, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { IconView } from "./icons-places";
import { Select } from "./RailControls";
import { DEFAULT_VIEW, isDefaultView, ONLY_KEYS, setRailView, SHOW_KEYS, SHOW_LABELS, useRailView, type OnlyKey } from "../lib/rail-view";
import type { GroupBy } from "../lib/rail-places";
import { hasUserPools, usePools } from "../lib/pools";

const GROUPS: { id: GroupBy; label: string; note?: string }[] = [
  { id: "place", label: "Place", note: "the app and profile" },
  { id: "project", label: "Project" },
  // AJ-E1702: offered once you've made a pool (greyed until then).
  { id: "pool", label: "Pool", note: "once you make a pool" },
  // The subscription its profiles are signed in to (AJ-E2109).
  { id: "account", label: "Account", note: "its profiles together" },
];

const ONLY_LABELS: Record<OnlyKey, string> = { app: "App", profile: "Profile", account: "Account", project: "Project", mac: "Mac" };

export function ViewPopover({ options }: { options: Record<OnlyKey, { value: string; label: string }[]> }): JSX.Element {
  const view = useRailView();
  const withPools = hasUserPools(usePools());
  // Pool kept as picked, with no pool of yours: the rail shows places.
  const grouped = view.group === "pool" && !withPools ? "place" : view.group;
  const [open, setOpen] = useState(false);
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const panelId = `rail-view-${useId().replace(/:/g, "")}`;
  const changed = !isDefaultView(view);
  const show = () => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (rect) setAt({ left: Math.max(8, Math.min(rect.left, window.innerWidth - 288)), top: rect.bottom + 6 });
    setOpen(true);
  };
  // ⇧V (App's key) opens it from anywhere, or closes it.
  const toggleRef = useRef(() => {});
  toggleRef.current = () => (open ? setOpen(false) : show());
  useEffect(() => {
    const toggle = () => toggleRef.current();
    window.addEventListener("aij:rail-view", toggle);
    return () => window.removeEventListener("aij:rail-view", toggle);
  }, []);
  useEffect(() => {
    if (!open) return;
    panelRef.current?.querySelector<HTMLElement>("input, button, select")?.focus();
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!panelRef.current?.contains(target) && !buttonRef.current?.contains(target)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);
  const legend = "px-0.5 pb-1 text-label font-semibold text-muted";
  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        aria-keyshortcuts="Shift+V"
        aria-label={changed ? "View, changed from the defaults" : "View"}
        title="View (⇧V)"
        data-testid="rail-view-button"
        onClick={() => (open ? setOpen(false) : show())}
        className={
          "relative flex size-[30px] shrink-0 items-center justify-center rounded-lg border outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
          (open || changed ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted hover:bg-surface hover:text-ink")
        }
      >
        <IconView width={14} height={14} />
        {changed && <span aria-hidden="true" data-testid="rail-view-dot" className="absolute right-1 top-1 size-1.5 rounded-full bg-accent" />}
      </button>
      {open &&
        at &&
        createPortal(
          <div
            ref={panelRef}
            id={panelId}
            role="dialog"
            aria-label="View"
            data-testid="rail-view"
            style={{ left: at.left, top: at.top, maxHeight: window.innerHeight - at.top - 12 }}
            className="fixed z-50 flex w-[280px] flex-col gap-3 overflow-y-auto rounded-xl border border-hairline bg-surface p-3 text-label text-ink shadow-card"
          >
            <fieldset className="flex flex-col gap-0.5">
              <legend className={legend}>Group by</legend>
              {GROUPS.map((group) => {
                const waits = group.id === "pool" && !withPools;
                return (
                <label key={group.id} className={"flex items-baseline gap-2 rounded px-1 py-0.5 " + (waits ? "text-muted" : "hover:bg-surface-2")}>
                  <input
                    type="radio"
                    // Not offered yet: a faint empty ring, never one that looks chosen.
                    className="accent-accent disabled:opacity-40"
                    name={`${panelId}-group`}
                    checked={grouped === group.id}
                    disabled={waits}
                    onChange={() => setRailView((current) => ({ ...current, group: group.id }))}
                  />
                  <span>{group.label}</span>
                  {group.note && (group.id !== "pool" || waits) && <span className="text-label text-muted">{group.note}</span>}
                </label>
                );
              })}
            </fieldset>
            <fieldset className="flex flex-col gap-0.5">
              <legend className={legend}>Show</legend>
              {SHOW_KEYS.map((key) => (
                <label key={key} className="flex items-center gap-2 rounded px-1 py-0.5 hover:bg-surface-2">
                  <input
                    type="checkbox"
                    className="accent-accent"
                    checked={view.show[key]}
                    data-testid={`rail-view-show-${key}`}
                    onChange={(event) => setRailView((current) => ({ ...current, show: { ...current.show, [key]: event.target.checked } }))}
                  />
                  <span>{SHOW_LABELS[key]}</span>
                </label>
              ))}
            </fieldset>
            <fieldset className="flex flex-col gap-1">
              <legend className={legend}>Only</legend>
              {ONLY_KEYS.filter((key) => options[key].length > 1 || view.only[key] !== undefined).map((key) => (
                <Select
                  key={key}
                  ariaLabel={`Only this ${ONLY_LABELS[key].toLowerCase()}`}
                  value={view.only[key] ?? ""}
                  onChange={(value) =>
                    setRailView((current) => {
                      const only = { ...current.only };
                      if (value) only[key] = value;
                      else delete only[key];
                      return { ...current, only };
                    })
                  }
                  options={[{ value: "", label: `${ONLY_LABELS[key]}: all` }, ...options[key]]}
                />
              ))}
            </fieldset>
            <button
              type="button"
              disabled={!changed}
              data-testid="rail-view-reset"
              onClick={() => setRailView(DEFAULT_VIEW)}
              className="self-start rounded-md border border-hairline px-2 py-1 text-label text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 disabled:opacity-60"
            >
              Back to the defaults
            </button>
          </div>,
          document.body,
        )}
    </>
  );
}
