/**
 * D-25 — first run, when no Account exists: three calm steps in one
 * window, as the vision canvas shows them.
 *
 * 1. **What's already here.** The scan, grouped by kind of setup — your
 *    Claude profiles, the default setup (~/.claude), other setups, and
 *    Codex homes (AJ-E1301, only when one is found) — each
 *    saying where it's open right now (the desktop app and/or a terminal:
 *    a folder isn't tied to one tool), preselected (signed in or used in
 *    the last 30 days), one
 *    button to bring them in. ADR-0018: by default each is used where it
 *    is (same sign-in and history as the Claude app; AI Juggler writes
 *    nothing of its own there); "Make separate copies" is one click away.
 *    Each setup comes in or doesn't on its own: the result offers Continue
 *    with the ones that came in, Try again, or — for a failed copy — Use
 *    it where it is instead; Stop keeps what finished. "Where we looked"
 *    shows every place the scan checked.
 * 2. **Sign in where needed.** One card per account that still needs the
 *    vendor's own sign-in (the shared login-queue card).
 * 3. **OS access.** Whatever this platform lists (D-25), each with one line
 *    of why and one Allow; polled so an entry flips the moment the user
 *    says yes. An empty list skips the step.
 *
 * The pack-secrets choice is no longer asked here: the first switch asks
 * while no choice exists (D-14, nothing is built before it).
 *
 * D-25 stage 4 (AJ-E1022): the two ways in that the old account wizard
 * handled are steps here too. **A fresh account**: name it (a free name is
 * filled in), then the same sign-in step. **Another folder**: the system's
 * folder picker, then a check of what's in it (size, sign-in, a too-large
 * history offers settings only) before AI Juggler copies it.
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { providerDisplayName } from "../lib/providers";
import { funnelEvent, funnelStep, reasonCode } from "../lib/funnel";
import {
  accountImport,
  accountsList,
  createAccount,
  pickDirectory,
  sourceInspect,
  statsEvent,
  accountLoginCommand,
  accountLoginState,
  environmentScanFull,
  errorText,
  osAccessList,
  osAccessOpenSettings,
  osAccessRequest,
  providersDiscover,
  signedIn,
  terminalSpawnLogin,
  type DiscoveredInfo,
  type OsAccessInfo,
  type SourceInspectInfo,
  type ProviderInstallData,
  type ScanReportData,
} from "../lib/bridge";

type Step = "found" | "fresh" | "folder" | "signin" | "access";
type Group = "profiles" | "default" | "other" | "codex";

/** A group's title for this tool: the profile and "other" words name
 * Claude only for Claude Code (review of #72: Codex said "Other Claude
 * setups"). */
export function groupTitle(group: Group, toolName: string, provider: string): string {
  // Codex homes are their own group in every list (AJ-E1301).
  if (group === "codex") return "Codex";
  if (group === "default") return "The default setup";
  if (provider !== "claude-code") return group === "profiles" ? `Your ${toolName} profiles` : `Other ${toolName} setups`;
  return group === "profiles" ? "Your Claude profiles" : "Other Claude setups";
}

const GROUPS: { id: Group; title: string }[] = [
  { id: "profiles", title: "Your Claude profiles" },
  { id: "default", title: "The default setup" },
  { id: "other", title: "Other Claude setups" },
  { id: "codex", title: "Codex" },
];

/** The tools first run on a fresh install brings in (AJ-E1301). */
const FIRST_RUN_TOOLS = ["claude-code", "codex"];
/** A name for a new account that no account has yet ("Claude Code 2"). */
export function freeLabel(base: string, taken: string[]): string {
  const used = new Set(taken.map((label) => label.trim().toLowerCase()));
  if (!used.has(base.toLowerCase())) return base;
  for (let n = 2; ; n += 1) if (!used.has(`${base} ${n}`.toLowerCase())) return `${base} ${n}`;
}

function humanBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

/** Grouped by kind of setup, not by tool: the same folder can be used by
 * the Claude desktop app and by `claude` in a terminal (0.0.6 dogfood —
 * the default ~/.claude was called "Terminal" while the desktop app had
 * it open). Codex homes are their own kind (AJ-E1301). */
export function groupOf(found: DiscoveredInfo): Group {
  if (found.harness === "codex") return "codex";
  if (found.path.includes("/ClaudeCodeProfiles/")) return "profiles";
  if (found.origin === "vendor-default") return "default";
  return "other";
}

const folderOf = (path: string) => path.replace(/\/+$/, "").split("/").pop() ?? path;

/** The name it's brought in as (changeable later in Accounts). */
export function labelOf(found: DiscoveredInfo): string {
  const folder = folderOf(found.path);
  if (groupOf(found) === "codex") return folder === ".codex" ? "Codex" : folder.replace(/^\./, "");
  if (groupOf(found) === "default") return "Default";
  return folder.replace(/^\./, "");
}

/** "In use now" / "Used today" / "Used 3 days ago" / the scan's words. */
export function usedWords(found: DiscoveredInfo, nowSecs: number): string {
  // Open right now, and where: read from the running processes.
  const open = found.in_use ?? [];
  if (open.includes("desktop") && open.includes("terminal")) return "Open in the Claude app and a terminal";
  if (open.includes("desktop")) return "Open in the Claude app";
  if (open.includes("terminal")) return "Running in a terminal";
  const last = found.last_used_secs;
  if (last === null || last === undefined) return found.inference.activity_words || "Not used yet";
  const age = Math.max(0, nowSecs - last);
  if (age < 15 * 60) return "In use now";
  if (age < 24 * 3600) return "Used today";
  const days = Math.round(age / 86_400);
  return days === 1 ? "Used yesterday" : `Used ${days} days ago`;
}

/** Preselected: signed in, or used in the last 30 days — never one that's
 * already in AI Juggler. */
export function preselected(found: DiscoveredInfo, nowSecs: number): boolean {
  if (found.managed) return false;
  if (found.login === "logged_in") return true;
  const last = found.last_used_secs;
  return last !== null && last !== undefined && nowSecs - last < 30 * 86_400;
}

const nowSecs = () => Math.floor(Date.now() / 1000);

/** Distinct, equally light colours by position (a hash of similar paths
 * gives near-identical hues). */
const PALETTE = ["#A98BFF", "#3DD6E0", "#F59E6B", "#5BD18B", "#FF8FB8", "#6FB3FF", "#E8C547", "#C9A7FF"];
export const colourAt = (index: number) => PALETTE[index % PALETTE.length]!;

