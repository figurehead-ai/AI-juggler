/**
 * AJ-E2108 — Continue on another account, on a session's page: the
 * accounts with their profiles beneath, of every tool, how much each has
 * left, the move's badge and a one-line check of what the profile lacks.
 * A profile on the session's own account gives no new room (one runway),
 * so it isn't offered. The click opens the switch screen and its checks
 * (AJ-E1542/1543), never a copy of them. The same for a session AI
 * Juggler runs and one another app runs: that one is read, never touched,
 * and continues as a new session here while it stays open in its app.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { AccountAvatar } from "../../components/Avatar";
import { Button, Card } from "../../components/primitives";
import { runwayOf } from "../../components/AccountRunway";
import { FidelityBadge } from "../../components/SwitchTargetRows";
import { switchTargets, type QuotaDisplayData, type SwitchBackSuggestionData, type SwitchTargetData } from "../../lib/bridge";
import { switchSetupCheck, type SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { accountDisplay, accountName, accountOfProfile, type SubscriptionInfo } from "../../lib/subscriptions";
import { appWords, type SessionModel } from "./model";
import { CardTitle } from "./CardTitle";

/** A target profile, as the page shows it. */
export type Target = {
  profileId: string;
  label: string;
  recipeId: string;
  blocked: string | null;
  /** What the move carries ("Full story, fresh start · Beta"), when known. */
  badge: { words: string; validated: boolean } | null;
  /** Another tool than the session's. */
  crossTool: boolean;
};

/** The one-line check, in words. */
export function setupWords(check: SetupCheckInfo | null | undefined): string | null {
  if (check === undefined) return null;
  if (check === null) return "Couldn't check its setup";
  if (check.offers.length === 0) return "Ready: same tools set up";
  const kinds = new Set(check.offers.map((offer) => offer.kind));
  const what =
    kinds.size === 1 && kinds.has("mcp-server")
      ? `${check.offers.length} MCP ${check.offers.length === 1 ? "server" : "servers"}`
      : `${check.offers.length} ${check.offers.length === 1 ? "thing" : "things"}`;
  const yours = check.offers.some((offer) => offer.verdict !== "copy");
  return `Needs ${what}, ${yours ? "some set up by you" : "set up as it moves"}`;
}

/** Targets grouped by account, the session's own account apart. */
export function byAccount(targets: Target[], accounts: SubscriptionInfo[], sourceProfile: string) {
  const source = accountOfProfile(accounts, sourceProfile);
  const groups = new Map<string, { account: SubscriptionInfo | undefined; targets: Target[] }>();
  for (const target of targets) {
    const account = accountOfProfile(accounts, target.profileId);
    const key = account?.key ?? `profile:${target.profileId}`;
    const group = groups.get(key) ?? { account, targets: [] };
    group.targets.push(target);
    groups.set(key, group);
  }
  const same = source?.state === "signed_in" ? (groups.get(source.key)?.targets ?? []) : [];
  if (source) groups.delete(source.key);
  return { others: [...groups.values()], sameAccount: same };
}

/** The backend's rows as the page's targets: never its own profile, which
 * is where it is already. */
export function targetsOf(rows: SwitchTargetData[], ownProfile: string): Target[] {
  return rows
    .filter((row) => row.account_id !== ownProfile)
    .map((row) => ({
      profileId: row.account_id,
      label: row.account_label,
      recipeId: row.recipe_id,
      blocked: row.filtered_reason,
      badge: row.badge ? { words: row.badge, validated: row.validated } : null,
      crossTool: row.cross_provider,
    }));
}

