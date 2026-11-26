/**
 * AJ-E3713 R3 — a post's gesture bar, calm (the mock's: faint icons, a
 * fill on hover, the label in a tip), and "Why am I seeing this?":
 *
 * - **More / Less like this** (l / m on the focused post): kept as a
 *   `preference` memory through ADR-0037's write gate, scoped to the kind
 *   of event, the App and the project (`lib/feed-leans`). The lean behind
 *   a post lights its arrow; the panel under the post says what it does,
 *   with Undo and the way to every memory (What Bot remembers).
 * - **Remind me**: a one-time reminder watch at a picked time, through
 *   the Watches flow. Shown only when the watches offer a reminder
 *   (D-127).
 * - **Follow**: a watch on the post's session, pool, project, Account or
 *   App for what it can report (finishes, asks you, fails …), made the
 *   Watches way (the Notifications prompt primed first, PAT-OSP);
 *   "Following" opens the watches that name it.
 * - **Ask Bot**: Bot opened on the post's thing, the post in the question
 *   box (only while Bot's AI is ready, D-144). **Open**: its thing.
 */
import { useState, type JSX, type ReactNode } from "react";
import { MenuItem, MenuLine, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { useTip } from "./WhyTip";
import { showToast } from "./Toast";
import { watchesChanged } from "./watches/WatchDoor";
import { errorText } from "../lib/bridge";
import { expandBot, openBot, startDraft } from "../lib/bot-store";
import { useBotReady } from "../lib/ai-availability";
import { lean, leanFor, scopeOf, unlean, useFeedLeans, whyWords } from "../lib/feed-leans";
import { followTarget, follow, remindAt, REMIND_CHOICES, useFollowing, useRemindOffered, type Followed } from "../lib/feed-watch";
import { openWatches } from "../lib/watches-open";
import { storyText, type StoryPart } from "../lib/feed-story";
import { clock } from "../lib/time-words";
import type { FeedItemInfo } from "../lib/ipc/feed";

/** The mock's icons (15 px, a 1.7 stroke). */
function Glyph({ children }: { children: ReactNode }): JSX.Element {
  return (
    <svg viewBox="0 0 24 24" width={15} height={15} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round">
      {children}
    </svg>
  );
}
const UP = <Glyph><path d="M7 11l5-5 5 5M12 6v12" /></Glyph>;
const DOWN = <Glyph><path d="M7 13l5 5 5-5M12 18V6" /></Glyph>;
const CLOCK = <Glyph><circle cx="12" cy="13" r="7" /><path d="M12 10v3l2 2M5 4l3 2M19 4l-3 2" /></Glyph>;
const PLUS = <Glyph><path d="M12 5v14M5 12h14" /></Glyph>;
const BUBBLE = <Glyph><path d="M5 5h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-8l-4 3v-3H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z" /><path d="M9.5 10.5h.01M14.5 10.5h.01" /></Glyph>;
const OPEN = <Glyph><path d="M7 17L17 7M9 7h8v8" /></Glyph>;

const G = "inline-flex h-7 items-center gap-[5px] rounded-[7px] px-2 text-label outline-none hover:bg-surface-3 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60";

/** One gesture: its icon, its label in a tip (and its name), and words
 * beside the icon when it has them ("Following", "Open"). */
function Gesture({ label, icon, words, on, onClick, testId, lean: leanAttr, expanded, row }: {
  label: string;
  icon: JSX.Element;
  words?: string;
  on?: boolean;
  onClick: (event: React.MouseEvent<HTMLButtonElement>) => void;
  testId: string;
  lean?: "more" | "less";
  expanded?: boolean;
  /** The post's row for J/K (Enter opens it). */
  row?: boolean;
}): JSX.Element {
  const { ref, show, hide, tip } = useTip<HTMLButtonElement>(label, "feed-gesture-tip", undefined, "presentation");
  return (
    <>
      <button
        ref={ref}
        type="button"
        aria-label={words ? undefined : label}
        aria-pressed={on === undefined ? undefined : on}
        aria-expanded={expanded}
        onClick={onClick}
        onMouseEnter={show}
        onMouseLeave={hide}
        onFocus={show}
        onBlur={hide}
        className={G + (on ? " text-accent-hi" : "")}
        data-testid={testId}
        data-feed-lean={leanAttr}
        data-home-row={row ? "" : undefined}
      >
        {icon}
        {words && <span>{words}</span>}
      </button>
      {!words && tip}
    </>
  );
}

/** The post's thing, as Bot's scope names it. */
function botScope(item: FeedItemInfo): Parameters<typeof openBot>[0] {
  const { target } = item;
  if (target.session_id) return { kind: "session", id: target.session_id, name: item.subject || "this session", sessions: [] };
  if (target.pool_id) return { kind: "pool", id: target.pool_id, name: item.facts.pool ?? "this pool", sessions: [] };
  if (target.project_id) return { kind: "project", id: target.project_id, name: item.facts.project ?? "this project", sessions: [] };
  if (target.account_id) return { kind: "account", id: target.account_id, name: item.author.name ?? "this Account", sessions: [] };
  return null;
}

/** "Why am I seeing this?": the lean behind the post, Undo, and where
 * every memory is. */
export function WhyPanel({ item }: { item: FeedItemInfo }): JSX.Element | null {
  const kept = useFeedLeans();
  const behind = leanFor(item, kept);
  if (!behind) return null;
  const { head, why } = whyWords(behind);
  const link = "rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";
  const undo = () => {
    void unlean(behind.key).then(
      () => showToast({ message: "Back to how it was.", detail: why.replace(/^You chose/, "Undid") }),
      (error: unknown) => showToast({ message: "That couldn't be undone.", detail: errorText(error), tone: "error" }),
    );
  };
  const manage = () => {
    expandBot();
    window.setTimeout(() => window.dispatchEvent(new Event("aij:bot-memories")), 0);
  };
  return (
    <div className="col-start-2 mb-1 mt-1.5 rounded-[10px] bg-surface-2 px-3 py-2 text-[0.78125rem] text-ink-2" data-testid="feed-why" data-lean={behind.lean}>
      <b className="font-semibold text-ink">{head}</b> {why}{" "}
      <button type="button" onClick={undo} className={link} data-testid="feed-why-undo">Undo</button>
      {" · "}
      <button type="button" onClick={manage} className={link} data-testid="feed-why-manage">Manage in What Bot remembers</button>
    </div>
  );
}

/** Remind me's time picker (a menu of times). */
function RemindMenu({ at, item, name, onClose }: { at: MenuAt; item: FeedItemInfo; name: string; onClose: () => void }): JSX.Element {
  // The times from when the menu opened.
  const [opened] = useState(() => Date.now());
  const pick = (minutes: number) => {
    onClose();
    const target = followTarget(item);
    if (!target) return;
    void remindAt(target, minutes).then(
      (when) => showToast({ message: `Reminder set for ${clock(when)}.`, detail: `About ${name}.`, action: { label: "Watches", run: () => openWatches({ kind: "list" }) } }),
      (error: unknown) => showToast({ message: "The reminder couldn't be set.", detail: errorText(error), tone: "error" }),
    );
  };
  return (
    <RowMenu heading="Remind me" ariaLabel="Remind me" at={at} onClose={onClose} testId="feed-remind-menu">
      {REMIND_CHOICES(opened).map((choice) => (
        <MenuItem key={choice.label} onClick={() => pick(choice.minutes)} testId="feed-remind-choice">
          <MenuLine icon={() => CLOCK} label={choice.label} />
        </MenuItem>
      ))}
    </RowMenu>
  );
}

export function GestureBar({ item, parts, why, onOpen, onLeaned, onWhy, extra }: {
  item: FeedItemInfo;
  parts: readonly StoryPart[];
  /** "Why am I seeing this?" is open under the post. */
  why: boolean;
  onOpen: () => void;
  /** A lean was kept: the post stays in place with its why open. */
  onLeaned: () => void;
  /** The lit arrow: open or close the why. */
  onWhy: () => void;
  /** A post's own action before Open (Mark reviewed). */
  extra?: ReactNode;
}): JSX.Element {
  const kept = useFeedLeans();
  const behind = leanFor(item, kept);
  const botReady = useBotReady();
  const target = followTarget(item);
  const following: Followed | null = useFollowing(target);
  const remindOffered = useRemindOffered();
  const [remind, setRemind] = useState<MenuAt | null>(null);
  const name = item.subject || item.facts.pool || item.facts.project || "this";
  const setLean = (which: "more" | "less") => {
    // The lit arrow is the lean behind this post: it says why (with Undo).
    if (behind?.lean === which) {
      onWhy();
      return;
    }
    const scope = scopeOf(item);
    const about = scope.project ? item.facts.project : scope.folder ? item.author.folder_label : null;
    // Pinned in place first, so the post the reader leaned never folds
    // away under them (its why and Undo stay at hand).
    onLeaned();
    void lean({ lean: which, group: scope.group, folder: scope.folder, project: scope.project, about }).then(
      (made) => (made ? undefined : showToast({ message: "That couldn't be kept.", tone: "warning" })),
      (error: unknown) => showToast({ message: "That couldn't be kept.", detail: errorText(error), tone: "error" }),
    );
  };
  const doFollow = () => {
    if (following) {
      openWatches({ kind: "filter", target: following.target, name });
      return;
    }
    if (!target) return;
    void follow(target).then(
      (made) => {
        watchesChanged();
        showToast({ message: `Following ${name}.`, detail: made.words, action: { label: "Edit", run: () => openWatches({ kind: "edit", id: made.watch.id }) } });
      },
      (error: unknown) => showToast({ message: `${name} couldn't be followed.`, detail: errorText(error), tone: "error" }),
    );
  };
  const ask = () => {
    openBot(botScope(item));
    startDraft(`About this post: “${storyText(parts)}” `);
  };
  return (
    <div className="col-start-2 flex items-center gap-0.5 border-t border-divider pt-1 text-faint" data-testid="feed-gestures">
      <Gesture label={behind?.lean === "more" ? "Why am I seeing this?" : "More like this"} icon={UP} on={behind?.lean === "more"} expanded={behind?.lean === "more" ? why : undefined} onClick={() => setLean("more")} testId="feed-more-like" lean="more" />
      <Gesture label={behind?.lean === "less" ? "Why am I seeing this?" : "Less like this"} icon={DOWN} on={behind?.lean === "less"} expanded={behind?.lean === "less" ? why : undefined} onClick={() => setLean("less")} testId="feed-less-like" lean="less" />
      {remindOffered && target && <Gesture label="Remind me" icon={CLOCK} onClick={(event) => setRemind(menuPoint(event))} expanded={remind !== null} testId="feed-remind" />}
      {target && <Gesture label={following ? "Following: see its watches" : "Follow"} icon={PLUS} words={following ? "Following" : undefined} on={following !== null} onClick={doFollow} testId="feed-follow" />}
      {botReady && botScope(item) && <Gesture label="Ask Bot" icon={BUBBLE} onClick={ask} testId="feed-ask-bot" />}
      <span className="flex-1" />
      {extra}
      <Gesture label="Open" icon={OPEN} words="Open" onClick={onOpen} testId="feed-open" row />
      {remind && <RemindMenu at={remind} item={item} name={name} onClose={() => setRemind(null)} />}
    </div>
  );
}