function StepMarks({ step, skipAccess }: { step: Step; skipAccess: boolean }): JSX.Element {
  // The fresh-account and folder steps are the first step's other doors.
  const first = step === "fresh" ? "Name the account" : step === "folder" ? "Check the folder" : "Bring in your accounts";
  const steps: Array<{ id: Step; label: string }> = [
    { id: step === "fresh" || step === "folder" ? step : "found", label: first },
    { id: "signin", label: "Sign in where needed" },
    ...(skipAccess ? [] : [{ id: "access" as Step, label: "Computer access" }]),
  ];
  const at = steps.findIndex((entry) => entry.id === step);
  return (
    <ol className="flex flex-wrap items-center gap-3 text-xs text-muted" aria-label="First-run steps">
      {steps.map((entry, index) => (
        <li key={entry.id} className="flex items-center gap-3" aria-current={index === at ? "step" : undefined}>
          {index > 0 && <span aria-hidden="true" className="h-px w-6 bg-hairline" />}
          <span className={"flex items-center gap-2 " + (index === at ? "font-semibold text-ink" : "")}>
            <span
              aria-hidden="true"
              className={
                "flex size-5 items-center justify-center rounded-full text-[11px] " +
                (index < at
                  ? "bg-success-soft text-success"
                  : index === at
                    ? "bg-accent text-canvas"
                    : "border border-hairline")
              }
            >
              {index < at ? "✓" : index + 1}
            </span>
            {entry.label}
          </span>
        </li>
      ))}
    </ol>
  );
}

function Check({ className = "" }: { className?: string }): JSX.Element {
  return (
    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className={className}>
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  );
}

type ImportState = {
  state: "pending" | "working" | "done" | "already" | "failed";
  error?: string;
  placement?: Placement;
};

/** ADR-0018: how a setup comes in. */
export type Placement = "in_place" | "copy";

/** The error, without the internal "account directory error at <path>:"
 * prefix — the row already says which setup it is. */
export function plainImportError(text: string): string {
  const plain = text.replace(/^account directory error at .+?: (?=\S)/, "");
  return plain.charAt(0).toUpperCase() + plain.slice(1);
}

/** The result line: what came in, what didn't, what already was. */
export function outcomeWords({
  stopped,
  done,
  waiting,
  failed,
  already,
}: {
  stopped: boolean;
  done: number;
  waiting: number;
  failed: number;
  already: string[];
}): string {
  const parts: string[] = [];
  if (stopped) parts.push(`Stopped. ${done} brought in${waiting > 0 ? `, ${waiting} still to go` : ""}.`);
  else if (done > 0 || failed === 0) parts.push(done === 1 ? "1 brought in." : `${done} brought in.`);
  if (failed > 0) parts.push(failed === 1 ? "1 couldn't come in." : `${failed} couldn't come in.`);
  if (already.length === 1) parts.push(`${already[0]} was already in AI Juggler.`);
  else if (already.length > 1) parts.push(`${already.length} were already in AI Juggler.`);
  if (done === 0 && failed === 0 && !stopped && already.length > 0) return parts.slice(1).join(" ");
  return parts.join(" ");
}

const PLACEMENTS: { id: Placement; title: string; detail: string }[] = [
  {
    id: "in_place",
    title: "Use them where they are",
    detail: "Same sign-in, settings and history as your Claude app. AI Juggler adds nothing of its own to them.",
  },
  {
    id: "copy",
    title: "Make separate copies",
    detail: "A fresh copy for AI Juggler only; your originals stay as they are. Each copy needs one sign-in.",
  },
];

