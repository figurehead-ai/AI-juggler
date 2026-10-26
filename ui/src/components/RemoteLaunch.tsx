/**
 * AJ-E1515 — New session on another Mac. With paired Macs, New session
 * shows "Runs on: This Mac · Studio Mac · …". Picking another Mac swaps
 * the sheet for that Mac's own choices: its projects (names only) and its
 * launch settings, as it shared them. Start goes over the link; that Mac
 * re-checks everything and answers in its own words, and the session it
 * started opens here as seen from this Mac. A Mac that can't start one
 * right now (not connected, view only) says why.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button, Card } from "./primitives";
import { IconTerminal } from "./icons";
import { errorText } from "../lib/bridge";
import { fleetLaunchOptions, fleetLaunchStart, type LaunchMachineInfo } from "../lib/ipc/fleet-launch";

export interface Where {
  machines: LaunchMachineInfo[];
  /** The paired Mac picked; `null` is this Mac. */
  machine: LaunchMachineInfo | null;
  pick: (machineId: string | null) => void;
}

/** The paired Macs New session can offer (none until any are paired). */
export function useWhere(): Where {
  const [machines, setMachines] = useState<LaunchMachineInfo[]>([]);
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => {
    void fleetLaunchOptions().then(setMachines, () => setMachines([]));
  }, []);
  return { machines, machine: machines.find((m) => m.machine_id === picked) ?? null, pick: setPicked };
}

/** "Runs on: This Mac · Studio Mac …" — only with paired Macs. */
export function MachinePicker({ where }: { where: Where }): JSX.Element | null {
  if (where.machines.length === 0) return null;
  const choices: [string | null, string][] = [[null, "This Mac"], ...where.machines.map((m): [string, string] => [m.machine_id, m.label])];
  return (
    <div role="radiogroup" aria-label="Runs on" className="mb-4 flex flex-wrap items-center gap-1.5 text-xs" data-testid="machine-picker">
      <span className="mr-1 text-section font-semibold text-ink">Runs on</span>
      {choices.map(([id, label]) => {
        const on = (where.machine?.machine_id ?? null) === id;
        return (
          <button
            key={id ?? "here"}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => where.pick(id)}
            className={
              "rounded-full border px-2.5 py-1 transition-colors " +
              (on ? "border-accent/50 bg-accent-soft text-ink" : "border-hairline text-muted hover:text-ink")
            }
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

/** New session for a paired Mac: its projects and launch settings. */
export function RemoteSessionSheet({ where, onClose }: { where: Where; onClose: () => void }): JSX.Element {
  const machine = where.machine!;
  const [project, setProject] = useState<string | null>(machine.projects[0]?.id ?? null);
  const [recipe, setRecipe] = useState<string | null>(machine.recipes[0]?.id ?? null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sheet = useRef<HTMLDivElement | null>(null);
  // As New session does: opening moves focus into the sheet (WCAG 2.4.3).
  useEffect(() => sheet.current?.querySelector<HTMLElement>("[role=radio][aria-checked=true]")?.focus(), []);
  const start = async () => {
    if (!recipe) return;
    setBusy(true);
    setError(null);
    try {
      const outcome = await fleetLaunchStart(machine.machine_id, recipe, project);
      if (outcome.accepted) onClose();
      else setError(outcome.message);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const choice = (on: boolean) =>
    "flex min-w-0 flex-col gap-0.5 rounded-xl border px-3 py-2 text-left text-xs transition-colors " +
    (on ? "border-accent/40 bg-accent-soft" : "border-hairline hover:bg-surface");
  return (
    <div
      ref={sheet}
      role="dialog"
      aria-modal="true"
      aria-label="New session"
      data-testid="remote-session-sheet"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
      onKeyDown={(event) => event.key === "Escape" && onClose()}
    >
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent-hi">
            <IconTerminal width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">New session on {machine.label}</h3>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
          <MachinePicker where={where} />
          {machine.why_not ? (
            <p className="text-xs text-muted" data-testid="remote-why-not">
              {machine.why_not}.
            </p>
          ) : (
            <>
              <p className="text-section font-semibold text-ink">Where on {machine.label}?</p>
              <div className="mt-2 grid grid-cols-2 gap-2" data-testid="remote-projects">
                {machine.projects.map((p) => (
                  <button key={p.id} type="button" aria-pressed={project === p.id} onClick={() => setProject(p.id)} className={choice(project === p.id)}>
                    <span className="truncate font-medium">{p.name}</span>
                  </button>
                ))}
                <button type="button" aria-pressed={project === null} onClick={() => setProject(null)} className={choice(project === null)}>
                  <span className="font-medium">Its launch setting's folder</span>
                  <span className="text-label text-muted">where it starts by default</span>
                </button>
              </div>
              <p className="mt-5 text-section font-semibold text-ink">Which account?</p>
              <div className="mt-2 flex flex-col gap-1.5" data-testid="remote-recipes">
                {machine.recipes.map((r) => (
                  <button key={r.id} type="button" aria-pressed={recipe === r.id} onClick={() => setRecipe(r.id)} className={choice(recipe === r.id)}>
                    <span className="font-medium">{r.account}</span>
                    <span className="text-label text-muted">
                      {r.tool} · Launch settings: {r.name}
                    </span>
                  </button>
                ))}
              </div>
            </>
          )}
          {error && (
            <p role="alert" className="mt-3 text-xs text-danger" data-testid="remote-error">
              {error}
            </p>
          )}
        </div>
        <div className="flex items-center justify-between gap-3 border-t border-hairline/60 px-6 py-3">
          <span className="text-label text-muted">{machine.label} checks it and starts it there.</span>
          <div className="flex gap-2">
            <Button size="sm" variant="secondary" onClick={onClose}>
              Close
            </Button>
            <Button size="sm" variant="primary" onClick={() => void start()} disabled={busy || !!machine.why_not || !recipe} data-testid="remote-start">
              Start there →
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
