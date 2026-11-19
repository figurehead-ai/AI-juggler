/**
 * AJ-E3816 — every word the Watches view says that isn't a watch's own
 * echo (those are the backend's, word for word): the page, the rows, the
 * menu, the form, the preview, the empty and ended states, the limit.
 * Plain words, one file, so the plain-words lint and a translator read it.
 */
import { THIS_MACHINE_IN_TEXT } from "../../lib/machine-words";

export const PAGE = {
  title: "Watches",
  lead: "AI Juggler tells you when something you picked happens. A watch only tells you; it never acts.",
  how: "How watches work",
  newWatch: "New watch",
  askBot: "Ask Bot",
  of: (n: number, limit: number) => `${n} of ${limit}`,
  watching: "Watching",
  watchingLine: (active: number, paused: number) => [active > 0 || paused === 0 ? `${active} active` : null, paused > 0 ? `${paused} paused` : null].filter(Boolean).join(" · "),
  ended: "Ended",
  endedLine: (n: number) => `${n} · kept for 30 days`,
  more: (n: number) => `${n} more`,
  show: "Show",
  shown: (shown: number, total: number) => `${shown} of ${total} shown`,
  showMore: (n: number) => `Show ${n} more`,
  loading: "Loading watches",
  failed: "Couldn't read the watches.",
  retry: "Try again",
  unavailable: `Watches aren't ready on ${THIS_MACHINE_IN_TEXT} yet. They start once AI Juggler has updated its records.`,
} as const;

/** The quiet-hours line under the head (only when the user has quiet hours). */
export const QUIET = {
  link: "Quiet hours",
  line: (from: string, to: string, breakThrough: number) =>
    [
      { text: "Quiet hours are " },
      { text: `${from} to ${to}`, strong: true },
      { text: `. Watch alerts wait and arrive together at ${to}` },
      ...(breakThrough > 0 ? [{ text: ", except " }, { text: `${breakThrough} ${breakThrough === 1 ? "watch" : "watches"}`, strong: true }, { text: " set to tell you anyway." }] : [{ text: "." }]),
    ] as { text: string; strong?: boolean }[],
};

export const ROW = {
  edit: "Edit",
  pause: "Pause",
  resume: "Resume",
  more: (echo: string) => `More for: ${echo}`,
  toldTimes: (n: number) => `Told you ${n} ${n === 1 ? "time" : "times"}`,
  waiting: "Waiting",
  paused: "Paused",
  neverTold: "never told you",
  last: (when: string, what: string | null) => (what ? `last ${when} · ${what}` : `last ${when}`),
  pausedSince: (when: string, told: number) => (told > 0 ? `since ${when} · told ${told} ${told === 1 ? "time" : "times"}` : `since ${when}`),
  quietChip: "Even during quiet hours",
  removedTarget: "removed",
} as const;

/** What a target is, in a row's second line and the picker. */
export const KIND = {
  session: "Session",
  app: "App",
  project: "Project",
  pool: "Pool",
  account: "Account",
  any: `Every session on ${THIS_MACHINE_IN_TEXT}`,
  sessions: (n: number) => `${n} sessions`,
  targets: (n: number) => `${n} targets`,
  everySessionInIt: "every session in it",
  members: (n: number) => `${n} ${n === 1 ? "member" : "members"}`,
  openSessions: (n: number) => `${n} ${n === 1 ? "session" : "sessions"} open`,
  appsSignIn: (n: number) => `${n} ${n === 1 ? "app signs" : "apps sign"} in with it`,
  roomAndLimits: "its room and limits",
} as const;

export const MENU = {
  heading: "Watch",
  edit: "Edit",
  pause: "Pause",
  pauseHint: "Keeps it, but tells you nothing",
  resume: "Resume",
  resumeHint: "Tells you again from now",
  quiet: "Even during quiet hours",
  quietOff: (to: string) => `Off: alerts wait till ${to}`,
  quietOn: "On: this watch can wake you",
  goTo: (name: string) => `Go to ${name}`,
  similar: "Watch something similar…",
  remove: "Remove",
  removeHint: "Stops at once. Undo for a few seconds.",
} as const;

export const TOAST = {
  removed: (what: string) => `Watch removed: ${what}`,
  undoing: "Bringing it back…",
  undone: "It's watching again.",
  undoFailed: (cause: string) => `${cause} The watch stays removed.`,
  paused: "Paused. It tells you nothing until you resume it.",
  resumed: "Watching again.",
  quietOn: "This watch can now tell you during quiet hours.",
  quietOff: "This watch now waits for the end of quiet hours.",
  made: "Watching.",
  saved: "Changes saved.",
  failed: (cause: string) => `That didn't work. ${cause}`,
} as const;

