/**
 * AJ-E3812 — every sentence Settings › AI › Models and AI › Jobs show:
 * no internal word, the consequence beside each action (serious choices
 * explained), figures as the list states them. The screens hold no words
 * of their own; the plain-words lint reads this file.
 */
import type { DownloadFailure } from "../../../types/generated/DownloadFailure";
import type { DownloadWait } from "../../../types/generated/DownloadWait";
import type { ModelJob } from "../../../types/generated/ModelJob";
import type { ModelSpeed } from "../../../types/generated/ModelSpeed";
import type { ModelVerdict } from "../../../types/generated/ModelVerdict";
import type { StartRefusal } from "../../../types/generated/StartRefusal";
import { MACHINE_WORD, SYSTEM_NAME, THIS_MACHINE, THIS_MACHINE_IN_TEXT } from "../../../lib/machine-words";

const GB = 1_000_000_000;

/** Bytes in GB as Finder counts them: one decimal under 100 GB. */
export function gb(bytes: number): string {
  const value = bytes / GB;
  if (value >= 100) return `${Math.round(value)} GB`;
  const shown = Math.round(value * 10) / 10;
  return `${shown === Math.trunc(shown) ? shown.toFixed(0) : shown.toFixed(1)} GB`;
}

/** A Mac's memory as Apple sells it (18 GB, not 19.3). */
export function memoryWords(bytes: number | null, wholeGb: number): string {
  return `${bytes ? Math.round(bytes / 2 ** 30) : wholeGb} GB`;
}

/** A chip generation in words. */
export function chipWords(appleSilicon: boolean, chip: number | null): string {
  if (!appleSilicon) return "Intel";
  return chip ? `M${chip}` : "Apple silicon";
}

export const JOB_NAMES: Record<ModelJob, string> = {
  review: "Turn reviews",
  chat: "Bot answers",
  attention: "Attention hints",
  feed: "Feed lines",
  digest: "Digest",
  watches: "Watches",
};

export const VERDICT_WORDS: Record<ModelVerdict, string> = {
  ships: "Works well",
  beta: "Beta",
  not_yet: "Not yet",
  not_tested: "Not tested",
};

export const SPEED_WORDS: Record<ModelSpeed, string> = { quick: "Quick", moderate: "Moderate", slow: "Slow" };

/** A list of names in a sentence: "A", "A and B", "A, B and C". */
export function listWords(names: readonly string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
}

export const PAGE = {
  crumb: "AI",
  title: "Models",
  lead: `The AI that writes AI Juggler's Beta reviews and Bot answers. It runs on ${THIS_MACHINE_IN_TEXT}; nothing you write is sent anywhere.`,
  howWeTest: "How we test models",
  thisMac: THIS_MACHINE,
  free: (bytes: number) => `${gb(bytes)} free`,
  offline: "Offline",
  loading: `Reading ${THIS_MACHINE_IN_TEXT}…`,
};

export const RECOMMENDED = {
  title: `Recommended for ${THIS_MACHINE_IN_TEXT}`,
  help: `One pick, from ${THIS_MACHINE_IN_TEXT}'s chip, memory and free disk`,
  builtIn: `Built into ${SYSTEM_NAME}`,
  reasonBuiltIn: `Apple Intelligence is on, so it's already part of ${THIS_MACHINE_IN_TEXT}: no download, no disk.`,
  reasonInstalled: `It's on ${THIS_MACHINE_IN_TEXT} already and fits it: nothing more to download.`,
  reasonSmallest: (memory: string) => `The smallest model that passed our tests, and it fits ${THIS_MACHINE_IN_TEXT}'s ${memory} of memory.`,
  reasonMost: (memory: string) => `It passed the most of our tests, and it fits ${THIS_MACHINE_IN_TEXT}'s ${memory} of memory.`,
  fitTitle: `Why it fits ${THIS_MACHINE_IN_TEXT}`,
  needsTitle: "What it needs",
  notYet: (count: number) => `${count} not yet`,
  adopt: "Use it",
  adoptLine: (jobs: string) => `Turns on ${jobs} as Beta, on this model. Off again any time below.`,
  downloadLine: (jobs: string) => `Stays on ${THIS_MACHINE_IN_TEXT}. Then ${jobs} start as Beta; off again any time below.`,
  inUse: "In use",
  inUseLine: "Automatic sends every job it passes here.",
  tipAppleLead: `${THIS_MACHINE} can also run`,
  tipAppleTitle: "Apple's on-device model",
  tipAppleRest: ", with no download. Turn on Apple Intelligence in System Settings and it becomes the pick.",
  openSystemSettings: "Open System Settings",
};

