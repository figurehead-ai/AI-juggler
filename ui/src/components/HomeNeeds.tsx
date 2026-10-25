/**
 * AJ-E2105 — Home's Needs you card: the first thing that waits on the
 * user, large (its title, where it is, and its primary: "Go to it ↵" for a
 * session, with "Continue on another account…" beside it), then the rest,
 * one line each with its own primary. Every kind of item sits here
 * (lib/home-needs.ts). Message text is shown only through `bubble`, a
 * session's last message when its page's reader provides one (AJ-E2108).
 */
import type { JSX, ReactNode } from "react";
import { Button } from "./primitives";
import { SectionTitle } from "./SectionTitle";
import { useActionHost } from "./ItemMenu";
import { PlaceMark } from "./PlaceMarks";
import { primaryWords, type HomeNeed } from "../lib/home-needs";
import { needWord } from "./PlaceRows";
import { markSeen } from "../lib/seen";
import { AskBatch } from "./AskCard";

export interface NeedHandlers {
  onOpenSession: (id: string) => void;
  onLimit?: (id: string) => void;
  onWait?: (id: string) => void;
  onSignIn?: (need: HomeNeed) => void;
  onOpenThisMac?: () => void;
  onOpenCleanup?: () => void;
}

function Mark({ need, size }: { need: HomeNeed; size: number }): JSX.Element {
  if (need.item) return <PlaceMark place={need.item.place} profile={need.item.accountLabel} size={size} />;
  const glyph = need.kind === "signin" ? "→" : need.severe ? "■" : "▲";
  return (
    <span aria-hidden="true" style={{ width: size, height: size }} className={"flex shrink-0 items-center justify-center rounded-lg text-xs font-bold " + (need.kind === "signin" ? "bg-accent-soft text-accent-hi" : need.severe ? "bg-danger-soft text-danger" : "bg-warning-soft text-warning")}>
      {glyph}
    </span>
  );
}

/** Runs an item's primary (or its second choice). */
function useRun(handlers: NeedHandlers): (need: HomeNeed, second?: boolean) => void {
  const host = useActionHost();
  return (need, second = false) => {
    const item = need.item;
    const session = item?.session;
    const conversation = item?.conversation;
    switch (need.kind) {
      case "session":
      case "failed":
        if (conversation) {
          markSeen(conversation.account_id, conversation.id);
          // "Continue on another account…" lives on its page (AJ-E2108).
          if (!second && conversation.live && host.goToConversation) void host.goToConversation(conversation);
          else host.openConversation?.(conversation);
        } else if (session) {
          handlers.onOpenSession(session.id);
          if (second) window.dispatchEvent(new Event("aij:focus-handover"));
        }
        return;
      case "limit":
        if (session) (second ? handlers.onWait : handlers.onLimit)?.(session.id);
        return;
      case "signin":
        handlers.onSignIn?.(need);
        return;
      case "mac":
        handlers.onOpenThisMac?.();
        return;
      case "heavy":
        if (need.sessionId) handlers.onOpenSession(need.sessionId);
        return;
      case "leftovers":
        handlers.onOpenCleanup?.();
    }
  };
}

/** Its second choice, when it has one. */
function secondWords(need: HomeNeed): string | null {
  if (need.kind === "session" && need.item?.need === "asks") return "Continue on another account…";
  if (need.kind === "limit") return "Wait";
  return null;
}

export function HomeNeeds({ needs, handlers, bubble, children }: { needs: HomeNeed[]; handlers: NeedHandlers; bubble?: ReactNode; children?: ReactNode }): JSX.Element | null {
  const run = useRun(handlers);
  const [first, ...rest] = needs;
  // Nothing waits: the heading says so, and the card stays away.
  if (!first && !children) return null;
  return (
    <section aria-labelledby="home-needs" className="flex min-w-0 flex-col gap-2.5" data-testid="home-needs">
      <SectionTitle id="home-needs">Needs you</SectionTitle>
      <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface p-4">
        <AskBatch titleOf={(id) => needs.find((need) => need.item?.session?.id === id)?.title} />
        {first ? (
          <div className="flex flex-col gap-3" data-home-item data-testid={`home-need-${first.kind}`} data-first="true" data-kind={first.kind}>
            <div className="flex min-w-0 items-start gap-3">
              <Mark need={first} size={32} />
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-[15px] font-semibold text-ink">{first.title}</span>
                <span className="truncate text-xs text-muted">{first.detail}</span>
              </div>
              {first.item && <span className={"shrink-0 text-xs font-semibold " + (first.severe ? "text-danger" : "text-warning")}>{needWord(first.item)}</span>}
            </div>
            {bubble}
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" variant="primary" data-home-row aria-keyshortcuts="Enter" onClick={() => run(first)}>
                {primaryWords(first)}
                <span aria-hidden="true" className="opacity-80">↵</span>
                <span className="sr-only"> — {first.title}</span>
              </Button>
              {secondWords(first) && (
                <Button size="sm" variant="secondary" data-home-action onClick={() => run(first, true)}>
                  {secondWords(first)}
                  <span className="sr-only"> — {first.title}</span>
                </Button>
              )}
            </div>
          </div>
        ) : (
          <p className="text-sm text-muted" data-testid="home-needs-none">
            Nothing waits on you. What's open is in the list and under Your apps.
          </p>
        )}
        {rest.length > 0 && (
          <ul className="flex flex-col border-t border-hairline/60 pt-1">
            {rest.map((need) => (
              <li key={need.key} data-home-item data-kind={need.kind} data-testid={`home-need-${need.kind}`} className="flex min-w-0 items-center gap-2.5 border-b border-hairline/40 py-2 last:border-b-0">
                <Mark need={need} size={22} />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-[13px] font-medium text-ink">{need.title}</span>
                  <span className="truncate text-[11px] text-muted">{need.item ? `${needWord(need.item)} · ${need.detail}` : need.detail}</span>
                </span>
                <Button size="sm" variant="secondary" data-home-row aria-keyshortcuts="A" onClick={() => run(need)}>
                  {primaryWords(need)}
                  <span className="sr-only"> — {need.title}</span>
                </Button>
              </li>
            ))}
          </ul>
        )}
        {children}
      </div>
    </section>
  );
}
