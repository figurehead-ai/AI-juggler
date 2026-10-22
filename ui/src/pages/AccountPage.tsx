/**
 * An account's page (AJ-E1533): its Overview — its windows, renewal,
 * fee, cleanup and preview opt-ins, what runs on it now and where it
 * works — and its Sessions. Moved from RightInspector (the size budget);
 * the cleanup toggles serve Settings too.
 */
import { useEffect, useRef, useState } from "react";
import { Button, Card, Chip, formatCountdown } from "../components/primitives";
import { WindowRing } from "../components/QuotaRings";
import { accountSetFee, quotaDisplay, signedIn, type AccountInfo, type InstanceInfo, type ProjectInfo, type QuotaDisplayData } from "../lib/bridge";
import { renewalLabel } from "../lib/renewal";
import { SessionsList, useSessionRows } from "./SessionsList";

export type AccountPanelProps = {
  account: AccountInfo;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
};

/** Account Overview (§5.4): rings with % text, forecast, affinity,
 * renewal field (E411), logged-out collapse to the sign-in card. */
export function AccountOverview(props: AccountPanelProps) {
  const account = props.account;
  const rows = props.sessions.filter((s) => s.account_id === account.id);
  const worst = rows
    .filter((s) => s.quota !== null)
    .reduce<InstanceInfo | null>((min, row) => {
      if (!min) return row;
      return (100 - row.quota!.used_percentage) < (100 - min.quota!.used_percentage) ? row : min;
    }, null);
  // Where this account works: projects it was used on, still where
  // they were (a moved one says so on its own page), newest first.
  const usualProjects = props.projects
    .filter((p) => p.accounts_used.includes(account.label) && !p.missing)
    .sort((a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0));

  if (!signedIn(account.login_state)) {
    // Logged-out collapse: the Overview IS the sign-in card (Req.3).
    return (
      <Card className="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center" data-testid="account-signin-card">
        <p className="text-[13px] font-medium text-ink">{account.label} is signed out</p>
        <p className="max-w-xs text-[11.5px] leading-relaxed text-muted">
          Sign in from the Accounts screen to see quota, usage and launch controls again. Nothing is
          deleted — the isolated account state is kept.
        </p>
        <Button size="sm" variant="primary" onClick={() => window.dispatchEvent(new Event("aij:open-accounts"))}>
          Open Accounts to sign in
        </Button>
      </Card>
    );
  }

  return (
    <div className="flex flex-col gap-3" data-testid="account-overview">
      <Card className="flex flex-wrap items-center gap-4 p-4">
        <div className="flex flex-wrap items-center gap-4" data-testid="account-rings">
          {/* AJ-E610: engine-evaluated windows — ring + % + badge + reset;
              an omitted window renders its reason, never an empty ring. */}
          <AccountWindowRings accountId={account.id} />
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-1 text-[11px]">
          <span className="flex items-center gap-2">
            <Chip tone="success">signed in</Chip>
            <span className="text-muted">plan: <span className="text-ink">{account.plan ?? "unknown"}</span></span>
          </span>
          <span data-testid="account-renewal" className="flex flex-wrap items-center gap-1.5">
            {(() => {
              const label = renewalLabel(account.renewal_date, props.now);
              if (account.renewal_date === null) {
                return <span className="text-muted">Expiry: unknown</span>;
              }
              if (label?.tier === "warning") {
                return <Chip tone="warning">⚠ {label.text}</Chip>;
              }
              if (label?.tier === "danger") {
                return <Chip tone="danger">⛔ {label.text}</Chip>;
              }
              return <span className="text-muted">{label?.text ?? "Expiry: unknown"}</span>;
            })()}
          </span>
          <RenewalEditor account={account} />
          <FeeEditor account={account} />
          {/*
           * AJ-E508 (D-08): the Account launch-settings area's TWO
           * separate tier toggles — never one bundled "auto-cleanup"
           * switch. Exact labels; both off by default.
           */}
          <TierToggles accountId={account.id} initial={account.cleanup_opt_in} />
          <label data-testid="account-preview-toggle" className="flex items-center gap-1.5">
            <input
              type="checkbox"
              aria-label="Live preview of desktop sessions (opt-in)"
              checked={account.preview_opt_in}
              onChange={(event) => {
                const optIn = event.target.checked;
                void import("../lib/bridge").then((bridge) =>
                  bridge.accountSetPreviewOptIn(account.id, optIn).catch(() => {
                    /* re-reads from props on the next list refresh */
                  }),
                );
              }}
            />
            <span className="text-muted">Live preview of desktop sessions (opt-in)</span>
          </label>
          <span data-testid="account-reset" className="text-muted">
            reset:{" "}
            {worst?.quota?.resets_at_ms ? (
              <span className="text-ink">{formatCountdown(worst.quota.resets_at_ms, props.now)}</span>
            ) : (
              <span className="text-faint">not known yet</span>
            )}
          </span>
        </div>
      </Card>
      {/* AJ-E1529: AI Juggler's and those started outside it — the same
          rows as the account's Sessions and a project's (AJ-E1533). */}
      <AccountRunning {...props} />
      <Card className="flex flex-col gap-1.5 p-4" data-testid="account-affinity">
        <p className="text-[11px] uppercase tracking-wider text-muted">Usual projects</p>
        {usualProjects.length === 0 ? (
          <p className="text-[11.5px] text-muted">No project affinity yet.</p>
        ) : (
          usualProjects.map((project) => (
            <button
              key={project.id}
              type="button"
              aria-label={project.name}
              onClick={() => props.onOpenProject(project.id)}
              className="flex items-center gap-2 py-1 text-left text-xs"
            >
              <span className="min-w-0 flex-1 truncate font-medium">{project.name}</span>
            </button>
          ))
        )}
      </Card>
    </div>
  );
}