export const NOTHING = {
  title: "Nothing to recommend",
  notAppleSilicon: `The tested models need a ${MACHINE_WORD} with Apple silicon. Every job keeps its plain version.`,
  notEnoughMemory: (memory: string) => `The tested models need more memory than ${THIS_MACHINE_IN_TEXT}'s ${memory}. Every job keeps its plain version.`,
  nothingListed: `No tested model fits ${THIS_MACHINE_IN_TEXT} yet. Every job keeps its plain version.`,
  offline: "Nothing to recommend while the list of tested models can't be read. Every job keeps its plain version.",
};

export const FIT = {
  appleAi: "Apple Intelligence",
  on: "On",
  memory: "Memory",
  needsMemory: (mac: string, needed: number) => `${mac} · needs ${needed} GB`,
  download: "Download",
  noDownload: `None: already on ${THIS_MACHINE_IN_TEXT}`,
  chip: "Chip",
  needsChip: (mac: string, needed: number) => `${mac} · needs M${needed} or later`,
  freeDisk: "Free disk",
  needsDisk: (free: number, needed: number) => `${gb(free)} · needs ${gb(needed)}`,
  installed: "Installed",
  installedOn: (size: number, day: string) => `${gb(size)}, ${day}`,
};

export const FACTS = {
  download: "Download",
  none: "None",
  builtIn: `built into ${SYSTEM_NAME}`,
  oneTime: "one time",
  memory: "Memory",
  shared: "Shared",
  withMacos: `with ${SYSTEM_NAME}`,
  memoryFloor: (min: number) => `${min} GB`,
  /** The list's figure is the smallest Mac we list it for, not a measurement. */
  memoryFloorNote: `${MACHINE_WORD} or more`,
  speed: "Speed",
  speedNote: `our test ${MACHINE_WORD}`,
};

export const DOWNLOAD = {
  button: (bytes: number) => `Download · ${gb(bytes)}`,
  downloading: "Downloading",
  done: (done: number, size: number) => `${gb(done)} of ${gb(size)}`,
  left: (minutes: number) => (minutes <= 1 ? "about 1 min left" : `about ${minutes} min left`),
  checking: "Checking the file",
  cancel: "Cancel",
  progressLabel: (name: string) => `Downloading ${name}`,
  how: "Checked before it's used. Pauses by itself in Low Power Mode or on a metered network; Cancel leaves nothing behind.",
  readyOnceRoom: "Ready once there's room. Nothing has been downloaded.",
};

export const WAITS: Record<DownloadWait["kind"], (wait: DownloadWait) => string> = {
  low_power: () => "Paused: Low Power Mode is on. It goes on by itself once it's off.",
  metered: () => "Paused: this network is metered. It goes on by itself on another network.",
  disk_short: (wait) => (wait.kind === "disk_short" ? `Paused: ${THIS_MACHINE_IN_TEXT} has ${gb(wait.free)} free and it needs ${gb(wait.needed)}. Free ${gb(wait.needed - wait.free)} and it goes on.` : ""),
  disk_unknown: () => "Paused: the free disk can't be read. It goes on once it can.",
};

export const FAILURES: Record<DownloadFailure, string> = {
  not_the_tested_file: "Download stopped: the file wasn't the one we tested, so it was deleted.",
  wrong_size: "Download stopped: the file wasn't the size we tested.",
  redirected: "Download stopped: the file came from somewhere we don't download from.",
  disk_full: "Download stopped: the disk filled up.",
  network: "Download stopped: the network kept failing.",
  disk: "Download stopped: the file couldn't be saved.",
};

export const FAILED_AFTER = "Nothing was kept. Download again when you're ready.";

export const DISK = {
  title: "Not enough free disk yet.",
  body: (size: number, needed: number, free: number) =>
    `A download needs its ${gb(size)} plus 10 GB kept free for your work: ${gb(needed)}. ${THIS_MACHINE} has ${gb(free)} free, so free up ${gb(needed - free)} and it's ready.`,
  openStorage: "Open Storage settings",
  checkAgain: "Check again",
  cardNeeds: (needed: number) => `Needs ${gb(needed)} free first`,
};

/** Why a Download click didn't start, when it isn't the disk or the
 * approval (each of those has its own card). */
export const REFUSALS: Partial<Record<StartRefusal, string>> = {
  not_offered: "This model isn't offered any more.",
  nothing_to_fetch: `Apple's model is part of ${SYSTEM_NAME}: nothing to download.`,
  already_installed: `It's on ${THIS_MACHINE_IN_TEXT} already.`,
  busy: "Another download is running. Cancel it or let it finish first.",
  disk_unknown: "The free disk can't be read, so nothing was downloaded.",
};

