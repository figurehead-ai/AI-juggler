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
import { ToolMark } from "../components/ToolMark";
import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { inPlaceProviders, ownSignIn, providerDisplayName, sameAsWords, shortSignIn, supportsInPlace, toolFound, type InPlaceSupport } from "../lib/providers";
import { firstRunToolIds, primaryToolId, toolSpec } from "../lib/tools";
import { InstallHow } from "../components/InstallHow";
import { AccessStep } from "../components/AccessStep";
import { Check } from "../components/StepCheck";
import { colourAt, FreshAccountStep, freeLabel } from "./FreshAccountStep";
import { StepMarks, TermsLine, type Step } from "./FirstRunSteps";
import { FigureheadStep } from "./FigureheadStep";
import { funnelEvent, funnelStep, reasonCode } from "../lib/funnel";
import { foundWords, useSetupScan } from "./firstRunScan";
import { FoundRow } from "./FirstRunFoundRow";
import { outcomeWords, plainImportError } from "./firstRunScan";
import { cliWords, DesktopApps, folderOf, freshKey, FreshGroups, labelOf, setUpFresh, setupNames, useFresh } from "./FirstRunMore";

export { desktopNoneWords, folderOf, labelOf, setupNames } from "./FirstRunMore";
export { outcomeWords, plainImportError } from "./firstRunScan";
import { accountImport, sourceInspect } from "../lib/ipc/account-import";
import { accountsList, accountLoginCommand, accountLoginState } from "../lib/ipc/accounts";
import { pickDirectory } from "../lib/ipc/project-folders";
import { statsEvent } from "../lib/ipc/usage-stats";
import { errorText, signedIn } from "../lib/bridge";
import { osAccessList } from "../lib/ipc/settings";
import { providersCapabilities, providersDiscover, type ProviderInstallData, type ScanReportData } from "../lib/ipc/providers";
import { terminalSpawnLogin } from "../lib/ipc/terminal-io";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";
import type { SourceInspectInfo } from "../types/generated/SourceInspectInfo";
import { byteSize } from "../lib/compact";
import { Radio } from "../components/controls";
import { lazyNamed } from "../lib/lazy";

// The sign-in terminal shows in the step itself (0.4.1: it opened behind
// this full-window flow); xterm loads only when a sign-in starts.
const TerminalView = lazyNamed(() => import("../components/TerminalView"), "TerminalView");

/** AJ-E1525: first run's groups come from each tool's spec (its
 * `setup_groups`, in order), never from a table here. */
type Group = { key: string; id: string; tool: string; title: string; showsInstall: boolean };


/** The groups for these tools, in the tools' order then each spec's. */
export function groupsFor(tools: string[]): Group[] {
  return tools.flatMap((tool) =>
    toolSpec(tool).setup_groups.map((group) => ({
      key: `${tool}:${group.id}`,
      id: group.id,
      tool,
      title: group.title,
      showsInstall: group.shows_install,
    })),
  );
}

/** Grouped by kind of setup, as the tool's spec defines them: the first
 * of its groups that matches the setup's path or scan origin, else its
 * catch-all (the 0.0.6 dogfood: by kind of setup, not by app). */
export function groupOf(found: DiscoveredInfo): string {
  const groups = toolSpec(found.harness).setup_groups;
  const match =
    groups.find((group) => group.path_contains !== null && found.path.includes(group.path_contains)) ??
    groups.find((group) => group.origin !== null && found.origin === group.origin) ??
    groups.find((group) => group.path_contains === null && group.origin === null) ??
    groups[0];
  return `${found.harness}:${match?.id ?? "setups"}`;
}

