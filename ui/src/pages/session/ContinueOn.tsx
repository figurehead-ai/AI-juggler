/**
 * AJ-E2108 — Continue in another app (D-62), on a session's page: the
 * accounts with their profiles beneath, of every tool, how much each has
 * left, the move's badge and a one-line check of what the profile lacks.
 * A profile on the session's own account gives no new room (one runway),
 * so it isn't offered. The click opens the switch screen and its checks
 * (AJ-E1542/1543), never a copy of them. The same for a session AI
 * Juggler runs and one another app runs: that one is read, never touched,
 * and continues on the other account, on Desktop or the CLI, while it
 * stays open in its app.
 * 0.3: Save handover… at its foot, ending with Reveal in Finder.
 * D-103.3: reading an ended session stays quiet. The card is folded, as
 * a terminal session's is: its account's notice one quiet line under its
 * title, and the search for room starts only when the user opens it (or
 * ⌘⇧H, the header's or the bell's Continue), never on opening the page. A
 * live one lists its apps at once, as before.
 */
import { RoomChip } from "../../components/RoomChip";
import { roomMarkFrom } from "../../lib/room-mark";
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { useHandoverFocus } from "../../lib/handover-focus";
import { CliAppMark, ProfileBadge } from "../../components/PlaceMarks";
import { Button, Card } from "../../components/primitives";
import { WhyButton } from "../../components/WhyTip";
import { FidelityBadge, RoomLine, StartThereLink, SuggestedMark } from "../../components/SwitchTargetRows";
import { switchTargets, type SwitchTargetData } from "../../lib/ipc/switch-targets";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import type { SwitchBackSuggestionData } from "../../lib/ipc/switchBack";
import { switchSetupCheck, type SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { NOT_SIGNED_IN, rankTargets, roomless, signInWords, type RoomTarget, type TargetRoom } from "../../lib/room-targets";
import { accountName, accountOfProfile, type SubscriptionInfo } from "../../lib/subscriptions";
import { appWords, sessionAppName, type SessionModel } from "./model";
import { cliAppId, cliAppName } from "../../lib/app-name";
import { CardTitle } from "./CardTitle";
import { toolSpec } from "../../lib/tools";
import { SaveHandover } from "../../components/HandoverView";
import { ContinueDialog } from "../../components/ContinueDialog";
import { AccountRoomLine, ContinueSummary } from "../../components/ContinueSummary";

/** A target App (D-62: a tool on one of its folders), as the page shows
 * it. */
export type Target = {
  profileId: string;
  label: string;
  /** The App's name ("Codex · Personal"), as the rail says it; its
   * label when its tool isn't known. */
  name?: string;
  recipeId: string;
  blocked: string | null;
  /** 0.3.4: held back because its tool's first-run setup isn't done there
   * ("Start a session on it" does it). */
  setupNeeded: boolean;
  /** 0.3.5: its tool's own first run isn't done there; the switch does it. */
  firstRun?: boolean;
  /** 0.4.1: its tool isn't signed in there: no room is shown for it. */
  signedOut?: boolean;
  /** 0.4.1: a signed-in desktop app profile on its account takes it. */
  desktop?: boolean;
  /** What the move carries ("Full story, fresh start · Beta"), when known. */
  badge: { words: string; validated: boolean } | null;
  /** Another tool than the session's. */
  crossTool: boolean;
  /** Its tool (its App's icon), when known. */
  tool?: string | null;
  /** Its account's room, as Home's Runway reads it (`lib/room-targets`). */
  room: TargetRoom | null;
  /** The one with the most room (the one order by room, `lib/room`). */
  suggested: boolean;
};

/** 0.3.5: a target whose tool hasn't done its own first run (a Claude app
 * profile's folder) says the switch does it, after what else it needs. */
export function withFirstRun(words: string | null, firstRun: boolean): string | null {
  if (!firstRun) return words;
  return words ? `${words} · its first-run setup too` : "Its first-run setup is done as it switches";
}

/** The one-line check, in words. */
export function setupWords(check: SetupCheckInfo | null | undefined): string | null {
  if (check === undefined) return null;
  if (check === null) return "Couldn't check its setup";
  if (check.offers.length === 0) return "Ready: same tools set up";
  // What this conversation didn't use doesn't count against the target
  // (`null`: the tool doesn't say, so it counts).
  const offers = check.offers.filter((offer) => offer.used !== false);
  if (offers.length === 0) return "Ready: has what this conversation used";
  const kinds = new Set(offers.map((offer) => offer.kind));
  const what =
    kinds.size === 1 && kinds.has("mcp-server")
      ? `${offers.length} MCP ${offers.length === 1 ? "server" : "servers"}`
      : `${offers.length} ${offers.length === 1 ? "thing" : "things"}`;
  const yours = offers.some((offer) => offer.verdict !== "copy");
  return `Needs ${what}, ${yours ? "some set up by you" : "set up as it switches"}`;
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
 * is where it is already. Ranked rows (`rankTargets`) keep their order,
 * room and suggestion. */
export function targetsOf(rows: Array<SwitchTargetData | RoomTarget>, ownProfile: string): Target[] {
  return rows
    .filter((row) => row.account_id !== ownProfile)
    .map((row) => ({
      room: "room" in row ? row.room : null,
      suggested: "room" in row ? row.suggested : false,
      profileId: row.account_id,
      label: row.account_label,
      name: row.provider ? cliAppName(row.provider, row.account_label) : row.account_label,
      recipeId: row.recipe_id,
      blocked: row.filtered_reason,
      setupNeeded: row.held_back === "set_up",
      firstRun: row.first_run === true,
      signedOut: row.signed_out === true,
      desktop: row.desktop === true,
      badge: row.badge ? { words: row.badge, validated: row.validated } : null,
      crossTool: row.cross_provider,
      tool: row.provider ?? null,
    }));
}

export function ContinueOn({
  model,
  accounts,
  displays,
  onPick,
  onCarryIn,
  carryInWhy = null,
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
  /** rc21: why it can't continue here (its tool won't open it by its id). */
  carryInWhy?: string | null;
  switchBack?: SwitchBackSuggestionData | null;
  onSwitchBack?: () => void;
}): JSX.Element {
  const source = model.switchSource;
  const outside = model.sessionId === null;
  const [rows, setRows] = useState<SwitchTargetData[] | null>(null);
  const [checks, setChecks] = useState<Record<string, SetupCheckInfo | null>>({});
  // ⌘⇧H, the row menu, Switch account…, a banner's or Home's "Continue
  // on another account…" (F8, 0.3.2): the list opens in a dialog over the
  // page, its Suggested account holding the keyboard.
  const [dialog, setDialog] = useState(false);
  // rc30: the targets are asked again whenever what they depend on can
  // have changed: the session learned its conversation, a turn started or
  // ended, or the dialog opened (a live session's first turn once left
  // every target greyed "hasn't started a conversation yet" until a
  // relaunch). The rows on screen stay while asking again; only another
  // session starts from "Looking for room…".
  const turn = `${model.conversationId ?? ""}|${model.state.working ? "working" : "still"}`;
  const asked = useRef<string | null>(null);
  // D-103.3: an ended one searches only once the user opens it.
  const quiet = !model.live;
  const [chosen, setChosen] = useState(false);
  const searching = !quiet || chosen || dialog;
  useEffect(() => {
    let alive = true;
    if (searching) {
      if (asked.current !== source) setRows(null);
      asked.current = source;
      switchTargets(source).then(
        (next) => alive && setRows(next),
        () => alive && setRows((now) => now ?? []),
      );
    }
    return () => {
      alive = false;
    };
  }, [source, turn, dialog, searching]);
  // The one order by room, with Home's "Best for new work" as Suggested
  // (the 0.2 walk, F1).
  const targets = useMemo(
    () => targetsOf(rankTargets(rows ?? [], accounts, displays, model.profileId), model.profileId),
    [rows, accounts, displays, model.profileId],
  );
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

  useHandoverFocus(() => setDialog(true), source);
  const pick = onPick && ((recipeId: string, label: string) => (setDialog(false), onPick(recipeId, label)));
  // Where the original stays, for a session another app runs.
  const app = appWords(model.places[0], model.tool);
  const list = (
    <>
    {switchBack && onSwitchBack && (
      <button type="button" onClick={onSwitchBack} data-testid="continue-switch-back" className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-2 text-left text-meta text-accent-hi outline-none hover:border-accent/60 focus-visible:ring-1 focus-visible:ring-accent/60">
        <span className="block">{switchBack.copy}</span>
        <span className="text-muted">Go back to where it came from ▸</span>
      </button>
    )}
    {rows === null ? (
      <p className="text-meta text-ink-2" role="status">Looking for room…</p>
    ) : others.length === 0 ? (
      <p className="text-meta text-ink-2" data-testid="continue-none">No other app to continue in.</p>
    ) : (
      <ul className="flex flex-col gap-3" aria-label="Apps to continue in">
        {others.map((group) => {
          const name = group.account ? accountName(group.account) : group.targets[0]!.label;
          // The account's room, from a target signed in there; "Not
          // signed in" only when none is.
          const room = (group.targets.find((target) => !roomless({ signed_out: target.signedOut, desktop: target.desktop })) ?? group.targets[0]!).room;
          return (
            <li key={group.account?.key ?? group.targets[0]!.profileId} className="flex flex-col gap-1.5" data-testid="continue-account" data-room={room?.kind}>
              <span className="flex items-baseline justify-between gap-2 text-meta">
                <span className="truncate font-medium text-ink">{name}</span>
                <RoomFigure room={room} />
              </span>
              {room && room.kind !== "blocked" && <RoomLine name={name} room={room} testId="continue-room" />}
              {group.targets.map((target) => {
                // Its own sign-in, when the account's line doesn't say it.
                const sign = signInWords({ signed_out: target.signedOut, desktop: target.desktop });
                const own = sign && !(sign === NOT_SIGNED_IN && room?.kind === "blocked") ? sign : null;
                const words = own ?? withFirstRun(target.recipeId in checks ? setupWords(checks[target.recipeId]) : null, target.firstRun === true);
                return (
                  <div key={target.profileId + target.recipeId} className="flex flex-col gap-0.5">
                    <Button
                      size="md"
                      variant="secondary"
                      className="justify-start"
                      disabled={!pick || target.blocked !== null}
                      title={target.blocked ?? `Continue in ${target.name ?? target.label}`}
                      onClick={() => pick?.(target.recipeId, target.label)}
                      data-testid="continue-target"
                      data-cross-tool={target.crossTool ? "true" : undefined}
                      data-suggested={target.suggested ? "true" : undefined}
                    >
                      {target.tool ? <CliAppMark tool={target.tool} folderId={target.profileId} folder={target.label} size={16} ring="var(--color-surface-2)" /> : <ProfileBadge app={cliAppId(target.profileId)} name={target.label} />}
                      <span className="truncate">{target.name ?? target.label}</span>
                      {target.suggested && <SuggestedMark className="ml-auto" />}
                    </Button>
                    {(target.blocked || words || (target.crossTool && target.badge)) && (
                      <span className="flex flex-wrap items-center gap-x-2 gap-y-1 pl-1 text-label text-muted" data-testid="continue-check">
                        {target.crossTool && target.badge && <FidelityBadge badge={target.badge.words} validated={target.badge.validated} />}
                        {target.blocked ?? words}
                      </span>
                    )}
                    {target.setupNeeded && <StartThereLink accountId={target.profileId} label={target.label} onStart={() => setDialog(false)} />}
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
        {target.name ?? target.label} shares this account, so it gives no new room.
      </p>
    ))}
    {outside && (
      <div className="flex flex-col gap-2 border-t border-hairline pt-3" data-testid="continue-outside">
        <p className="text-meta leading-[1.5] text-ink-2">
          {/* 0.3.5: where it goes on, said plainly: Desktop or CLI, chosen before it switches. */}
          {`Picking one continues it on that account, in the ${toolSpec(model.tool).desktop_app ?? "desktop"} app (Desktop) or in a terminal (CLI): you choose before it switches. ${model.live ? `The original stays open in ${app ?? "its app"}, untouched.` : "The original stays as it is."}`}
        </p>
        {model.live ? (
          <p className="text-label text-muted" data-testid="continue-same-profile">
            To carry on with it in {sessionAppName(model)} itself, close it in {app ?? "its app"} first: one conversation can't have two writers.
          </p>
        ) : (
          onCarryIn && (
            <WhyButton size="md" variant="secondary" why={carryInWhy} onClick={onCarryIn} data-testid="continue-carry-in" className="w-fit">
              Continue in AI Juggler
            </WhyButton>
          )
        )}
      </div>
    )}
    </>
  );
  const shown = dialog ? (
    <p className="text-meta text-ink-2" data-testid="continue-in-dialog">
      Choosing in the dialog…
    </p>
  ) : (
    list
  );
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="continue-on">
      {quiet ? (
        <details open={chosen} onToggle={(event) => setChosen(event.currentTarget.open)} className="group" data-testid="continue-fold">
          <ContinueSummary />
          {chosen && <div className="mt-3 flex flex-col gap-3">{shown}</div>}
        </details>
      ) : (
        <>
          <CardTitle>Continue in another app</CardTitle>
          {shown}
        </>
      )}
      {quiet && !chosen && <AccountRoomLine accountId={model.profileId} />}
      {/* 0.3: save its handover, beside the switch actions (a session
          another app runs is only read for it). */}
      <SaveHandover sessionId={source} />
      {dialog && (
        <ContinueDialog title="Continue in another app" detail={`Now in ${sessionAppName(model)}`} onClose={() => setDialog(false)}>
          {list}
        </ContinueDialog>
      )}
    </Card>
  );
}

/** An account's room, short, beside its name ("75% left", "Out of room",
 * "Using credits"). */
function RoomFigure({ room }: { room: TargetRoom | null }): JSX.Element {
  if (room?.kind === "credits") return <span className="shrink-0 text-ink">Using credits</span>;
  if (room?.kind === "blocked") return <span className="shrink-0 text-muted" data-testid="continue-signed-out">{NOT_SIGNED_IN}</span>;
  if (room === null || room.left === null) return <span className="shrink-0 text-muted">not measured</span>;
  // 0.4.1: out or low, the one mark every screen shows (never suggested).
  if (room.kind === "out" || room.kind === "low") return <RoomChip mark={roomMarkFrom(room.left, room.resetsAtMs)} compact testId="switch-target-room" />;
  return <span className="shrink-0 tabular-nums text-muted">{`${room.left}% left`}</span>;
}
