/**
 * AJ-E1507: one sheet, diff first, to make a project's rules files and
 * MCP servers the same across tools and accounts.
 *
 * Rules: every rules file the project's tools read, with how it stands
 * beside the master; the user picks the master. The first file that
 * differs shows what applying the master would change, and Apply writes
 * that file only. A tool that can read another file (an import line) can
 * be pointed at the master instead of getting a copy.
 *
 * Servers: each account's MCP servers, and what one could get from
 * another, with the target tool's verdict. Secrets never move.
 *
 * Every change says what it did and can be undone here, or later from
 * Settings › Handovers and keys.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { errorText, type ProjectInfo } from "../lib/bridge";
import { agoWords } from "../lib/time-words";
import { projectSyncApply, projectSyncDiff, projectSyncRules, type RulesDiffInfo, type RulesFileInfo, type RulesSyncInfo } from "../lib/ipc/projectSync";
import { ServersSection } from "./ProjectSyncServers";
import { DoneLine, SYNC_HEADING, type Done } from "./SyncDoneLine";
import { Radio } from "./controls";

/** A file the master can be applied to that differs from it, or isn't
 * there. */
const canApply = (file: RulesFileInfo) => file.can_apply && (file.standing === "differs" || file.standing === "missing");

/** One the project's summary counts: the sheet opens on the first. A
 * missing file for a tool that hasn't worked here can be applied, but
 * isn't pushed. */
const needsApply = (file: RulesFileInfo) => canApply(file) && file.counts;

/** Sent when the sheet closes, with the project's id: whatever shows the
 * project's rules (its header line) reads them again. */
export const PROJECT_SYNC_EVENT = "aij:project-sync";

export function ProjectSyncSheet({ project, onClose }: { project: ProjectInfo; onClose: () => void }): JSX.Element {
  useEffect(() => () => void window.dispatchEvent(new CustomEvent(PROJECT_SYNC_EVENT, { detail: project.id })), [project.id]);
  return (
    <Modal title="Rules and MCP servers" subtitle={project.name} onClose={onClose} wide testId="project-sync">
      <div className="flex max-h-[70vh] flex-col gap-5 overflow-y-auto pr-1">
        <RulesSection project={project} />
        <ServersSection project={project} />
      </div>
    </Modal>
  );
}

