/**
 * D-62 decision 18 (AJ-E3007): New pool opens this wizard. Four steps:
 * a template, where and who (from what exists), roles and talk, and a
 * review in sentences. "Set up by hand on the canvas" is on every step;
 * Back keeps every choice; Escape and Cancel leave nothing made. Making
 * the pool is the user's own calls in one go, then its canvas opens.
 *
 * It covers the window like the app's other full sheets, with the steps
 * down the side (wireframes 1–4).
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";

import { keepTabInside } from "../../components/form";
import { Button } from "../../components/primitives";
import { useActionHost } from "../../components/ItemMenu";
import { showToast } from "../../components/Toast";
import { errorText, type AccountInfo, type InstanceInfo, type ProjectInfo } from "../../lib/bridge";
import { useApps } from "../../lib/apps";
import { useConversations } from "../../lib/conversations";
import { MEMBER_KINDS, type MemberKind } from "../../lib/ipc/pool-model";
import { broadCandidates, draftOf, makePool, sessionCandidates, type Candidate, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { refreshPools, usePools } from "../../lib/pools";
import { sessionState } from "../../lib/rail-items";
import { useSubscriptions } from "../../lib/subscriptions";
import { useTools } from "../../lib/tools";
import { THIS_MACHINE } from "../../lib/machine-words";
import { nameOf } from "../../pages/pool/actions";
import { NewPoolReview } from "./NewPoolReview";
import { NewPoolRoles } from "./NewPoolRoles";
import { NewPoolTemplate } from "./NewPoolTemplate";
import { NewPoolWho } from "./NewPoolWho";

const STEPS = ["Template", "Where and who", "Roles and talk", "Review"] as const;

/** `base`, or `base 2`, `base 3`… when a pool already has the name. */
export function freeName(base: string, taken: string[]): string {
  const names = new Set(taken.map((name) => name.trim().toLowerCase()));
  for (let n = 1; ; n += 1) {
    const name = n === 1 ? base : `${base} ${n}`;
    if (!names.has(name.toLowerCase())) return name;
  }
}

/** The project with the most sessions open now, `null` when none is. */
export function busiestProject(sessions: InstanceInfo[]): string | null {
  const counts = new Map<string, number>();
  for (const session of sessions) {
    if (session.project_id && sessionState(session).state !== "ended") counts.set(session.project_id, (counts.get(session.project_id) ?? 0) + 1);
  }
  return [...counts].sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;
}

