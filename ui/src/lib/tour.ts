/**
 * AJ-E1546 — the guided tour's registry and its memory. Each step names
 * the feature it shows and the version that feature came in, the element
 * its coach mark points at (by its id, on the real UI), Bot's words for it
 * (Bot is the tour's guide, the maintainer's amendment: every word here
 * is scripted, so the tour works with no engine), the clip shown when the
 * element isn't on screen, and the question "Ask Bot about this" drafts
 * when Bot is ready.
 *
 * Completion is stored per step id. A step seen is never shown again by
 * the offer; after an update, only steps not yet seen are offered, once,
 * as "New in 0.5.x · Show me" — and only to someone who took the tour
 * before (a newcomer is offered the whole tour by the setup wizard). Help
 * › Take the tour shows every step, any time.
 */
import type { ClipId } from "./clips";
import { THIS_MACHINE, THIS_MACHINE_IN_TEXT } from "./machine-words";

export interface TourStep {
  id: string;
  /** The feature it shows (one id per feature). */
  feature: string;
  /** The version the feature came in ("0.5.0"). */
  since: string;
  /** The coach mark's anchor: a selector for the element's id. */
  target: string;
  title: string;
  /** Bot's words, in its voice. */
  text: string;
  /** Its words while no AI is set up yet, when they differ (D-144). */
  textNoAi?: string;
  /** Shown in place of the coach mark when the element isn't on screen. */
  clip: ClipId | null;
  /** What "Ask Bot about this" asks (only while Bot is ready). */
  ask: string;
  /** Opened for the step and closed after it (the first stop: Bot's panel). */
  opens?: "bot";
}

export const TOUR: readonly TourStep[] = [
  {
    id: "bot",
    feature: "bot",
    since: "0.5.0",
    target: "[data-testid='bot-panel']",
    opens: "bot",
    title: "I'm Bot, your guide",
    text: "I'm your AI assistant for everything AI Juggler sees. Open me here or with ⌘J and ask in plain words: what needs you, what changed in a project, how much an account has left. I cite every answer, and I never act on my own.",
    textNoAi: `I'm your AI assistant for everything AI Juggler sees. I need an AI on ${THIS_MACHINE_IN_TEXT} first: open me here or with ⌘J and Set up Bot walks you through it. Until then, Search (⌘K) finds any session, task or account.`,
    clip: "what-bot",
    ask: "What can you tell me about my sessions?",
  },
  {
    id: "open-now",
    feature: "open-now",
    since: "0.2.0",
    target: "[data-testid='rail-open-now']",
    title: "Open now",
    text: "Every session that's running, in every app and account, in one list. Click one to see it, or find any session with Search (⌘K).",
    clip: "what-sessions",
    ask: "What's running right now?",
  },
  {
    id: "needs-you",
    feature: "needs-you",
    since: "0.2.0",
    target: "[data-testid='rail-needs-you']",
    title: "Needs you",
    text: "What waits on you comes first: a session that asks you something, looks stuck or hit its limit.",
    clip: "how-juggle",
    ask: "What needs me right now?",
  },
  {
    id: "session",
    feature: "session-page",
    since: "0.2.0",
    target: "[data-testid='rail'] [data-instance-id]",
    title: "A session's page",
    text: "Open a session to see its conversation, what it changed and what it used. Its terminal runs right there.",
    clip: "how-start",
    ask: "Which sessions finished today?",
  },
  {
    id: "usage",
    feature: "usage-runway",
    since: "0.1.0",
    target: "[data-testid='nav-usage']",
    title: "Usage and runway",
    text: "How much each account has left, and when it comes back, so you carry on before you hit the wall.",
    clip: "what-usage",
    ask: "How much do my accounts have left?",
  },
  {
    id: "switch",
    feature: "switch-move",
    since: "0.4.0",
    target: "[data-testid='switch-account']",
    title: "Continue in another app, or Move",
    text: "When an account runs low, Switch account… carries the session on in another app or account. Move takes the same conversation along.",
    clip: "how-juggle",
    ask: "Which account has the most room left?",
  },
  {
    id: "pools",
    feature: "pools",
    since: "0.4.6",
    target: "[data-testid='nav-pools']",
    title: "Pools",
    text: "Sessions in different apps can work as a team: one leads, others build or check, and they message each other.",
    clip: null,
    ask: "What's happening in my pools?",
  },
  {
    id: "processes",
    feature: "processes",
    since: "0.3.0",
    target: "[data-testid='user-menu-button']",
    title: `Processes and ${THIS_MACHINE}`,
    text: `In your menu: what ${THIS_MACHINE_IN_TEXT} is running for your sessions, how hard it's working, and what's left behind to clean up.`,
    clip: "what-mac",
    ask: "Is anything left running that I can stop?",
  },
  {
    id: "settings",
    feature: "settings",
    since: "0.1.0",
    target: "[data-testid='user-menu-button']",
    title: "Settings",
    text: "Your menu has Settings too (⌘,): notifications, keeping work going, and the models I run on. That's the tour.",
    clip: null,
    ask: "How do I turn on notifications?",
  },
];

