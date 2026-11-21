/**
 * D-136 — the words of Bot's panel and page (labels, menus, sheets, the
 * confirm card), apart from the sentences code composes from the records
 * (`bot-words.ts`, which re-exports these).
 */
import { clock, dayWords } from "./time-words";
import { THIS_MACHINE_IN_TEXT } from "./machine-words";

/** The fixed answers (ADR-0034 §6.1, §6.6). */
export const OUT_OF_SCOPE = "I answer questions about what’s happening in AI Juggler. For coding, ask your agent.";
export const HELP = "I look things up in AI Juggler’s own records: your sessions, tasks, pools and usage. Ask what needs you, what finished, what’s running or how much room an account has.";

export const WORDS = {
  title: "Bot",
  iconTip: "Bot",
  placeholder: "Ask about your sessions, tasks, pools or usage…",
  placeholderAbout: (name: string) => `Ask about ${name}…`,
  askLabel: "Ask",
  plusLabel: "More ways to ask",
  newTopic: "New topic",
  newTopicTip: "Bot stops using what came before for follow-ups. Nothing is deleted.",
  topicDivider: (at: number) => `New topic · ${clock(at)}`,
  topicNote: "Follow-ups start fresh from here. Nothing above is deleted.",
  searchLabel: "Search the conversation",
  expandLabel: "Expand to the Bot page",
  moreLabel: "More",
  closeLabel: "Close",
  aboutChip: "About:",
  widen: "Ask about everything",
  looked: "Looked at",
  goTo: "Go to it",
  watch: "Watch…",
  earlier: (since: number | null) => (since ? `Earlier, since ${new Date(since).toLocaleDateString([], { day: "numeric", month: "short" })}` : "Earlier"),
  earlierPlain: "Earlier",
  askedBefore: (at: number, now: number) => `You asked this ${dayWords(at, now) === "Yesterday" ? "yesterday" : `on ${dayWords(at, now)}`}. Here’s how it stands now:`,
  then: "then",
  now: "now",
  gone: "No longer here",
  more: (n: number) => `and ${n} more`,
  footnote: `Your questions stay on ${THIS_MACHINE_IN_TEXT}.`,
  footnoteNoModel: `Look-ups need no AI. Your questions stay on ${THIS_MACHINE_IN_TEXT}.`,
  // The empty state: what Bot does today (D-136 point 1).
  eyebrow: "Bot",
  emptyTitle: "Ask about your work",
  emptyLead: "One conversation with AI Juggler’s Bot. It looks things up in AI Juggler’s own records and links to each one.",
  can: [
    { mark: "yes", text: "Answers questions about your sessions, tasks, pools and usage" },
    { mark: "yes", text: "Sets a watch, after you confirm it" },
    { mark: "no", text: "Doesn’t talk to your agents or change anything else" },
    { mark: "later", text: "Bot will do more in a later version." },
  ] as const,
  tryOne: "Try one",
  examples: ["What needs me?", "What finished today?", "What’s blocked?"] as const,
  betaRowTitle: "Written answers",
  betaRowLead: (model: string) => `A short answer above the records, written by ${model} on ${THIS_MACHINE_IN_TEXT}.`,
  turnOn: "Turn on",
  offAgain: "Off again any time",
  // No model.
  noModelTitle: "Look things up",
  noModelLead: "Ask about your sessions, tasks, pools or usage. Bot finds the records and links to each one. It doesn’t talk to your agents.",
  goStraight: "Or go straight to",
  watchSession: "Watch a session…",
  offerTitle: "Get short written answers",
  offerLead: (size: string) => `A small AI file (${size}) lets Bot answer in a sentence or two, with links to its records. It stays on ${THIS_MACHINE_IN_TEXT}, and nothing you ask leaves it.`,
  offerGet: (size: string) => `Download · ${size}`,
  notNow: "Not now",
  offerFoot: "Downloads once.",
  offerChoose: "Choose another in Settings › AI › Models",
  // The menus.
  remember: "Remember this…",
  rememberHint: "Your words, kept as a name or a note",
  watchHint: "Get told when it finishes or asks",
  whatBotRemembers: "What Bot remembers",
  openPage: "Open the Bot page",
  betaSwitch: "Written answers · Beta",
  betaOff: "Off: you get the records only",
  betaOn: "On: a short answer above the records",
  clear: "Clear the conversation…",
  clearHint: `Deletes it from ${THIS_MACHINE_IN_TEXT}`,
  settingsAi: "Settings › AI",
  // The written answer (Beta).
  writtenLabel: "Written by AI · Beta",
  checked: (n: number) => `Checked against the ${n === 1 ? "record" : `${n} records`} below`,
  howMade: "How this was made",
  madeBy: (model: string) => `Written by ${model}, on ${THIS_MACHINE_IN_TEXT}. Nothing was sent anywhere.`,
  madeCheck: (n: number) => `Every name, number and state in it was found in ${n === 1 ? "record 1" : `records 1–${n}`}. An answer that fails this check is never shown; you get the records instead.`,
  madeBeta: "Beta: tested on sample work, not on real messages yet.",
  wrong: "This is wrong",
  wrongSaid: `Thanks: noted on ${THIS_MACHINE_IN_TEXT}.`,
  recordsOnly: "Show records only",
  wrongKept: `“This is wrong” is kept on ${THIS_MACHINE_IN_TEXT} only.`,
  copy: "Copy",
  // The clear sheet (its own backup words: CLEAR_AND_BACKUPS).
  clearTitle: "Clear the whole conversation?",
  clearBody: (n: number, since: number | null) =>
    `Deletes every question and answer from ${THIS_MACHINE_IN_TEXT}${n > 0 ? `: ${n} ${n === 1 ? "question" : "questions"}${since ? ` since ${new Date(since).toLocaleDateString([], { day: "numeric", month: "short" })}` : ""}` : ""}. You can’t undo this.`,
  clearStays: (notes: number, watches: number) => `Your ${notes} ${notes === 1 ? "note" : "notes"} and ${watches} ${watches === 1 ? "watch" : "watches"} stay.`,
  clearStaysLink: "What Bot remembers",
  clearButton: "Clear conversation",
  cancel: "Cancel",
  // Find.
  findPlaceholder: "Find in the conversation",
  findCount: (at: number, n: number) => (n === 0 ? "No matches" : `${at} of ${n}`),
  findUp: "Previous match",
  findDown: "Next match",
  done: "Done",
  // The page.
  pageLead: `One conversation · on ${THIS_MACHINE_IN_TEXT} only`,
  tabConversation: "Conversation",
  tabMemory: "What Bot remembers",
  backToPanel: "Back to the panel",
  watchesBotSet: "Watches",
  watchesLink: "Watches",
  all: (n: number) => `All ${n}`,
  // The confirm card (D-136 point 5).
  confirmEyebrow: "Confirm · New watch",
  tellMeWhen: "Tell me when it",
  watchButton: "Watch",
  change: "Change…",
  actsOnly: "Bot acts only through a card like this, after you confirm.",
  watchSet: (words: string) => `Watching: ${words}`,
  outOfScope: OUT_OF_SCOPE,
  help: HELP,
  asking: "Looking it up…",
  answered: (said: string) => `Bot: ${said}`,
};

