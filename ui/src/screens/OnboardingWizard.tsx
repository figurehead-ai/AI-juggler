import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconJuggler, IconProfiles, IconTerminal } from "../components/icons";
import { funnelEvent, funnelStep, reasonCode } from "../lib/funnel";
import { LoginQueueCard } from "../components/LoginQueueCard";
import { ExpertSetup } from "../components/ExpertSetup";
import {
  PackPolicyChooser,
  packPolicyRestatement,
  type PackPolicyChoice,
  type PackPolicyWire,
} from "../components/PackPolicyChooser";
import { pickDirectory, sourceInspect,
  signedIn,
} from "../lib/bridge";
import type { SourceInspectInfo } from "../types/generated/SourceInspectInfo";
import {
  accountImport,
  accountLoginCommand,
  accountLoginState,
  createAccount,
  environmentScanFull,
  hasLiveBackend,
  loginOpenWeb,
  providersDiscover,
  scanCancel,
  terminalSpawnLogin,
  type ProbeRowData,
  type ScanReportData,
  type RecommendationData,
  recommendationFor,
  type AccountInfo,
  type DiscoveredInfo,
  type ImportResult,
  type ProviderInstallData,
  errorText,
  isolationCheck,
  type IsolationRowData,
  accountSetRenewal,
  redactionPolicyGet,
  redactionPolicySetGlobal,
} from "../lib/bridge";

const PLANS: { id: string | null; label: string; hint: string }[] = [
  { id: null, label: "Not sure yet", hint: "you can set this later" },
  { id: "free", label: "Free", hint: "trying things out" },
  { id: "pro", label: "Pro", hint: "regular daily use" },
  // AJ-E303 Req.4: Max 5× joins the list (Deep-03 §2.1) — never
  // inferred, chosen.
  { id: "max-5x", label: "Max 5×", hint: "regular heavy use" },
  { id: "max-20x", label: "Max 20×", hint: "extreme parallel work" },
];

const STEPS = [
  "install",
  "detect",
  "identity",
  "login",
  "signins",
  "summary",
] as const;
type Step = (typeof STEPS)[number];

/**
 * AJ-E305: flow (a) shows FOUR step dots (gate → scan → sign-ins →
 * ready); the manual single-account path folds its confirm into the
 * scan phase. Internal steps stay distinct; the dots are a phase
 * map, not a step list.
 */
const STEP_DOT: Record<Step, number> = {
  install: 0,
  detect: 1,
  identity: 1,
  login: 2,
  signins: 2,
  summary: 3,
};
const DOT_LABELS = ["Gate", "Scan", "Sign-ins", "Ready"];

function humanBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** One Step-0 provider row: Installed vX · Not installed · Installed
 * (vX, unverified). A missing binary is a gate, not an error. */
