/**
 * AJ-E3816 — no watches yet (D-126: an inviting empty state): what a
 * watch is in one line, New watch as the one primary, Bot as the second
 * way in when Bot has a route, a line drawing (three sessions, one watch,
 * a bell rung once), and up to three starters built from what is open now
 * and what this Mac can report. A starter opens the form filled in;
 * nothing starts until Watch is pressed.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../primitives";
import { watchCatalogue, watchDraftWords, type EchoParts, type WatchDraftInfo } from "../../lib/ipc/watch-view";
import { EchoHead } from "./WatchParts";
import type { TargetOption } from "./watches-view";
import { EMPTY, PAGE } from "./watches-words";
import { IconAdd, IconBotHead, IconCheckCircle, IconChevronR, IconGauge, IconMessage, type WatchIcon } from "./watch-icons";

/** A starter: a draft, what it is, its badge. */
export interface Starter {
  id: string;
  draft: WatchDraftInfo;
  icon: WatchIcon;
  badge: string;
  line: string;
}

/** The starters the proposal names, offered only where this Mac can
 * report them: the most recently active open session, for Finishes;
 * any session, asks you and waits 10 minutes; the account in use, runs
 * below 20 %. */
export async function startersFor(openNow: readonly TargetOption[], accountInUse: TargetOption | null): Promise<Starter[]> {
  const out: Starter[] = [];
  const offers = async (option: TargetOption, code: string) => (await watchCatalogue(option.target).catch(() => [])).some((entry) => entry.code === code);
  const session = openNow[0];
  if (session && (await offers(session, "finishes"))) {
    out.push({ id: "finishes", icon: IconCheckCircle, badge: EMPTY.once, line: `${session.line.replace(/ · [^·]+$/, "")} · ${EMPTY.workingNow}`, draft: { targets: [session.target], matching: "each", conditions: [{ code: "finishes" }], lifetime: { kind: "once" }, evenInQuietHours: false } });
  }
  const any: TargetOption = { target: { kind: "any" }, key: "any", kind: "any", name: "Any session", line: "", tool: null, open: false, at: 0 };
  if (await offers(any, "asks_you")) {
    out.push({ id: "asks", icon: IconMessage, badge: EMPTY.always, line: EMPTY.catchesMissed, draft: { targets: [{ kind: "any" }], matching: "each", conditions: [{ code: "asks_you", param: { kind: "minutes", value: 10 } }], lifetime: { kind: "always" }, evenInQuietHours: false } });
  }
  if (accountInUse && (await offers(accountInUse, "runway_below"))) {
    out.push({ id: "runway", icon: IconGauge, badge: EMPTY.today, line: accountInUse.line, draft: { targets: [accountInUse.target], matching: "each", conditions: [{ code: "runway_below", param: { kind: "percent", value: 20 } }], lifetime: { kind: "today" }, evenInQuietHours: false } });
  }
  return out;
}

/** Three sessions, one watch, a bell rung once (drawn in the theme's tokens). */
function Drawing(): JSX.Element {
  const card = (y: number) => (
    <g key={y}>
      <rect x="40" y={y} width="88" height="36" rx="10" fill="var(--color-surface)" stroke="color-mix(in srgb, var(--color-hairline) 75%, transparent)" />
      <rect x="50" y={y + 10} width="16" height="16" rx="5" fill="var(--color-surface-3)" />
      <rect x="74" y={y + 12} width="40" height="5" rx="2.5" fill="var(--color-surface-3)" />
      <rect x="74" y={y + 21} width="26" height="4" rx="2" fill="var(--color-surface-3)" />
    </g>
  );
  return (
    <div aria-hidden="true" className="h-[252px] overflow-hidden rounded-2xl bg-canvas-hi bg-[radial-gradient(color-mix(in_srgb,var(--color-ink)_8%,transparent)_1px,transparent_1.2px)] bg-[length:14px_14px] inset-ring-1 inset-ring-hairline/75 max-[1100px]:hidden" data-testid="watches-drawing">
      <svg width="100%" height="100%" viewBox="0 0 420 252" fill="none" strokeLinecap="round" strokeLinejoin="round">
        <g stroke="var(--color-hairline)" strokeWidth="1.5">
          <path d="M128 66 C190 66 196 126 236 126" />
          <path d="M128 126 H236" />
          <path d="M128 186 C190 186 196 126 236 126" />
          <path d="M296 126 C318 126 322 92 344 92" stroke="var(--color-accent)" strokeDasharray="4 4" />
        </g>
        {[48, 108, 168].map(card)}
        <circle cx="266" cy="126" r="30" fill="color-mix(in srgb, var(--color-accent) 14%, var(--color-surface))" stroke="var(--color-accent)" strokeWidth="1.5" />
        <g transform="translate(250 110) scale(1.35)" stroke="var(--color-accent-hi)" strokeWidth="1.75">
          <path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" />
          <circle cx="12" cy="12" r="2.75" />
        </g>
        <rect x="336" y="70" width="64" height="44" rx="12" fill="var(--color-surface)" stroke="color-mix(in srgb, var(--color-hairline) 75%, transparent)" />
        <g transform="translate(356 80)" stroke="var(--color-ink-2)" strokeWidth="1.6">
          <path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15L6 16Z" transform="scale(.95)" />
        </g>
        <circle cx="392" cy="74" r="5" fill="var(--color-accent-fill)" />
        <text x="368" y="138" textAnchor="middle" fontSize="12" fill="var(--color-muted)">
          once
        </text>
      </svg>
    </div>
  );
}