export const ENDED = {
  endedAt: (when: string) => `Ended ${when}`,
  endedAtMidnight: "Ended at midnight",
  told: (n: number) => (n === 0 ? "never told you" : `told you ${n} ${n === 1 ? "time" : "times"}`),
  closedAt: (name: string, when: string) => `${name} closed at ${when}`,
  wasLifetime: (words: string) => `it was “${words.replace(/\.$/, "")}”`,
  untilMidnight: "Until midnight",
  removedTarget: (kind: string, name: string) => `the ${kind} ${name} was removed`,
  noLonger: (name: string) => `${name} no longer reports it`,
  watchInstead: (app: string) => `Watch ${app} instead…`,
  renew: "Renew",
  renewToday: "Renew for today",
  pickNew: "Pick a new target…",
  remove: (echo: string) => `Remove: ${echo}`,
} as const;

export const EMPTY = {
  eyebrow: "No watches yet",
  heading: "Get told when an agent finishes, asks you or gets stuck",
  body: "Pick a session, an app, a project or an account, and what to watch it for. AI Juggler tells you once, the way it tells you everything else, and keeps your quiet hours.",
  orAskBot: "or ask Bot: “tell me when api finishes”",
  orAskBotNamed: (name: string) => `or ask Bot: “tell me when ${name} finishes”`,
  startersTitle: "Start from one of these",
  startersLine: "From what's open now. Each opens the form filled in; nothing starts until you press Watch.",
  setUp: "Set it up",
  once: "Once",
  always: "Always",
  today: "Today",
  workingNow: "working now",
  catchesMissed: "Catches a question you missed",
  illustration: "Three sessions, one watch, one bell rung once",
} as const;

export const MANY = {
  search: "Search watches",
  filter: "Show",
  all: "All",
  toldToday: "Told you today",
  neverTold: "Never told",
  paused: "Paused",
  sort: "Sort",
  sortLast: "Last told",
  sortNewest: "Newest",
  sortTarget: "Target",
  none: "No watch matches.",
  limit: (limit: number) => `${limit} watches is the most AI Juggler keeps at once.`,
  limitLine: (never: number) => `To add one, remove one or let one end.${never > 0 ? ` ${never} ${never === 1 ? "has" : "have"} never told you anything.` : ""}`,
  showThose: (n: number) => `Show ${n === 1 ? "that one" : `those ${n}`}`,
} as const;

export const FORM = {
  back: "Watches",
  newTitle: "New watch",
  newLead: "Pick what to watch, then what for.",
  editTitle: "Edit watch",
  editLead: (made: string, told: number) => `Made ${made} · ${told === 0 ? "never told you" : `told you ${told} ${told === 1 ? "time" : "times"}`}`,
  what: "Watch what",
  whatHelp: "Sessions, apps, projects, pools or accounts",
  forWhat: "For what",
  forWhatHelp: (n: number) => (n > 1 ? "Only what every one of them can report" : "Only what it can report"),
  forWhatBoth: "Only what both can report",
  until: "Until",
  quiet: "Quiet hours",
  matchingLabel: (n: number, kind: string) => `With ${n} ${kind}, tell you`,
  each: "For each one",
  all: "When all have",
  more: (n: number, list: string) => `${n} more: ${list}`,
  fewer: "Fewer",
  upTo: "Up to 4. Any one of them tells you.",
  pickTargetFirst: "Pick what to watch first.",
  nothingOffered: "Nothing can be watched on all of these together. Pick fewer, or ones from the same kind of app.",
  forParam: "for",
  minutes: "minutes",
  below: "below",
  percent: "%",
  to: "to",
  from: "from",
  anyAgent: "any agent",
  quietTitle: "Even during quiet hours",
  quietHelp: (from: string, to: string) => `Off: an alert between ${from} and ${to} waits and arrives at ${to}. On: this watch can wake you.`,
  quietNone: "You have no quiet hours, so every alert comes at once.",
  cancel: "Cancel",
  watch: "Watch",
  save: "Save changes",
  removeWatch: "Remove watch",
  startsNow: (n: number, limit: number) => `Starts watching now · ${n} of ${limit}.`,
  changesNow: "Changes it at once. Its history stays.",
  saving: "Saving…",
  limitReached: (limit: number) => `${limit} watches is the most AI Juggler keeps at once. Remove one first.`,
  endedNote: "This watch has ended. Saving brings it back with these choices.",
  targetGone: "What it watched is gone. Pick a new target.",
} as const;