function ProviderRow({
  install,
  selected,
  onNotice,
}: {
  install: ProviderInstallData;
  selected: boolean;
  onNotice: () => void;
}): JSX.Element {
  const missing = install.status === "not_installed";
  const unverified = install.status === "unverified";
  const [showHelp, setShowHelp] = useState(false);
  return (
    <div
      data-testid="provider-row"
      data-status={install.status}
      data-selected={selected && !missing ? "true" : "false"}
      className={
        "flex flex-col gap-2 rounded-xl border px-4 py-3.5 " +
        (selected && !missing ? "border-accent/40 bg-accent-soft" : "border-hairline")
      }
    >
      <button
        type="button"
        aria-pressed={selected && !missing}
        aria-expanded={missing ? showHelp : undefined}
        onClick={() => {
          // A missing binary toggles the install help (the GATE is
          // the disabled Continue button — the row itself never
          // advances, and a disabled button can't even be clicked).
          if (missing) {
            setShowHelp((open) => !open);
            if (!showHelp) onNotice();
          }
        }}
        className="flex items-center gap-2 text-left"
      >
        <span className="min-w-0 flex-1 text-sm font-medium">{install.display_name}</span>
        {install.status === "installed" && (
          <Chip tone="success">Installed v{install.version}</Chip>
        )}
        {unverified && <Chip tone="warning">Installed (v{install.version}, unverified)</Chip>}
        {missing && <Chip tone="neutral">Not installed</Chip>}
        {install.status === "probe_failed" && <Chip tone="warning">Couldn't check</Chip>}
      </button>
      {unverified && install.notes.length > 0 && (
        <p className="text-[11px] leading-relaxed text-muted">{install.notes.join(" · ")}</p>
      )}
      {missing && showHelp && (
        <div
          className="flex flex-col gap-2 rounded-lg border border-hairline bg-canvas px-3 py-2.5"
          data-testid="install-help"
        >
          <p className="text-[11px] leading-relaxed text-muted">
            {install.display_name} isn't installed on this Mac. AI Juggler drives the CLI
            itself — install it first, or continue without it and set it up later.
          </p>
          <div className="flex gap-2">
            <a
              href={install.install_url}
              target="_blank"
              rel="noreferrer"
              className="rounded-lg border border-accent/40 px-2.5 py-1 text-[11px] text-accent-hi transition-colors hover:border-accent/60"
            >
              Install {install.display_name} →
            </a>
            <button
              type="button"
              onClick={() => setShowHelp(false)}
              className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-muted transition-colors hover:border-accent/30"
            >
              Continue without it
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

/** Denied probes in a report (the wizard's honest-empty guard: an
 * empty result with ANY denied probe never reads "no setups
 * found"). */
function reportDeniedCount(probes: ProbeRowData[]): number {
  return probes.filter((probe) => probe.health === "denied").length;
}

/** One probe row inside Details (text health, never colour-only). */
function ScanProbeRow({ probe }: { probe: ProbeRowData }): JSX.Element {
  const tone =
    probe.health === "ok"
      ? "text-muted"
      : probe.health === "partial"
        ? "text-warning"
        : "text-danger";
  const word =
    probe.health === "ok" ? "checked" : probe.health === "partial" ? "partial" : "couldn't check";
  return (
    <li className="flex flex-col text-[11px] leading-relaxed" data-testid="scan-probe-row">
      <span>
        <span className="text-ink">{probe.label}</span>{" "}
        <span className={tone}>{word}</span>
        <span className="text-muted"> — {probe.detail}</span>
      </span>
      {probe.reason && <span className="text-muted">{probe.reason}</span>}
    </li>
  );
}

/** One candidate card (pass2/03 §4.2 wireframe, AJ-E303): line 1
 * provider · path · login chip · confidence words; line 2 the
 * inference line ("Likely: Work · Active this week"); actions Import
 * / Edit / Ignore; already-managed renders disabled inline. All notes
 * render (never notes[0]-truncated); the keychain caveat reads
 * "signed in here — you'll sign in once more after import". */
function CandidateCards({
  installs,
  edits,
  editing,
  onEdit,
  onEditField,
  onImport,
  onIgnore,
}: {
  installs: DiscoveredInfo[];
  edits: Record<string, { purpose: string; plan: string | null }>;
  editing: string | null;
  onEdit: (path: string) => void;
  onEditField: (path: string, patch: { purpose?: string; plan?: string | null }) => void;
  onImport: (found: DiscoveredInfo, purpose: string) => void;
  onIgnore: (path: string) => void;
}): JSX.Element {
  const confidenceWord = (wire: string): string =>
    wire === "found" ? "Found" : wire === "likely" ? "Likely" : "Guessed";
  return (
    <ul className="flex flex-col gap-2" data-testid="candidate-cards">
      {installs.map((found) => {
        const edit = edits[found.path];
        const purpose = edit?.purpose || found.inference.purpose;
        const plan = edit?.plan ?? null;
        const managed = found.managed;
        const loginWord =
          found.inference.login === "signed-in"
            ? "Signed in"
            : found.inference.login === "maybe-expired"
              ? "May be expired"
              : "Not signed in";
        return (
          <li
            key={found.path}
            data-testid="candidate-card"
            data-managed={managed ? "true" : "false"}
            data-confidence={found.inference.purpose_confidence}
            // Managed cards stay readable (no opacity dimming): the
            // disabled actions + inline label carry the state.
            className={
              "flex flex-col gap-2 rounded-xl border px-4 py-3.5 " +
              (managed ? "border-hairline/60" : "border-hairline")
            }
          >
            <div className="flex items-center gap-2">
              <IconProfiles width={14} height={14} />
              <span className="text-xs font-medium">{found.harness === "claude-code" ? "Claude" : found.harness}</span>
              <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-ink" title={found.path}>
                {found.path}
              </span>
              <Chip
                tone={
                  found.inference.login === "signed-in"
                    ? "success"
                    : found.inference.login === "maybe-expired"
                      ? "warning"
                      : "neutral"
                }
              >
                {loginWord}
              </Chip>
            </div>
            <p className="text-[11px] leading-relaxed text-muted">
              <span className="text-ink">
                {confidenceWord(found.inference.purpose_confidence)}: {purpose}
              </span>{" "}
              · {found.inference.activity_words}
              {found.signals.length > 1 && (
                <span className="text-muted"> · {found.signals.length} signals merged</span>
              )}
              {found.bytes > 0 && (
                <span className="text-muted"> · {humanBytes(found.bytes)} of state</span>
              )}
            </p>
            {found.inference.keychain_bound && (
              <p className="text-[11px] text-warning" data-testid="keychain-caveat">
                signed in here — you'll sign in once more after import (the keychain entry is
                path-locked)
              </p>
            )}
            {found.notes.length > 0 && (
              <ul className="flex flex-col gap-0.5 text-[10px] leading-relaxed text-muted">
                {found.notes.map((note) => (
                  <li key={note}>{note}</li>
                ))}
              </ul>
            )}
            {editing === found.path ? (
              <div className="flex flex-col gap-2 rounded-lg border border-hairline bg-canvas px-3 py-2.5" data-testid="inline-edit">
                <label className="block text-[10px] uppercase tracking-wider text-faint">
                  Name
                  <input
                    value={purpose}
                    onChange={(event) => onEditField(found.path, { purpose: event.target.value })}
                    className="mt-1 w-full rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-xs text-ink outline-none focus:border-accent/50"
                    data-testid="inline-edit-purpose"
                  />
                </label>
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-faint">Plan</p>
                  <div className="mt-1 flex flex-wrap gap-1.5">
                    {PLANS.map((tier) => (
                      <button
                        key={tier.label}
                        type="button"
                        onClick={() => onEditField(found.path, { plan: tier.id })}
                        className={
                          "rounded-lg border px-2 py-1 text-[10px] transition-colors " +
                          (plan === tier.id
                            ? "border-accent/40 bg-accent-soft text-accent-hi"
                            : "border-hairline text-muted hover:border-accent/30")
                        }
                      >
                        {tier.label}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                disabled={managed}
                aria-label={
                  managed
                    ? `Already in AI Juggler: ${found.path}`
                    : `Import ${found.path} as “${purpose}”`
                }
                title={managed ? "Already managed by AI Juggler" : undefined}
                onClick={() => onImport(found, purpose)}
                className="rounded-lg border border-accent/40 bg-accent-soft px-2.5 py-1 text-[11px] text-accent-hi transition-colors hover:border-accent/60 disabled:border-hairline disabled:bg-transparent disabled:text-faint"
                data-testid="import-as"
              >
                {managed ? "Already in AI Juggler" : `Import as “${purpose}”`}
              </button>
              <button
                type="button"
                disabled={managed}
                onClick={() => onEdit(found.path)}
                aria-label={`Edit the inferred name and plan for ${found.path}`}
                className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-muted transition-colors hover:border-accent/30 disabled:opacity-50"
              >
                Edit
              </button>
              <button
                type="button"
                disabled={managed}
                onClick={() => onIgnore(found.path)}
                aria-label={`Ignore ${found.path} for this import`}
                className="rounded-lg border border-hairline px-2.5 py-1 text-[11px] text-muted transition-colors hover:border-accent/30 disabled:opacity-50"
              >
                Ignore
              </button>
            </div>
          </li>
        );
      })}
    </ul>
  );
}

/** Today's LOCAL calendar date as `YYYY-MM-DD` (renewal dates are
 * local calendar days, never timezone-converted — AJ-E411). */
function localToday(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** AJ-E309 Req.3 (D-12): the optional renewal date on a Confirm row —
 * collapsed to "expiry unknown" until the user opens it; a native
 * date input; [Skip] clears it without penalty (Apple HIG: optional
 * means skippable). Never inferred. */
function RenewalField({
  value,
  onChange,
  accountLabel,
  testId = "renewal-date",
}: {
  value: string;
  onChange: (next: string) => void;
  accountLabel: string;
  testId?: string;
}): JSX.Element {
  const [open, setOpen] = useState(value.length > 0);
  const inputId = useId();
  if (!open) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted" data-testid={testId} data-state="unknown">
        <span>Expiry unknown</span>
        <button
          type="button"
          onClick={() => setOpen(true)}
          className="rounded text-[11px] text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
          aria-label={`Add a renewal date for ${accountLabel} (optional)`}
          data-testid={`${testId}-add`}
        >
          ＋ Renewal date
        </button>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-1" data-testid={testId} data-state={value ? "set" : "open"}>
      <label htmlFor={inputId} className="text-[11px] text-muted">
        Renewal date (optional) — we'll warn you before it
      </label>
      <div className="flex flex-wrap items-center gap-2">
        <input
          id={inputId}
          type="date"
          // A renewal in the past is almost always a typo — the native
          // picker starts at today (local calendar, never UTC).
          min={localToday()}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          aria-label={`Renewal date for ${accountLabel} (optional)`}
          className="w-fit rounded-lg border border-hairline bg-canvas px-2 py-1 text-xs text-ink outline-none focus:border-accent/50"
          data-testid={`${testId}-input`}
        />
        <button
          type="button"
          onClick={() => {
            onChange("");
            setOpen(false);
          }}
          className="rounded-md px-2 py-1 text-[11px] text-muted outline-none hover:bg-surface hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid={`${testId}-skip`}
        >
          Skip
        </button>
      </div>
      {value === "" && (
        <span className="text-[11px] text-muted">Left empty, expiry stays unknown — never guessed.</span>
      )}
    </div>
  );
}

/** The Customise view (§5): per-item toggles — the default-ON set
 * first, quiet items under "Found but quiet", fresh adds listed;
 * every item shows its one-line rationale and (AJ-E309) an optional
 * renewal date. Accept executes the chosen set sequentially (⌘↩
 * accepts once the policy question, when asked, is answered). */
function ProposalBlock({
  recommendation,
  choices,
  onChoice,
  onAccept,
  renewals,
  onRenewal,
  policySlot,
  acceptBlocked,
}: {
  recommendation: RecommendationData;
  choices: Record<number, boolean>;
  onChoice: (index: number, on: boolean) => void;
  onAccept: () => void;
  renewals: Record<number, string>;
  onRenewal: (index: number, date: string) => void;
  /** The first-run policy chooser (merged with Confirm). */
  policySlot?: JSX.Element | null;
  /** Why Accept is unavailable (the unanswered policy), or null. */
  acceptBlocked: string | null;
}): JSX.Element {
  const hintId = useId();
  const on = recommendation.items.filter((_, index) => choices[index]);
  const quiet = recommendation.items.filter((item) => !item.default_on);
  const row = (item: RecommendationData["items"][number], index: number): JSX.Element => {
    const isQuiet = !item.default_on;
    const checked = choices[index] ?? item.default_on;
    return (
      <li
        key={`${item.provider}-${item.label}-${index}`}
        className="flex flex-col gap-1 border-b border-hairline/50 pb-1.5 last:border-0 last:pb-0"
        data-testid="proposal-item"
        data-quiet={isQuiet ? "true" : "false"}
      >
        <label className="flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={checked}
            onChange={(event) => onChoice(index, event.target.checked)}
            className="size-3.5 accent-[var(--color-accent)]"
            data-testid="proposal-toggle"
            aria-label={`${item.label}: ${item.rationale}`}
          />
          <span className="font-medium">{item.label}</span>
          <span className="text-muted">{item.rationale}</span>
        </label>
        {checked && (
          <div className="pl-5">
            <RenewalField
              value={renewals[index] ?? ""}
              onChange={(date) => onRenewal(index, date)}
              accountLabel={item.label}
            />
          </div>
        )}
      </li>
    );
  };
  return (
    <div
      className="flex flex-col gap-3 rounded-xl border border-hairline px-4 py-3.5"
      data-testid="proposal-block"
    >
      <p className="text-xs leading-relaxed text-muted">{recommendation.summary}</p>
      {recommendation.start_here && (
        <p className="text-xs text-muted" data-testid="start-here">
          Start here: <span className="font-mono text-ink">{recommendation.start_here}</span> —
          your most recent project.
        </p>
      )}
      <ul className="flex flex-col gap-1.5">
        {recommendation.items
          .map((item, index) => ({ item, index }))
          .filter(({ item }) => item.default_on)
          .map(({ item, index }) => row(item, index))}
      </ul>
      {quiet.length > 0 && (
        <div className="flex flex-col gap-1.5" data-testid="proposal-quiet">
          <p className="text-[11px] font-medium text-muted">
            Found but quiet — dormant setups stay unimported until you tick them.
          </p>
          <ul className="flex flex-col gap-1.5">
            {recommendation.items
              .map((item, index) => ({ item, index }))
              .filter(({ item }) => !item.default_on)
              .map(({ item, index }) => row(item, index))}
          </ul>
        </div>
      )}
      {policySlot}
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant="primary"
          onClick={onAccept}
          disabled={acceptBlocked !== null}
          aria-describedby={acceptBlocked !== null ? hintId : undefined}
          data-testid="accept-setup"
        >
          Accept setup → ({on.length})
        </Button>
        {acceptBlocked !== null ? (
          <span id={hintId} className="text-[11px] text-muted" data-testid="accept-blocked">
            {acceptBlocked}
          </span>
        ) : (
          <span className="text-[10px] text-muted">⌘↩ accepts</span>
        )}
      </div>
    </div>
  );
}

/**
 * The onboarding wizard (ADR-0006, spec §12.1; AJ-E301): one shell
 * for EVERY Provider (`provider` parameter — no baked-in harness),
 * gated at Step 0 by the install probe so a missing CLI is caught
 * before any account exists. Detect what exists → offer import or a
 * guided fresh start → sign in with the vendor's own flow → a summary
 * that says exactly what happened and what wasn't touched.
 *
 * A11y: `role="dialog"` + focus trap + return-focus; Esc closes only
 * when idle; the close control is disabled while an import/create is
 * in flight (with an explicit "still running" confirm as the escape
 * hatch); focus moves to the primary button on every step change.
 */
export function OnboardingWizard({
  provider,
  locked = false,
  onClose,
  onAccountCreated,
  openLoginTerminal,
  onLaunchFirstSession,
  onAddAnother,
}: {
  /** The Provider this run sets up (AJ-E301 Req.1 — explicit at every
   * call site: first-run, Accounts → Add account, summary loop-back). */
  provider: string;
  /** AJ-E306 flows (c)/(d): the Provider is LOCKED — no Step-0
   * multi-Provider gate and no zero-found re-picker; the scan and
   * identity run scoped to `provider` directly. */
  locked?: boolean;
  onClose: () => void;
  onAccountCreated: () => void;
  openLoginTerminal: (id: string, title: string) => void;
  /** AJ-E305 Req.4: [Launch first session →] on the set Summary —
   * opens the launch sheet with the Start-here project preselected. */
  onLaunchFirstSession?: (startHere: string | null) => void;
  /** AJ-E305 Req.4: [Add another account] on the set Summary. */
  onAddAnother?: () => void;
}) {
  const [step, setStep] = useState<Step>(locked ? "detect" : "install");
  const [installs, setInstalls] = useState<ProviderInstallData[] | null>(null);
  const [report, setReport] = useState<ScanReportData | null>(null);
  // AJ-E303: inline edits (path → purpose/plan) + reversible ignores.
  // Editing NEVER re-runs the scan (Req.5 — pinned by test).
  const [edits, setEdits] = useState<Record<string, { purpose: string; plan: string | null }>>({});
  const [editing, setEditing] = useState<string | null>(null);
  const [ignored, setIgnored] = useState<Record<string, boolean>>({});
  const [showIgnored, setShowIgnored] = useState(false);
  // AJ-E304: the set-level proposal + the sequential accept.
  const [proposalOpen, setProposalOpen] = useState(false);
  const [recommendation, setRecommendation] = useState<RecommendationData | null>(null);
  const [choices, setChoices] = useState<Record<number, boolean>>({});
  const [acceptRun, setAcceptRun] = useState<
    { label: string; state: "pending" | "done" | "failed"; error?: string }[] | null
  >(null);
  const [setSummary, setSetSummary] = useState<
    { label: string; state: "done" | "failed"; error?: string }[] | null
  >(null);
  // AJ-E305 Req.3: the queued sign-ins after a set accept — one card
  // per created/imported Account that still needs the vendor login.
  // AJ-E307: the Confirm-source panel's data + the manual folder
  // path + the degraded choice + structured import errors.
  const [inspect, setInspect] = useState<SourceInspectInfo | null>(null);
  const [manualSource, setManualSource] = useState<string | null>(null);
  const [degradedChoice, setDegradedChoice] = useState(false);
  // AJ-E309 Req.1 (D-14): the pack policy choice — asked on the first
  // run only while NO global choice exists; nothing is pre-selected,
  // and "Decide later" writes nothing (no pack is staged until the
  // user chooses). The write happens on the accept/create click.
  const [policyChoice, setPolicyChoice] = useState<PackPolicyChoice | null>(null);
  const [policyKnown, setPolicyKnown] = useState<{
    global: PackPolicyWire | null;
    effective: PackPolicyWire | null;
    source: "global" | "override" | "unset";
  } | null>(null);
  const [policyWriteFailed, setPolicyWriteFailed] = useState(false);
  // AJ-E309 Req.3 (D-12): optional, user-entered renewal dates — the
  // single-account Confirm row, and one per proposal item on the set
  // path. Empty persists as unknown ("expiry unknown").
  const [renewalDate, setRenewalDate] = useState("");
  const [renewals, setRenewals] = useState<Record<number, string>>({});
  // The Accounts this run created (Summary restatement + Expert setup).
  const [created, setCreated] = useState<
    { id: string; label: string; renewal: string | null; renewalSaved: boolean }[]
  >([]);
  // Funnel bookkeeping — each of these is emitted once per run.
  const [proposalCustomised, setProposalCustomised] = useState(false);
  const emittedRef = useRef<Set<string>>(new Set());
  // AJ-E308 Req.7: the isolation checklist on the set Summary.
  const [isolation, setIsolation] = useState<IsolationRowData[] | null>(null);
  useEffect(() => {
    if (step !== "summary" || setSummary === null || isolation !== null) return;
    let cancelled = false;
    void isolationCheck("__preview__").then((rows) => {
      if (!cancelled) setIsolation(rows);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, setSummary]);
  const [importError, setImportError] = useState<
    { kind: "duplicate" | "moved" | "other"; text: string } | null
  >(null);
  const [signinQueue, setSigninQueue] = useState<
    | {
        accountId: string;
        label: string;
        provider: string;
        loginNote: string | null;
      }[]
    | null
  >(null);
  const [signinIndex, setSigninIndex] = useState(0);
  // AJ-E305 Req.2: the zero-found collapse may re-target this run at
  // another installed Provider (the picker writes the override).
  const [providerOverride, setProviderOverride] = useState<string | null>(null);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [importing, setImporting] = useState<DiscoveredInfo | null>(null);
  const [label, setLabel] = useState("");
  const [plan, setPlan] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryable, setRetryable] = useState(false);
  const [confirmClose, setConfirmClose] = useState(false);
  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [importResult, setImportResult] = useState<ImportResult | null>(null);
  const [loginState, setLoginState] = useState<string | null>(null);
  const [command, setCommand] = useState("");
  const [copied, setCopied] = useState(false);

  const dialogRef = useRef<HTMLDivElement | null>(null);
  const primaryRef = useRef<HTMLButtonElement | null>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  const activeProvider = providerOverride ?? provider;

  // Step 0 probes the Providers (off the UI thread; a missing binary
  // is a row, not an error).
  useEffect(() => {
    let cancelled = false;
    void providersDiscover().then((found) => {
      if (!cancelled) setInstalls(found);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // Step 1 runs the 10-signal scan automatically once the gate is
  // passed — progress streams as "n of 10"; closing cancels.
  useEffect(() => {
    if (step !== "detect" || report !== null) return;
    let cancelled = false;
    const providers = Math.max(1, installs?.length ?? 1);
    funnelEvent("scan_started", { providers });
    void environmentScanFull((done, total) => {
      if (!cancelled) setProgress({ done, total });
    }).then((found) => {
      if (!cancelled) {
        setReport(found);
        setProgress(null);
        // AJ-E309 Req.6/7: counts and a health word only — never the
        // paths or the inferred names themselves.
        const health = found.cancelled
          ? "cancelled"
          : found.probes.some((probe) => probe.health === "denied")
            ? "denied"
            : found.probes.some((probe) => probe.health === "partial")
              ? "partial"
              : "ok";
        funnelEvent("scan_completed", { providers, found: found.installs.length, health });
        for (const candidate of found.installs) {
          if (!candidate.managed) funnelEvent("inference_shown", { field: "purpose" });
        }
      }
    });
    return () => {
      cancelled = true;
      void scanCancel();
    };
  }, [step, report]);

  useEffect(() => {
    if (account) void accountLoginCommand(account.id).then(setCommand);
  }, [account]);

  // AJ-E304: the proposal computes once per scan (staging only).
  useEffect(() => {
    if (step !== "detect" || recommendation !== null) return;
    let cancelled = false;
    void recommendationFor().then((proposal) => {
      if (!cancelled) {
        if (proposal.items.length > 0) {
          funnelEvent("proposal_shown", { items: proposal.items.length });
        }
        setRecommendation(proposal);
        setChoices(
          Object.fromEntries(proposal.items.map((item, index) => [index, item.default_on])),
        );
      }
    });
    return () => {
      cancelled = true;
    };
  }, [step, recommendation]);

  // AJ-E309: what the user already chose (any earlier run, the
  // switch chooser, or Accounts → Pack secrets). Only an UNSET global
  // is asked about — and only on the first-run flow (a): the locked
  // add-Provider/add-Account flows keep their click budgets and say
  // the active choice on Summary instead.
  useEffect(() => {
    let alive = true;
    void redactionPolicyGet(activeProvider)
      .then((info) => {
        if (!alive) return;
        const asWire = (value: string | null): PackPolicyWire | null =>
          value === "strip" || value === "keep" || value === "ask" ? value : null;
        setPolicyKnown({
          global: asWire(info.global),
          effective: asWire(info.effective),
          source: info.source === "override" || info.source === "global" ? info.source : "unset",
        });
      })
      .catch(() => {
        if (alive) setPolicyKnown({ global: null, effective: null, source: "unset" });
      });
    return () => {
      alive = false;
    };
  }, [activeProvider]);
  const policyNeeded = !locked && policyKnown !== null && policyKnown.global === null;
  // Deliberate choices get no fast-path: the accept/create stays
  // disabled until the user picks an option (Decide later counts) —
  // and, on the first-run flow, while the stored choice is still
  // being read (a fast click must never skip the question).
  const policyBlocking =
    !locked && (policyKnown === null || (policyNeeded && policyChoice === null));

  // ⌘↩ accepts the proposal when it is open (pass2/03 §5) — never
  // while the policy is still unanswered.
  useEffect(() => {
    if (!proposalOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (
        (event.metaKey || event.ctrlKey) &&
        event.key === "Enter" &&
        acceptRun === null &&
        !policyBlocking
      ) {
        event.preventDefault();
        void acceptTheSetup();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [proposalOpen, acceptRun, choices, policyBlocking, policyChoice, renewals]);

  // AJ-E309 Req.2: the global policy writes ONLY on the explicit
  // accept/create click. "Decide later" writes nothing.
  const commitPolicy = async () => {
    if (!policyNeeded || policyChoice === null) return;
    if (!emittedRef.current.has("policy")) {
      emittedRef.current.add("policy");
      funnelEvent("policy_chosen", { choice: policyChoice });
    }
    if (policyChoice === "decide_later") return;
    try {
      await redactionPolicySetGlobal(policyChoice);
      setPolicyKnown({ global: policyChoice, effective: policyChoice, source: "global" });
      setPolicyWriteFailed(false);
    } catch {
      // The accounts still get set up; Summary says the choice wasn't
      // saved and where to make it.
      setPolicyWriteFailed(true);
    }
  };

  // D-12: a user-entered date only; failures are reported per Account
  // on Summary, never silently dropped.
  const saveRenewal = async (accountId: string, date: string | null): Promise<boolean> => {
    if (date === null) return true;
    try {
      await accountSetRenewal(accountId, date);
      return true;
    } catch {
      return false;
    }
  };

  // Accept executes the chosen set SEQUENTIALLY with per-item
  // progress; a failed item degrades ITSELF only (Req.6). Accounts
  // that still need the vendor login queue for the sign-ins step
  // (AJ-E305 Req.3) — Skip there never blocks the Summary.
  const acceptTheSetup = async () => {
    if (!recommendation || policyBlocking) return;
    // The accept IS the identity confirmation on the set path — the
    // labels arrived pre-filled from inference and commit here.
    funnelStep("a-first-install", "confirm");
    await commitPolicy();
    const picked = recommendation.items
      .map((item, index) => ({ item, index }))
      .filter(({ index }) => choices[index]);
    funnelEvent("proposal_accepted", { items: picked.length });
    setAcceptRun(
      picked.map(({ item }) => ({ label: item.label, state: "pending" as const })),
    );
    const results: { label: string; state: "done" | "failed"; error?: string }[] = [];
    const made: { id: string; label: string; renewal: string | null; renewalSaved: boolean }[] = [];
    const queue: {
      accountId: string;
      label: string;
      provider: string;
      loginNote: string | null;
    }[] = [];
    for (const { item, index } of picked) {
      const renewal = renewals[index]?.trim() ? renewals[index]!.trim() : null;
      try {
        if (item.action === "import" && item.path) {
          funnelEvent("import_started");
          const imported = await accountImport(item.provider, item.path, item.label, null);
          funnelEvent("import_completed", { degraded: imported.degraded === true });
          results.push({ label: item.label, state: "done" });
          made.push({
            id: imported.account.id,
            label: imported.account.label,
            renewal,
            renewalSaved: await saveRenewal(imported.account.id, renewal),
          });
          if (imported.login === "expired") funnelEvent("login_expired");
          if (!signedIn(imported.login)) {
            queue.push({
              accountId: imported.account.id,
              label: imported.account.label,
              provider: item.provider,
              loginNote: imported.login_note,
            });
          }
        } else {
          const createdAccount = await createAccount(item.provider, item.label, null);
          results.push({ label: item.label, state: "done" });
          made.push({
            id: createdAccount.id,
            label: createdAccount.label,
            renewal,
            renewalSaved: await saveRenewal(createdAccount.id, renewal),
          });
          if (!signedIn(createdAccount.login_state)) {
            queue.push({
              accountId: createdAccount.id,
              label: createdAccount.label,
              provider: item.provider,
              loginNote: null,
            });
          }
        }
      } catch (cause) {
        if (item.action === "import") funnelEvent("import_failed", { reason_code: reasonCode(cause) });
        results.push({ label: item.label, state: "failed", error: errorText(cause) });
      }
      setAcceptRun(
        picked.map(({ item: pickedItem }, at) => ({
          label: pickedItem.label,
          state: at < results.length ? results[at]!.state : "pending",
          error: at < results.length ? results[at]!.error : undefined,
        })),
      );
    }
    if (queue.length > 0) funnelEvent("login_queued", { count: queue.length });
    onAccountCreated();
    setAcceptRun(null);
    setImporting(null);
    setAccount(null);
    setCreated(made);
    setSetSummary(results);
    setSigninQueue(queue);
    setSigninIndex(0);
    goto(queue.length > 0 ? "signins" : "summary");
  };

  // AJ-E307 Req.3: inspect the chosen source at Confirm-source time
  // (metadata-only) — sizes, breakdown, evidence, keychain note.
  const activeSource =
    importing !== null ? importing.path : manualSource !== null ? manualSource : null;
  const activeOrigin = importing !== null ? importing.origin : manualSource !== null ? "manual" : null;
  useEffect(() => {
    if (step !== "identity" || activeSource === null) return;
    let cancelled = false;
    setInspect(null);
    void sourceInspect(activeProvider, activeSource).then((found) => {
      if (!cancelled) setInspect(found);
    });
    return () => {
      cancelled = true;
    };
  }, [step, activeSource, activeProvider]);

  const currentSignin = signinQueue !== null ? signinQueue[signinIndex] ?? null : null;

  // A finished (or skipped-through) queue lands on the Summary — the
  // queue itself never blocks it.
  useEffect(() => {
    if (step === "signins" && signinQueue !== null && signinIndex >= signinQueue.length) {
      funnelStep("a-first-install", "summary");
      setStep("summary");
    }
  }, [step, signinIndex, signinQueue]);
  // AJ-E305 Req.8: wizard state survives an accidental close on the
  // Confirm step — label/plan/selection restore on reopen. Labels and
  // plans ONLY (never a credential-adjacent field).
  const persistKey = `aij.wizard.state.${provider}`;
  useEffect(() => {
    try {
      const saved = window.sessionStorage.getItem(persistKey);
      if (saved === null) return;
      const parsed = JSON.parse(saved) as {
        label?: string;
        plan?: string | null;
        choices?: Record<number, boolean>;
        edits?: Record<string, { purpose: string; plan: string | null }>;
        ignored?: Record<string, boolean>;
      };
      if (typeof parsed.label === "string" && parsed.label.length > 0) setLabel(parsed.label);
      if (parsed.plan !== undefined) setPlan(parsed.plan);
      if (parsed.choices !== undefined) setChoices(parsed.choices);
      if (parsed.edits !== undefined) setEdits(parsed.edits);
      if (parsed.ignored !== undefined) setIgnored(parsed.ignored);
      window.sessionStorage.removeItem(persistKey);
    } catch {
      // Corrupt state restores nothing — the wizard starts clean.
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    if (step !== "identity" && step !== "detect") return;
    // Persist on the Confirm step only; anything past it commits.
    if (step === "identity") {
      window.sessionStorage.setItem(
        persistKey,
        JSON.stringify({ label, plan, choices, edits, ignored }),
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, label, plan, choices, edits, ignored]);

  // Focus trap + return-focus + Esc (idle only — a running setup
  // never dies by stray key).
  useEffect(() => {
    restoreFocusRef.current = document.activeElement as HTMLElement | null;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusables = dialogRef.current?.querySelectorAll<HTMLElement>(
        "button:not(:disabled), a[href], input, [tabindex]:not([tabindex='-1'])",
      );
      if (focusables === undefined || focusables.length === 0) return;
      const first = focusables[0]!;
      const last = focusables[focusables.length - 1]!;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      restoreFocusRef.current?.focus?.();
    };
  }, [busy, onClose]);

  // Focus moves to the primary button on every step change.
  useEffect(() => {
    primaryRef.current?.focus();
  }, [step]);

  const gate = useMemo(() => {
    const mine = installs?.find((row) => row.provider === activeProvider) ?? null;
    return { mine, ready: mine !== null && mine.status !== "not_installed" };
  }, [installs, activeProvider]);

  const title = useMemo(() => {
    if (importing) return `Import “${importing.path.split("/").pop() || importing.path}”`;
    return "Add an account";
  }, [importing]);

  const createTheAccount = async () => {
    if (policyBlocking) return;
    setBusy(true);
    setError(null);
    setImportError(null);
    await commitPolicy();
    // D-12: user-entered only; empty stays unknown.
    const renewal = renewalDate.trim().length > 0 ? renewalDate.trim() : null;
    try {
      if (activeSource !== null) {
        // AJ-E309 Req.7: the inferred name was overridden when the
        // committed label differs from it (the field only, never the
        // value).
        if (importing !== null && label.trim() !== importing.inference.purpose) {
          funnelEvent("inference_edit", { field: "purpose" });
        }
        funnelEvent("import_started");
        try {
          const result = await accountImport(
            activeProvider,
            activeSource,
            label.trim(),
            plan,
            activeOrigin,
            degradedChoice,
          );
          funnelEvent("import_completed", { degraded: result.degraded === true });
          if (result.login === "expired") funnelEvent("login_expired");
          setImportResult(result);
          setAccount(result.account);
          setCreated([
            {
              id: result.account.id,
              label: result.account.label,
              renewal,
              renewalSaved: await saveRenewal(result.account.id, renewal),
            },
          ]);
        } catch (cause) {
          funnelEvent("import_failed", { reason_code: reasonCode(cause) });
          const text = errorText(cause);
          setBusy(false);
          setDegradedChoice(false);
          if (text.includes("Already imported as")) {
            setImportError({ kind: "duplicate", text });
          } else if (text.includes("not a directory") || text.includes("does not exist") || text.includes("may have moved")) {
            setImportError({ kind: "moved", text: "That setup moved or vanished — rescan to see what's here now." });
          } else {
            setRetryable(
              cause !== null &&
                typeof cause === "object" &&
                (cause as { retryable?: unknown }).retryable === true,
            );
            setError(text);
          }
          return;
        }
      } else {
        const createdAccount = await createAccount(activeProvider, label.trim(), plan);
        setAccount(createdAccount);
        setCreated([
          {
            id: createdAccount.id,
            label: createdAccount.label,
            renewal,
            renewalSaved: await saveRenewal(createdAccount.id, renewal),
          },
        ]);
      }
      setBusy(false);
      goto("login");
    } catch (cause) {
      setBusy(false);
      setRetryable(
        cause !== null &&
          typeof cause === "object" &&
          (cause as { retryable?: unknown }).retryable === true,
      );
      setError(errorText(cause));
    }
  };

  // AJ-E307: the manual-folder entry — native picker, metadata-
  // inspected at Confirm before anything is copied. Shared by the
  // Detect link and the Expert-setup hatch.
  const importSpecificFolder = async () => {
    const picked = await pickDirectory();
    if (picked === null) return;
    setManualSource(picked);
    setImporting(null);
    // Zero typed fields on the manual path too — a sensible default
    // label, editable at Confirm.
    setLabel((current) => (current.trim().length > 0 ? current : "Main"));
    goto("identity");
  };

  const recheck = async () => {
    if (!account) return;
    setLoginState(await accountLoginState(account.id));
    onAccountCreated();
  };

  const requestClose = () => {
    // In-flight setup never dies silently (Req.5): block the close,
    // offer the explicit "it will appear in Accounts" confirm.
    if (busy) {
      setConfirmClose(true);
      return;
    }
    onClose();
  };

  const providerName =
    installs?.find((row) => row.provider === activeProvider)?.display_name ?? activeProvider;

  // Forward transitions emit their funnel step (AJ-E305 quality
  // gate); Back stays silent — the funnel records progress, not
  // navigation noise.
  const FUNNEL_BY_STEP: Record<Step, string> = {
    install: "gate",
    detect: "scan",
    identity: "confirm",
    login: "signins",
    signins: "signins",
    summary: "summary",
  };
  const goto = (next: Step) => {
    funnelStep("a-first-install", FUNNEL_BY_STEP[next] ?? next);
    setStep(next);
  };

  // The funnel starts at the gate — mount IS the first step (a
  // locked flow (c)/(d) skips the gate and starts at the scan).
  useEffect(() => {
    funnelStep("a-first-install", locked ? "scan" : "gate");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // AJ-E309 (Nielsen H3): the Summary restates the ACTIVE pack policy
  // and each new Account's renewal date ("expiry unknown" when empty —
  // never inferred, D-12).
  const restatement = (
    <div className="flex flex-col gap-1 rounded-xl border border-hairline px-4 py-3" data-testid="choices-restated">
      <p className="text-xs text-ink" data-testid="policy-restatement">
        {packPolicyRestatement(
          policyKnown?.effective ?? null,
          policyKnown?.source ?? "unset",
          providerName,
        )}
      </p>
      {policyWriteFailed && (
        <p className="text-[11px] text-danger" role="alert">
          Your pack choice couldn't be saved — set it in Accounts → Pack secrets.
        </p>
      )}
      {created.map((entry) => (
        <p key={entry.id} className="text-[11px] text-muted" data-testid="renewal-restatement">
          {entry.label}:{" "}
          {entry.renewal === null
            ? "expiry unknown"
            : entry.renewalSaved
              ? `renews ${entry.renewal} — we'll warn you before it`
              : `renewal date ${entry.renewal} couldn't be saved — add it in Accounts`}
        </p>
      ))}
      <p className="text-[11px] text-muted">Change these anytime in Accounts.</p>
    </div>
  );
  const expertAfter = (
    <ExpertSetup
      provider={activeProvider}
      accounts={created.map((entry) => ({ id: entry.id, label: entry.label }))}
      surface="wizard"
      showMetricsExport
    />
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={`${providerName} setup wizard`}
        className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden rounded-card border border-hairline bg-canvas shadow-card"
        data-testid="onboarding-wizard"
      >
        {/* Wizard header: progress + close */}
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent">
            <IconJuggler width={18} height={18} />
          </span>
          <h3 className="text-sm font-semibold">{title}</h3>
          <div
            role="group"
            className="ml-auto flex items-center gap-1.5"
            aria-label={`Step ${STEP_DOT[step] + 1} of 4: ${DOT_LABELS[STEP_DOT[step]]}`}
          >
            {DOT_LABELS.map((_, i) => (
              <span
                key={i}
                className={
                  "h-1 rounded-full transition-all " +
                  (STEP_DOT[step] >= i ? "w-5 bg-accent" : "w-2 bg-hairline")
                }
              />
            ))}
          </div>
          <button
            type="button"
            onClick={requestClose}
            aria-label="Close setup"
            title={busy ? "Setup is still running — it will appear in Accounts" : undefined}
            className="ml-3 rounded-md px-2 py-1 text-[11px] text-faint hover:bg-surface hover:text-ink"
          >
            ✕
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
          {/* -------------------------------------------------- install */}
          {step === "install" && (
            <div className="flex flex-col gap-4" data-testid="wizard-step-install">
              <div>
                <p className="text-sm font-medium">First, the CLI itself.</p>
                <p className="mt-1 text-xs leading-relaxed text-muted">
                  AI Juggler drives each provider's own command-line tool. Here's what's
                  installed on this Mac — you can't continue with a provider that isn't
                  installed yet (that never ends well three steps later).
                </p>
              </div>

              {installs === null ? (
                <div className="flex flex-col gap-2" data-testid="install-checking">
                  {[0, 1].map((row) => (
                    <div
                      key={row}
                      className="h-16 animate-pulse rounded-xl border border-hairline bg-surface"
                    />
                  ))}
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  {installs.map((install) => (
                    <ProviderRow
                      key={install.provider}
                      install={install}
                      selected={provider === install.provider}
                      onNotice={() => {
                        /* Informing only — the gate lives on Continue. */
                      }}
                    />
                  ))}
                </div>
              )}
              {error && <p className="text-xs text-danger">{error}</p>}
            </div>
          )}

          {/* --------------------------------------------------- detect */}
          {step === "detect" && (
            <div className="flex flex-col gap-4">
              <div>
                <p className="text-sm font-medium">
                  Let's look at what {providerName} already has.
                </p>
                <p className="mt-1 text-xs leading-relaxed text-muted">
                  AI Juggler scans the usual spots for existing setups — your standard
                  install, configs left by other tools, anything an AI assistant may have
                  created. Importing keeps your settings and history; the original is never
                  touched.
                </p>
              </div>

              {recommendation !== null &&
                recommendation.items.length > 0 &&
                acceptRun === null &&
                setSummary === null && (
                <div
                  className="flex flex-col gap-2.5 rounded-xl border border-accent/30 bg-accent-soft px-4 py-3.5"
                  data-testid="proposal-teaser"
                >
                  <p className="text-xs leading-relaxed text-ink" data-testid="proposal-summary">
                    {recommendation.summary}
                  </p>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => setProposalOpen(true)}
                      data-testid="use-recommended"
                    >
                      Use recommended setup →
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        setProposalOpen((open) => !open);
                        if (!proposalCustomised) {
                          setProposalCustomised(true);
                          funnelEvent("proposal_customised");
                        }
                      }}
                      data-testid="proposal-customise"
                      aria-expanded={proposalOpen}
                    >
                      Customise
                    </Button>
                  </div>
                </div>
              )}
              {proposalOpen && recommendation !== null && acceptRun === null && (
                <ProposalBlock
                  recommendation={recommendation}
                  choices={choices}
                  onChoice={(index, on) => {
                    setChoices((current) => ({ ...current, [index]: on }));
                    if (!proposalCustomised) {
                      setProposalCustomised(true);
                      funnelEvent("proposal_customised");
                    }
                  }}
                  onAccept={() => void acceptTheSetup()}
                  renewals={renewals}
                  onRenewal={(index, date) =>
                    setRenewals((current) => ({ ...current, [index]: date }))
                  }
                  policySlot={
                    policyNeeded ? (
                      <PackPolicyChooser value={policyChoice} onChange={setPolicyChoice} />
                    ) : null
                  }
                  acceptBlocked={
                    policyBlocking ? "Choose how packs handle secrets first — or Decide later." : null
                  }
                />
              )}

              {acceptRun !== null ? (
                <ul className="flex flex-col gap-1.5" data-testid="accept-progress" aria-live="polite">
                  {acceptRun.map((entry) => (
                    <li
                      key={entry.label}
                      className="flex items-center gap-2 rounded-lg border border-hairline px-3 py-2 text-xs"
                      data-testid="accept-item"
                      data-state={entry.state}
                    >
                      <span
                        className={
                          entry.state === "done"
                            ? "text-success"
                            : entry.state === "failed"
                              ? "text-danger"
                              : "text-muted"
                        }
                      >
                        {entry.state === "done" ? "✓" : entry.state === "failed" ? "✕" : "…"}
                      </span>
                      <span className="min-w-0 flex-1 truncate">{entry.label}</span>
                      <span className="text-[10px] text-muted">
                        {entry.state === "pending"
                          ? "importing…"
                          : entry.state === "done"
                            ? "imported"
                            : "failed"}
                      </span>
                      {entry.state === "failed" && entry.error && (
                        <span className="text-[10px] text-danger">{entry.error}</span>
                      )}
                    </li>
                  ))}
                </ul>
              ) : report === null ? (
                <div
                  className="flex items-center gap-3 rounded-xl border border-hairline px-4 py-5"
                  aria-live="polite"
                  data-testid="scan-progress"
                >
                  <span className="dot-live size-2 rounded-full bg-live" />
                  <span className="text-xs text-muted">
                    Checking your machine… {progress ? `${progress.done} of ${progress.total} checks` : "…"}
                  </span>
                </div>
              ) : reportDeniedCount(report.probes) > 0 ? (
                <div
                  className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-4"
                  data-testid="scan-denied"
                >
                  <p className="text-xs leading-relaxed text-muted">
                    We couldn't check {reportDeniedCount(report.probes)}{" "}
                    {reportDeniedCount(report.probes) === 1 ? "place" : "places"} — your setups{" "}
                    {reportDeniedCount(report.probes) === 1 ? "there won't" : "there won't"}{" "}
                    appear.
                  </p>
                  {report.cancelled && (
                    <p className="text-[11px] text-faint">The check was stopped early.</p>
                  )}
                </div>
              ) : report.installs.length === 0 &&
                (recommendation === null || recommendation.items.length === 0) ? (
                <div
                  className="flex flex-col gap-3 rounded-xl border border-hairline px-4 py-4"
                  data-testid="zero-found"
                >
                  <div>
                    <p className="text-sm font-medium">Create your first account</p>
                    <p className="mt-1 text-xs leading-relaxed text-muted">
                      Nothing to import{locked ? ` — start a fresh ${providerName} account` : " — pick the Provider to start with. You can add more anytime"}.
                    </p>
                  </div>
                  {locked && (
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={() => {
                        setImporting(null);
                        setLabel((current) =>
                          current.trim().length > 0 ? current : "Main",
                        );
                        goto("identity");
                      }}
                      data-testid="zero-found-create"
                    >
                      Create your first {providerName} account
                    </Button>
                  )}
                  <ul className={locked ? "hidden" : "flex flex-col gap-1.5"}>
                    {[...(installs ?? [])]
                      .sort((a, b) => Number(b.status !== "not_installed") - Number(a.status !== "not_installed"))
                      .map((row) => {
                        const ready = row.status !== "not_installed";
                        return (
                          <li key={row.provider} className="flex items-center gap-2">
                            <button
                              type="button"
                              disabled={!ready}
                              aria-label={
                                ready
                                  ? `Create your first ${row.display_name} account`
                                  : `${row.display_name} isn't installed`
                              }
                              onClick={() => {
                                setProviderOverride(row.provider);
                                setImporting(null);
                                // Prefill only when nothing survived a
                                // restore (Req.8 keeps the draft).
                                setLabel((current) =>
                                  current.trim().length > 0 ? current : "Main",
                                );
                                goto("identity");
                              }}
                              className={
                                "flex min-w-0 flex-1 items-center gap-2 rounded-lg border px-3 py-2 text-left text-xs transition-colors " +
                                (ready
                                  ? "border-hairline hover:border-accent/40 hover:bg-surface"
                                  : "cursor-not-allowed border-hairline/50 opacity-50")
                              }
                              data-testid="zero-found-provider"
                              data-installed={ready ? "true" : "false"}
                            >
                              <span className="font-medium">{row.display_name}</span>
                              <span className="text-[10px] text-muted">
                                {ready ? `installed · ${row.version ?? ""}` : "not installed"}
                              </span>
                            </button>
                            {!ready && row.install_url && (
                              <a
                                href={row.install_url}
                                target="_blank"
                                rel="noreferrer"
                                className="shrink-0 text-[11px] text-accent underline-offset-2 hover:underline"
                              >
                                Install {row.display_name} →
                              </a>
                            )}
                          </li>
                        );
                      })}
                  </ul>
                </div>
              ) : (
                <CandidateCards
                  installs={report.installs.filter((found) => !ignored[found.path])}
                  edits={edits}
                  editing={editing}
                  onEdit={(path) => setEditing((current) => (current === path ? null : path))}
                  onEditField={(path, patch) =>
                    setEdits((current) => ({
                      ...current,
                      [path]: {
                        purpose: patch.purpose ?? current[path]?.purpose ?? "",
                        plan: patch.plan !== undefined ? patch.plan : (current[path]?.plan ?? null),
                      },
                    }))
                  }
                  onImport={(found, purpose) => {
                    setImporting(found);
                    setLabel(purpose);
                    goto("identity");
                  }}
                  onIgnore={(path) =>
                    setIgnored((current) => ({ ...current, [path]: true }))
                  }
                />
              )}
              {report !== null &&
                report.installs.filter((found) => ignored[found.path]).length > 0 && (
                  <div className="flex flex-col gap-1.5">
                    <button
                      type="button"
                      aria-expanded={showIgnored}
                      onClick={() => setShowIgnored((open) => !open)}
                      className="self-start text-[11px] text-muted underline-offset-2 hover:text-ink hover:underline"
                      data-testid="ignored-toggle"
                    >
                      Ignored ({report.installs.filter((found) => ignored[found.path]).length}){" "}
                      {showIgnored ? "▾" : "▸"}
                    </button>
                    {showIgnored && (
                      <ul className="flex flex-col gap-1" data-testid="ignored-list">
                        {report.installs
                          .filter((found) => ignored[found.path])
                          .map((found) => (
                            <li
                              key={found.path}
                              className="flex items-center gap-2 rounded-lg border border-hairline px-3 py-1.5 text-[11px]"
                            >
                              <span className="min-w-0 flex-1 truncate font-mono text-muted">
                                {found.path}
                              </span>
                              <button
                                type="button"
                                onClick={() =>
                                  setIgnored((current) => {
                                    const next = { ...current };
                                    delete next[found.path];
                                    return next;
                                  })
                                }
                                className="text-muted underline hover:text-ink"
                              >
                                Unignore
                              </button>
                            </li>
                          ))}
                      </ul>
                    )}
                  </div>
                )}

              {report !== null && (
                <>
                  <button
                    type="button"
                    onClick={() => void importSpecificFolder()}
                    data-testid="import-specific-folder"
                    className="self-start text-xs text-muted underline-offset-2 hover:text-ink hover:underline"
                  >
                    Import from a specific folder…
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setImporting(null);
                      setManualSource(null);
                      setLabel("");
                      goto("identity");
                    }}
                    className="self-start text-xs text-muted underline-offset-2 hover:text-ink hover:underline"
                  >
                    {report.installs.length > 0
                      ? "None of these — add a fresh account"
                      : "Continue with a fresh account →"}
                  </button>
                  <div className="flex flex-col gap-1.5">
                    <button
                      type="button"
                      aria-expanded={detailsOpen}
                      onClick={() => setDetailsOpen((open) => !open)}
                      className="self-start text-[11px] text-muted underline-offset-2 hover:text-ink hover:underline"
                      data-testid="scan-details-toggle"
                    >
                      Details {detailsOpen ? "▾" : "▸"}
                    </button>
                    {detailsOpen && (
                      <ul
                        className="flex flex-col gap-1 rounded-lg border border-hairline bg-canvas px-3 py-2"
                        data-testid="scan-details"
                      >
                        {report.probes.map((probe) => (
                          <ScanProbeRow key={probe.id} probe={probe} />
                        ))}
                      </ul>
                    )}
                  </div>
                  <ExpertSetup
                    provider={activeProvider}
                    surface="wizard"
                    showMetricsExport
                    onImportFolder={() => void importSpecificFolder()}
                  />
                </>
              )}
            </div>
          )}

          {/* ------------------------------------------------- identity */}
          {step === "identity" && (
            <div className="flex flex-col gap-4">
              {/* AJ-E307 Req.3: Confirm-source — path, origin badge,
                  total size, per-folder breakdown, keychain note —
                  all BEFORE any copy starts. */}
              {activeSource !== null && inspect !== null && (
                <div
                  className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3"
                  data-testid="confirm-source"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <Chip tone="accent">{activeOrigin ?? "manual"}</Chip>
                    <span className="truncate font-mono text-[11px] text-muted">
                      {activeSource}
                    </span>
                    <span className="ml-auto text-[10px] text-muted">
                      {inspect.file_count.toLocaleString()} files ·{" "}
                      {humanBytes(inspect.total_bytes)}
                      {inspect.truncated ? "+" : ""}
                    </span>
                  </div>
                  <ul className="flex flex-col gap-0.5" data-testid="source-breakdown">
                    {inspect.folders.slice(0, 5).map((folder) => (
                      <li
                        key={folder.name}
                        className="flex items-center justify-between text-[11px] text-muted"
                      >
                        <span className="truncate font-mono">{folder.name}</span>
                        <span>{humanBytes(folder.bytes)}</span>
                      </li>
                    ))}
                  </ul>
                  {inspect.total_bytes > inspect.ceiling_bytes ? (
                    <p className="text-[11px] leading-relaxed text-muted" role="alert">
                      The history in this folder is large ({humanBytes(inspect.total_bytes)}) —
                      import the settings only, or start fresh.
                    </p>
                  ) : inspect.keychain_bound ? (
                    <p className="text-[11px] leading-relaxed text-muted">
                      Signed in via the macOS Keychain — you'll sign in once more after
                      importing (the keychain entry is locked to this folder).
                    </p>
                  ) : null}
                </div>
              )}
              {activeSource !== null && inspect !== null && !inspect.is_config && (
                <div
                  className="flex flex-col gap-2 rounded-xl border border-danger/30 bg-danger-soft px-4 py-3"
                  role="alert"
                  data-testid="source-not-config"
                >
                  <p className="text-xs leading-relaxed text-ink">
                    This folder doesn't look like a {providerName} setup — no settings,
                    credentials, or session history found.
                  </p>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setManualSource(null);
                      setImporting(null);
                      goto("detect");
                    }}
                  >
                    Pick a different folder
                  </Button>
                </div>
              )}
              {importError?.kind === "duplicate" && (
                <div
                  className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3"
                  role="alert"
                  data-testid="duplicate-blocked"
                >
                  <p className="text-xs leading-relaxed text-ink">{importError.text}</p>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      onAccountCreated();
                      onClose();
                    }}
                  >
                    Open it
                  </Button>
                </div>
              )}
              {importError?.kind === "moved" && (
                <div
                  className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3"
                  role="alert"
                  data-testid="source-vanished"
                >
                  <p className="text-xs leading-relaxed text-ink">{importError.text}</p>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setManualSource(null);
                      setImporting(null);
                      setImportError(null);
                      setReport(null);
                      goto("detect");
                    }}
                  >
                    Rescan
                  </Button>
                </div>
              )}
              {importing ? (
                <div className="rounded-xl border border-hairline bg-canvas px-4 py-3 text-xs leading-relaxed text-muted">
                  Importing from{" "}
                  <span className="font-mono text-[11px]">{importing.path}</span>
                  {signedIn(importing.login) &&
                    " — your settings and session history come along."}
                  {importing.login === "logged_out" &&
                    " — its settings come along; you'll sign in after."}
                </div>
              ) : (
                <p className="text-xs leading-relaxed text-muted">
                  Each account is a separate {providerName} login, fully isolated — what
                  happens in one never touches the others, or your existing install.
                </p>
              )}

              <label className="block text-[11px] uppercase tracking-wider text-muted">
                What will you use this account for?
                <input
                  value={label}
                  onChange={(event) => setLabel(event.target.value)}
                  placeholder="e.g. Work, Personal, Side project"
                  className="mt-1.5 w-full rounded-lg border border-hairline bg-canvas px-3 py-2.5 text-sm text-ink outline-none placeholder:text-faint focus:border-accent/50"
                />
              </label>

              <div>
                <p className="text-[11px] uppercase tracking-wider text-muted">Plan</p>
                <div className="mt-1.5 grid grid-cols-2 gap-2">
                  {PLANS.map((tier) => (
                    <button
                      key={tier.label}
                      type="button"
                      onClick={() => setPlan(tier.id)}
                      className={
                        "rounded-lg border px-3 py-2 text-left text-xs transition-colors " +
                        (plan === tier.id
                          ? "border-accent/40 bg-accent-soft text-accent-hi"
                          : "border-hairline hover:border-hairline/80 hover:bg-surface")
                      }
                    >
                      <span className="block font-medium">{tier.label}</span>
                      <span className="block text-[10px] text-muted">{tier.hint}</span>
                    </button>
                  ))}
                </div>
              </div>

              {/* AJ-E309 Req.3 (D-12): optional, user-entered only. */}
              <RenewalField
                value={renewalDate}
                onChange={setRenewalDate}
                accountLabel={label.trim() || "this account"}
              />

              {/* AJ-E309 Req.1: the first-run policy choice, merged
                  with Confirm — no extra step; nothing pre-selected. */}
              {policyNeeded && (
                <PackPolicyChooser value={policyChoice} onChange={setPolicyChoice} />
              )}

              {/* AJ-E309 Req.5: the expert hatch (no account yet —
                  the Provider's folder; the shell line follows once
                  the account exists). */}
              <ExpertSetup
                provider={activeProvider}
                surface="wizard"
                showMetricsExport
                onImportFolder={() => void importSpecificFolder()}
              />

              {error && (
                <div className="flex flex-col gap-1.5" data-testid="wizard-error">
                  <p className="text-xs text-danger">{error}</p>
                  {retryable && (
                    <button
                      type="button"
                      onClick={() => void createTheAccount()}
                      className="self-start text-[11px] text-muted underline hover:text-ink"
                    >
                      Try again
                    </button>
                  )}
                </div>
              )}
            </div>
          )}

          {/* ---------------------------------------------------- login */}
          {step === "login" && account && (
            <div className="flex flex-col gap-4">
              {signedIn(loginState) || signedIn(account.login_state) ? (
                <div className="flex flex-col items-center gap-3 py-6 text-center">
                  <span className="dot-live size-2.5 rounded-full bg-success" />
                  <p className="text-sm font-medium">Signed in and ready</p>
                  <p className="text-xs text-muted">
                    {importResult?.login_note ?? "This account is authenticated."}
                  </p>
                </div>
              ) : (
                <>
                  <div>
                    <p className="text-sm font-medium">
                      Sign in to finish setting up “{account.label}”.
                    </p>
                    <p className="mt-1 text-xs leading-relaxed text-muted">
                      This is {providerName}'s own sign-in — AI Juggler never sees your
                      credentials.
                      {importResult?.login_note && (
                        <span className="mt-2 block rounded-lg border border-hairline bg-canvas px-3 py-2 text-[11px] leading-relaxed text-faint">
                          {importResult.login_note}
                        </span>
                      )}
                    </p>
                  </div>

                  <div className="rounded-xl border border-hairline bg-canvas px-3 py-2.5 font-mono text-[11px] leading-relaxed text-success">
                    {command || "…"}
                  </div>

                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={async () => {
                        const terminal = await terminalSpawnLogin(account.id);
                        openLoginTerminal(terminal.id, `Sign in · ${account.label}`);
                      }}
                    >
                      <IconTerminal width={14} height={14} />
                      Sign in inside AI Juggler
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => void loginOpenWeb(provider)}
                    >
                      Open the {providerName} site
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={async () => {
                        await navigator.clipboard.writeText(command);
                        setCopied(true);
                        window.setTimeout(() => setCopied(false), 1500);
                      }}
                    >
                      {copied ? "Copied" : "Copy command"}
                    </Button>
                    <Button size="sm" variant="secondary" onClick={() => void recheck()}>
                      Re-check
                    </Button>
                  </div>

                  {loginState === "logged_out" && (
                    <p className="text-[11px] text-faint">
                      Not signed in yet — complete {providerName}'s sign-in (the terminal
                      opened in the panel), then Re-check.
                    </p>
                  )}
                </>
              )}
            </div>
          )}

          {/* ------------------------------------------------- signins */}
          {step === "signins" && signinQueue !== null && (
            <div className="flex flex-col gap-4" data-testid="signin-queue">
              <div>
                <p className="text-sm font-medium">
                  Sign in{" "}
                  {signinQueue.length > 1
                    ? `(${signinIndex + 1} of ${signinQueue.length})`
                    : "to finish setting up"}
                </p>
                <p className="mt-1 text-xs leading-relaxed text-muted">
                  Skipping is fine; you can sign in anytime from Accounts.
                </p>
              </div>
              {currentSignin !== null && (
                <LoginQueueCard
                  key={currentSignin.accountId}
                  accountId={currentSignin.accountId}
                  label={currentSignin.label}
                  providerName={currentSignin.provider}
                  initialState="logged_out"
                  loginNote={currentSignin.loginNote}
                  onSpawned={(terminalId) =>
                    openLoginTerminal(terminalId, `Sign in · ${currentSignin.label}`)
                  }
                  onReady={() => {
                    funnelEvent("login_completed");
                    setSigninIndex((index) => index + 1);
                  }}
                  onSkip={() => {
                    funnelEvent("login_skipped");
                    setSigninIndex((index) => index + 1);
                  }}
                />
              )}
              {signinQueue.length > 1 && (
                <p className="text-[11px] text-muted">
                  {signinQueue.length - signinIndex} remaining — skip never blocks your setup.
                </p>
              )}
            </div>
          )}

          {/* -------------------------------------------------- summary */}
          {step === "summary" && setSummary !== null && (
            <div className="flex flex-col gap-4" data-testid="set-summary">
              <div className="flex items-center gap-2">
                <span className="dot-live size-2.5 rounded-full bg-success" />
                <p className="text-sm font-medium">Your juggling set is ready</p>
              </div>
              <ul className="flex flex-col gap-2 text-xs leading-relaxed text-muted">
                {setSummary.map((entry) => (
                  <li key={entry.label} className="flex gap-2" data-testid="set-summary-item" data-state={entry.state}>
                    <span className={entry.state === "done" ? "text-success" : "text-danger"}>
                      {entry.state === "done" ? "✓" : "✕"}
                    </span>
                    <span>
                      <span className="text-ink">{entry.label}</span> —{" "}
                      {entry.state === "done" ? "imported" : `failed: ${entry.error ?? "unknown"}`}
                    </span>
                  </li>
                ))}
              </ul>
              {isolation !== null && (
                <div className="flex flex-col gap-1.5" data-testid="isolation-checklist">
                  <p className="text-[11px] font-medium text-muted">
                    Isolation verified — every account fenced
                  </p>
                  <ul className="flex flex-col gap-0.5">
                    {isolation.map((row) => (
                      <li
                        key={row.id}
                        className="flex items-center gap-2 text-[11px]"
                        data-testid="isolation-row"
                        data-pass={row.pass ? "true" : "false"}
                      >
                        <span className={row.pass ? "text-success" : "text-danger"}>
                          {row.pass ? "✓" : "✕"}
                        </span>
                        <span className="text-muted">{row.label}</span>
                        {!row.pass && (
                          <span className="text-danger" role="alert">
                            {row.detail}
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {restatement}
              {recommendation?.start_here && (
                <p className="text-xs text-muted" data-testid="start-here">
                  Start here: <span className="font-mono text-ink">{recommendation.start_here}</span>{" "}
                  — your most recent project.
                </p>
              )}
              {/* AJ-E305 Req.4: the Summary exits — launch (with the
                  Start-here project preselected), keep going, or done. */}
              <div className="flex flex-wrap gap-2" data-testid="summary-exits">
                <Button
                  ref={primaryRef}
                  size="sm"
                  variant="primary"
                  disabled={isolation?.some((row) => !row.pass) ?? false}
                  title={
                    isolation?.some((row) => !row.pass)
                      ? `Isolation check failed: ${isolation.find((row) => !row.pass)!.label}`
                      : undefined
                  }
                  onClick={() => {
                    window.sessionStorage.removeItem(persistKey);
                    onAccountCreated();
                    onLaunchFirstSession?.(recommendation?.start_here ?? null);
                    onClose();
                  }}
                  data-testid="launch-first-session"
                >
                  Launch first session →
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => {
                    window.sessionStorage.removeItem(persistKey);
                    onAccountCreated();
                    onAddAnother?.();
                  }}
                  data-testid="add-another-account"
                >
                  Add another account
                </Button>
                <Button size="sm" variant="secondary" onClick={() => {
                  window.sessionStorage.removeItem(persistKey);
                  onAccountCreated();
                  onClose();
                }}>
                  Done
                </Button>
              </div>
              {expertAfter}
            </div>
          )}
          {step === "summary" && account && setSummary === null && (
            <div className="flex flex-col gap-4">
              <div className="flex items-center gap-2">
                <span className="dot-live size-2.5 rounded-full bg-success" />
                <p className="text-sm font-medium">“{account.label}” is ready</p>
                {account.plan && <Chip tone="accent">{account.plan}</Chip>}
              </div>

              <ul className="flex flex-col gap-2 text-xs leading-relaxed text-muted">
                <li className="flex gap-2">
                  <span className="text-success">✓</span>
                  {importing
                    ? `Imported ${importResult ? importResult.files_copied : 0} files${
                        importResult ? ` (${humanBytes(importResult.bytes_copied)})` : ""
                      } into a private, isolated copy` +
                      (importResult && importResult.skipped.length > 0
                        ? ` — ${importResult.skipped.length} cache folders skipped`
                        : "")
                    : `Created a private, isolated ${providerName} account with default Launch settings`}
                </li>
                {importing && (
                  <li className="flex gap-2">
                    <span className="text-success">✓</span>
                    Your original at{" "}
                    <span className="font-mono text-[10.5px]">{importing.path}</span> was not
                    modified
                  </li>
                )}
                <li className="flex gap-2">
                  <span className="text-success">✓</span>
                  Launch it anytime via <span className="text-ink">New session</span> in the
                  rail, or from a project's “launch with” list
                </li>
              </ul>

              {restatement}
              {expertAfter}

              {!hasLiveBackend() && (
                <p className="text-[11px] text-muted">(preview mode — data shown is sample data)</p>
              )}
            </div>
          )}
        </div>

        {/* Footer actions */}
        <div className="flex items-center gap-2 border-t border-hairline/60 px-6 py-4">
          {step === "detect" && (
            <Button size="sm" variant="secondary" onClick={() => setStep("install")}>
              Back
            </Button>
          )}
          {step === "identity" && (
            <Button size="sm" variant="secondary" onClick={() => setStep("detect")}>
              Back
            </Button>
          )}
          {step === "login" && (
            <Button size="sm" variant="secondary" onClick={() => setStep("identity")}>
              Back
            </Button>
          )}
          <span className="flex-1" />
          {step === "install" && (
            <Button
              ref={primaryRef}
              size="sm"
              variant="primary"
              disabled={installs === null || !gate.ready}
              title={
                installs !== null && !gate.ready && gate.mine
                  ? `${gate.mine.display_name} isn't installed — install it first or continue without it`
                  : undefined
              }
              onClick={() => goto("detect")}
              data-testid="install-continue"
            >
              Continue with {gate.mine?.display_name ?? providerName}
            </Button>
          )}
          {step === "identity" &&
          activeSource !== null &&
          inspect !== null &&
          inspect.is_config &&
          inspect.total_bytes > inspect.ceiling_bytes ? (
            <>
              {policyBlocking && (
                <span id="wizard-policy-hint" className="text-[11px] text-muted" data-testid="create-blocked">
                  Choose how packs handle secrets first.
                </span>
              )}
              <Button
                ref={primaryRef}
                size="sm"
                variant="primary"
                disabled={busy || label.trim().length === 0 || policyBlocking}
                aria-describedby={policyBlocking ? "wizard-policy-hint" : undefined}
                onClick={() => {
                  setDegradedChoice(true);
                  void createTheAccount();
                }}
                data-testid="import-settings-only"
              >
                {busy ? "Importing…" : "Import settings only — skip history"}
              </Button>
              <Button
                size="sm"
                variant="secondary"
                disabled={busy}
                onClick={() => {
                  setManualSource(null);
                  setImporting(null);
                  setLabel("");
                  goto("detect");
                }}
                data-testid="add-fresh-instead"
              >
                Add fresh
              </Button>
            </>
          ) : step === "identity" ? (
            <>
              {policyBlocking && (
                <span id="wizard-policy-hint" className="text-[11px] text-muted" data-testid="create-blocked">
                  Choose how packs handle secrets first.
                </span>
              )}
              <Button
                ref={primaryRef}
                size="sm"
                variant="primary"
                disabled={busy || label.trim().length === 0 || policyBlocking}
                aria-describedby={policyBlocking ? "wizard-policy-hint" : undefined}
                onClick={() => void createTheAccount()}
                data-testid="create-account"
              >
                {busy ? "Setting up…" : activeSource !== null ? "Import account" : "Create account"}
              </Button>
            </>
          ) : null}
          {step === "login" && (
            <Button
              ref={primaryRef}
              size="sm"
              variant="primary"
              onClick={() => {
                funnelEvent(
                  signedIn(loginState) || signedIn(account?.login_state) ? "login_completed" : "login_skipped",
                );
                onAccountCreated();
                goto("summary");
              }}
            >
              {signedIn(loginState) ? "Continue" : "I'll sign in later"}
            </Button>
          )}
          {step === "signins" && (
            /* The queue's own [Skip for now] drives progression — the
               footer stays out of the way (Skip never blocks Summary). */
            <span className="text-[11px] text-muted">
              {signinQueue !== null && currentSignin !== null
                ? `${signinQueue.length - signinIndex} of ${signinQueue.length} to go`
                : "…"}
            </span>
          )}
          {step === "summary" && setSummary === null && (
            <Button
              ref={primaryRef}
              size="sm"
              variant="primary"
              onClick={() => {
                window.sessionStorage.removeItem(persistKey);
                onAccountCreated();
                onClose();
              }}
            >
              {signedIn(loginState) ? "Continue" : "Done"}
            </Button>
          )}
        </div>
      </div>

      {confirmClose && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 p-6">
          <Card className="w-full max-w-sm p-5" data-testid="wizard-close-confirm">
            <p className="text-sm font-medium">Setup is still running</p>
            <p className="mt-1.5 text-xs leading-relaxed text-muted">
              Closing now is safe — the account will finish setting up and appear in
              Accounts. You can re-check its sign-in there.
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <Button size="sm" variant="secondary" onClick={() => setConfirmClose(false)}>
                Keep waiting
              </Button>
              <Button size="sm" variant="primary" onClick={onClose}>
                Close anyway
              </Button>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