function StarterCard({ starter, onOpen }: { starter: Starter; onOpen: (draft: WatchDraftInfo) => void }): JSX.Element {
  const [echo, setEcho] = useState<EchoParts | null>(null);
  useEffect(() => {
    let live = true;
    void watchDraftWords(starter.draft).then((words) => live && setEcho(words.echo));
    return () => {
      live = false;
    };
  }, [starter.draft]);
  const Icon = starter.icon;
  return (
    <button type="button" onClick={() => onOpen(starter.draft)} className="flex min-w-0 flex-col gap-2.5 rounded-card bg-surface p-3.5 text-left inset-ring-1 inset-ring-hairline/75 outline-none hover:inset-ring-faint/60 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-starter" data-starter={starter.id}>
      <span className="flex items-center gap-2.5">
        <span aria-hidden="true" className="flex size-7 items-center justify-center rounded-lg bg-accent-soft text-accent-hi">
          <Icon width={16} height={16} />
        </span>
        <span className="flex-1" />
        <span className="inline-flex h-[22px] items-center rounded-full bg-surface-3 px-2 text-label font-medium text-ink-2">{starter.badge}</span>
      </span>
      <span className="text-body font-semibold leading-snug text-ink-2">{echo ? <EchoHead spans={echo.head} /> : " "}</span>
      <span className="text-meta text-muted">{starter.line}</span>
      <span className="inline-flex items-center gap-1 text-meta font-medium text-accent-hi">
        {EMPTY.setUp}
        <IconChevronR width={12} height={12} aria-hidden="true" />
      </span>
    </button>
  );
}

export function WatchesEmpty({ starters, onNew, onAskBot, onStarter }: { starters: Starter[]; onNew: () => void; onAskBot: (() => void) | null; onStarter: (draft: WatchDraftInfo) => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-[26px]" data-testid="watches-empty">
      <div className="grid grid-cols-[minmax(0,1fr)_420px] items-center gap-10 rounded-card bg-surface px-7 pb-[26px] pt-7 inset-ring-1 inset-ring-hairline/75 max-[1100px]:grid-cols-1">
        <div className="flex flex-col gap-3.5">
          <span className="text-label font-semibold uppercase tracking-[0.07em] text-accent-hi">{EMPTY.eyebrow}</span>
          <h3 className="max-w-[22ch] text-[24px] font-semibold leading-[1.3] tracking-[-0.01em] text-ink">{EMPTY.heading}</h3>
          <p className="max-w-[52ch] text-body text-muted">{EMPTY.body}</p>
          <div className="mt-1.5 flex flex-wrap items-center gap-3.5">
            <Button variant="primary" onClick={onNew} data-testid="watches-empty-new">
              <IconAdd width={15} height={15} aria-hidden="true" />
              {PAGE.newWatch}
            </Button>
            {onAskBot && (
              <button type="button" onClick={onAskBot} className="inline-flex items-center gap-1.5 rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watches-empty-bot">
                <IconBotHead width={15} height={15} aria-hidden="true" />
                {EMPTY.orAskBot}
              </button>
            )}
          </div>
        </div>
        <Drawing />
      </div>
      {starters.length > 0 && (
        <section aria-labelledby="watch-starters" className="flex flex-col gap-3">
          <div className="flex flex-wrap items-baseline gap-x-2.5">
            <h3 id="watch-starters" className="text-section font-semibold text-ink">
              {EMPTY.startersTitle}
            </h3>
            <span className="text-meta text-muted">{EMPTY.startersLine}</span>
          </div>
          <div className="grid grid-cols-3 gap-3 max-[900px]:grid-cols-1">
            {starters.map((starter) => (
              <StarterCard key={starter.id} starter={starter} onOpen={onStarter} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