/** The lifetime choices, and the line under them. */
export const LIFE = {
  once: "Once",
  untilEnds: "Until it ends",
  untilTheyEnd: "Until they end",
  today: "Today",
  until: "Until…",
  always: "Always",
  suggested: (condition: string) => `Suggested for “${condition}”`,
  onceLine: "Tells you once, then ends. You'll find it under Ended.",
  untilEndsLine: (names: string) => `Ends by itself once ${names} ${names.includes(" and ") ? "have both" : "has"} ended. You'll find it under Ended.`,
  todayLine: "Ends at midnight tonight.",
  untilLine: (when: string) => `Ends ${when}.`,
  untilPick: "Pick a day and a time.",
  alwaysLine: "Keeps watching until you remove it.",
  day: "Day",
  time: "Time",
  past: "That time has passed. Pick a later one.",
} as const;

/** The live preview beside the form. */
export const PREVIEW = {
  title: "Looks like this",
  updates: "updates as you choose",
  was: "Was",
  exact: "Word for word what AI Juggler watches.",
  app: "AI Juggler",
  now: "now",
  yourWatch: "your watch",
  openIt: "open it",
  quietNote: "A finish is a quiet note in the feed and the notification centre.",
  calmNote: (what: string) => `${what} is a quiet note in the feed and the notification centre.`,
  needNote: (what: string) => `${what} also shows in Needs you; you get one alert, not two.`,
  quietWaits: (from: string, to: string) => `Between ${from} and ${to} it waits till morning.`,
  quietWakes: (from: string, to: string) => `Between ${from} and ${to} it can still wake you.`,
  noQuiet: "It comes at once: you have no quiet hours.",
  onceEnds: "Tells you once, then ends.",
  untilEnds: (n: number) => (n > 1 ? `Ends when ${n === 2 ? "both" : "all"} sessions end.` : "Ends when the session ends."),
  todayEnds: "Ends at midnight tonight.",
  untilEndsAt: (when: string) => `Ends ${when}.`,
  alwaysEnds: "Keeps watching until you remove it.",
  footer: "A watch only tells you. It never sends, stops or approves.",
  summary: "Looks like this:",
} as const;

/** The target picker. */
export const PICKER = {
  field: "Watch what",
  add: "Add more, or search…",
  addMore: "Add more…",
  search: "Search sessions, apps, projects…",
  openNow: "Open now",
  recent: "Recent",
  all: "All",
  back: "All",
  results: "Matches",
  sessions: "Sessions",
  apps: "Apps",
  projects: "Projects",
  pools: "Pools",
  accounts: "Accounts",
  any: "Any session",
  anyLine: `Every session on ${THIS_MACHINE_IN_TEXT}`,
  picked: (n: number, max: number) => `${n} picked · up to ${max}`,
  keys: "↑↓ to move · Space to pick",
  remove: (name: string) => `Remove ${name}`,
  none: "Nothing matches.",
  full: (max: number) => `Up to ${max}.`,
  anyAlone: `Any session watches everything on ${THIS_MACHINE_IN_TEXT}, so it can't sit beside other targets.`,
  everySession: "every session in it",
} as const;

/** The doors into the page from elsewhere. */
export const DOORS = {
  watch: "Watch…",
  watched: (n: number) => `Watched · ${n}`,
  editWatch: "Edit watch",
  centreLink: (n: number) => `Watches ${n}`,
  manage: (n: number) => `Manage watches · ${n}`,
} as const;

/** Each condition as a thing that happens, for the preview's lines. */
export const NOUN: Record<string, string> = {
  finishes: "A finish",
  ends: "An ending",
  fails: "A failure",
  asks_you: "A question",
  stuck: "Looking stuck",
  idle_for: "Sitting idle",
  hits_limit: "A usage limit",
  runway_below: "Running low",
  runway_resets: "Room again",
  leaves_processes: "Processes left running",
  restarted: "A restart",
  pool_message: "A message",
  task_changes: "A task moving",
  commits: "A commit",
  starts: "A new session",
};

/** Each condition's line on its choice card. */
export const CONDITION_LINE: Record<string, string> = {
  finishes: "A turn ends and it waits for you",
  asks_you: "A question, or an OK to go on",
  stuck: "Working, but nothing moves",
  fails: "A turn or the session stops with an error",
  ends: "The session closes, however it ended",
  idle_for: "Open, but nothing happens",
  hits_limit: "It reaches a usage limit",
  runway_below: "Its room drops below a share",
  runway_resets: "Its room comes back",
  leaves_processes: "It ends with processes still running",
  restarted: "Keep alive starts it again after a crash",
  pool_message: "A message arrives in the pool",
  task_changes: "A task moves to a state",
  commits: "A commit lands in the project",
  starts: "A new session starts",
};

/** The parameter's choices. */
export const PARAM = {
  rightAway: "right away",
  asSoon: "as soon as it looks so",
  minutes: (n: number) => `${n}`,
  done: "done",
  blocked: "blocked",
  inReview: "in review",
} as const;
