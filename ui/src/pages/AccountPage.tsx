/**
 * A config folder's limits and usage (0.1.5), as the App page (a CLI App
 * is a tool on its folder, AJ-E3204) and the Account page (AJ-E3205)
 * show them. D-62 retired the folder's own page: its sessions, projects
 * and settings are its App's now.
 */
import { useEffect, useState } from "react";
import { Section } from "../components/charts/ChartCard";
import { runwayOf } from "../components/AccountRunway";
import { UsageDashboard } from "../components/usage/Dashboard";
import { nothingRead, RunwayPanel } from "../components/usage/Runway";
import { useWeekByProfile } from "../components/usage/RunwayCards";
import { IconGauge, Segmented } from "../components/usage/controls";
import { useRange } from "../components/usage/useUsage";
import { accountColor } from "../lib/marks";
import { RANGES } from "../lib/usage/range";
import { quotaDisplay, type QuotaDisplayData } from "../lib/ipc/quota";
import { MeasureLimits } from "../components/MeasureLimits";
import type { AccountInfo } from "../types/generated/AccountInfo";

/** 0.1.5: the profile's limits first: a gauge per window with its reset
 * and the reading's age, the last week's windows, and its tokens. */
/** `measure`: "Measure its limits" while nothing is read (the Account
 * page), measuring in place (0.4.4, `MeasureLimits`). */
export function AccountLimits({ account, now, tone = "neutral", measure = false }: { account: AccountInfo; now: number; tone?: "neutral" | "colour"; measure?: boolean }) {
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
  // Limits are the Account's: its own colour on its own page; beside an
  // App's mark (the App's page) the app's accent, as colour is the Apps'.
  const color = tone === "colour" ? accountColor({ folder: account.id }) : "var(--color-accent)";
  const measuredHere = display !== null && runwayOf(display).left !== null;
  return (
    <Section title="Limits" icon={<IconGauge />} sub={display?.unavailable_reason ? undefined : "what's left in each window, and the last 7 days"} className="@container" testId="account-limits">
      <div data-testid="account-rings">
        {display === null ? (
          <p className="text-meta text-ink-2" data-testid="account-rings-loading">
            Reading its limits…
          </p>
        ) : display.unavailable_reason ? (
          <p className="text-meta text-ink-2" data-testid="quota-unavailable">
            {display.unavailable_reason}
          </p>
        ) : (
          <RunwayPanel display={display} limitsProfile={measuredHere ? account.id : null} usage={week.values([account.id])} usageTotal={week.total([account.id])} color={color} name={account.label} now={now} />
        )}
        {display !== null && measure && nothingRead(display) && (
          <p className="mt-2 text-meta" data-testid="account-limits-measure">
            <MeasureLimits folders={[account.id]} tool={account.provider} name={account.label} onMeasured={() => void quotaDisplay(account.id).then(setDisplay)} />
          </p>
        )}
      </div>
    </Section>
  );
}

/** The profile's usage: the dashboard scoped to it, by project. */
export function AccountUsage({ account, accounts }: { account: AccountInfo; accounts: AccountInfo[] }) {
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
