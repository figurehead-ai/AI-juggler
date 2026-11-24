/**
 * One setup on first run's found step (and Accounts › Add an app's): its
 * tick, name, where it's used (chips), its sign-in and when it was used in
 * plain words, and where it is in bringing in. ▸ Details lists its folder,
 * each field with where it came from, and the scan's notes, verbatim. No
 * Found, Likely or Guessed words anywhere (AJ-E1545 F18; Add too, F29).
 */
import { useId, useState, type JSX } from "react";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import { Checkbox } from "../components/controls";
import { Check } from "../components/StepCheck";
import type { Paired } from "./FirstRunToolGroup";
import { chooseAppFolder } from "../lib/choose-app-folder";
import { fieldRows, mayHaveExpired, rowConfidence, sourceWords } from "./first-run-confidence";

export type RowProgress = { state: "pending" | "working" | "done" | "already" | "failed"; error?: string } | undefined;

/** The row's state at its right: the bring-in's progress (its sign-in is
 * in the status line). */
function RowState({ found, progress }: { found: DiscoveredInfo; progress: RowProgress }): JSX.Element {
  return (
    <span className="flex w-32 shrink-0 items-center justify-end gap-1.5 text-xs">
      {progress?.state === "working" ? (
        <span className="text-muted">Bringing in…</span>
      ) : progress?.state === "done" ? (
        <span className="flex items-center gap-1.5 text-success"><Check />Brought in</span>
      ) : progress?.state === "already" ? (
        <span className="text-muted" title={progress.error}>Already in AI Juggler</span>
      ) : progress?.state === "failed" ? (
        <span className="text-danger">Couldn't bring in</span>
      ) : found.managed ? (
        <span className="text-muted">Already in AI Juggler</span>
      ) : null}
    </span>
  );
}

/** Where it's used: the desktop app and/or a terminal (AJ-E2327's pairing
 * joins a profile and its folder into one row). Every row has one (the
 * 0.5.1 walk's F1): a folder no desktop app is seen using is its tool's
 * terminal folder. */
function chipsOf(found: DiscoveredInfo, desktop: boolean): string[] {
  const open = found.in_use ?? [];
  const onDesktop = desktop || open.includes("desktop");
  const inTerminal = open.includes("terminal") || !onDesktop;
  return [onDesktop ? "Desktop" : null, inTerminal ? "Terminal" : null].filter((chip): chip is string => chip !== null);
}

/** An uncertain pairing asks, with the one thing to do about it (the
 * 0.5.1 walk's F2): "Choose its folder…" (AJ-E2327's), for an App already
 * added and for a profile not added yet alike. Before Add the pick is
 * kept with the profile and its Add links it (D-84: nothing is added
 * here); the row then follows the folder chosen. */
function PairingAsk({ paired }: { paired: Paired }): JSX.Element {
  const [done, setDone] = useState(false);
  if (done) return <span data-testid="first-run-unsure"> · Its folder was chosen</span>;
  return (
    <span data-testid="first-run-unsure">
      {" · Is this right? "}
      <button
        type="button"
        className="text-accent-hi hover:underline"
        onClick={(event) => {
          event.preventDefault();
          void chooseAppFolder(paired.id, paired.title, { added: paired.added }).then((chosen) => chosen && setDone(true));
        }}
      >
        Choose its folder…
      </button>
    </span>
  );
}

/** Its sign-in, in plain words. */
function statusWords(found: DiscoveredInfo): string {
  if (found.login === "logged_in") return "Signed in";
  if (mayHaveExpired(found)) return "Its sign-in may have expired: you can sign in later";
  return "Not signed in: you can sign in later";
}

