/**
 * D-25, 0.1.5 — Usage: a dashboard of what your sessions used. The range
 * first (24 hours to all), then the figures with their change on the
 * period before, usage over time split by account, profile, tool, model
 * or project, who and what used it, each account's limits and their last
 * week, when you work, and the top projects and sessions, each opening its
 * page. The frame draws at once and the last summary shows from memory.
 * With nothing set up it says what will appear, and how to get there.
 * Every number here is from the backend's summaries, never a sum of rows;
 * Prices and details reads its rows a page at a time (`CostMatrix`).
 */
import { useState, type JSX } from "react";
import { WelcomePointer } from "../components/Welcome";
import { BudgetCaps } from "../components/BudgetCaps";
import { MachineUsageCard } from "../components/MachineUsage";
import { Button, Card } from "../components/primitives";
import { Modal } from "../components/form";
import { lazyNamed } from "../lib/lazy";
import { SPAN, UsageDashboard } from "../components/usage/Dashboard";
import { RunwayCards } from "../components/usage/RunwayCards";
import { Segmented } from "../components/usage/controls";
import { useRange } from "../components/usage/useUsage";
import { RANGES } from "../lib/usage/range";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";

/** Loaded once it's opened: the table pages its rows. */
const CostMatrix = lazyNamed(() => import("../components/CostMatrix"), "CostMatrix");

export function UsageScreen({
  accounts,
  projects = [],
  onAddAccounts,
  onOpenHome,
}: {
  accounts: AccountInfo[];
  sessions: InstanceInfo[];
  /** AJ-E1514: a cap can count a group of projects. */
  projects?: ProjectInfo[];
  onAddAccounts: () => void;
  /** AJ-E1524: back to the welcome while no account is in. */
  onOpenHome?: () => void;
}): JSX.Element {
  const [range, setRange] = useRange("usage");
  const [prices, setPrices] = useState(false);
  const profiles = accounts.length;
  return (
    <div className="mx-auto flex w-full max-w-content flex-col gap-5 pb-8" data-testid="usage-screen">
      <header className="flex flex-wrap items-end gap-x-6 gap-y-3">
        <div className="min-w-[16rem] flex-1">
          <h2 className="text-title text-ink">Usage</h2>
          <p className="mt-1 max-w-[70ch] text-meta text-muted">
            {profiles === 0
              ? "What your sessions use, measured on this Mac."
              : `What your sessions used on ${profiles} ${profiles === 1 ? "profile" : "profiles"}, measured on this Mac from each session's own files.`}
          </p>
        </div>
        {profiles > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" variant="secondary" onClick={() => setPrices(true)} data-testid="usage-prices-open">
              Prices and details
            </Button>
            <Segmented label="Range" options={RANGES.map(({ id, label }) => ({ id, label }))} value={range} onChange={setRange} testId="usage-range" />
          </div>
        )}
      </header>

      {profiles === 0 ? (
        <Card className="flex flex-col items-start gap-3 p-card" data-testid="usage-no-accounts">
          <p className="text-section text-ink">Nothing to measure yet</p>
          <p className="max-w-xl text-body text-muted">
            Once your accounts are in, this shows what every session used over time, by account, tool, model and project, each account's 5-hour and weekly
            windows with their history, and when you work.
          </p>
          <Button size="sm" variant="primary" onClick={onAddAccounts}>
            Find my accounts
          </Button>
          {onOpenHome && <WelcomePointer onOpenHome={onOpenHome} />}
        </Card>
      ) : (
        <UsageDashboard kind="all" scope={{}} range={range} accounts={accounts} runway={<RunwayCards accounts={accounts} className={SPAN.full} />} />
      )}

      <div className="grid grid-cols-1 gap-grid xl:grid-cols-2">
        {profiles > 0 && <BudgetCaps accounts={accounts} projects={projects} />}
        <MachineUsageCard />
      </div>

      {prices && (
        <Modal title="Prices and details" subtitle="What your usage would cost at API list prices, by session and day" table onClose={() => setPrices(false)} testId="usage-prices">
          <CostMatrix />
        </Modal>
      )}

    </div>
  );
}
