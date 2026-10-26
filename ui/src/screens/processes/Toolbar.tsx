/**
 * AJ-E2404 — the Processes view's controls: its views (Needs attention
 * first), a search that also takes `port:5173` and `pid:123`, and filters
 * that are all off until chosen, each one shown as a chip to take off.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { IconSliders } from "../../components/icons-actions";
import { processState, STATE_FILTERS } from "../../lib/process-states";
import type { ProcessCountsInfo } from "../../lib/ipc/processes";

export type ViewId = "attention" | "running" | "ended" | "ports" | "history";

export interface Filters {
  states: string[];
  hasPort: boolean;
  heavy: boolean;
  showInternal: boolean;
  app: string | null;
  account: string | null;
  project: string | null;
}

export const NO_FILTERS: Filters = { states: [], hasPort: false, heavy: false, showInternal: false, app: null, account: null, project: null };

export interface Choice {
  id: string;
  label: string;
}

const VIEWS: { id: ViewId; label: string }[] = [
  { id: "attention", label: "Needs attention" },
  { id: "running", label: "Everything running" },
  { id: "ended", label: "Ended" },
  { id: "ports", label: "Ports" },
  { id: "history", label: "Cleanup history" },
];

export function ViewTabs({ view, counts, onView }: { view: ViewId; counts: ProcessCountsInfo | null; onView: (view: ViewId) => void }): JSX.Element {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const move = (at: number) => {
    const next = (at + VIEWS.length) % VIEWS.length;
    onView(VIEWS[next]!.id);
    refs.current[next]?.focus();
  };
  return (
    <div role="tablist" aria-label="Which processes" className="inline-flex rounded-xl border border-hairline bg-surface-2 p-0.5" data-testid="process-views">
      {VIEWS.map((item, at) => {
        const selected = item.id === view;
        const count = item.id === "attention" ? counts?.attention : item.id === "running" ? counts?.processes : undefined;
        return (
          <button
            key={item.id}
            ref={(element) => {
              refs.current[at] = element;
            }}
            type="button"
            role="tab"
            aria-selected={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onView(item.id)}
            onKeyDown={(event) => {
              if (event.key === "ArrowRight") move(at + 1);
              if (event.key === "ArrowLeft") move(at - 1);
            }}
            data-testid={`process-view-${item.id}`}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-body font-medium transition-colors ${selected ? "bg-surface text-ink shadow-card" : "text-muted hover:text-ink"}`}
          >
            {item.label}
            {count !== undefined && count > 0 && <span className={`rounded-full px-1.5 text-label tabular-nums ${item.id === "attention" ? "bg-warning-soft text-warning" : "bg-surface-3 text-muted"}`}>{count}</span>}
          </button>
        );
      })}
    </div>
  );
}

function Check({ on, label, onChange, testId }: { on: boolean; label: string; onChange: (on: boolean) => void; testId: string }): JSX.Element {
  return (
    <label className="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-meta text-ink hover:bg-surface-2">
      <input type="checkbox" checked={on} onChange={(event) => onChange(event.target.checked)} className="accent-accent" data-testid={testId} />
      {label}
    </label>
  );
}

function Pick({ label, value, choices, onChange, testId }: { label: string; value: string | null; choices: Choice[]; onChange: (id: string | null) => void; testId: string }): JSX.Element | null {
  if (choices.length < 2) return null;
  return (
    <label className="flex flex-col gap-1 px-2 py-1 text-meta text-muted">
      {label}
      <select value={value ?? ""} onChange={(event) => onChange(event.target.value || null)} className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-meta text-ink" data-testid={testId}>
        <option value="">Any</option>
        {choices.map((choice) => (
          <option key={choice.id} value={choice.id}>
            {choice.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/** How many filters are on. */
export function filterCount(filters: Filters): number {
  return filters.states.length + Number(filters.hasPort) + Number(filters.heavy) + Number(filters.showInternal) + Number(!!filters.app) + Number(!!filters.account) + Number(!!filters.project);
}

export function FiltersButton({ filters, onChange, apps, accounts, projects }: { filters: Filters; onChange: (next: Filters) => void; apps: Choice[]; accounts: Choice[]; projects: Choice[] }): JSX.Element {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!open) return;
    const down = (event: MouseEvent) => {
      if (box.current && !box.current.contains(event.target as Node)) setOpen(false);
    };
    const key = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("mousedown", down);
      document.removeEventListener("keydown", key);
    };
  }, [open]);
  const count = filterCount(filters);
  const toggleState = (word: string, on: boolean) => onChange({ ...filters, states: on ? [...filters.states, word] : filters.states.filter((state) => state !== word) });
  return (
    <div ref={box} className="relative">
      <Button variant="secondary" size="md" aria-expanded={open} aria-haspopup="dialog" onClick={() => setOpen(!open)} data-testid="process-filters">
        <IconSliders width={14} height={14} aria-hidden="true" />
        Filters{count > 0 ? ` (${count})` : ""}
      </Button>
      {open && (
        <div role="dialog" aria-label="Filters" className="absolute right-0 top-11 z-30 flex w-72 flex-col gap-1 rounded-card border border-hairline bg-surface p-2 shadow-card" data-testid="process-filters-panel">
          <p className="px-2 pt-1 text-label font-medium text-muted">State</p>
          {STATE_FILTERS.map((word) => (
            <Check key={word} on={filters.states.includes(word)} label={processState(word).label} onChange={(on) => toggleState(word, on)} testId={`filter-state-${word}`} />
          ))}
          <div className="my-1 border-t border-hairline" />
          <Check on={filters.hasPort} label="Listening on a port" onChange={(hasPort) => onChange({ ...filters, hasPort })} testId="filter-port" />
          <Check on={filters.heavy} label="Heavy" onChange={(heavy) => onChange({ ...filters, heavy })} testId="filter-heavy" />
          <Check on={filters.showInternal} label="Show AI Juggler's own" onChange={(showInternal) => onChange({ ...filters, showInternal })} testId="filter-internal" />
          <Pick label="App" value={filters.app} choices={apps} onChange={(app) => onChange({ ...filters, app })} testId="filter-app" />
          <Pick label="Profile" value={filters.account} choices={accounts} onChange={(account) => onChange({ ...filters, account })} testId="filter-account" />
          <Pick label="Project" value={filters.project} choices={projects} onChange={(project) => onChange({ ...filters, project })} testId="filter-project" />
          {count > 0 && (
            <Button variant="ghost" size="sm" className="mt-1 self-start" onClick={() => onChange(NO_FILTERS)}>
              Clear filters
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

export function SearchBox({ value, onChange }: { value: string; onChange: (text: string) => void }): JSX.Element {
  return (
    <input
      type="search"
      value={value}
      onChange={(event) => onChange(event.target.value)}
      placeholder="Search, or port:5173"
      aria-label="Search processes"
      className="h-9 w-64 rounded-xl border border-hairline bg-surface-2 px-3 text-body text-ink placeholder:text-muted focus:border-accent/60 focus:outline-none"
      data-testid="process-search"
    />
  );
}