export function FoundRow({
  found,
  name,
  used,
  live,
  toolName,
  colour,
  first,
  paired,
  checked,
  disabled,
  progress,
  onCheck,
}: {
  found: DiscoveredInfo;
  name: string;
  used: string;
  /** Open or in use now (the used words show live). */
  live: boolean;
  toolName: string;
  colour: string;
  first: boolean;
  /** Its desktop profile, by AJ-E2327's evidence: one row for both. */
  paired?: Paired;
  checked: boolean;
  disabled: boolean;
  progress: RowProgress;
  onCheck: (checked: boolean) => void;
}): JSX.Element {
  const id = `first-run-${found.path}`;
  const detailsId = useId();
  const [open, setOpen] = useState(false);
  const confidence = rowConfidence(found);
  const source = sourceWords(found, toolName);
  return (
    <div className={(first ? "" : "border-t border-hairline/60 ") + (found.managed ? "opacity-60" : "hover:bg-surface-2/60")} data-testid="first-run-row" data-confidence={confidence}>
      <div className="flex items-center gap-2 pr-3">
        <label htmlFor={id} className={"flex min-w-0 flex-1 items-center gap-4 py-3 pl-5 " + (found.managed ? "" : "cursor-pointer")}>
          <Checkbox id={id} checked={checked} disabled={found.managed || disabled} onChange={(event) => onCheck(event.target.checked)} className="shrink-0" />
          <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: colour }} />
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="flex min-w-0 items-center gap-2">
              <span className="truncate text-section font-medium text-ink">{paired?.title ?? name}</span>
              {/* AJ-E1545: where it's used, as chips; no confidence badges. */}
              {chipsOf(found, paired !== undefined).map((chip) => (
                <span key={chip} className="shrink-0 rounded-md bg-surface-3 px-1.5 text-label text-ink-2" data-testid="first-run-chip">
                  {chip}
                </span>
              ))}
            </span>
            <span className="truncate text-meta text-muted" data-testid="first-run-status">
              {statusWords(found)} · <span className={live ? "text-live" : ""}>{used}</span>
              {paired && !paired.certain && <PairingAsk paired={paired} />}
            </span>
          </span>
          <RowState found={found} progress={progress} />
        </label>
        <button
          type="button"
          aria-expanded={open}
          aria-controls={detailsId}
          aria-label={`Details for ${name}`}
          onClick={() => setOpen((value) => !value)}
          className="flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 text-label text-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent"
          data-testid="first-run-details-toggle"
        >
          <span aria-hidden="true" className={"inline-block transition-transform motion-reduce:transition-none " + (open ? "rotate-90" : "")}>▸</span>
          Details
        </button>
      </div>
      {open && (
        <div id={detailsId} className="flex flex-col gap-2 pb-3 pl-[4.25rem] pr-5 text-label" data-testid="first-run-details">
          <p className="text-muted" data-testid="first-run-folder">
            Its folder: <span className="text-ink-2">{found.path.replace(/^\/Users\/[^/]+/, "~")}</span> · {source}
          </p>
          {/* AJ-E1545 (F18), F29: no Found/Likely/Guessed words, in the
              details either; where each came from says enough. */}
          <dl className="grid grid-cols-[auto_auto_1fr] items-baseline gap-x-3 gap-y-1">
            {fieldRows(found, { name, used }).map((row) => (
              <div key={row.field} className="contents" data-testid="first-run-field">
                <dt className="text-muted">{row.field}</dt>
                <dd className="text-ink">{row.value}</dd>
                <dd className="text-muted">from {row.source}</dd>
              </div>
            ))}
          </dl>
          {found.notes.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-muted" data-testid="first-run-notes">
              {found.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

/** AJ-E3306 (pass2/03 §5 rule 3): a tool installed here with nothing set
 * up, offered as a fresh App beside the setups found: its tick (off until
 * the user ticks it; nothing is made without Bring in), the name it gets,
 * that it is installed but not set up, and that its sign-in comes after.
 * `installed` says the first part whole ("OpenCode 1.19.5 installed",
 * "Codex is installed"; F18: never "installed is installed"). */
export function FreshRow({
  tool,
  name,
  installed,
  colour,
  checked,
  disabled,
  progress,
  onCheck,
}: {
  tool: string;
  name: string;
  installed: string;
  colour: string;
  checked: boolean;
  disabled: boolean;
  progress: RowProgress;
  onCheck: (checked: boolean) => void;
}): JSX.Element {
  const id = `first-run-fresh-${tool}`;
  return (
    <div className="hover:bg-surface-2/60" data-testid="first-run-fresh-row" data-tool={tool}>
      <label htmlFor={id} className="flex min-w-0 cursor-pointer items-center gap-4 py-3 pl-5 pr-3">
        <Checkbox id={id} checked={checked} disabled={disabled} onChange={(event) => onCheck(event.target.checked)} className="shrink-0" />
        <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: colour }} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-section font-medium text-ink">{name}</span>
          <span className="truncate text-label text-muted">{installed}, with nothing set up yet · AI Juggler sets it up in its own folder</span>
        </span>
        <span className="flex w-32 shrink-0 items-center justify-end gap-1.5 text-xs">
          {progress?.state === "working" ? (
            <span className="text-muted">Setting up…</span>
          ) : progress?.state === "done" ? (
            <span className="flex items-center gap-1.5 text-success"><Check />Set up</span>
          ) : progress?.state === "failed" ? (
            <span className="text-danger" title={progress.error}>Couldn't set up</span>
          ) : (
            <span className="text-muted">Sign in after</span>
          )}
        </span>
      </label>
    </div>
  );
}
