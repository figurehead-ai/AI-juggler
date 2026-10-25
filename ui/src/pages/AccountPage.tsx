/**
 * An account's page (AJ-E1533): its Overview — its limits and usage
 * first (0.1.5), what runs on it now and where it works, then its
 * settings (renewal, fee, cleanup and preview opt-ins) — and its Sessions. Moved from RightInspector (the size budget);
 * the cleanup toggles serve Settings too.
 */
import { AccountCapture } from "./AccountCapture";
import { AccountSettingsSection } from "./account/Settings";
import { useEffect, useState } from "react";
import { Button, Card } from "../components/primitives";
import { Section } from "../components/charts/ChartCard";
import { runwayOf } from "../components/AccountRunway";
import { UsageDashboard } from "../components/usage/Dashboard";
import { RunwayPanel } from "../components/usage/Runway";
import { useWeekByProfile } from "../components/usage/RunwayCards";
import { IconFolder, IconGauge, IconPlay, Segmented } from "../components/usage/controls";
import { useRange } from "../components/usage/useUsage";
import { accountColor } from "../lib/palette";
import { RANGES } from "../lib/usage/range";
import { quotaDisplay, signedIn, type AccountInfo, type InstanceInfo, type ProjectInfo, type QuotaDisplayData } from "../lib/bridge";
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
        <p className="max-w-xs text-meta leading-relaxed text-muted">
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
    <div className="@container flex w-full max-w-content flex-col gap-grid" data-testid="account-overview">
      {/* 0.1.5: limits and usage lead; the settings sit in their own section below. */}
      <AccountLimits account={account} now={props.now} />
      <AccountUsage account={account} accounts={[account]} />
      <div className="grid grid-cols-1 gap-grid @min-[900px]:grid-cols-2">
        {/* AJ-E1529: AI Juggler's and those started outside it — the same
            rows as the account's Sessions and a project's (AJ-E1533). */}
        <AccountRunning {...props} />
        <Section title="Usual projects" icon={<IconFolder />} sub="where this profile works" testId="account-affinity">
          {usualProjects.length === 0 ? (
            <p className="text-meta text-muted">No project affinity yet.</p>
          ) : (
            <ul className="flex flex-col">
              {usualProjects.map((project) => (
                <li key={project.id}>
                  <button
                    type="button"
                    aria-label={project.name}
                    onClick={() => props.onOpenProject(project.id)}
                    className="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-body font-medium text-ink outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"
                  >
                    <span className="min-w-0 flex-1 truncate">{project.name}</span>
                    <span className="text-label text-muted">Open ›</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Section>
      </div>
      <AccountSettingsSection account={account} now={props.now} resetsAtMs={worst?.quota?.resets_at_ms ?? null} />
      {/* AJ-E1530 Req.1: opt-in, shown only where it applies — a quiet
          row at the end, never on the Overview's first screen. */}
      <AccountCapture accountId={account.id} />
    </div>
  );
}


/** What runs on an account now, AI Juggler's and the others: its
 * Sessions rows that are live. */
function AccountRunning(props: AccountPanelProps) {
  const rows = useSessionRows(props.sessions, { accountId: props.account.id }, props.onOpenSession, props.now).filter((row) => row.live);
  return (
    <Section title="Running now" icon={<IconPlay />} sub={rows.length === 0 ? undefined : `${rows.length} open`} testId="account-running">
      <SessionsList rows={rows} empty="Nothing running on this profile now." />
    </Section>
  );
}


/** An account's sessions, on every project. */
export function AccountSessions(props: AccountPanelProps) {
  const rows = useSessionRows(props.sessions, { accountId: props.account.id }, props.onOpenSession, props.now);
  return <SessionsList rows={rows} empty="No sessions on this account yet." />;
}


/** 0.1.5: the profile's limits first: a gauge per window with its reset
 * and the reading's age, the last week's windows, and its tokens. */
function AccountLimits({ account, now }: { account: AccountInfo; now: number }) {
  const [display, setDisplay] = useState<QuotaDisplayData | null>(null);
  useEffect(() => {
    let alive = true;
    quotaDisplay(account.id).then((payload) => {
      if (alive) setDisplay(payload);
    });
    return () => {
      alive = false;
    };
  }, [account.id]);
  const week = useWeekByProfile();
  const color = accountColor(account.id);
  const measuredHere = display !== null && runwayOf(display).left !== null;
  return (
    <Section title="Limits" icon={<IconGauge />} sub={display?.unavailable_reason ? undefined : "what's left in each window, and the last 7 days"} className="@container" testId="account-limits">
      <div data-testid="account-rings">
        {display === null ? (
          <p className="text-meta text-muted" data-testid="account-rings-loading">
            Reading its limits…
          </p>
        ) : display.unavailable_reason ? (
          <p className="text-meta text-muted" data-testid="quota-unavailable">
            {display.unavailable_reason}
          </p>
        ) : (
          <RunwayPanel display={display} limitsProfile={measuredHere ? account.id : null} usage={week.values([account.id])} usageTotal={week.total([account.id])} color={color} name={account.label} now={now} />
        )}
      </div>
    </Section>
  );
}

/** The profile's usage: the dashboard scoped to it, by project. */
function AccountUsage({ account, accounts }: { account: AccountInfo; accounts: AccountInfo[] }) {
  const [range, setRange] = useRange("account");
  return (
    <section aria-label="Usage" className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="text-section text-ink">Usage on {account.label}</h3>
        <Segmented label="Range" options={RANGES.map(({ id, label }) => ({ id, label }))} value={range} onChange={setRange} />
      </div>
      <UsageDashboard kind="profile" scope={{ profile: account.id }} range={range} accounts={accounts} testId="account-usage-dashboard" />
    </section>
  );
}



export { TierToggles } from "./account/Settings";