function FoundStep({
  report,
  cli,
  toolName,
  provider,
  tools,
  codexCli,
  onImported,
  onAddFresh,
  onImportFolder,
  stopRef,
  onBusy,
}: {
  report: ScanReportData | null;
  cli: ProviderInstallData | null;
  toolName: string;
  provider: string;
  /** The tools whose setups this list brings in: the chosen one when
   * adding, every supported one on first run (AJ-E1301). */
  tools: string[];
  /** Codex's install row when Codex rides along beside another tool. */
  codexCli: ProviderInstallData | null;
  onImported: (signins: Signin[]) => void;
  onAddFresh: () => void;
  onImportFolder: () => void;
  /** Set by Stop (the Close button or Esc while bringing in): the setup
   * in progress finishes, the rest wait. */
  stopRef: { current: boolean };
  onBusy: (busy: boolean) => void;
}): JSX.Element {
  const now = useMemo(nowSecs, []);
  const installs = useMemo(
    () => (report?.installs ?? []).filter((found) => tools.includes(found.harness)),
    [report, tools],
  );
  // Named by Provider when they are all one kind; mixed, just "setups".
  const codexCount = installs.filter((found) => found.harness === "codex").length;
  // Codex's install row: the chosen tool's when adding Codex, else the
  // one that rides along on first run.
  const codexRow = provider === "codex" ? cli : codexCli;
  // One tool: its name. Every tool (first run): named when the setups are
  // all one kind, else just "setups".
  const kind =
    tools.length === 1
      ? toolName
      : codexCount === 0
        ? "Claude Code"
        : codexCount === installs.length
          ? "Codex"
          : "";
  // The defaults come straight from the scan (ready on first paint);
  // the user's ticks override them.
  const defaults = useMemo(
    () => Object.fromEntries(installs.map((found) => [found.path, preselected(found, now)])),
    [installs, now],
  );
  const [overrides, setPicked] = useState<Record<string, boolean>>({});
  const picked: Record<string, boolean> = { ...defaults, ...overrides };
  const [progress, setProgress] = useState<Record<string, ImportState>>({});
  const [busy, setBusy] = useState(false);
  const [showLooked, setShowLooked] = useState(false);

  // ADR-0018: only a tool whose adapter can use a folder where it is
  // offers it (Codex comes in as a copy until its adapter can).
  const canInPlace = provider === "claude-code";
  const [placement, setPlacement] = useState<Placement>(canInPlace ? "in_place" : "copy");
  // A failed copy can be retried "where it is" on its own.
  const [rowPlacement, setRowPlacement] = useState<Record<string, Placement>>({});
  // Only Claude Code setups can be used where they are; a Codex row in
  // first run's every-tool list always comes in as a copy (AJ-E1301).
  const placementOf = (found: DiscoveredInfo): Placement =>
    found.harness === "claude-code" ? (rowPlacement[found.path] ?? placement) : "copy";
  // Sign-ins of the ones that came in survive retries.
  const signinsRef = useRef<Map<string, Signin>>(new Map());
  const [stopped, setStopped] = useState(false);
  const [sawAlready, setSawAlready] = useState(false);

  const isDone = (found: DiscoveredInfo) => {
    const state = progress[found.path]?.state;
    return state === "done" || state === "already";
  };
  const chosen = installs.filter((found) => picked[found.path] && !found.managed && !isDone(found));
  const runImports = async (rows: DiscoveredInfo[], how: (found: DiscoveredInfo) => Placement = placementOf) => {
    stopRef.current = false;
    setStopped(false);
    setBusy(true);
    onBusy(true);
    funnelEvent("import_started");
    let failed = 0;
    let already = 0;
    let wasStopped = false;
    for (const found of rows) {
      if (stopRef.current) {
        wasStopped = true;
        break;
      }
      const mode = how(found);
      setProgress((current) => ({ ...current, [found.path]: { state: "working", placement: mode } }));
      try {
        const result = await accountImport(found.harness, found.path, labelOf(found), null, found.origin, false, mode === "in_place");
        if (result.login === "expired") countLogin(found.harness, "expired");
        if (result.login !== "logged_in") {
          signinsRef.current.set(result.account.id, {
            accountId: result.account.id,
            label: result.account.label,
            loginNote: result.login_note,
            provider: found.harness,
            colour: colourAt(installs.indexOf(found)),
          });
        }
        setProgress((current) => ({ ...current, [found.path]: { state: "done", placement: mode } }));
      } catch (cause) {
        const code = (cause as { code?: string })?.code;
        if (code === "already_imported") {
          already += 1;
          // Nothing to do — and not "brought in": it already was (as a copy,
          // perhaps; the error says which).
          setProgress((current) => ({
            ...current,
            [found.path]: { state: "already", error: errorText(cause), placement: mode },
          }));
        } else {
          failed += 1;
          setProgress((current) => ({
            ...current,
            [found.path]: { state: "failed", error: plainImportError(errorText(cause)), placement: mode },
          }));
        }
      }
    }
    funnelEvent(failed === 0 && !wasStopped ? "import_completed" : "import_failed");
    setBusy(false);
    onBusy(false);
    if (wasStopped) setStopped(true);
    // Something already in: say so before moving on.
    if (already > 0) setSawAlready(true);
    if (failed === 0 && !wasStopped && already === 0) onImported([...signinsRef.current.values()]);
  };
  const failedRows = installs.filter((found) => progress[found.path]?.state === "failed");
  const doneCount = installs.filter((found) => progress[found.path]?.state === "done").length;
  const waiting = chosen.filter((found) => progress[found.path] === undefined);
  const alreadyRows = installs.filter((found) => progress[found.path]?.state === "already");
  const showOutcome = !busy && (failedRows.length > 0 || stopped || sawAlready);
  const continueNow = () => onImported([...signinsRef.current.values()]);
  // The result takes focus (the button the user pressed is gone).
  const continueRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (showOutcome) continueRef.current?.focus();
  }, [showOutcome]);

  if (report === null) {
    return (
      <div className="flex flex-col gap-3" role="status" data-testid="first-run-scanning">
        <h1 className="text-[28px] font-semibold tracking-tight">Looking for {toolName} on this Mac…</h1>
        <p className="text-sm text-muted">{provider === "claude-code"
            ? "Your default setup, your Claude profiles and anything else that looks like one. Nothing is changed."
            : "Your default setup and anything else that looks like one. Nothing is changed."}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6" data-testid="first-run-found">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {installs.length === 0
            ? `No ${toolName} setups found yet.`
            : installs.length === 1
              ? `You already have a ${kind} setup on this Mac.`
              : `You already have ${installs.length} ${kind ? `${kind} setups` : "setups"} on this Mac.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          {installs.length === 0
            ? `Add your first account and AI Juggler sets it up in its own folder. You'll sign in with ${toolName}'s own sign-in.`
            : placement === "in_place"
              ? "Pick the ones to bring in. AI Juggler uses each one where it is, so it stays signed in and keeps its history, the same as everywhere else you use it."
              : "Pick the ones to bring in. AI Juggler makes its own copy of each; your originals are never changed, and you can keep using them as before."}
        </p>
      </div>

      {codexCount > 0 && codexCli && codexCli.status === "not_installed" && (
        <p className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" data-testid="first-run-no-codex">
          Codex itself isn't installed yet. You can bring these in now; to start sessions, install it first:{" "}
          <span className="font-mono text-xs">npm install -g @openai/codex</span>
        </p>
      )}

      {cli && cli.status === "not_installed" && (
        <p className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" data-testid="first-run-no-cli">
          {toolName} itself isn't installed yet. You can bring these in now; to start sessions, install it first
          {provider === "claude-code" ? (
            <>
              :{" "}<span className="font-mono text-xs">npm install -g @anthropic-ai/claude-code</span>
            </>
          ) : cli.install_url ? (
            <>
              {" "}from{" "}
              <a className="underline" href={cli.install_url} target="_blank" rel="noreferrer">
                its install page
              </a>
              .
            </>
          ) : (
            "."
          )}
        </p>
      )}

      {installs.length > 0 && (
        <div className="overflow-hidden rounded-2xl border border-hairline bg-surface">
          {GROUPS.map((group) => {
            const rows = installs.filter((found) => groupOf(found) === group.id);
            if (rows.length === 0) return null;
            return (
              <fieldset key={group.id} className="border-b border-hairline last:border-b-0" data-testid={`first-run-group-${group.id}`}>
                <legend className="sr-only">{groupTitle(group.id, toolName, provider)}</legend>
                <div aria-hidden="true" className="flex items-center justify-between gap-3 bg-surface-2 px-5 py-2.5 text-xs">
                  <span className="font-semibold text-ink">
                    {groupTitle(group.id, toolName, provider)} <span className="font-normal text-faint">· {rows.length}</span>
                  </span>
                  {group.id === "default" && cli && cli.status !== "not_installed" && (
                    <span className="text-faint">{toolName} {cli.version ?? ""} installed</span>
                  )}
                  {group.id === "codex" && codexRow && codexRow.status !== "not_installed" && (
                    <span className="text-faint">Codex {codexRow.version ?? ""} installed</span>
                  )}
                </div>
                {rows.map((found) => {
                  const row = progress[found.path];
                  const colour = colourAt(installs.indexOf(found));
                  const id = `first-run-${found.path}`;
                  return (
                    <label
                      key={found.path}
                      htmlFor={id}
                      className={
                        "flex items-center gap-4 border-t border-hairline/60 px-5 py-3 first-of-type:border-t-0 " +
                        (found.managed ? "opacity-60" : "cursor-pointer hover:bg-surface-2/60")
                      }
                      data-testid="first-run-row"
                    >
                      <input
                        id={id}
                        type="checkbox"
                        checked={!!picked[found.path]}
                        disabled={found.managed || busy}
                        onChange={(event) => setPicked((current) => ({ ...current, [found.path]: event.target.checked }))}
                        className="size-4 shrink-0 accent-[var(--color-accent)]"
                      />
                      <span
                        aria-hidden="true"
                        className="size-2.5 shrink-0 rounded-full"
                        style={{ background: colour }}
                      />
                      <span className="flex min-w-0 flex-1 flex-col">
                        <span className="truncate text-[15px] font-medium text-ink">{labelOf(found)}</span>
                        <span className="truncate font-mono text-[11px] text-faint" title={found.path}>
                          {found.path.replace(/^\/Users\/[^/]+/, "~")}
                        </span>
                      </span>
                      <span className={"shrink-0 text-xs " + ((found.in_use ?? []).length > 0 || usedWords(found, now) === "In use now" ? "text-live" : "text-muted")}>
                        {usedWords(found, now)}
                      </span>
                      <span className="flex w-32 shrink-0 items-center justify-end gap-1.5 text-xs">
                        {row?.state === "working" ? (
                          <span className="text-muted">Bringing in…</span>
                        ) : row?.state === "done" ? (
                          <span className="flex items-center gap-1.5 text-success"><Check />Brought in</span>
                        ) : row?.state === "already" ? (
                          <span className="text-muted" title={row.error}>Already in AI Juggler</span>
                        ) : row?.state === "failed" ? (
                          <span className="text-danger">Couldn't bring in</span>
                        ) : found.managed ? (
                          <span className="text-muted">Already in AI Juggler</span>
                        ) : found.login === "logged_in" ? (
                          <span className="flex items-center gap-1.5 text-success"><Check />Signed in</span>
                        ) : (
                          <span className="text-muted">Needs sign-in</span>
                        )}
                      </span>
                    </label>
                  );
                })}
              </fieldset>
            );
          })}
        </div>
      )}

      {installs.length > 0 && !showOutcome && canInPlace && (
        <div role="radiogroup" aria-label="How AI Juggler uses them" className="grid gap-2 sm:grid-cols-2" data-testid="first-run-placement">
          {PLACEMENTS.map((option) => (
            <label
              key={option.id}
              className={
                "flex cursor-pointer gap-3 rounded-xl border px-4 py-3 " +
                (placement === option.id ? "border-accent bg-accent-soft/40" : "border-hairline bg-surface hover:bg-surface-2/60")
              }
              data-testid={`first-run-placement-${option.id}`}
            >
              <input
                type="radio"
                name="first-run-placement"
                value={option.id}
                aria-labelledby={`first-run-placement-${option.id}-title`}
                aria-describedby={`first-run-placement-${option.id}-detail`}
                checked={placement === option.id}
                disabled={busy}
                onChange={() => setPlacement(option.id)}
                className="mt-0.5 size-4 shrink-0 accent-[var(--color-accent)]"
              />
              <span className="flex flex-col gap-0.5">
                <span id={`first-run-placement-${option.id}-title`} className="text-sm font-medium text-ink">
                  {option.title}
                  {option.id === "in_place" && <span className="ml-1.5 text-xs font-normal text-muted">Recommended</span>}
                </span>
                <span id={`first-run-placement-${option.id}-detail`} className="text-xs leading-relaxed text-muted">{option.detail}</span>
              </span>
            </label>
          ))}
        </div>
      )}
      {installs.length > 0 && !showOutcome && canInPlace && codexCount > 0 && (
        <p className="-mt-3 text-xs text-muted" data-testid="first-run-codex-copies">
          Codex setups always come in as copies.
        </p>
      )}

      {showOutcome && (
        <div
          className="flex flex-col gap-3 rounded-2xl border border-hairline bg-surface px-5 py-4"
          role="alert"
          data-testid="first-run-outcome"
        >
          <p className="text-[15px] font-medium text-ink">
            {outcomeWords({
              stopped,
              done: doneCount,
              waiting: waiting.length,
              failed: failedRows.length,
              already: alreadyRows.map(labelOf),
            })}
          </p>
          {failedRows.length > 0 && (
            <ul className="flex flex-col gap-2" data-testid="first-run-failed">
              {failedRows.map((found) => (
                <li key={found.path} className="flex flex-wrap items-start justify-between gap-x-4 gap-y-1 text-sm">
                  <span className="min-w-0 flex-1">
                    <span className="font-medium text-ink">{labelOf(found)}</span>
                    <span className="block text-xs leading-relaxed text-danger">{progress[found.path]?.error}</span>
                  </span>
                  {progress[found.path]?.placement === "copy" && found.harness === "claude-code" && (
                    <button
                      type="button"
                      className="shrink-0 text-xs text-accent-hi hover:underline"
                      data-testid="first-run-use-in-place"
                      onClick={() => {
                        setRowPlacement((current) => ({ ...current, [found.path]: "in_place" }));
                        void runImports([found], () => "in_place");
                      }}
                    >
                      Use it where it is instead
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
          <div className="flex flex-wrap items-center justify-end gap-3">
            {failedRows.length > 0 && (
              <Button onClick={() => void runImports(failedRows)} data-testid="first-run-retry">
                {failedRows.length === 1 ? "Try again" : `Try again for ${failedRows.length}`}
              </Button>
            )}
            {stopped && waiting.length > 0 && (
              <Button onClick={() => void runImports(waiting)} data-testid="first-run-rest">
                Bring in the rest
              </Button>
            )}
            <Button variant="primary" onClick={continueNow} data-testid="first-run-continue" ref={continueRef}>
              {doneCount === 0 ? (failedRows.length > 0 || waiting.length > 0 ? "Continue without them" : "Continue") : doneCount === 1 ? "Continue with 1 account" : `Continue with ${doneCount} accounts`}
            </Button>
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-5 text-sm">
          <button type="button" className="text-accent-hi hover:underline" onClick={onAddFresh}>
            {installs.length === 0 ? "Add an account" : "Add a fresh account instead"}
          </button>
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onImportFolder}>
            Import from another folder…
          </button>
          {report.probes.length > 0 && (
            <button
              type="button"
              className="text-muted hover:text-ink hover:underline"
              aria-expanded={showLooked}
              onClick={() => setShowLooked((open) => !open)}
            >
              Where we looked
            </button>
          )}
        </div>
        {installs.length > 0 && !showOutcome && (
          <div className="flex items-center gap-3">
            <span className="hidden text-xs text-faint md:inline">Names and plans can be changed later.</span>
            <Button
              variant="primary"
              disabled={chosen.length === 0 || busy}
              onClick={() => void runImports(chosen)}
              data-testid="first-run-bring-in"
            >
              {busy
                ? "Bringing them in…"
                : chosen.length === 1
                  ? "Bring in 1 account"
                  : `Bring in ${chosen.length} accounts`}
            </Button>
          </div>
        )}
      </div>

      {showLooked && (
        <ul className="flex flex-col gap-1 rounded-xl border border-hairline bg-surface px-4 py-3 text-xs" data-testid="first-run-looked">
          {report.probes.map((probe) => (
            <li key={probe.label} className="leading-relaxed">
              <span className="text-ink">{probe.label}</span>{" "}
              <span className={probe.health === "ok" ? "text-muted" : probe.health === "partial" ? "text-warning" : "text-danger"}>
                {probe.health === "ok" ? "checked" : probe.health === "partial" ? "partly checked" : "couldn't check"}
              </span>
              <span className="text-muted"> — {probe.detail}</span>
              {probe.reason && <span className="block text-muted">{probe.reason}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

type Signin = { accountId: string; label: string; loginNote: string | null; colour: string; provider: string };

/** AJ-E1103 `onboarding.login`: how a first-run sign-in ended (the schema's
 * provider names; anything else is "other"). */
function countLogin(provider: string, outcome: "completed" | "skipped" | "expired"): void {
  const known = ["claude-code", "codex", "zcode"].includes(provider) ? provider : "other";
  statsEvent("onboarding.login", { provider: known, outcome });
}

/** One account's sign-in: status, Sign in, and the rest behind More. */
function SigninRow({
  entry,
  active,
  onStart,
  onReady,
  onSkip,
}: {
  onSkip?: () => void;
  entry: Signin;
  active: boolean;
  onStart: () => void;
  onReady: () => void;
}): JSX.Element {
  const [state, setState] = useState<string | null>("logged_out");
  const [more, setMore] = useState(false);
  const [command, setCommand] = useState<string | null>(null);
  const [skipped, setSkipped] = useState(false);
  const ready = signedIn(state);
  // Every 2 s while waiting (the login-queue card's rule): flips the row
  // the moment the vendor's sign-in completes; never steals focus.
  useEffect(() => {
    if (ready) return;
    let alive = true;
    const tick = () =>
      void accountLoginState(entry.accountId).then(
        (next) => alive && setState(next),
        () => null,
      );
    tick();
    const timer = window.setInterval(tick, 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [entry.accountId, ready]);
  useEffect(() => {
    if (ready) onReady();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);
  return (
    <li className="flex flex-col border-t border-hairline/60 first:border-t-0" data-testid="first-run-signin-row">
      <div className="flex flex-wrap items-center gap-4 px-5 py-3">
        <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: entry.colour }} />
        <span className="min-w-0 flex-1 truncate text-[15px] font-medium text-ink">{entry.label}</span>
        <span className="text-xs" role="status">
          {ready ? (
            <span className="flex items-center gap-1.5 text-success"><Check />Signed in</span>
          ) : skipped ? (
            <span className="text-muted">Later</span>
          ) : active ? (
            <span className="text-live">Finish signing in in the window that opened</span>
          ) : (
            <span className="text-muted">Not signed in yet</span>
          )}
        </span>
        {!ready && (
          <>
            <Button size="sm" variant={active ? "secondary" : "primary"} onClick={onStart}>
              {active ? "Open again" : "Sign in"}{" "}
              <span className="sr-only">{entry.label}</span>
            </Button>
            <button
              type="button"
              className="rounded-md px-2 py-1 text-muted hover:bg-surface-2 hover:text-ink"
              aria-expanded={more}
              aria-label={`More for ${entry.label}`}
              onClick={() => {
                setMore((open) => !open);
                if (command === null) void accountLoginCommand(entry.accountId).then(setCommand, () => setCommand(""));
              }}
            >
              ⋯
            </button>
          </>
        )}
      </div>
      {more && !ready && (
        <div className="flex flex-col gap-2 px-5 pb-3 pl-11 text-xs text-muted">
          {entry.loginNote && <span>{entry.loginNote}</span>}
          {command && (
            <span>
              Or run it yourself: <span className="font-mono text-ink">{command}</span>
            </span>
          )}
          <button type="button" className="self-start text-accent-hi hover:underline" onClick={() => {
            if (!skipped) onSkip?.();
            setSkipped(true);
          }}>
            Sign in later
          </button>
        </div>
      )}
    </li>
  );
}

function SigninStep({
  queue,
  provider,
  openLoginTerminal,
  onDone,
}: {
  provider: string;
  queue: Signin[];
  openLoginTerminal: (id: string, title: string) => void;
  onDone: () => void;
}): JSX.Element {
  const [ready, setReady] = useState<Record<string, boolean>>({});
  const [skippedIds, setSkippedIds] = useState<Record<string, boolean>>({});
  const [active, setActive] = useState<string | null>(null);
  const [runAll, setRunAll] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const left = queue.filter((entry) => !ready[entry.accountId]);
  const start = useCallback(
    (entry: Signin) => {
      setError(null);
      setActive(entry.accountId);
      void terminalSpawnLogin(entry.accountId).then(
        (terminal) => openLoginTerminal(terminal.id, `Sign in · ${entry.label}`),
        (cause) => setError(errorText(cause)),
      );
    },
    [openLoginTerminal],
  );
  const markReady = (entry: Signin) => {
    if (!ready[entry.accountId]) countLogin(entry.provider || provider, "completed");
    setReady((current) => ({ ...current, [entry.accountId]: true }));
    if (runAll) {
      const next = queue.find((other) => other.accountId !== entry.accountId && !ready[other.accountId]);
      if (next) start(next);
      else setRunAll(false);
    }
  };
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-signin">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {left.length === 0
            ? "Everyone's signed in."
            : left.length === 1
              ? "One account needs a quick sign-in."
              : `${left.length} accounts need a quick sign-in.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          {(() => {
            // One tool: its own sign-in by name. First run with every tool
            // (AJ-E1301): each account opens its own tool's sign-in.
            const tools = [...new Set(queue.map((entry) => entry.provider || provider))];
            return tools.length > 1
              ? `Each opens its own tool's sign-in (${tools.map((tool) => `${providerDisplayName(tool)}'s`).join(" or ")}), for that account's folder; AI Juggler never sees your password and notices when you're done.`
              : `Each opens ${providerDisplayName(tools[0] ?? provider)}'s own sign-in, for that account's folder; AI Juggler never sees your password and notices when you're done.`;
          })()}
        </p>
      </div>
      <ul className="overflow-hidden rounded-2xl border border-hairline bg-surface">
        {queue.map((entry) => (
          <SigninRow
            onSkip={() => {
              setSkippedIds((current) => ({ ...current, [entry.accountId]: true }));
              countLogin(entry.provider || provider, "skipped");
            }}
            key={entry.accountId}
            entry={entry}
            active={active === entry.accountId}
            onStart={() => start(entry)}
            onReady={() => markReady(entry)}
          />
        ))}
      </ul>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center justify-between gap-4">
        {left.length > 0 ? (
          <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={() => {
              for (const entry of left) if (!skippedIds[entry.accountId]) countLogin(entry.provider || provider, "skipped");
              onDone();
            }} data-testid="first-run-signin-next">
            Sign in later
          </button>
        ) : (
          <span />
        )}
        {left.length === 0 ? (
          <Button variant="primary" onClick={onDone}>Continue</Button>
        ) : left.length > 1 && !runAll ? (
          <Button
            variant="primary"
            onClick={() => {
              setRunAll(true);
              start(left[0]!);
            }}
          >
            Sign in to all {left.length}, one after another
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function AccessStep({
  entries,
  setEntries,
  onDone,
}: {
  entries: OsAccessInfo[];
  setEntries: (next: OsAccessInfo[]) => void;
  onDone: () => void;
}): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  // Polled while open: an entry flips the moment the system dialog says yes.
  useEffect(() => {
    const timer = window.setInterval(() => {
      void osAccessList().then(setEntries, () => null);
    }, 1500);
    return () => window.clearInterval(timer);
  }, [setEntries]);
  const allow = (entry: OsAccessInfo) => {
    setError(null);
    const step =
      entry.action === "settings" ? osAccessOpenSettings(entry.id).then(() => osAccessList()) : osAccessRequest(entry.id);
    void step.then(setEntries, (cause) => setError(errorText(cause)));
  };
  const allAllowed = entries.every((entry) => entry.status === "allowed");
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-access">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {allAllowed
            ? "All set."
            : entries.length === 1
              ? "One optional permission makes it smoother."
              : `${entries.length} optional permissions make it smoother.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          Allow opens the system's own prompt, and this page updates as soon as you say yes. Skip any of them: AI Juggler asks again only when you first use what it's for.
        </p>
      </div>
      <ul className="flex flex-col gap-3">
        {entries.map((entry) => (
          <li
            key={entry.id}
            className={
              "flex flex-wrap items-center gap-4 rounded-2xl border bg-surface p-5 " +
              (entry.status === "allowed" ? "border-success/40" : "border-hairline")
            }
            data-testid={`first-run-access-${entry.id}`}
          >
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <span className="text-[15px] font-semibold text-ink">{entry.title}</span>
              <span className="text-sm leading-relaxed text-muted">
                {entry.why} <span className="text-faint">The system calls this {entry.system_name}.</span>
              </span>
              {entry.note && entry.status !== "allowed" && <span className="text-xs text-faint">{entry.note}</span>}
            </div>
            {entry.status === "allowed" ? (
              <span className="flex items-center gap-1.5 text-sm font-medium text-success" role="status">
                <Check />
                Allowed
              </span>
            ) : (
              <Button variant={entry.action === "settings" ? "secondary" : "primary"} onClick={() => allow(entry)}>
                {entry.action === "settings" ? "Open System Settings" : "Allow"}{" "}
                <span className="sr-only">{entry.title}</span>
              </Button>
            )}
          </li>
        ))}
      </ul>
      {error && (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      <div className="flex items-center justify-between gap-4">
        {!allAllowed ? (
          <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={onDone}>
            Skip for now
          </button>
        ) : (
          <span />
        )}
        <Button variant="primary" onClick={onDone} data-testid="first-run-start">
          Start working
        </Button>
      </div>
    </div>
  );
}

/** The plain error for a failed import (duplicate, moved, or the rest). */
function importProblem(cause: unknown): { kind: "duplicate" | "moved" | "other"; text: string } {
  const text = errorText(cause);
  if ((cause as { code?: string })?.code === "already_imported" || text.includes("Already imported as")) {
    return { kind: "duplicate", text };
  }
  if (text.includes("not a directory") || text.includes("does not exist") || text.includes("may have moved")) {
    return { kind: "moved", text: "That folder moved or is gone. Pick it again." };
  }
  return { kind: "other", text };
}

function FreshStep({
  toolName,
  provider,
  taken,
  canGoBack,
  onBack,
  onImportFolder,
  onCreated,
}: {
  toolName: string;
  provider: string;
  taken: string[] | null;
  canGoBack: boolean;
  onBack: () => void;
  onImportFolder: () => void;
  onCreated: (signin: Signin | null) => void;
}): JSX.Element {
  const [label, setLabel] = useState<string | null>(null);
  const shown = label ?? (taken === null ? "" : freeLabel(toolName, taken));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const account = await createAccount(provider, shown.trim(), null);
      onCreated(
        signedIn(account.login_state)
          ? null
          : { accountId: account.id, label: account.label, loginNote: null, colour: colourAt(taken?.length ?? 0), provider },
      );
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <form
      className="flex flex-col gap-6"
      data-testid="first-run-fresh"
      onSubmit={(event) => {
        event.preventDefault();
        if (shown.trim().length > 0 && !busy) void create();
      }}
    >
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">Add a fresh account.</h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          AI Juggler sets it up in its own folder, apart from anything you already use. Next you sign in with{" "}
          {toolName}'s own sign-in; AI Juggler never sees your password.
        </p>
      </div>
      <label className="flex max-w-md flex-col gap-1.5 text-sm font-medium text-ink">
        Name
        <input
          autoFocus
          value={shown}
          onChange={(event) => setLabel(event.target.value)}
          placeholder="e.g. Work, Personal"
          className="rounded-lg border border-hairline bg-surface px-3 py-2.5 text-[15px] font-normal text-ink outline-none placeholder:text-faint focus:border-accent/60"
          data-testid="first-run-fresh-name"
        />
        <span className="text-xs font-normal text-faint">What it's for — you can rename it and set its plan later in Accounts.</span>
      </label>
      {error && (
        <p className="text-xs text-danger" role="alert" data-testid="first-run-fresh-error">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-5 text-sm">
          {canGoBack && (
            <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onBack}>
              Back
            </button>
          )}
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onImportFolder}>
            Import from a folder instead…
          </button>
        </div>
        <Button variant="primary" type="submit" disabled={busy || shown.trim().length === 0} data-testid="first-run-fresh-create">
          {busy ? "Setting it up…" : "Create and sign in"}
        </Button>
      </div>
    </form>
  );
}

function FolderStep({
  path,
  provider,
  toolName,
  onBack,
  onPickAgain,
  onAddFresh,
  onImported,
  onDone,
}: {
  path: string;
  provider: string;
  toolName: string;
  onBack: () => void;
  onPickAgain: () => void;
  onAddFresh: () => void;
  onImported: (signin: Signin | null) => void;
  onDone: () => void;
}): JSX.Element {
  const [inspect, setInspect] = useState<SourceInspectInfo | null>(null);
  const [inspectError, setInspectError] = useState<string | null>(null);
  const folder = folderOf(path);
  const [label, setLabel] = useState(folder === ".claude" ? "Default" : folder.replace(/^\./, ""));
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<ReturnType<typeof importProblem> | null>(null);
  useEffect(() => {
    let alive = true;
    setInspect(null);
    setInspectError(null);
    void sourceInspect(provider, path).then(
      (found) => alive && setInspect(found),
      (cause) => alive && setInspectError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
  }, [provider, path]);
  const large = inspect !== null && inspect.total_bytes > inspect.ceiling_bytes;
  const bringIn = async (degraded: boolean) => {
    setBusy(true);
    setProblem(null);
    funnelEvent("import_started");
    try {
      const result = await accountImport(provider, path, label.trim(), null, "manual", degraded);
      if (result.login === "expired") countLogin(provider, "expired");
      funnelEvent("import_completed", { degraded: result.degraded === true });
      onImported(
        signedIn(result.login)
          ? null
          : { accountId: result.account.id, label: result.account.label, loginNote: result.login_note, colour: colourAt(0), provider },
      );
    } catch (cause) {
      funnelEvent("import_failed", { reason_code: reasonCode(cause) });
      setProblem(importProblem(cause));
      setBusy(false);
    }
  };
  const shortPath = path.replace(/^\/Users\/[^/]+/, "~");
  const usable = inspect !== null && inspect.is_config && !inspect.already_managed;
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-folder">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">Bring in this folder?</h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          AI Juggler makes its own copy; the folder itself isn't changed, and you can keep using it as before.
        </p>
      </div>
      <div className="flex flex-col gap-3 rounded-2xl border border-hairline bg-surface px-5 py-4" data-testid="first-run-folder-check">
        <span className="truncate font-mono text-xs text-muted" title={path}>
          {shortPath}
        </span>
        {inspect === null && inspectError === null && (
          <span className="text-sm text-muted" role="status">
            Checking what's in it…
          </span>
        )}
        {inspectError && (
          <span className="text-sm text-danger" role="alert">
            Couldn't check this folder: {inspectError}
          </span>
        )}
        {inspect !== null && (
          <>
            <span className="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-ink" role="status">
              <span>
                {inspect.file_count.toLocaleString()} files · {humanBytes(inspect.total_bytes)}
                {inspect.truncated ? "+" : ""}
              </span>
              {inspect.is_config &&
                (signedIn(inspect.login) ? (
                  <span className="flex items-center gap-1.5 text-success">
                    <Check />
                    Signed in
                  </span>
                ) : (
                  <span className="text-muted">Needs sign-in</span>
                ))}
            </span>
            {inspect.keychain_bound && (
              <span className="text-xs leading-relaxed text-muted" data-testid="first-run-folder-keychain">
                Its sign-in is kept in the macOS Keychain for the original folder, so you'll sign in once more after
                bringing it in.
              </span>
            )}
            {inspect.folders.length > 0 && (
              <details className="text-xs text-muted">
                <summary className="cursor-pointer outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
                  What's in it
                </summary>
                <ul className="mt-2 flex flex-col gap-0.5" data-testid="source-breakdown">
                  {inspect.folders.slice(0, 5).map((entry) => (
                    <li key={entry.name} className="flex max-w-md items-center justify-between gap-4">
                      <span className="truncate font-mono">{entry.name}</span>
                      <span>{humanBytes(entry.bytes)}</span>
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </>
        )}
      </div>
      {inspect !== null && !inspect.is_config && (
        <p className="rounded-xl border border-danger/30 bg-danger-soft px-4 py-3 text-sm text-ink" role="alert" data-testid="source-not-config">
          This folder doesn't look like a {toolName} setup: no settings, sign-in or session history in it.
        </p>
      )}
      {inspect !== null && inspect.is_config && inspect.already_managed && (
        <p className="rounded-xl border border-hairline px-4 py-3 text-sm text-ink" role="status" data-testid="first-run-folder-managed">
          This folder is already in AI Juggler.
        </p>
      )}
      {usable && large && (
        <p className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" role="status" data-testid="first-run-folder-large">
          Its session history is large ({humanBytes(inspect.total_bytes)}). Bring in the settings only, or add a fresh
          account instead.
        </p>
      )}
      {usable && (
        <label className="flex max-w-md flex-col gap-1.5 text-sm font-medium text-ink">
          Name
          <input
            value={label}
            onChange={(event) => setLabel(event.target.value)}
            className="rounded-lg border border-hairline bg-surface px-3 py-2.5 text-[15px] font-normal text-ink outline-none focus:border-accent/60"
            data-testid="first-run-folder-name"
          />
        </label>
      )}
      {problem && (
        <div className="flex flex-wrap items-center gap-3 text-sm" role="alert" data-testid={`first-run-folder-${problem.kind}`}>
          <span className={problem.kind === "other" ? "text-danger" : "text-ink"}>{problem.text}</span>
          {problem.kind === "duplicate" && (
            <Button size="sm" variant="secondary" onClick={onDone}>
              Done
            </Button>
          )}
        </div>
      )}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-5 text-sm">
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onBack}>
            Back
          </button>
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onPickAgain} data-testid="first-run-folder-again">
            Pick a different folder…
          </button>
          {large && (
            <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onAddFresh}>
              Add a fresh account instead
            </button>
          )}
        </div>
        {usable && (
          <Button
            variant="primary"
            disabled={busy || label.trim().length === 0}
            onClick={() => void bringIn(large)}
            data-testid="first-run-folder-bring-in"
          >
            {busy ? "Bringing it in…" : large ? "Bring in settings only" : "Bring it in"}
          </Button>
        )}
      </div>
    </div>
  );
}

export function FirstRun({
  onDone,
  openLoginTerminal,
  onAccountsChanged,
  mode = "first-run",
  provider,
}: {
  /** First run is over: Home. */
  onDone: () => void;
  /** D-25: "add" reuses the same three steps from Accounts › Add
   * account — closable, and not counted as a first install. */
  mode?: "first-run" | "add";
  /** AJ-E1023: the tool the account is for (chosen before, when more
   * than one is installed). Absent — first run on a fresh install — the
   * list brings in every supported tool's setups (AJ-E1301). */
  provider?: string;
  /** An account was created or brought in (the caller refreshes). */
  onAccountsChanged?: () => void;
  openLoginTerminal: (id: string, title: string) => void;
}): JSX.Element {
  const [step, setStep] = useState<Step>("found");
  const [folder, setFolder] = useState<string | null>(null);
  const [taken, setTaken] = useState<string[] | null>(null);
  // AJ-E1301: adding names its tool; first run on a fresh install brings
  // in every supported tool's setups (a fresh account or a folder import
  // there is Claude Code's, as before).
  const primary = provider ?? "claude-code";
  const tools = useMemo(() => (provider ? [provider] : FIRST_RUN_TOOLS), [provider]);
  const toolName = providerDisplayName(primary);
  const [report, setReport] = useState<ScanReportData | null>(null);
  const [cli, setCli] = useState<ProviderInstallData | null>(null);
  const [codexCli, setCodexCli] = useState<ProviderInstallData | null>(null);
  const [queue, setQueue] = useState<Signin[]>([]);
  const [access, setAccess] = useState<OsAccessInfo[] | null>(null);
  // ADR-0018 §5: while a bring-in runs, Close and Esc mean Stop — the
  // setup in progress finishes cleanly and what came in stays.
  const stopRef = useRef(false);
  const [importing, setImporting] = useState(false);
  const [stopping, setStopping] = useState(false);
  const leave = useCallback(() => {
    if (importing) {
      stopRef.current = true;
      setStopping(true);
      return;
    }
    onDone();
  }, [importing, onDone]);

  useEffect(() => {
    if (mode === "first-run") funnelStep("a-first-install", "found");
    funnelEvent("scan_started");
    void environmentScanFull().then(
      (scan) => {
        setReport(scan);
        funnelEvent("scan_completed");
      },
      () => setReport({ installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } as unknown as ScanReportData),
    );
    void providersDiscover().then(
      (rows) => {
        setCli(rows.find((row) => row.provider === primary) ?? null);
        // First run with every tool: Codex's own row for its group.
        setCodexCli(
          tools.includes("codex") && primary !== "codex"
            ? (rows.find((row) => row.provider === "codex") ?? null)
            : null,
        );
      },
      () => setCli(null),
    );
    void accountsList().then(
      (rows) => setTaken(rows.map((row) => row.label)),
      () => setTaken([]),
    );
    void osAccessList().then(setAccess, () => setAccess([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- once, on open
  }, []);

  // A modal dialog takes focus when it opens (WCAG 2.4.3) and can be left
  // with Escape from anywhere in it.
  const frameRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    frameRef.current?.focus();
  }, []);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      // A popover inside that already took the Escape keeps the screen open.
      if (event.key === "Escape" && !event.defaultPrevented) leave();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [leave]);

  // D-25: adding, with nothing new on this Mac to bring in — go straight
  // to a fresh account rather than show an empty list.
  const nothingNew = report !== null && !report.installs.some((found) => tools.includes(found.harness) && !found.managed);
  const autoFresh = useRef(false);
  useEffect(() => {
    if (mode === "add" && nothingNew && !autoFresh.current) {
      autoFresh.current = true;
      setStep("fresh");
    }
  }, [mode, nothingNew]);

  // "Import from another folder…": the system's picker first; nothing
  // changes when it is cancelled.
  const pickFolder = useCallback(async () => {
    const picked = await pickDirectory();
    if (picked === null) return;
    setFolder(picked);
    setStep("folder");
  }, []);

  // Nothing to ask for: this OS needs nothing, or it is all allowed.
  const skipAccess = access !== null && access.every((entry) => entry.status === "allowed");
  const finish = useCallback(() => {
    if (mode === "first-run") funnelStep("a-first-install", "done");
    onDone();
  }, [onDone, mode]);
  // One account made or brought in: its sign-in, or straight on.
  const added = (signin: Signin | null) => {
    onAccountsChanged?.();
    if (signin) {
      setQueue([signin]);
      funnelEvent("login_queued");
      if (mode === "first-run") funnelStep("a-first-install", "signin");
      setStep("signin");
    } else {
      toAccess();
    }
  };
  const toAccess = useCallback(() => {
    if (skipAccess) {
      finish();
      return;
    }
    if (mode === "first-run") funnelStep("a-first-install", "access");
    setStep("access");
  }, [finish, skipAccess, mode]);

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col overflow-y-auto bg-canvas text-ink outline-none"
      role="dialog"
      aria-modal="true"
      aria-label={mode === "add" ? "Add accounts" : "Set up AI Juggler"}
      data-testid="first-run"
      data-mode={mode}
      ref={frameRef}
      tabIndex={-1}
    >
      <div data-tauri-drag-region className="flex h-11 shrink-0 items-center justify-end px-4">
        {/* Always a visible way out (0.0.7 dogfood: "no way to close"):
            first run → Home, which offers "Find my accounts" to come back. */}
        <Button
          size="sm"
          variant="secondary"
          onClick={leave}
          disabled={stopping}
          aria-keyshortcuts="Escape"
          data-testid="first-run-close"
        >
          {stopping ? "Stopping after this one…" : importing ? "Stop" : mode === "add" ? "Close" : "Not now"}
        </Button>
      </div>
      <div className="flex flex-1 justify-center px-6 pb-16 pt-6">
        <div className="flex w-full max-w-3xl flex-col gap-7">
          <StepMarks step={step} skipAccess={skipAccess} />
          {step === "found" && (
            <FoundStep
              report={report}
              cli={cli}
              toolName={toolName}
              provider={primary}
              tools={tools}
              codexCli={codexCli}
              onAddFresh={() => setStep("fresh")}
              onImportFolder={() => void pickFolder()}
              stopRef={stopRef}
              onBusy={(busy) => {
                setImporting(busy);
                if (!busy) setStopping(false);
              }}
              onImported={(signins) => {
                onAccountsChanged?.();
                setQueue(signins);
                if (signins.length > 0) {
                  funnelEvent("login_queued");
                  if (mode === "first-run") funnelStep("a-first-install", "signin");
                  setStep("signin");
                } else {
                  toAccess();
                }
              }}
            />
          )}
          {step === "fresh" && (
            <FreshStep
              toolName={toolName}
              provider={primary}
              taken={taken}
              canGoBack={!nothingNew || mode === "first-run"}
              onBack={() => setStep("found")}
              onImportFolder={() => void pickFolder()}
              onCreated={added}
            />
          )}
          {step === "folder" && folder !== null && (
            <FolderStep
              path={folder}
              provider={primary}
              toolName={toolName}
              onBack={() => setStep(mode === "add" && nothingNew ? "fresh" : "found")}
              onPickAgain={() => void pickFolder()}
              onAddFresh={() => setStep("fresh")}
              onImported={added}
              onDone={onDone}
            />
          )}
          {step === "signin" && <SigninStep queue={queue} provider={primary} openLoginTerminal={openLoginTerminal} onDone={toAccess} />}
          {step === "access" && access !== null && <AccessStep entries={access} setEntries={setAccess} onDone={finish} />}
        </div>
      </div>
    </div>
  );
}