function RulesSection({ project }: { project: ProjectInfo }) {
  const [rules, setRules] = useState<RulesSyncInfo | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(
    async (master: string | null) => {
      const next = await projectSyncRules(project.id, master);
      setRules(next);
      setOpen((current) =>
        current && next.files.some((file) => file.name === current && canApply(file))
          ? current
          : (next.files.find(needsApply)?.name ?? null),
      );
    },
    [project.id],
  );
  useEffect(() => {
    void load(picked).catch((cause) => setError(errorText(cause)));
  }, [load, picked]);

  if (!rules) {
    return error ? <p className="text-xs text-danger" role="alert">{error}</p> : <p className="text-xs text-ink-2">Reading the rules files…</p>;
  }
  const master = rules.files.find((file) => file.name === rules.master) ?? null;
  return (
    <section className="flex flex-col gap-2" aria-labelledby="sync-rules-heading" data-testid="project-sync-rules">
      <div className="flex items-baseline justify-between gap-2">
        <h3 id="sync-rules-heading" className={SYNC_HEADING}>
          Rules
        </h3>
        {rules.summary && (
          <span className={`text-label ${rules.in_sync ? "text-muted" : "text-warning"}`} data-testid="project-sync-summary">
            {rules.summary}
          </span>
        )}
      </div>
      {rules.files.length === 0 ? (
        <p className="text-xs text-ink-2">None of your tools reads a rules file in a project.</p>
      ) : (
        <fieldset className="flex flex-col gap-1.5">
          <legend className="sr-only">The master copy</legend>
          {rules.files.map((file) => (
            <RulesRow
              key={file.name}
              file={file}
              isMaster={file.name === rules.master}
              onPick={() => setPicked(file.name)}
              isOpen={open === file.name}
              onToggle={() => setOpen(open === file.name ? null : file.name)}
            />
          ))}
        </fieldset>
      )}
      {rules.files.length > 0 && !master && (
        <p className="text-xs text-ink-2">None of these files is here with rules of its own yet, so there's nothing to compare.</p>
      )}
      {master &&
        rules.files
          .filter((file) => file.name === open && canApply(file))
          .map((file) => (
            <ApplyPanel
              key={`${file.name}|${master.name}`}
              project={project}
              master={master}
              file={file}
              onApplied={(applied) => {
                setDone(applied);
                void load(picked).catch((cause) => setError(errorText(cause)));
              }}
            />
          ))}
      {done && (
        <DoneLine
          key={done.record ?? done.message}
          done={done}
          onUndone={() => void load(picked).catch((cause) => setError(errorText(cause)))}
        />
      )}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

function RulesRow({
  file,
  isMaster,
  onPick,
  isOpen,
  onToggle,
}: {
  file: RulesFileInfo;
  isMaster: boolean;
  onPick: () => void;
  isOpen: boolean;
  onToggle: () => void;
}) {
  // Warning colour only for what the summary counts.
  const differs = file.counts;
  return (
    <div
      className={`flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border px-3 py-2 text-xs ${isMaster ? "border-accent/60 bg-accent-soft" : "border-hairline"}`}
      data-testid={`sync-file-${file.name}`}
    >
      <label className={`flex items-center gap-2 ${file.can_be_master ? "" : "pl-5"}`}>
        {file.can_be_master && <Radio name="rules-master" checked={isMaster} onChange={onPick} aria-label={`Use ${file.name} as the master`} />}
        <span className="font-mono text-label text-ink">{file.name}</span>
      </label>
      <span className="text-muted">{file.tools.join(", ")}</span>
      <span className={differs ? "text-warning" : "text-muted"}>{file.words}</span>
      {file.modified_ms !== null && <span className="text-muted">changed {agoWords(file.modified_ms, Date.now())}</span>}
      {canApply(file) && (
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={isOpen}
          className="ml-auto rounded border border-hairline px-1.5 py-0.5 text-label hover:bg-surface-2"
        >
          {isOpen ? "Hide changes" : "Show changes"}
        </button>
      )}
    </div>
  );
}

/** The diff applying the master to one file would make, and Apply. */
function ApplyPanel({
  project,
  master,
  file,
  onApplied,
}: {
  project: ProjectInfo;
  master: RulesFileInfo;
  file: RulesFileInfo;
  onApplied: (done: Done) => void;
}) {
  const [how, setHow] = useState<"copy" | "read">("copy");
  const [diff, setDiff] = useState<RulesDiffInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setDiff(null);
    setError(null);
    projectSyncDiff(project.id, master.name, file.name, how).then(
      (next) => alive && setDiff(next),
      (cause) => alive && setError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
  }, [project.id, master.name, file.name, how]);
  const apply = async () => {
    setBusy(true);
    setError(null);
    try {
      const applied = await projectSyncApply(project.id, master.name, file.name, how, file.sha256, master.sha256 ?? "");
      onApplied({ message: applied.message, record: applied.record });
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-hairline p-3" data-testid="sync-apply">
      <p className="text-xs text-ink">
        Applying <span className="font-mono">{master.name}</span> to <span className="font-mono">{file.name}</span>
        {diff && diff.added + diff.removed > 0 && (
          <span className="text-muted">
            {" "}
            · {diff.added} {diff.added === 1 ? "line" : "lines"} added, {diff.removed} removed
          </span>
        )}
      </p>
      {file.import_line && (
        <div className="flex gap-1.5" role="group" aria-label="How to apply it">
          <HowButton pressed={how === "copy"} onClick={() => setHow("copy")}>
            Copy the rules
          </HowButton>
          <HowButton pressed={how === "read"} onClick={() => setHow("read")}>
            {`Read ${master.name} instead`}
          </HowButton>
        </div>
      )}
      {diff ? <DiffView diff={diff} /> : <p className="text-xs text-ink-2">Comparing…</p>}
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      <div className="flex justify-end">
        <Button size="sm" variant="primary" disabled={busy || !diff} onClick={() => void apply()} data-testid="sync-apply-button">
          {busy ? "Applying…" : `Apply to ${file.name}`}
        </Button>
      </div>
    </div>
  );
}

function HowButton({ pressed, onClick, children }: { pressed: boolean; onClick: () => void; children: string }) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={`rounded-lg border px-2 py-1 text-label ${pressed ? "border-accent/60 bg-accent-soft text-ink" : "border-hairline text-muted hover:bg-surface-2"}`}
    >
      {children}
    </button>
  );
}

/** Each change with the lines around it, numbered as the file is now. */
export function DiffView({ diff }: { diff: RulesDiffInfo }) {
  if (diff.groups.length === 0) return <p className="text-xs text-ink-2">Nothing changes.</p>;
  return (
    <div className="overflow-x-auto rounded border border-hairline bg-canvas font-mono text-label leading-relaxed" data-testid="sync-diff">
      {diff.groups.map((group, index) => (
        <div key={index} className={index > 0 ? "border-t border-dashed border-hairline" : ""}>
          {group.map((line, at) => (
            <div
              key={at}
              className={`flex gap-2 px-2 ${line.change === "added" ? "bg-success-soft" : line.change === "removed" ? "bg-danger-soft" : ""}`}
            >
              <span className="w-8 shrink-0 select-none text-right text-muted">{line.old_number ?? ""}</span>
              <span className="w-3 shrink-0 select-none text-muted" aria-hidden="true">
                {line.change === "added" ? "+" : line.change === "removed" ? "−" : ""}
              </span>
              <span className="sr-only">{line.change === "added" ? "Added: " : line.change === "removed" ? "Removed: " : ""}</span>
              <span className="whitespace-pre text-ink">{line.text || " "}</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