export const OTHERS = {
  title: "Other tested models",
  help: "Each passed our tests for at least one job. Downloading one doesn't change Automatic.",
  helpFits: `Each passed our tests and fits ${THIS_MACHINE_IN_TEXT}. Smallest first.`,
  helpDisk: "They need more free disk too.",
  stays: `Stays on ${THIS_MACHINE_IN_TEXT}`,
  notYet: (jobs: string) => `Not yet: ${jobs}`,
  licence: (name: string) => `${name}, its licence`,
  smallest: "Smallest",
  fastest: "Fastest",
  installed: "Installed",
  unreadTitle: "Can't read the list of tested models",
  unreadOffline: "You're offline, so only Apple's built-in model shows. Installed models keep working, and the full list comes back when you're online.",
  unreadLater: "Only Apple's built-in model shows until AI Juggler reads the list; it checks once a day. Installed models keep working.",
  unreadNoBuiltIn: "No model shows until AI Juggler reads the list; it checks once a day. Installed models keep working.",
};

/** The honesty line, by the verdicts (the maintainer's fix 3). */
export const HONESTY = "These figures come from our tests on sample work we wrote, not on real messages.";

export const INSTALLED = {
  title: "Installed",
  helpNone: "Nothing downloaded yet",
  helpCount: (count: number) => `${count} downloaded`,
  helpOffline: "works offline",
  helpWithBuiltIn: "plus the built-in one",
  use: "AI models use",
  onThisMac: `on ${THIS_MACHINE_IN_TEXT}`,
  legendNone: "A model you download shows here with its size and Remove. Clear cache never removes it.",
  legendOne: "One downloaded model covers every job it passes. Clear cache never removes it.",
  legendOffline: (name: string) => `${name} keeps working offline. Clear cache never removes it.`,
  neverCleared: "Clear cache never removes these.",
  noModelsTitle: "No models yet",
  noModelsLine: "Every job keeps its plain version until you download one.",
  noDisk: "No disk",
  builtIn: `Built into ${SYSTEM_NAME}`,
  readyToUse: "ready to use",
  usedBy: (jobs: string) => `used by ${jobs}`,
  usedByNothing: "used by nothing now",
  downloaded: (day: string) => `Downloaded ${day}`,
  withdrawn: "We no longer list this model. It keeps working until you remove it.",
  remove: "Remove",
  keep: "Keep it",
  removeSized: (bytes: number) => `Remove · ${gb(bytes)}`,
  confirmTitle: (name: string) => `Remove ${name}?`,
  confirmFrees: (bytes: number) => `Frees ${gb(bytes)}.`,
  confirmBack: (jobs: string, model: string | null) => `${jobs} ${jobs.includes(" and ") ? "go" : "goes"} back to Automatic${model ? ` (${model})` : ""}.`,
  confirmHides: "Jobs only it could do go back to their plain version.",
  confirmAgain: "You can download it again from Other tested models.",
  confirmGone: "You can't download it again: we no longer list it.",
};

export const APPROVAL = {
  title: "Download a second model?",
  sub: (have: string) => `You already have ${have}. Most people need only one.`,
  adds: (jobs: string) => `Passed our tests for ${jobs}.`,
  diskTitle: "Disk used by AI models",
  now: "Now",
  after: "After",
  addsBytes: (size: number, freeAfter: number) => `Adds ${gb(size)}. ${THIS_MACHINE} keeps ${gb(freeAfter)} free afterwards.`,
  staysTitle: `Stays on ${THIS_MACHINE_IN_TEXT}.`,
  stays: "It runs here, and nothing you write is sent anywhere. Remove it any time in Installed.",
  whatChanges: "What changes if I add it?",
  changes: "Automatic keeps the recommended model. Pick this one for a job in Each job uses below, or leave it unused.",
  asked: "Asked for every extra model.",
  notNow: "Not now",
  close: "Close",
};

export const JOBS = {
  title: "Each job uses",
  help: "Automatic follows the recommendation. Pick a model only if you want a different one.",
  paneTitle: "Jobs",
  paneLead: "What each AI job does, whether it's on, and which model it uses.",
  beta: "Beta",
  automatic: "Automatic",
  automaticHint: (model: string | null) => (model ? `Uses the recommended model: ${model}` : "Uses the recommended model"),
  pickerLabel: (job: string) => `Model for ${job}`,
  pickerHeading: (job: string) => `Installed and passed for ${job}`,
  noModelNeeded: "No model needed",
  notYet: "Not yet",
  noModel: "No model yet",
  descriptions: {
    review: "Writes a short review when an agent's turn ends",
    chat: "Writes answers from what Bot looks up",
    attention: "Built from the facts, so it needs no model",
    feed: "Plain lines for now. No model passes yet",
    digest: "Plain roundup for now. No model passes yet",
    watches: "The Watches form for now. No model passes yet",
  } satisfies Record<ModelJob, string>,
  off: (what: string) => `Off. ${what}`,
  on: "On",
  onWith: (model: string) => `On · Beta with ${model}`,
  onPicked: (model: string) => `On · your pick, ${model}`,
  needsModel: "Needs a model: download one above",
  pickHint: (verdict: string, note: string | null) => (note ? `${verdict} · ${note}` : verdict),
  pickGone: "no longer listed",
  switchLabel: (job: string) => `${job} Beta`,
};