/** What runs on an account now, AI Juggler's and the others: its
 * Sessions rows that are live. */
function AccountRunning(props: AccountPanelProps) {
  const rows = useSessionRows(props.sessions, { accountId: props.account.id }, props.onOpenSession, props.now).filter((row) => row.live);
  return (
    <section className="flex flex-col gap-1.5" aria-labelledby="account-running-title" data-testid="account-running">
      <p id="account-running-title" className="text-[11px] uppercase tracking-wider text-muted">
        Running now
      </p>
      <SessionsList rows={rows} empty="Nothing running on this account now." />
    </section>
  );
}


/** An account's sessions, on every project. */
export function AccountSessions(props: AccountPanelProps) {
  const rows = useSessionRows(props.sessions, { accountId: props.account.id }, props.onOpenSession, props.now);
  return <SessionsList rows={rows} empty="No sessions on this account yet." />;
}


/** AJ-E610: the account card's engine-driven window rings. */
function AccountWindowRings({ accountId }: { accountId: string }) {
  const [display, setDisplay] = useState<QuotaDisplayData | null>(null);
  useEffect(() => {
    let alive = true;
    quotaDisplay(accountId).then((payload) => {
      if (alive) setDisplay(payload);
    });
    return () => {
      alive = false;
    };
  }, [accountId]);
  if (!display) {
    return (
      <p className="text-xs text-muted" data-testid="account-rings-loading">
        loading quota…
      </p>
    );
  }
  if (display.unavailable_reason) {
    return (
      <p className="text-xs text-muted" data-testid="quota-unavailable">
        {display.unavailable_reason}
      </p>
    );
  }
  return (
    <>
      <WindowRing window={display.five_hour} kind="5h" omittedReason={display.five_hour_omitted_reason} />
      <WindowRing window={display.weekly} kind="weekly" omittedReason={display.weekly_omitted_reason} />
    </>
  );
}


/** AJ-E610/E609: the user-entered flat fee behind the dual view's
 * ratio — explicit save only; no plan-tier inference ever writes it. */
function FeeEditor({ account }: { account: AccountInfo }) {
  const [fee, setFee] = useState("");
  const [status, setStatus] = useState("");
  useEffect(() => {
    setFee(localStorage.getItem(`aij.fee.${account.id}`) ?? "");
  }, [account.id]);
  return (
    <span className="flex items-center gap-1.5" data-testid="fee-editor">
      <label className="text-[11px] text-muted" htmlFor={`fee-${account.id}`}>
        Flat fee $/mo
      </label>
      <input
        id={`fee-${account.id}`}
        aria-label="Flat fee, dollars a month"
        value={fee}
        onChange={(event) => setFee(event.target.value)}
        placeholder="e.g. 200"
        inputMode="decimal"
        className="w-16 rounded border border-hairline bg-canvas px-1.5 py-0.5 text-[11px] text-ink outline-none focus:border-accent/50"
      />
      <button
        type="button"
        className="text-[11px] underline focus:outline-none focus:ring-2 focus:ring-accent"
        onClick={() => {
          const parsed = fee.trim() === "" ? null : Number.parseFloat(fee);
          if (parsed !== null && Number.isNaN(parsed)) {
            setStatus("enter a number");
            return;
          }
          localStorage.setItem(`aij.fee.${account.id}`, fee.trim());
          void accountSetFee(account.id, parsed).then(
            () => setStatus("saved"),
            () => setStatus("save failed"),
          );
        }}
      >
        save
      </button>
      {status && (
        <span role="status" className="text-[10px] text-faint">
          {status}
        </span>
      )}
    </span>
  );
}