export function ContinueOn({
  model,
  accounts,
  displays,
  onPick,
  onCarryIn,
  switchBack,
  onSwitchBack,
}: {
  model: SessionModel;
  accounts: SubscriptionInfo[];
  displays: QuotaDisplayData[];
  /** Opens the switch screen from `model.switchSource`: AI Juggler's
   * session, or the session another app runs, alike. */
  onPick?: (recipeId: string, label: string) => void;
  /** Continues a session another app runs here, on its own profile. */
  onCarryIn?: () => void;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
}): JSX.Element {
  const source = model.switchSource;
  const outside = model.sessionId === null;
  const [rows, setRows] = useState<SwitchTargetData[] | null>(null);
  const [checks, setChecks] = useState<Record<string, SetupCheckInfo | null>>({});
  useEffect(() => {
    let alive = true;
    setRows(null);
    switchTargets(source).then(
      (next) => alive && setRows(next),
      () => alive && setRows([]),
    );
    return () => {
      alive = false;
    };
  }, [source]);
  const targets = useMemo(() => targetsOf(rows ?? [], model.profileId), [rows, model.profileId]);
  const { others, sameAccount } = byAccount(targets, accounts, model.profileId);
  // The one-line checks, for the first few profiles offered.
  useEffect(() => {
    let alive = true;
    for (const target of others.flatMap((group) => group.targets).slice(0, 4)) {
      if (target.recipeId in checks) continue;
      const id = target.recipeId;
      switchSetupCheck(source, id).then(
        (check) => alive && setChecks((now) => ({ ...now, [id]: check })),
        () => alive && setChecks((now) => ({ ...now, [id]: null })),
      );
    }
    return () => {
      alive = false;
    };
    // `checks` is read to skip what's done; asking again on its change would loop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source, others.length]);

  // Where the original stays, for a session another app runs.
  const app = appWords(model.places[0], model.tool);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="continue-on">
      <CardTitle>Continue on another account</CardTitle>
      {switchBack && onSwitchBack && (
        <button type="button" onClick={onSwitchBack} data-testid="continue-switch-back" className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-2 text-left text-meta text-accent-hi outline-none hover:border-accent/60 focus-visible:ring-1 focus-visible:ring-accent/60">
          <span className="block">{switchBack.copy}</span>
          <span className="text-muted">Go back to where it came from ▸</span>
        </button>
      )}
      {rows === null ? (
        <p className="text-meta text-muted" role="status">Looking for room…</p>
      ) : others.length === 0 ? (
        <p className="text-meta text-muted" data-testid="continue-none">No other account to continue on.</p>
      ) : (
        <ul className="flex flex-col gap-3" aria-label="Accounts to continue on">
          {others.map((group) => {
            const display = group.account ? accountDisplay(group.account, displays) : undefined;
            const left = runwayOf(display).left;
            return (
              <li key={group.account?.key ?? group.targets[0]!.profileId} className="flex flex-col gap-1.5" data-testid="continue-account">
                <span className="flex items-baseline justify-between gap-2 text-meta">
                  <span className="truncate font-medium text-ink">{group.account ? accountName(group.account) : group.targets[0]!.label}</span>
                  <span className={"shrink-0 tabular-nums " + (left !== null && left <= 25 ? "text-warning" : "text-muted")}>{left === null ? "not measured" : `${left}% left`}</span>
                </span>
                {group.targets.map((target) => {
                  const words = target.recipeId in checks ? setupWords(checks[target.recipeId]) : null;
                  return (
                    <div key={target.profileId + target.recipeId} className="flex flex-col gap-0.5">
                      <Button
                        size="md"
                        variant="secondary"
                        className="justify-start"
                        disabled={!onPick || target.blocked !== null}
                        title={target.blocked ?? `Continue on ${target.label}`}
                        onClick={() => onPick?.(target.recipeId, target.label)}
                        data-testid="continue-target"
                        data-cross-tool={target.crossTool ? "true" : undefined}
                      >
                        <AccountAvatar accountId={target.profileId} label={target.label} size={14} />
                        <span className="truncate">{target.label}</span>
                        {left !== null && <span className="ml-auto tabular-nums text-muted">{left}% left</span>}
                      </Button>
                      {(target.blocked || words || (target.crossTool && target.badge)) && (
                        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 pl-1 text-label text-muted" data-testid="continue-check">
                          {target.crossTool && target.badge && <FidelityBadge badge={target.badge.words} validated={target.badge.validated} />}
                          {target.blocked ?? words}
                        </span>
                      )}
                    </div>
                  );
                })}
              </li>
            );
          })}
        </ul>
      )}
      {sameAccount.map((target) => (
        <p key={target.profileId} className="text-label text-muted" data-testid="continue-no-room">
          {target.label} shares this account, so it gives no new room.
        </p>
      ))}
      {outside && (
        <div className="flex flex-col gap-2 border-t border-hairline pt-3" data-testid="continue-outside">
          <p className="text-meta leading-[1.5] text-muted">
            {model.live
              ? `Picking one continues it as a new session here in AI Juggler. The original stays open in ${app ?? "its app"}, untouched.`
              : "Picking one continues it as a new session here in AI Juggler. The original stays as it is."}
          </p>
          {model.live ? (
            <p className="text-label text-muted" data-testid="continue-same-profile">
              To carry on with it on {model.profileLabel} itself, close it in {app ?? "its app"} first: one conversation can't have two writers.
            </p>
          ) : (
            onCarryIn && (
              <Button size="md" variant="secondary" onClick={onCarryIn} data-testid="continue-carry-in" className="w-fit">
                Continue in AI Juggler
              </Button>
            )
          )}
        </div>
      )}
    </Card>
  );
}
