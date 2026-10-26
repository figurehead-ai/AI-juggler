/**
 * AJ-E2405 — automatic cleanup, shown where its work is: each account's
 * two rules (off by default; ADR-0010's tiers, unchanged), a Preview
 * before turning one on that says what it would tidy now (the sweep's own
 * rule, nothing stopped), and Pause for every account.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { cleanupPauseSet, cleanupTiersSet } from "../../lib/bridge";
import { processesDryRun, processesPolicy, type CleanupPolicyInfo, type RulePreviewInfo } from "../../lib/ipc/processes";

function Switch({ on, label, onChange, testId }: { on: boolean; label: string; onChange: (on: boolean) => void; testId: string }): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      data-testid={testId}
      className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors ${on ? "border-accent bg-accent" : "border-hairline bg-surface-3"}`}
    >
      <span className={`inline-block size-3.5 rounded-full bg-white shadow transition-transform ${on ? "translate-x-[18px]" : "translate-x-[3px]"}`} />
    </button>
  );
}

function Preview({ preview }: { preview: RulePreviewInfo }): JSX.Element {
  const later = preview.items.filter((item) => item.when !== "now");
  return (
    <div className="mt-2 flex flex-col gap-1 rounded-lg border border-hairline bg-surface-2 p-2.5" data-testid={`rule-preview-${preview.account_id}-${preview.tier}`} role="status">
      <p className="text-meta font-medium text-ink">{preview.words}</p>
      {later.length > 0 && (
        <ul className="flex flex-col gap-0.5">
          {later.map((item) => (
            <li key={item.pid} className="text-label text-muted">
              {item.name}: {item.when === "later" ? `later (${item.reason ?? "after its wait"})` : (item.reason ?? "never")}
            </li>
          ))}
        </ul>
      )}
      <p className="text-label text-muted">A preview: nothing was stopped.</p>
    </div>
  );
}

export function RulesPanel({ policy: seeded }: { policy?: CleanupPolicyInfo | null }): JSX.Element {
  const [policy, setPolicy] = useState<CleanupPolicyInfo | null>(seeded ?? null);
  const [preview, setPreview] = useState<RulePreviewInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (seeded) return;
    let alive = true;
    void processesPolicy().then(
      (next) => alive && setPolicy(next),
      (failure: unknown) => alive && setError(String(failure)),
    );
    return () => {
      alive = false;
    };
  }, [seeded]);

  const setTier = async (accountId: string, tier: 1 | 2, on: boolean) => {
    const account = policy?.accounts.find((a) => a.account_id === accountId);
    if (!account || !policy) return;
    const tier1 = tier === 1 ? on : account.tier1;
    const tier2 = tier === 2 ? on : account.tier2;
    await cleanupTiersSet(accountId, tier1, tier2);
    setPolicy({ ...policy, accounts: policy.accounts.map((a) => (a.account_id === accountId ? { ...a, tier1, tier2 } : a)) });
  };
  const pause = async () => {
    if (!policy) return;
    await cleanupPauseSet(!policy.paused);
    setPolicy({ ...policy, paused: !policy.paused });
  };

  return (
    <section aria-labelledby="rules-title" className="flex flex-col gap-3 rounded-card border border-hairline bg-surface p-4" data-testid="cleanup-rules">
      <div className="flex items-baseline justify-between gap-3">
        <h3 id="rules-title" className="text-section text-ink">
          Automatic cleanup
        </h3>
        {policy && (
          <Button size="sm" variant="ghost" onClick={() => void pause()} data-testid="rules-pause">
            {policy.paused ? "Resume" : "Pause all"}
          </Button>
        )}
      </div>
      <p className="text-meta text-muted">
        Off unless you turn it on, per account. It only ever tidies dev servers and watchers AI Juggler started, after their session ended, and never another app's.
      </p>
      {policy?.paused && <p className="text-meta font-medium text-warning">Paused for every account.</p>}
      {policy?.suspended_until_ms && <p className="text-meta font-medium text-warning">Held after a restart until you confirm it in Settings › Advanced.</p>}
      {error && <p className="text-meta text-danger">Couldn't read the rules: {error}</p>}
      <ul className="flex flex-col divide-y divide-hairline">
        {(policy?.accounts ?? []).map((account) => (
          <li key={account.account_id} className="flex flex-col gap-2 py-2.5" data-testid={`rules-account-${account.account_id}`}>
            <span className="text-body font-medium text-ink">{account.label}</span>
            {([1, 2] as const).map((tier) => {
              const on = tier === 1 ? account.tier1 : account.tier2;
              const label = tier === 1 ? "Tidy left-behind dev servers" : "Also after a crash";
              const shown = preview?.account_id === account.account_id && preview.tier === tier;
              return (
                <div key={tier} className="flex flex-col">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-meta text-ink">{label}</span>
                    <span className="flex items-center gap-2">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => void (shown ? setPreview(null) : processesDryRun(account.account_id, tier).then(setPreview))}
                        aria-expanded={shown}
                        data-testid={`rules-preview-${account.account_id}-${tier}`}
                      >
                        Preview
                      </Button>
                      <Switch on={on} label={`${label} for ${account.label}`} onChange={(next) => void setTier(account.account_id, tier, next)} testId={`rules-switch-${account.account_id}-${tier}`} />
                    </span>
                  </div>
                  {shown && <Preview preview={preview} />}
                </div>
              );
            })}
          </li>
        ))}
      </ul>
    </section>
  );
}