/// The `Renewal date (optional)` editor (AJ-E411 Req.4): date input +
/// Clear, helper text `Entered by you - AI Juggler does not read billing`,
/// on the account's own page.
function RenewalEditor({ account }: { account: AccountInfo }) {
  const inputRef = useRef<HTMLInputElement>(null);
  const commit = (next: string | null) => {
    void import("../lib/bridge")
      .then((bridge) => bridge.accountSetRenewal(account.id, next))
      .catch(() => {
        /* invalid input is rejected server-side; the next list refresh restores truth */
      });
  };
  return (
    <span className="flex flex-wrap items-center gap-1.5 text-[11px]" data-testid="renewal-editor">
      <label htmlFor={`renewal-${account.id}`} className="text-muted">
        Renewal date (optional)
      </label>
      <input
        ref={inputRef}
        id={`renewal-${account.id}`}
        type="date"
        value={account.renewal_date ?? ""}
        onChange={(event) => commit(event.target.value === "" ? null : event.target.value)}
        className="rounded-lg border border-hairline bg-surface px-1.5 py-0.5 text-[11px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      />
      <Button
        size="sm"
        variant="secondary"
        onClick={() => commit(null)}
        disabled={account.renewal_date === null}
      >
        Clear
      </Button>
      <span className="text-[10px] text-faint">Entered by you - AI Juggler does not read billing</span>
    </span>
  );
}


/**
 * AJ-E508 (D-08): the Account launch-settings area's per-tier
 * opt-ins — TWO separate toggles with the exact labels, both off by
 * default, never bundled (ADR-0010's amendment). Deliberately
 * mouse-only (no shortcut could enable automation).
 */
/** `explain` (default on) adds the D-08 rules under the toggles; a list
 * of Accounts that states them once turns it off. */
export function TierToggles(props: { accountId: string; initial: boolean; explain?: boolean }) {
  const [tiers, setTiers] = useState({ tier1: props.initial, tier2: false });
  const [loaded, setLoaded] = useState(false);
  // The tier table is the source of truth (the account row only
  // mirrors Tier 1 for older readers).
  useEffect(() => {
    let alive = true;
    import("../lib/bridge")
      .then((bridge) => bridge.cleanupTiersGet(props.accountId))
      .then((settings) => {
        if (alive) {
          setTiers({ tier1: settings.tier1_opt_in, tier2: settings.tier2_opt_in });
          setLoaded(true);
        }
      })
      .catch(() => {
        /* keep the both-off default on failure — never a silent opt-in */
      });
    return () => {
      alive = false;
    };
  }, [props.accountId]);
  const set = (tier: "tier1" | "tier2", value: boolean) => {
    setTiers((current) => ({ ...current, [tier]: value }));
    void import("../lib/bridge")
      .then((bridge) =>
        bridge.cleanupTiersSet(
          props.accountId,
          tier === "tier1" ? value : tiers.tier1,
          tier === "tier2" ? value : tiers.tier2,
        ),
      )
      .catch(() => {
        /* the checkboxes re-read on the next mount */
      });
  };
  return (
    <span className="flex flex-col gap-1" data-testid="account-tier-toggles">
      <label data-testid="account-cleanup-toggle" className="flex items-center gap-1.5">
        <input
          type="checkbox"
          aria-label="Automatically tidy left-behind dev servers"
          checked={tiers.tier1}
          disabled={!loaded}
          onChange={(event) => set("tier1", event.target.checked)}
        />
        <span className="text-muted">Automatically tidy left-behind dev servers</span>
      </label>
      <label data-testid="account-tier2-toggle" className="flex items-center gap-1.5">
        <input
          type="checkbox"
          aria-label="Also tidy after a crash (with notification)"
          checked={tiers.tier2}
          disabled={!loaded}
          onChange={(event) => set("tier2", event.target.checked)}
        />
        <span className="text-muted">Also tidy after a crash (with notification)</span>
      </label>
      {props.explain !== false && (
        <span className="text-[10px] leading-relaxed text-muted">
          Both off by default — only verified dev servers and watchers of this account's clean
          exits are tidied; shells, MCP servers and databases are never touched automatically
          and every stop is written to the log. Crash tidies wait 5 minutes and ask first
          with an Undo window.
        </span>
      )}
    </span>
  );
}
