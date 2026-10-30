/**
 * 0.3 — "What did Codex get told?" and "Was my key hidden?", answered as
 * depth on demand. The rows are the ones "Start from a saved handover…"
 * shows a saved one with (From, Where, Continues as, Carries, Doesn't
 * carry, Withheld, the brief), shared here so a switch's handover and a
 * saved one read alike. After a switch to another tool, its done screen
 * and its edge in the Switch Graph show one line ("Hidden from Codex: 2
 * API keys · Left out: 12 earlier turns, 1 note") and Open the handover;
 * a switch on the same tool copies the whole session and writes none, so
 * nothing shows. Save handover… on a session's page ends with the same
 * line, Open the handover and Reveal in Finder. Read-only: counts and
 * kinds only, never a secret's value.
 */
import { useEffect, useId, useState, type JSX, type ReactNode } from "react";
import { Button } from "./primitives";
import { Modal } from "./form";
import { errorText } from "../lib/bridge";
import { packsOpenPreview } from "../lib/ipc/packs";
import { handoverOpen, handoverReveal, handoverSave, handoverSummary, type HandoverSummaryInfo, type SavedHandoverInfo } from "../lib/ipc/handover";
import type { WithheldInfo } from "../types/generated/WithheldInfo";

/** What the rows show: a saved pack's (`PackOpenInfo`) or a switch's
 * handover (`HandoverSheetInfo`) alike. */
export interface HandoverRowsData {
  from_tool: string;
  from_account: string;
  created_at_ms: number;
  to_tool?: string;
  to_account?: string;
  project_root: string;
  branch?: string;
  fidelity?: string | null;
  carries: string[];
  not_carried: string[];
  /** null: a handover from before this was recorded. */
  withheld: WithheldInfo[] | null;
  /** Absent from a pack read by an older build's fixtures: none. */
  kept?: WithheldInfo[];
  brief: string;
}

const counted = (rows: WithheldInfo[]) => rows.map((row) => `${row.count} ${row.what}`).join(", ");

/** The Withheld row, in words: what was hidden, and what was sent as it
 * was because the user chose so (never called hidden). */
export function withheldWords(withheld: WithheldInfo[] | null, kept: WithheldInfo[] = []): string {
  if (withheld === null) return "Not recorded for this handover.";
  const sent = kept.length > 0 ? ` Sent as they were, as you chose: ${counted(kept)}.` : "";
  if (withheld.length === 0) return kept.length > 0 ? `Nothing hidden.${sent}` : "Nothing: no secrets were found when it was made.";
  return `${counted(withheld)}, when it was made. They stay out.${sent}`;
}

function Row({ label, children }: { label: string; children: ReactNode }): JSX.Element {
  return (
    <div className="grid grid-cols-[7.5rem_1fr] gap-2 text-label">
      <span className="text-muted">{label}</span>
      <span className="min-w-0 text-ink">{children}</span>
    </div>
  );
}

/** The handover's rows, read-only. `whereNote` follows the folder (the
 * pack picker's "not on this computer"). */