/** "In use now" / "Used today" / "Used 3 days ago" / the scan's words. */
export function usedWords(found: DiscoveredInfo, nowSecs: number): string {
  // Open right now, and where: read from the running processes.
  const open = found.in_use ?? [];
  const app = toolSpec(found.harness).desktop_app;
  const inApp = app ? `the ${app} app` : "its desktop app";
  if (open.includes("desktop") && open.includes("terminal")) return `Open in ${inApp} and a terminal`;
  if (open.includes("desktop")) return `Open in ${inApp}`;
  if (open.includes("terminal")) return "Running in a terminal";
  const last = found.last_used_secs;
  // No conversations, so no age: never an archive tier's words
  // ("Archive candidate · 0 d" on a setup made today, AJ-E3306 PF1).
  if (last === null || last === undefined) return "Not used yet";
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


type ImportState = {
  state: "pending" | "working" | "done" | "already" | "failed";
  error?: string;
  placement?: Placement;
};

/** ADR-0018: how a setup comes in. */
export type Placement = "in_place" | "copy";


/** The two ways in; `tools` are the ones that can be used where they
 * are (ADR-0018), which name what "the same" is the same as. */
function placements(tools: string[], one: boolean): { id: Placement; title: string; detail: string }[] {
  return [
    {
      id: "in_place",
      title: one ? "Use it where it is" : "Use them where they are",
      detail: `Same sign-in, settings and history ${sameAsWords(tools)}. AI Juggler adds nothing of its own to ${one ? "it" : "them"}.`,
    },
    {
      id: "copy",
      title: one ? "Make a separate copy" : "Make separate copies",
      detail: one
        ? "A fresh copy for AI Juggler only; your original stays as it is. The copy needs one sign-in."
        : "A fresh copy for AI Juggler only; your originals stay as they are. Each copy needs one sign-in.",
    },
  ];
}

function FoundStep({
  report,
  scanError,
  onScanAgain,
  cli,
  toolName,
  provider,
  tools,
  installRows,
  onImported,
  onAddFresh,
  onImportFolder,
  stopRef,
  onBusy,
  inPlace,
  taken,
}: {
  report: ScanReportData | null;
  /** The names of the Apps already in AI Juggler (`null` while read). */
  taken: string[] | null;
  /** Why looking for setups failed, when it did. */
  scanError: string | null;
  onScanAgain: () => void;
  /** ADR-0018: the tools that can be used where they are (from the
   * backend's provider rows); `null` while they load. */
  inPlace: InPlaceSupport | null;
  cli: ProviderInstallData | null;
  toolName: string;
  provider: string;
  /** The tools whose setups this list brings in: the chosen one when
   * adding, every supported one on first run (AJ-E1301). */
  tools: string[];
  /** Each tool's install row (first run's other tools ride along). */
  installRows: Record<string, ProviderInstallData>;
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
  // Rows of the chosen tool itself (its "not installed" note covers these).
  const ownCount = installs.filter((found) => found.harness === provider).length;
  // One tool: its name. Every tool (first run): named when the setups are
  // all one tool's, else just "setups".
  const found = [...new Set(installs.map((row) => row.harness))];
  const kind = tools.length === 1 ? toolName : found.length === 0 ? toolName : found.length === 1 ? providerDisplayName(found[0]!) : "";
  // The other tools riding along whose setups are here but which aren't
  // installed themselves.
  const missingAlong = tools.filter(
    (tool) => tool !== provider && installs.some((row) => row.harness === tool) && installRows[tool]?.status === "not_installed",
  );
  // The defaults come straight from the scan (ready on first paint);
  // the user's ticks override them. Only first run (every tool) ticks
  // them for you: adding an app for one tool with several setups found
  // starts with none ticked, so bringing in one doesn't mean unticking the
  // rest (AJ-E3514 F5); a lone setup is still ticked.
  const defaults = useMemo(
    () => Object.fromEntries(installs.map((found) => [found.path, (tools.length > 1 || installs.length === 1) && preselected(found, now)])),
    [installs, now, tools.length],
  );
  const [overrides, setPicked] = useState<Record<string, boolean>>({});
  const picked: Record<string, boolean> = { ...defaults, ...overrides };
  const names = useMemo(() => setupNames(installs, taken ?? []), [installs, taken]);
  const nameOf = (row: DiscoveredInfo) => names.get(row.path) ?? labelOf(row);
  const { fresh, freshNames, freshPicked, setFreshPicked } = useFresh(report, tools, taken, names);
  const [progress, setProgress] = useState<Record<string, ImportState>>({});
  const [busy, setBusy] = useState(false);
  const [showLooked, setShowLooked] = useState(false);

  // ADR-0018: only a tool whose adapter can use a folder where it is
  // offers it — and only when one of its setups is on the list.
  const canUseInPlace = (id: string) => inPlace !== null && supportsInPlace(inPlace, id);
  const inPlaceTools = [...new Set(installs.map((found) => found.harness).filter(canUseInPlace))];
  const copyOnlyTools = [...new Set(installs.map((found) => found.harness).filter((id) => !canUseInPlace(id)))];
  const canInPlace = inPlaceTools.length > 0;
  const [chosenPlacement, setPlacement] = useState<Placement>("in_place");
  const placement: Placement = canInPlace ? chosenPlacement : "copy";
  // A failed copy can be retried "where it is" on its own.
  const [rowPlacement, setRowPlacement] = useState<Record<string, Placement>>({});
  // A tool that can't be used where it is always comes in as a copy.
  const placementOf = (found: DiscoveredInfo): Placement =>
    canUseInPlace(found.harness) ? (rowPlacement[found.path] ?? placement) : "copy";
  // Sign-ins of the ones that came in survive retries.
  const signinsRef = useRef<Map<string, Signin>>(new Map());
  const [stopped, setStopped] = useState(false);
  const [sawAlready, setSawAlready] = useState(false);

  const isDone = (found: DiscoveredInfo) => {
    const state = progress[found.path]?.state;
    return state === "done" || state === "already";
  };
  const chosen = installs.filter((found) => picked[found.path] && !found.managed && !isDone(found));
  const freshChosen = fresh.filter((tool) => freshPicked[tool] && progress[freshKey(tool)]?.state !== "done");
  const runImports = async (rows: DiscoveredInfo[], how: (found: DiscoveredInfo) => Placement = placementOf, freshTools: string[] = []) => {
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
        const result = await accountImport(found.harness, found.path, nameOf(found), null, found.origin, false, mode === "in_place");
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
    // The tools with nothing set up, after the setups: each a fresh App in
    // AI Juggler's own folder, its sign-in queued.
    for (const tool of freshTools) {
      if (stopRef.current) {
        wasStopped = true;
        break;
      }
      const key = freshKey(tool);
      setProgress((current) => ({ ...current, [key]: { state: "working" } }));
      try {
        const signin = await setUpFresh(tool, freshNames[tool] ?? providerDisplayName(tool), colourAt(installs.length + fresh.indexOf(tool)));
        if (signin) signinsRef.current.set(signin.accountId, signin);
        setProgress((current) => ({ ...current, [key]: { state: "done" } }));
      } catch (cause) {
        failed += 1;
        setProgress((current) => ({ ...current, [key]: { state: "failed", error: plainImportError(errorText(cause)) } }));
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
  const failedFresh = fresh.filter((tool) => progress[freshKey(tool)]?.state === "failed");
  const doneCount = [...installs.map((found) => found.path), ...fresh.map(freshKey)].filter((key) => progress[key]?.state === "done").length;
  const waiting = chosen.filter((found) => progress[found.path] === undefined);
  const waitingFresh = freshChosen.filter((tool) => progress[freshKey(tool)] === undefined);
  const alreadyRows = installs.filter((found) => progress[found.path]?.state === "already");
  const showOutcome = !busy && (failedRows.length > 0 || failedFresh.length > 0 || stopped || sawAlready);
  const listed = installs.length > 0 || fresh.length > 0;
  const continueNow = () => onImported([...signinsRef.current.values()]);
  // The result takes focus (the button the user pressed is gone).
  const continueRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (showOutcome) continueRef.current?.focus();
  }, [showOutcome]);

  // The bring-in choice depends on what each tool supports: wait for
  // both (never a flash of "copies" before the choice appears).
  if (report === null || inPlace === null) {
    return (
      <div className="flex flex-col gap-3" role="status" data-testid="first-run-scanning">
        <h1 className="text-stat font-semibold tracking-tight">
          {tools.length > 1 ? "Looking for your setups on this computer…" : `Looking for ${toolName} on this computer…`}
        </h1>
        <p className="text-sm text-ink-2">{toolSpec(provider).scan_hint}</p>
      </div>
    );
  }

  const words = foundWords(scanError, installs.length, toolName, kind, placement === "in_place", provider, fresh.map(providerDisplayName));
  return (
    <div className="flex flex-col gap-6" data-testid="first-run-found">
      <div className="flex flex-col gap-2">
        <h1 className="text-stat font-semibold leading-tight tracking-tight">{words.heading}</h1>
        <p className="max-w-2xl text-section leading-relaxed text-ink-2">{words.lead}</p>
        {scanError !== null && (
          <div>
            <Button onClick={onScanAgain} data-testid="first-run-scan-again">
              Try again
            </Button>
          </div>
        )}
      </div>

      {missingAlong.map((tool) => (
        <p key={tool} className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" data-testid={`first-run-no-${tool}`}>
          {providerDisplayName(tool)} itself isn't installed yet. You can bring these in now; to start sessions, install it first
          <InstallHow tool={tool} row={installRows[tool] ?? null} />
        </p>
      ))}

      {/* The chosen tool's own note — not over rows of another tool (a
          Codex-only first run has the Codex note above). */}
      {cli && cli.status === "not_installed" && (ownCount > 0 || installs.length === 0) && (
        <p className="rounded-xl border border-warning/40 bg-warning-soft px-4 py-3 text-sm text-ink" data-testid="first-run-no-cli">
          {toolName} itself isn't installed yet.{" "}
          {ownCount > 0 ? "You can bring these in now; to start sessions, install it first" : "To start sessions, install it first"}
          <InstallHow tool={provider} row={cli} />
        </p>
      )}

      {listed && (
        <div className="overflow-hidden rounded-2xl border border-hairline bg-surface">
          {groupsFor(tools).map((group) => {
            const rows = installs.filter((found) => groupOf(found) === group.key);
            if (rows.length === 0) return null;
            const install = installRows[group.tool];
            return (
              <fieldset key={group.key} className="border-b border-hairline last:border-b-0" data-testid={`first-run-group-${group.id}`}>
                <legend className="sr-only">{group.title}</legend>
                <div aria-hidden="true" className="flex items-center justify-between gap-3 bg-surface-2 px-5 py-2.5 text-xs">
                  <span className="flex items-center gap-1.5 font-semibold text-ink">
                    <ToolMark tool={group.tool} size={14} className="text-muted" />
                    <span>{group.title} <span className="font-normal text-muted">· {rows.length}</span></span>
                  </span>
                  {group.showsInstall && install && toolFound(install) && (
                    <span className="text-muted" data-testid={`first-run-cli-${group.id}`}>{cliWords(providerDisplayName(group.tool), install)}</span>
                  )}
                </div>
                {rows.map((found, index) => (
                  <FoundRow
                    key={found.path}
                    found={found}
                    name={nameOf(found)}
                    used={usedWords(found, now)}
                    live={(found.in_use ?? []).length > 0 || usedWords(found, now) === "In use now"}
                    toolName={providerDisplayName(found.harness)}
                    colour={colourAt(installs.indexOf(found))}
                    first={index === 0}
                    checked={!!picked[found.path]}
                    disabled={busy}
                    progress={progress[found.path]}
                    onCheck={(checked) => setPicked((current) => ({ ...current, [found.path]: checked }))}
                  />
                ))}
              </fieldset>
            );
          })}
          <FreshGroups fresh={fresh} names={freshNames} picked={freshPicked} onPick={(tool, on) => setFreshPicked((current) => ({ ...current, [tool]: on }))}
            installRows={installRows} colourFrom={installs.length} busy={busy} progress={(tool) => progress[freshKey(tool)]} />
        </div>
      )}

      {tools.length > 1 && <DesktopApps bundles={report.desktop_apps} tools={tools} />}

      {installs.length > 0 && !showOutcome && canInPlace && (
        <div role="radiogroup" aria-label="How AI Juggler uses them" className="grid gap-2 sm:grid-cols-2" data-testid="first-run-placement">
          {placements(inPlaceTools, installs.length === 1).map((option) => (
            <label
              key={option.id}
              className={
                "flex cursor-pointer gap-3 rounded-xl border px-4 py-3 " +
                (placement === option.id ? "border-accent bg-accent-soft/40" : "border-hairline bg-surface hover:bg-surface-2/60")
              }
              data-testid={`first-run-placement-${option.id}`}
            >
              <Radio
                name="first-run-placement"
                value={option.id}
                aria-labelledby={`first-run-placement-${option.id}-title`}
                aria-describedby={`first-run-placement-${option.id}-detail`}
                checked={placement === option.id}
                disabled={busy}
                onChange={() => setPlacement(option.id)}
                className="mt-0.5 shrink-0"
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
      {installs.length > 0 && !showOutcome && canInPlace && copyOnlyTools.length > 0 && (
        <p className="-mt-3 text-xs text-ink-2" data-testid="first-run-copy-only">
          {copyOnlyTools.map(providerDisplayName).join(" and ")} setups always come in as copies.
        </p>
      )}

      {showOutcome && (
        <div
          className="flex flex-col gap-3 rounded-2xl border border-hairline bg-surface px-5 py-4"
          role="alert"
          data-testid="first-run-outcome"
        >
          <p className="text-section font-medium text-ink">
            {outcomeWords({
              stopped,
              done: doneCount,
              waiting: waiting.length + waitingFresh.length,
              failed: failedRows.length + failedFresh.length,
              already: alreadyRows.map(nameOf),
            })}
          </p>
          {(failedRows.length > 0 || failedFresh.length > 0) && (
            <ul className="flex flex-col gap-2" data-testid="first-run-failed">
              {failedFresh.map((tool) => (
                <li key={freshKey(tool)} className="text-sm">
                  <span className="font-medium text-ink">{freshNames[tool]}</span>
                  <span className="block text-xs leading-relaxed text-danger">{progress[freshKey(tool)]?.error}</span>
                </li>
              ))}
              {failedRows.map((found) => (
                <li key={found.path} className="flex flex-wrap items-start justify-between gap-x-4 gap-y-1 text-sm">
                  <span className="min-w-0 flex-1">
                    <span className="font-medium text-ink">{nameOf(found)}</span>
                    <span className="block text-xs leading-relaxed text-danger">{progress[found.path]?.error}</span>
                  </span>
                  {progress[found.path]?.placement === "copy" && canUseInPlace(found.harness) && (
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
            {failedRows.length + failedFresh.length > 0 && (
              <Button onClick={() => void runImports(failedRows, placementOf, failedFresh)} data-testid="first-run-retry">
                {failedRows.length + failedFresh.length === 1 ? "Try again" : `Try again for ${failedRows.length + failedFresh.length}`}
              </Button>
            )}
            {stopped && waiting.length + waitingFresh.length > 0 && (
              <Button onClick={() => void runImports(waiting, placementOf, waitingFresh)} data-testid="first-run-rest">
                Bring in the rest
              </Button>
            )}
            <Button variant="primary" onClick={continueNow} data-testid="first-run-continue" ref={continueRef}>
              {doneCount === 0 ? (failedRows.length + failedFresh.length > 0 || waiting.length + waitingFresh.length > 0 ? "Continue without them" : "Continue") : doneCount === 1 ? "Continue with 1 app" : `Continue with ${doneCount} apps`}
            </Button>
          </div>
        </div>
      )}

      {/* Always in view, so Bring in is never below the fold (0.4 design review). */}
      <div className="sticky bottom-0 z-10 -mx-2 flex flex-wrap items-center justify-between gap-4 border-t border-hairline bg-canvas/95 px-2 py-3 backdrop-blur" data-testid="first-run-actions">
        <div className="flex flex-wrap items-center gap-5 text-sm">
          <button type="button" className="text-accent-hi hover:underline" onClick={onAddFresh}>
            {listed ? "Add a fresh app instead" : "Add an app"}
          </button>
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onImportFolder}>
            Import from another folder…
          </button>
          {/* AJ-E2906: scan again at any time; ticks on setups still found stay. */}
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onScanAgain} disabled={busy} data-testid="first-run-look-again">
            Look again
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
        {listed && !showOutcome && (
          <div className="flex items-center gap-3">
            <span className="hidden text-xs text-muted md:inline">Names and plans can be changed later.</span>
            <Button
              variant="primary"
              disabled={chosen.length + freshChosen.length === 0 || busy}
              onClick={() => void runImports(chosen, placementOf, freshChosen)}
              data-testid="first-run-bring-in"
            >
              {busy
                ? "Bringing them in…"
                : chosen.length + freshChosen.length === 1
                  ? "Bring in 1 app"
                  : `Bring in ${chosen.length + freshChosen.length} apps`}
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

export { colourAt, freeLabel };

export type Signin = { accountId: string; label: string; loginNote: string | null; colour: string; provider: string };

/** AJ-E1103 `onboarding.login`: how a first-run sign-in ended (the
 * backend maps the tool to the schema's provider names, "other" for any
 * it doesn't list). */
function countLogin(provider: string, outcome: "completed" | "skipped" | "expired"): void {
  statsEvent("onboarding.login", { provider, outcome });
}

/** One account's sign-in: status, Sign in, and the rest behind More. */
function SigninRow({
  entry,
  active,
  terminal,
  onStart,
  onReady,
  onSkip,
}: {
  onSkip?: () => void;
  entry: Signin;
  active: boolean;
  /** The sign-in's terminal, once it started: shown under the row. */
  terminal: string | null;
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
        <span className="min-w-0 flex-1 truncate text-section font-medium text-ink">{entry.label}</span>
        <span className="text-xs" role="status">
          {ready ? (
            <span className="flex items-center gap-1.5 text-success">
              <Check />
              {state === "api_key" ? "Signed in with an API key" : "Signed in"}
            </span>
          ) : skipped ? (
            <span className="text-muted">Later</span>
          ) : active ? (
            <span className="text-live">Finish signing in below</span>
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
      {active && terminal && !ready && (
        <div className="mx-5 mb-3 h-72 overflow-hidden rounded-lg border border-hairline" data-testid="first-run-signin-terminal">
          <TerminalView id={terminal} title={`Sign in · ${entry.label}`} />
        </div>
      )}
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
  openLoginTerminal: (id: string, title: string, options?: { select?: boolean }) => void;
  onDone: () => void;
}): JSX.Element {
  const [terminals, setTerminals] = useState<Record<string, string>>({});
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
        (terminal) => {
          setTerminals((current) => ({ ...current, [entry.accountId]: terminal.id }));
          // Known to the window too (it stays reachable after this flow
          // closes), but not shown there: two views would fight its size.
          openLoginTerminal(terminal.id, `Sign in · ${entry.label}`, { select: false });
        },
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
        <h1 className="text-stat font-semibold leading-tight tracking-tight">
          {left.length === 0
            ? "Everyone's signed in."
            : left.length === 1
              ? "One app needs a quick sign-in."
              : `${left.length} apps need a quick sign-in.`}
        </h1>
        <p className="max-w-2xl text-section leading-relaxed text-ink-2">
          {(() => {
            // One tool: its own sign-in by name. First run with every tool
            // (AJ-E1301): each account opens its own tool's sign-in.
            const tools = [...new Set(queue.map((entry) => entry.provider || provider))];
            return tools.length > 1
              ? `Each opens its own tool's sign-in — ${tools.map(shortSignIn).join(" or ")} — for that app's folder; AI Juggler never sees your password and notices when you're done.`
              : `Each opens ${ownSignIn(tools[0] ?? provider)}, for that app's folder; AI Juggler never sees your password and notices when you're done.`;
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
            terminal={terminals[entry.accountId] ?? null}
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
        <h1 className="text-stat font-semibold leading-tight tracking-tight">Bring in this folder?</h1>
        <p className="max-w-2xl text-section leading-relaxed text-ink-2">
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
                {inspect.file_count.toLocaleString()} files · {byteSize(inspect.total_bytes)}
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
                      <span>{byteSize(entry.bytes)}</span>
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
          Its session history is large ({byteSize(inspect.total_bytes)}). Bring in the settings only, or add a fresh
          app instead.
        </p>
      )}
      {usable && (
        <label className="flex max-w-md flex-col gap-1.5 text-sm font-medium text-ink">
          Name
          <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
            value={label}
            onChange={(event) => setLabel(event.target.value)}
            className="rounded-lg border border-hairline bg-surface px-3 py-2.5 text-section font-normal text-ink outline-none focus:border-accent/60"
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
              Add a fresh app instead
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
  startAt = "found",
  resumeAtAccess = false,
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
  /** AJ-E1524: the welcome's "Add a fresh account" opens on that step. */
  startAt?: "found" | "fresh";
  /** An account was created or brought in (the caller refreshes). */
  onAccountsChanged?: () => void;
  openLoginTerminal: (id: string, title: string, options?: { select?: boolean }) => void;
  /** Back after Reopen AI Juggler: start at the access step. */
  resumeAtAccess?: boolean;
}): JSX.Element {
  const [step, setStep] = useState<Step>(resumeAtAccess ? "access" : startAt);
  const [folder, setFolder] = useState<string | null>(null);
  const [taken, setTaken] = useState<string[] | null>(null);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  // AJ-E1301: adding names its tool; first run on a fresh install brings
  // in every supported tool's setups (a fresh account or a folder import
  // there is Claude Code's, as before).
  const primary = provider ?? primaryToolId() ?? "";
  const tools = useMemo(() => (provider ? [provider] : firstRunToolIds()), [provider]);
  const toolName = providerDisplayName(primary);
  const { report, scanError, scan } = useSetupScan();
  const [cli, setCli] = useState<ProviderInstallData | null>(null);
  const [installRows, setInstallRows] = useState<Record<string, ProviderInstallData>>({});
  const [inPlace, setInPlace] = useState<InPlaceSupport | null>(null);
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
    scan();
    void providersDiscover().then(
      (rows) => {
        setCli(rows.find((row) => row.provider === primary) ?? null);
        // Every tool's own row, for its group's heading and its note.
        setInstallRows(Object.fromEntries(rows.map((row) => [row.provider, row])));
      },
      () => setCli(null),
    );
    // ADR-0018: what each tool supports, without probing installs. If the
    // call fails the choice stays, in place first (the user decides; the
    // backend refuses what it can't adopt).
    void providersCapabilities().then(
      (rows) => setInPlace(inPlaceProviders(rows)),
      () => setInPlace("unknown"),
    );
    void accountsList().then(
      (rows) => {
        setTaken(rows.map((row) => row.label));
        setAccounts(rows);
      },
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
  // AJ-E2906: Look again from there scans again; what it finds new is
  // listed on the found step, as in first run.
  const lookedAgain = useRef(false);
  const lookAgain = useCallback(() => {
    lookedAgain.current = true;
    scan();
  }, [scan]);
  useEffect(() => {
    if (!lookedAgain.current || report === null) return;
    lookedAgain.current = false;
    if (!nothingNew) setStep("found");
  }, [report, nothingNew]);

  // "Import from another folder…": the system's picker first; nothing
  // changes when it is cancelled.
  const pickFolder = useCallback(async () => {
    const picked = await pickDirectory();
    if (picked === null) return;
    setFolder(picked);
    setStep("folder");
  }, []);

  // Nothing to ask for: this OS needs nothing, or it is all allowed.
  // Adding an account later never asks again: first run asked once, and
  // the step itself promises to ask again only on first use.
  const skipAccess = mode === "add" || (access !== null && access.every((entry) => entry.status === "allowed"));
  // D-46: first run ends with the optional Figurehead account step.
  const finish = useCallback(() => (mode === "first-run" ? setStep("account") : onDone()), [onDone, mode]);
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
      aria-label={mode === "add" ? "Add apps" : "Set up AI Juggler"}
      data-testid="first-run"
      data-mode={mode}
      ref={frameRef}
      tabIndex={-1}
    >
      <div data-tauri-drag-region className="flex h-11 shrink-0 items-center justify-end px-4">
        {/* Always a visible way out (0.0.7 dogfood: "no way to close"):
            first run → Home, which offers "Find my apps" to come back. */}
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
          <StepMarks step={step} skipAccess={skipAccess} withAccount={mode === "first-run"} />
          {step === "found" && (
            <FoundStep
              report={report}
              scanError={scanError}
              onScanAgain={scan}
              inPlace={inPlace}
              cli={cli}
              toolName={toolName}
              provider={primary}
              tools={tools}
              installRows={installRows}
              taken={taken}
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
            <FreshAccountStep
              toolName={toolName}
              provider={primary}
              taken={taken}
              accounts={accounts}
              canGoBack={!nothingNew || mode === "first-run"}
              onBack={() => setStep("found")}
              onImportFolder={() => void pickFolder()}
              onLookAgain={mode === "add" && (nothingNew || lookedAgain.current) ? lookAgain : null}
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
          {step === "account" && <FigureheadStep onDone={() => (funnelStep("a-first-install", "done"), onDone())} />}
          {(step === "found" || step === "fresh" || step === "folder") && <TermsLine tools={step === "found" ? tools : [primary]} />}
        </div>
      </div>
    </div>
  );
}
