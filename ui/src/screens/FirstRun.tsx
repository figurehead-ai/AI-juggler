/**
 * D-25 — first run, when no Account exists: three calm steps in one
 * window, as the vision canvas shows them.
 *
 * 1. **What's already here.** The scan, grouped the way the user thinks of
 *    it — Claude desktop app profiles, Claude Code in the terminal, other
 *    setups — preselected (signed in or used in the last 30 days), one
 *    button to bring them in. Imports are copies; originals are never
 *    touched. "Where we looked" shows every place the scan checked.
 * 2. **Sign in where needed.** One card per copy that still needs the
 *    vendor's own sign-in (the shared login-queue card).
 * 3. **OS access.** Whatever this platform lists (D-25), each with one line
 *    of why and one Allow; polled so an entry flips the moment the user
 *    says yes. An empty list skips the step.
 *
 * The pack-secrets choice is no longer asked here: the first switch asks
 * while no choice exists (D-14, nothing is built before it).
 */
import { useCallback, useEffect, useMemo, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { funnelEvent, funnelStep } from "../lib/funnel";
import {
  accountImport,
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
  type ProviderInstallData,
  type ScanReportData,
} from "../lib/bridge";

type Step = "found" | "signin" | "access";
type Group = "desktop" | "terminal" | "other";

const GROUPS: Array<{ id: Group; title: string }> = [
  { id: "desktop", title: "Claude desktop app profiles" },
  { id: "terminal", title: "Claude Code in the terminal" },
  { id: "other", title: "Other Claude setups" },
];

/** Where a found setup belongs, as the user thinks of it. */
export function groupOf(found: DiscoveredInfo): Group {
  if (found.path.includes("/ClaudeCodeProfiles/")) return "desktop";
  if (found.origin === "vendor-default" || found.origin === "env-override") return "terminal";
  return "other";
}

const folderOf = (path: string) => path.replace(/\/+$/, "").split("/").pop() ?? path;

/** The name it's brought in as (changeable later in Accounts). */
export function labelOf(found: DiscoveredInfo): string {
  const folder = folderOf(found.path);
  if (groupOf(found) === "terminal" && folder === ".claude") return "Terminal";
  return folder.replace(/^\./, "");
}

/** "In use now" / "Used today" / "Used 3 days ago" / the scan's words. */
export function usedWords(found: DiscoveredInfo, nowSecs: number): string {
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
  const steps: Array<{ id: Step; label: string }> = [
    { id: "found", label: "Bring in your accounts" },
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

type ImportState = { state: "pending" | "working" | "done" | "failed"; error?: string };

function FoundStep({
  report,
  cli,
  onImported,
  onAddFresh,
  onImportFolder,
}: {
  report: ScanReportData | null;
  cli: ProviderInstallData | null;
  onImported: (signins: Signin[]) => void;
  onAddFresh: () => void;
  onImportFolder: () => void;
}): JSX.Element {
  const now = useMemo(nowSecs, []);
  const installs = useMemo(
    () => (report?.installs ?? []).filter((found) => found.harness === "claude-code"),
    [report],
  );
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

  const chosen = installs.filter((found) => picked[found.path] && !found.managed);
  const bringIn = async () => {
    setBusy(true);
    funnelEvent("import_started");
    const signins: Signin[] = [];
    let failed = 0;
    for (const found of chosen) {
      setProgress((current) => ({ ...current, [found.path]: { state: "working" } }));
      try {
        const result = await accountImport(found.harness, found.path, labelOf(found), null, found.origin);
        if (result.login !== "logged_in") {
          signins.push({
            accountId: result.account.id,
            label: result.account.label,
            loginNote: result.login_note,
            colour: colourAt(installs.indexOf(found)),
          });
        }
        setProgress((current) => ({ ...current, [found.path]: { state: "done" } }));
      } catch (cause) {
        const code = (cause as { code?: string })?.code;
        if (code === "already_imported") {
          setProgress((current) => ({ ...current, [found.path]: { state: "done" } }));
        } else {
          failed += 1;
          setProgress((current) => ({ ...current, [found.path]: { state: "failed", error: errorText(cause) } }));
        }
      }
    }
    funnelEvent(failed === 0 ? "import_completed" : "import_failed");
    setBusy(false);
    if (failed === 0) onImported(signins);
  };
  const failedAny = Object.values(progress).some((row) => row.state === "failed");

  if (report === null) {
    return (
      <div className="flex flex-col gap-3" role="status" data-testid="first-run-scanning">
        <h1 className="text-[28px] font-semibold tracking-tight">Looking for Claude Code on this Mac…</h1>
        <p className="text-sm text-muted">Your standard setup, Claude desktop profiles and anything else that looks like one. Nothing is changed.</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6" data-testid="first-run-found">
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">
          {installs.length === 0
            ? "No Claude Code setups found yet."
            : installs.length === 1
              ? "You already have a Claude Code setup on this Mac."
              : `You already have ${installs.length} Claude Code setups on this Mac.`}
        </h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          {installs.length === 0
            ? "Add your first account and AI Juggler sets it up in its own folder. You'll sign in with the provider's own sign-in."
            : "Pick the ones to bring in. AI Juggler makes its own copy of each; your originals are never changed, and you can keep using them as before."}
        </p>
      </div>

      {cli && cli.status === "not_installed" && (
        <p className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" data-testid="first-run-no-cli">
          Claude Code itself isn't installed yet. You can bring these in now; to start sessions, install it first:{" "}
          <span className="font-mono text-xs">npm install -g @anthropic-ai/claude-code</span>
        </p>
      )}

      {installs.length > 0 && (
        <div className="overflow-hidden rounded-2xl border border-hairline bg-surface">
          {GROUPS.map((group) => {
            const rows = installs.filter((found) => groupOf(found) === group.id);
            if (rows.length === 0) return null;
            return (
              <fieldset key={group.id} className="border-b border-hairline last:border-b-0" data-testid={`first-run-group-${group.id}`}>
                <legend className="sr-only">{group.title}</legend>
                <div aria-hidden="true" className="flex items-center justify-between gap-3 bg-surface-2 px-5 py-2.5 text-xs">
                  <span className="font-semibold text-ink">
                    {group.title} <span className="font-normal text-faint">· {rows.length}</span>
                  </span>
                  {group.id === "terminal" && cli && cli.status !== "not_installed" && (
                    <span className="text-faint">Claude Code {cli.version ?? ""} installed</span>
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
                      <span className={"shrink-0 text-xs " + (usedWords(found, now) === "In use now" ? "text-live" : "text-muted")}>
                        {usedWords(found, now)}
                      </span>
                      <span className="flex w-32 shrink-0 items-center justify-end gap-1.5 text-xs">
                        {row?.state === "working" ? (
                          <span className="text-muted">Bringing in…</span>
                        ) : row?.state === "done" ? (
                          <span className="flex items-center gap-1.5 text-success"><Check />Brought in</span>
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

      {failedAny && (
        <ul className="flex flex-col gap-1 text-xs text-danger" role="alert">
          {installs
            .filter((found) => progress[found.path]?.state === "failed")
            .map((found) => (
              <li key={found.path}>
                {labelOf(found)}: {progress[found.path]?.error}
              </li>
            ))}
        </ul>
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
        {installs.length > 0 && (
          <div className="flex items-center gap-3">
            <span className="hidden text-xs text-faint md:inline">Names and plans can be changed later.</span>
            <Button
              variant="primary"
              disabled={chosen.length === 0 || busy}
              onClick={() => void bringIn()}
              data-testid="first-run-bring-in"
            >
              {busy
                ? "Bringing them in…"
                : failedAny
                  ? "Try again"
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

type Signin = { accountId: string; label: string; loginNote: string | null; colour: string };

/** One account's sign-in: status, Sign in, and the rest behind More. */
function SigninRow({
  entry,
  active,
  onStart,
  onReady,
}: {
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
          <button type="button" className="self-start text-accent-hi hover:underline" onClick={() => setSkipped(true)}>
            Sign in later
          </button>
        </div>
      )}
    </li>
  );
}

function SigninStep({
  queue,
  openLoginTerminal,
  onDone,
}: {
  queue: Signin[];
  openLoginTerminal: (id: string, title: string) => void;
  onDone: () => void;
}): JSX.Element {
  const [ready, setReady] = useState<Record<string, boolean>>({});
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
          macOS ties each Claude sign-in to its own folder, so every copy signs in once. Each opens Claude's own sign-in; AI Juggler never sees your password and notices when you're done.
        </p>
      </div>
      <ul className="overflow-hidden rounded-2xl border border-hairline bg-surface">
        {queue.map((entry) => (
          <SigninRow
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
          <button type="button" className="text-sm text-muted hover:text-ink hover:underline" onClick={onDone} data-testid="first-run-signin-next">
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

export function FirstRun({
  onDone,
  onAddFresh,
  onImportFolder,
  openLoginTerminal,
}: {
  /** First run is over: Home. */
  onDone: () => void;
  /** "Add a fresh account" — the account wizard. */
  onAddFresh: () => void;
  /** "Import from another folder…" — the wizard's folder import. */
  onImportFolder: () => void;
  openLoginTerminal: (id: string, title: string) => void;
}): JSX.Element {
  const [step, setStep] = useState<Step>("found");
  const [report, setReport] = useState<ScanReportData | null>(null);
  const [cli, setCli] = useState<ProviderInstallData | null>(null);
  const [queue, setQueue] = useState<Signin[]>([]);
  const [access, setAccess] = useState<OsAccessInfo[] | null>(null);

  useEffect(() => {
    funnelStep("a-first-install", "found");
    funnelEvent("scan_started");
    void environmentScanFull().then(
      (scan) => {
        setReport(scan);
        funnelEvent("scan_completed");
      },
      () => setReport({ installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } as unknown as ScanReportData),
    );
    void providersDiscover().then(
      (rows) => setCli(rows.find((row) => row.provider === "claude-code") ?? null),
      () => setCli(null),
    );
    void osAccessList().then(setAccess, () => setAccess([]));
  }, []);

  const skipAccess = access !== null && access.length === 0;
  const finish = useCallback(() => {
    funnelStep("a-first-install", "done");
    onDone();
  }, [onDone]);
  const toAccess = useCallback(() => {
    if (skipAccess) {
      finish();
      return;
    }
    funnelStep("a-first-install", "access");
    setStep("access");
  }, [finish, skipAccess]);

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col overflow-y-auto bg-canvas text-ink"
      role="dialog"
      aria-modal="true"
      aria-label="Set up AI Juggler"
      data-testid="first-run"
    >
      <div data-tauri-drag-region className="h-11 shrink-0" />
      <div className="flex flex-1 justify-center px-6 pb-16 pt-6">
        <div className="flex w-full max-w-3xl flex-col gap-7">
          <StepMarks step={step} skipAccess={skipAccess} />
          {step === "found" && (
            <FoundStep
              report={report}
              cli={cli}
              onAddFresh={onAddFresh}
              onImportFolder={onImportFolder}
              onImported={(signins) => {
                setQueue(signins);
                if (signins.length > 0) {
                  funnelEvent("login_queued");
                  funnelStep("a-first-install", "signin");
                  setStep("signin");
                } else {
                  toAccess();
                }
              }}
            />
          )}
          {step === "signin" && <SigninStep queue={queue} openLoginTerminal={openLoginTerminal} onDone={toAccess} />}
          {step === "access" && access !== null && <AccessStep entries={access} setEntries={setAccess} onDone={finish} />}
        </div>
      </div>
    </div>
  );
}
