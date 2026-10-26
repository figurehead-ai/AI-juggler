/**
 * A profile's settings (moved from its Overview's first screen in 0.1.5:
 * limits and usage lead there): plan and renewal, the flat fee, the two
 * tidy-up opt-ins and the live preview, grouped like the Settings panes.
 */
import { useEffect, useRef, useState } from "react";
import { Section } from "../../components/charts/ChartCard";
import { IconSettings } from "../../components/usage/controls";
import { Button, Chip, formatCountdown } from "../../components/primitives";
import { accountSetFee, type AccountInfo } from "../../lib/bridge";
import { renewalLabel } from "../../lib/renewal";

export function AccountSettingsSection({ account, now, resetsAtMs }: { account: AccountInfo; now: number; resetsAtMs: number | null }) {
  const label = renewalLabel(account.renewal_date, now);
  return (
    <Section title="Settings" icon={<IconSettings />} sub="for this profile" testId="account-settings">
        <dl className="grid grid-cols-1 gap-x-8 gap-y-4 text-body @min-[700px]:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <dt className="text-meta font-semibold text-ink">Plan and renewal</dt>
            <dd className="flex flex-col gap-2 text-meta">
              <span className="flex flex-wrap items-center gap-2">
                <Chip tone="success">signed in</Chip>
                <span className="text-muted">
                  plan: <span className="text-ink">{account.plan ?? "unknown"}</span>
                </span>
              </span>
              <span data-testid="account-renewal" className="flex flex-wrap items-center gap-1.5">
                {account.renewal_date === null ? (
                  <span className="text-muted">Expiry: unknown</span>
                ) : label?.tier === "warning" ? (
                  <Chip tone="warning">⚠ {label.text}</Chip>
                ) : label?.tier === "danger" ? (
                  <Chip tone="danger">⛔ {label.text}</Chip>
                ) : (
                  <span className="text-muted">{label?.text ?? "Expiry: unknown"}</span>
                )}
              </span>
              <RenewalEditor account={account} />
              <FeeEditor account={account} />
            </dd>
          </div>
          <div className="flex flex-col gap-1.5">
            <dt className="text-meta font-semibold text-ink">Tidying up</dt>
            <dd className="text-meta">
              {/*
               * AJ-E508 (D-08): the Account launch-settings area's TWO
               * separate tier toggles — never one bundled "auto-cleanup"
               * switch. Exact labels; both off by default.
               */}
              <TierToggles accountId={account.id} initial={account.cleanup_opt_in} />
            </dd>
          </div>
          <div className="flex flex-col gap-1.5">
            <dt className="text-meta font-semibold text-ink">Desktop sessions</dt>
            <dd className="text-meta">
              <label data-testid="account-preview-toggle" className="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  aria-label="Live preview of desktop sessions (opt-in)"
                  checked={account.preview_opt_in}
                  onChange={(event) => {
                    const optIn = event.target.checked;
                    void import("../../lib/bridge").then((bridge) =>
                      bridge.accountSetPreviewOptIn(account.id, optIn).catch(() => {
                        /* re-reads from props on the next list refresh */
                      }),
                    );
                  }}
                />
                <span className="text-muted">Live preview of desktop sessions (opt-in)</span>
              </label>
            </dd>
          </div>
          <div className="flex flex-col gap-1.5">
            <dt className="text-meta font-semibold text-ink">Its newest session</dt>
            <dd data-testid="account-reset" className="text-meta text-muted">
              Its limit resets{" "}
              {resetsAtMs ? (
                <span className="text-ink">{formatCountdown(resetsAtMs, now)}</span>
              ) : (
                <span className="text-muted">not known yet</span>
              )}
            </dd>
          </div>
        </dl>
      </Section>
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
      <label className="text-meta text-muted" htmlFor={`fee-${account.id}`}>
        Flat fee $/mo
      </label>
      <input
        id={`fee-${account.id}`}
        aria-label="Flat fee, dollars a month"
        value={fee}
        onChange={(event) => setFee(event.target.value)}
        placeholder="e.g. 200"
        inputMode="decimal"
        className="w-16 rounded border border-hairline bg-canvas px-1.5 py-0.5 text-meta text-ink outline-none focus:border-accent/50"
      />
      <button
        type="button"
        className="text-meta underline focus:outline-none focus:ring-2 focus:ring-accent"
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
        <span role="status" className="text-label text-muted">
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
    void import("../../lib/bridge")
      .then((bridge) => bridge.accountSetRenewal(account.id, next))
      .catch(() => {
        /* invalid input is rejected server-side; the next list refresh restores truth */
      });
  };
  return (
    <span className="flex flex-wrap items-center gap-1.5 text-meta" data-testid="renewal-editor">
      <label htmlFor={`renewal-${account.id}`} className="text-muted">
        Renewal date (optional)
      </label>
      <input
        ref={inputRef}
        id={`renewal-${account.id}`}
        type="date"
        value={account.renewal_date ?? ""}
        onChange={(event) => commit(event.target.value === "" ? null : event.target.value)}
        className="rounded-lg border border-hairline bg-surface px-1.5 py-0.5 text-meta text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      />
      <Button
        size="sm"
        variant="secondary"
        onClick={() => commit(null)}
        disabled={account.renewal_date === null}
      >
        Clear
      </Button>
      <span className="text-label text-muted">Entered by you - AI Juggler does not read billing</span>
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
    import("../../lib/bridge")
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
    void import("../../lib/bridge")
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
        <span className="text-label leading-relaxed text-muted">
          Both off by default — only verified dev servers and watchers of this account's clean
          exits are tidied; shells, MCP servers and databases are never touched automatically
          and every stop is written to the log. Crash tidies wait 5 minutes and ask first
          with an Undo window.
        </span>
      )}
    </span>
  );
}