const DONE_KEY = "aij.tour.done";
const OFFERED_KEY = "aij.tour.offered";

function readIds(key: string): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(key) ?? "[]") as unknown;
    return Array.isArray(raw) ? raw.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function addIds(key: string, ids: readonly string[]): void {
  try {
    localStorage.setItem(key, JSON.stringify([...new Set([...readIds(key), ...ids])]));
  } catch {
    // A private window: offered again next time.
  }
}

/** The steps seen. */
export function doneSteps(): string[] {
  return readIds(DONE_KEY);
}

/** A step was seen. */
export function markDone(id: string): void {
  addIds(DONE_KEY, [id]);
}

/** "0.5.1" → [0, 5, 1]; anything else sorts first. */
function parts(version: string): number[] {
  return version.split(".").map((part) => Number.parseInt(part, 10) || 0);
}

export function newer(a: string, b: string): boolean {
  const [x, y] = [parts(a), parts(b)];
  for (let at = 0; at < Math.max(x.length, y.length); at += 1) {
    if ((x[at] ?? 0) !== (y[at] ?? 0)) return (x[at] ?? 0) > (y[at] ?? 0);
  }
  return false;
}

/** After an update: the steps not seen nor offered yet, and the words
 * "New in 0.5.x" (the newest feature's major.minor). `null`: nothing to
 * offer — a newcomer who never took the tour, or nothing new. */
export function newOffer(steps: readonly TourStep[], done: readonly string[], offered: readonly string[]): { ids: string[]; label: string } | null {
  if (done.length === 0) return null;
  const fresh = steps.filter((step) => !done.includes(step.id) && !offered.includes(step.id));
  if (fresh.length === 0) return null;
  const newest = fresh.reduce((best, step) => (newer(step.since, best) ? step.since : best), "0.0.0");
  const [major, minor] = parts(newest);
  return { ids: fresh.map((step) => step.id), label: `New in ${major}.${minor}.x` };
}

/** The offer for this Mac's memory. */
export function currentOffer(): { ids: string[]; label: string } | null {
  return newOffer(TOUR, doneSteps(), readIds(OFFERED_KEY));
}

/** The offer was answered (Show me or put away): never again. */
export function markOffered(ids: readonly string[]): void {
  addIds(OFFERED_KEY, ids);
}

/** Starts the tour: every step, or only `ids` (the "New in" offer). */
export const TAKE_TOUR_EVENT = "aij:take-tour";

export function startTour(ids?: readonly string[]): void {
  window.dispatchEvent(new CustomEvent<{ ids?: readonly string[] }>(TAKE_TOUR_EVENT, { detail: { ids } }));
}

/** The steps to show for a start's ids. */
export function stepsFor(ids?: readonly string[]): TourStep[] {
  return ids ? TOUR.filter((step) => ids.includes(step.id)) : [...TOUR];
}