export function NewPool({
  sessions,
  projects,
  accounts,
  project,
  into,
  onClose,
  onByHand,
}: {
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  /** The project in view, the scope it starts with. */
  project: string | null;
  /** Set up this existing pool (an empty one's "Use a template") rather
   * than make a new one. */
  into?: { id: string; name: string };
  onClose: () => void;
  onByHand: () => void;
}): JSX.Element {
  const pools = usePools();
  const host = useActionHost();
  const tools = useTools();
  const apps = useApps(accounts);
  const subscriptions = useSubscriptions();
  const conversations = useConversations().all;
  const taken = useMemo(() => (pools?.pools ?? []).map((pool) => pool.name), [pools]);
  // The project in view, else the one with the most sessions open now: a
  // one-project template starts where the work is.
  const start = useMemo(() => project ?? busiestProject(sessions), [project, sessions]);
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState<Draft>(() => ({ ...draftOf(templateById("writer-reviewer"), start), name: into?.name ?? freeName("Writer and reviewer", taken) }));
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const primaryRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => primaryRef.current?.focus(), [step]);

  const projectName = (id: string | null) => (id === null ? "No project" : (projects.find((one) => one.id === id)?.name ?? "A project"));
  const thisMac = useMemo(() => {
    const counts = new Map<string, number>();
    for (const session of sessions) counts.set(session.machine_id, (counts.get(session.machine_id) ?? 0) + 1);
    return [...counts].sort((a, b) => b[1] - a[1])[0]?.[0] ?? "this-mac";
  }, [sessions]);
  const candidates = useMemo(() => {
    const now = Date.now();
    const out = {} as Record<MemberKind, Candidate[]>;
    for (const kind of MEMBER_KINDS) {
      if (kind === "session") out[kind] = sessionCandidates({ draft, sessions, conversations, pools, tools, projectName, nameOf, now });
      else if (kind === "everyone") out[kind] = [{ ref: "all", kind, label: draft.every ? "Everyone, in every project" : "Everyone in this pool's projects", detail: "every session in scope, now and later", group: "open", reach: "later", blocked: null, lastMs: 0 }];
      else out[kind] = broadCandidates({ kind, apps: apps ?? [], subscriptions: subscriptions ?? [], tools, sessions, thisMac, macName: (id) => (id === thisMac ? THIS_MACHINE : id) });
    }
    return out;
    // projectName reads projects, which is a dependency.
  }, [draft, sessions, conversations, pools, tools, apps, subscriptions, thisMac, projects]);

  const choose = (id: string) => {
    const template = templateById(id);
    setDraft((was) => ({ ...draftOf(template, start), members: was.members, name: into || template.id === "blank" ? was.name : freeName(template.name, taken) }));
  };
  const fix = (row: Candidate) => {
    if (!row.fix) return;
    if (row.fix.action === "scope") setDraft((was) => ({ ...was, every: false, projects: [...was.projects, row.fix!.arg || null] }));
    else if (row.fix.action === "open-pool") host.openPool?.(row.fix.arg);
    else if (row.fix.action === "resume") {
      const session = sessions.find((one) => one.id === row.fix!.arg);
      if (session) void host.continueSession?.(session);
    } else {
      const [accountId, id] = row.fix.arg.split("/");
      const conversation = conversations.find((one) => one.account_id === accountId && one.id === id);
      if (conversation) void host.continueConversation?.(conversation);
    }
  };
  const make = async () => {
    if (!draft.name.trim()) {
      showToast({ message: "Name the pool first", tone: "warning" });
      return;
    }
    setBusy(true);
    try {
      const id = await makePool(draft, into?.id);
      await refreshPools();
      onClose();
      host.openPool?.(id);
    } catch (cause) {
      showToast({ message: "Couldn't make the pool", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const last = step === STEPS.length - 1;
  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-label="New pool"
      className="fixed inset-0 z-50 flex flex-col bg-canvas"
      data-testid="new-pool"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
        keepTabInside(dialogRef.current, event);
      }}
    >
      <header className="flex items-center gap-3 border-b border-hairline bg-surface px-5 py-3">
        <h2 className="text-title font-semibold text-ink">
          New pool{step > 0 && <span className="text-ink-2"> · {templateById(draft.template).name}</span>}
        </h2>
      </header>
      <div className="flex min-h-0 flex-1">
        <nav aria-label="Steps" className="flex w-56 shrink-0 flex-col gap-1 border-r border-hairline bg-surface p-3">
          {STEPS.map((label, at) => (
            <button
              key={label}
              type="button"
              onClick={() => at < step && setStep(at)}
              aria-current={at === step ? "step" : undefined}
              disabled={at > step}
              className={"flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (at === step ? "bg-accent-soft font-semibold text-ink" : "text-ink-2 disabled:opacity-60")}
              data-testid="new-pool-step"
            >
              <span className={"flex size-5 shrink-0 items-center justify-center rounded-full text-label font-semibold " + (at === step ? "bg-accent text-white" : at < step ? "bg-success text-white" : "bg-surface-3 text-muted")}>
                {at < step ? "✓" : at + 1}
              </span>
              {label}
            </button>
          ))}
        </nav>
        <main className="min-w-0 flex-1 overflow-y-auto p-5">
          <div className="mx-auto flex max-w-5xl flex-col gap-4">
            {step === 0 && <NewPoolTemplate chosen={draft.template} onChoose={choose} />}
            {step === 1 && (
              <NewPoolWho
                draft={draft}
                onDraft={setDraft}
                candidates={candidates}
                projects={projects.map((one) => ({ id: one.id, name: one.name, open: sessions.some((session) => session.project_id === one.id && sessionState(session).state !== "ended") }))}
                onFix={fix}
              />
            )}
            {step === 2 && <NewPoolRoles draft={draft} onDraft={setDraft} />}
            {step === 3 && <NewPoolReview draft={draft} onName={(name) => setDraft((was) => ({ ...was, name }))} projectName={projectName} nameFixed={!!into} />}
          </div>
        </main>
      </div>
      <footer className="flex items-center gap-2 border-t border-hairline bg-surface px-5 py-3">
        <Button variant="ghost" onClick={onByHand} data-testid="new-pool-by-hand">
          Set up by hand on the canvas
        </Button>
        <span className="flex-1" />
        <Button variant="ghost" onClick={onClose} data-testid="new-pool-cancel">
          Cancel
        </Button>
        {step > 0 && (
          <Button variant="secondary" onClick={() => setStep(step - 1)} data-testid="new-pool-back">
            Back
          </Button>
        )}
        <Button ref={primaryRef} variant="primary" disabled={busy} onClick={() => (last ? void make() : setStep(step + 1))} data-testid="new-pool-next">
          {last ? (into ? "Set up the pool" : "Make the pool") : `Next: ${STEPS[step + 1]!.toLowerCase()}`}
        </Button>
      </footer>
    </div>
  );
}