export function HandoverRows({ data, whereNote }: { data: HandoverRowsData; whereNote?: ReactNode }): JSX.Element {
  const made = new Date(data.created_at_ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  return (
    <>
      <Row label="From">
        {data.from_tool} on {data.from_account} · {made}
      </Row>
      {data.to_tool && (
        <Row label="To">
          {data.to_tool} on {data.to_account}
        </Row>
      )}
      <Row label="Where">
        <span className="font-mono">{data.project_root}</span>
        {data.branch ? ` · ${data.branch}` : null}
        {whereNote}
      </Row>
      {data.fidelity && <Row label="Continues as">{data.fidelity}</Row>}
      <Row label="Carries">{data.carries.join(", ")}</Row>
      {data.not_carried.length > 0 && <Row label="Doesn't carry">{data.not_carried.join("; ")}</Row>}
      <Row label="Withheld">
        <span data-testid="pack-withheld">{withheldWords(data.withheld, data.kept)}</span>
      </Row>
      <details className="text-label">
        <summary className="cursor-pointer text-muted">The handover's brief</summary>
        <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded bg-canvas p-2 font-mono text-label text-ink" aria-label="The handover's brief (read-only)" tabIndex={0}>
          {data.brief}
        </pre>
      </details>
    </>
  );
}

const LINK = "inline-flex min-h-6 w-fit items-center text-label text-accent-hi underline-offset-2 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 rounded";

/** "Open the handover": its rows below (`inline`, inside a dialog or a
 * panel), or in a sheet of their own. Loaded on the click, not before. */
function OpenTheHandover({ title, load, inline }: { title: string; load: () => Promise<HandoverRowsData>; inline: boolean }): JSX.Element {
  const [open, setOpen] = useState(false);
  const [data, setData] = useState<HandoverRowsData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const panelId = useId();
  const toggle = () => {
    const next = !open;
    setOpen(next);
    if (next && data === null) {
      setError(null);
      load().then(setData, (cause) => setError(errorText(cause)));
    }
  };
  const body =
    error !== null ? (
      <p role="alert" className="text-label text-danger" data-testid="handover-open-error">
        {error}
      </p>
    ) : data === null ? (
      <p role="status" className="text-label text-muted">
        Opening the handover…
      </p>
    ) : (
      <HandoverRows data={data} />
    );
  return (
    <>
      <button type="button" onClick={toggle} aria-expanded={inline ? open : undefined} aria-controls={inline && open ? panelId : undefined} className={LINK} data-testid="handover-open">
        {inline && open ? "Hide the handover" : "Open the handover"}
      </button>
      {open &&
        (inline ? (
          <section id={panelId} aria-label={title} className="flex flex-col gap-2 rounded-lg border border-hairline bg-surface p-3" data-testid="handover-sheet">
            {body}
          </section>
        ) : (
          <Modal title={title} onClose={() => setOpen(false)} wide testId="handover-sheet">
            <div className="flex flex-col gap-2">{body}</div>
          </Modal>
        ))}
    </>
  );
}

/** A switch's handover: its line and Open the handover, or nothing when
 * the switch wrote none. Pass `summary` when it's known (the graph);
 * else it's asked for by the edge. */
export function SwitchHandover({ edgeId, summary, inline = true }: { edgeId: string; summary?: HandoverSummaryInfo | null; inline?: boolean }): JSX.Element | null {
  const [asked, setAsked] = useState<HandoverSummaryInfo | null | undefined>(summary);
  useEffect(() => {
    if (summary !== undefined) {
      setAsked(summary);
      return;
    }
    let alive = true;
    handoverSummary(edgeId).then(
      (found) => alive && setAsked(found),
      () => alive && setAsked(null),
    );
    return () => {
      alive = false;
    };
  }, [edgeId, summary]);
  if (!asked) return null;
  return (
    <div className="flex flex-col gap-1.5" data-testid="switch-handover">
      {asked.line && (
        <p className="text-meta text-ink" data-testid="handover-line">
          {asked.line}
        </p>
      )}
      <OpenTheHandover
        title={`What ${asked.to_tool} was told`}
        inline={inline}
        load={() => handoverOpen(edgeId).then((sheet) => ({ ...sheet, withheld: sheet.withheld ?? null }))}
      />
    </div>
  );
}

/** A saved handover: where it went, its line, Open the handover and
 * Reveal in Finder. */
export function SavedHandover({ saved, inline = false }: { saved: SavedHandoverInfo; inline?: boolean }): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  return (
    <div className="flex flex-col gap-1.5" data-testid="saved-handover">
      <div role="status" className="flex flex-col gap-1.5">
        <p className="text-meta text-ink">
          Handover saved to <span className="font-mono text-label [overflow-wrap:anywhere]">{saved.shown}</span>
        </p>
        {saved.line && (
          <p className="text-meta text-ink" data-testid="handover-line">
            {saved.line}
          </p>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <OpenTheHandover title="The saved handover" inline={inline} load={() => packsOpenPreview(saved.path)} />
        <button
          type="button"
          className={LINK}
          data-testid="handover-reveal"
          onClick={() => {
            setError(null);
            handoverReveal(saved.path).catch((cause) => setError(errorText(cause)));
          }}
        >
          Reveal in Finder
        </button>
      </div>
      {error !== null && (
        <p role="alert" className="text-label text-danger">
          {error}
        </p>
      )}
    </div>
  );
}

/** Save handover… for a session (its page, beside Continue on another
 * account): saved under AI Juggler's folder, keys handled as the user
 * chose, then what was saved. */
export function SaveHandover({ sessionId }: { sessionId: string }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState<SavedHandoverInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setSaved(null);
    setError(null);
  }, [sessionId]);
  const save = async () => {
    setBusy(true);
    setError(null);
    setSaved(null);
    try {
      setSaved(await handoverSave(sessionId));
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex flex-col gap-2 border-t border-hairline pt-3" data-testid="save-handover">
      <Button size="sm" variant="secondary" className="w-fit" disabled={busy} onClick={() => void save()} title="Saves this session's handover to read or start from later, with keys and passwords handled as you chose" data-testid="save-handover-button">
        {busy ? "Saving…" : "Save handover…"}
      </Button>
      {saved && <SavedHandover saved={saved} />}
      {error !== null && (
        <p role="alert" className="text-label text-danger" data-testid="save-handover-error">
          {error}
        </p>
      )}
    </div>
  );
}
