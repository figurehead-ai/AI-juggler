/**
 * AJ-E2108 — Continue on another account, on a session's page: the
 * accounts with their profiles beneath, how much each has left, and a
 * one-line check of what the profile lacks. A profile on the session's
 * own account gives no new room (one runway), so it isn't offered. The
 * click opens the switch screen and its checks (AJ-E1542/1543), never a
 * copy of them. A session AI Juggler didn't start is carried in first.
 */
import { useEffect, useMemo, useState, type JSX } from "react";
import { AccountAvatar } from "../../components/Avatar";
import { Button, Card } from "../../components/primitives";
import { runwayOf } from "../../components/AccountRunway";
import { switchTargets, type QuotaDisplayData, type SwitchBackSuggestionData, type SwitchTargetData } from "../../lib/bridge";
import { switchSetupCheck, type SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { accountDisplay, accountName, accountOfProfile, type SubscriptionInfo } from "../../lib/subscriptions";
import type { SessionModel } from "./model";
import { CardTitle } from "./CardTitle";

/** A target profile, as the page shows it. */
export type Target = { profileId: string; label: string; recipeId: string | null; blocked: string | null };

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

export function ContinueOn({
  model,
  accounts,
  labels,
  displays,
  onPick,
  onCarryIn,
  switchBack,
  onSwitchBack,
}: {
  model: SessionModel;
  accounts: SubscriptionInfo[];
  /** The profiles' names, by id. */
  labels: ReadonlyMap<string, string>;
  displays: QuotaDisplayData[];
  /** Opens the switch screen (a session AI Juggler runs). */
  onPick?: (recipeId: string, label: string) => void;
  /** Carries a session another app runs into AI Juggler. */
  onCarryIn?: () => void;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
}): JSX.Element {
  const own = model.sessionId !== null && onPick !== undefined;
  const [rows, setRows] = useState<SwitchTargetData[] | null>(own ? null : []);
  const [checks, setChecks] = useState<Record<string, SetupCheckInfo | null>>({});
  useEffect(() => {
    if (!own || model.sessionId === null) return;
    let alive = true;
    switchTargets(model.sessionId).then(
      (next) => alive && setRows(next),
      () => alive && setRows([]),
    );
    return () => {
      alive = false;
    };
  }, [own, model.sessionId]);
  const targets: Target[] = useMemo(() => {
    // Its own profile is where it is already: never a target.
    if (own) return (rows ?? []).filter((row) => row.account_id !== model.profileId).map((row) => ({ profileId: row.account_id, label: row.account_label, recipeId: row.recipe_id, blocked: row.filtered_reason }));
    // Another app's session: every signed-in profile of its tool, to read.
    return accounts
      .filter((account) => account.tool === model.tool && account.state === "signed_in")
      .flatMap((account) => account.profiles)
      .filter((profile) => profile.id !== model.profileId)
      .map((profile) => ({ profileId: profile.id, label: profile.id, recipeId: null, blocked: null }));
  }, [own, rows, accounts, model.tool, model.profileId]);
  const { others, sameAccount } = byAccount(targets, accounts, model.profileId);
  // The one-line checks, for the first few profiles offered.
  useEffect(() => {
    if (!own || model.sessionId === null) return;
    let alive = true;
    const sessionId = model.sessionId;
    for (const target of others.flatMap((group) => group.targets).slice(0, 4)) {
      if (!target.recipeId || target.recipeId in checks) continue;
      const id = target.recipeId;
      switchSetupCheck(sessionId, id).then(
        (check) => alive && setChecks((now) => ({ ...now, [id]: check })),
        () => alive && setChecks((now) => ({ ...now, [id]: null })),
      );
    }
    return () => {
      alive = false;
    };
    // `checks` is read to skip what's done; asking again on its change would loop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [own, model.sessionId, others.length]);

  const label = (target: Target) => (own ? target.label : (labels.get(target.profileId) ?? target.profileId));
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
                  <span className="truncate font-medium text-ink">{group.account ? accountName(group.account) : label(group.targets[0]!)}</span>
                  <span className={"shrink-0 tabular-nums " + (left !== null && left <= 25 ? "text-warning" : "text-muted")}>{left === null ? "not measured" : `${left}% left`}</span>
                </span>
                {group.targets.map((target) => {
                  const words = target.recipeId && target.recipeId in checks ? setupWords(checks[target.recipeId]) : null;
                  return (
                    <div key={target.profileId + (target.recipeId ?? "")} className="flex flex-col gap-0.5">
                      <Button
                        size="md"
                        variant="secondary"
                        className="justify-start"
                        disabled={!own || target.blocked !== null}
                        title={target.blocked ?? (own ? `Continue on ${label(target)}` : "Continue it in AI Juggler first")}
                        onClick={() => target.recipeId && onPick?.(target.recipeId, label(target))}
                        data-testid="continue-target"
                      >
                        <AccountAvatar accountId={target.profileId} label={label(target)} size={14} />
                        <span className="truncate">{label(target)}</span>
                        {left !== null && <span className="ml-auto tabular-nums text-muted">{left}% left</span>}
                      </Button>
                      {(target.blocked || words) && (
                        <span className="pl-1 text-label text-muted" data-testid="continue-check">
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
          {label(target)} shares this account, so it gives no new room.
        </p>
      ))}
      {!own && (
        <div className="flex flex-col gap-2 border-t border-hairline pt-3">
          <p className="text-meta leading-[1.5] text-muted">
            {model.live
              ? "It's open in its app now. Close it there, continue it in AI Juggler, then move it from there."
              : "To move it, continue it in AI Juggler first; its page then offers these accounts."}
          </p>
          {onCarryIn && (
            <Button size="md" variant="secondary" disabled={model.live} onClick={onCarryIn} data-testid="continue-carry-in" className="w-fit">
              Continue in AI Juggler
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}
