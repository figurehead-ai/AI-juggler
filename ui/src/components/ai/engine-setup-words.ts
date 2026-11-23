/**
 * AJ-E3823 — every word the engine-setup guide shows ("Set up Bot", the
 * Meet Bot step, the top of Settings › AI › Models). Plain words for a
 * newcomer who knows nothing: "the small AI that writes Bot's answers",
 * never a bare "model"; each AI named, what it costs, where it lives and
 * what to click. Machine and system names come from the one seam
 * (`machine-words.ts`), so each OS sets them once.
 */
import { MACHINE_WORD, SYSTEM_NAME, THIS_MACHINE, THIS_MACHINE_IN_TEXT } from "../../lib/machine-words";
import { gb } from "../../screens/settings/ai/models-words";
import type { AppleSetup } from "./engine-setup-view";

/** Apple's AI, by the name the guide gives it. */
const APPLE_NAME = "Apple’s built-in AI";

export const SETUP = {
  eyebrow: "Bot",
  title: "Set up Bot",
  lead: `Bot is your AI assistant for everything AI Juggler sees. It needs a small AI that writes its answers, here on ${THIS_MACHINE_IN_TEXT}.`,
  checking: `Checking what ${THIS_MACHINE_IN_TEXT} can do…`,
  choose: "Choose the small AI that writes Bot’s answers",
  recommended: "Recommended",
  inUse: "In use",
  checkAgain: "Check again",
  checkingAgain: "Checking…",
  skip: "Skip for now",
  skipLine: "Bot stays set up for later: its icon opens this guide.",
};

/** The recommendation line above the two choices. */
export const RECOMMEND = {
  ready: `Recommended: ${APPLE_NAME}. It’s ready, with nothing to download.`,
  off: "Recommended: Turn on Apple Intelligence (free, nothing to download).",
  preparing: `Recommended: ${APPLE_NAME}. ${SYSTEM_NAME} is still downloading it, so there’s nothing for you to download.`,
  download: (name: string) => `Recommended: ${name}. ${APPLE_NAME} can’t run on ${THIS_MACHINE_IN_TEXT}.`,
  unknown: (name: string | null) =>
    `AI Juggler couldn’t check ${APPLE_NAME} yet, so nothing is recommended. Check again first${name ? `; ${name} is the other choice` : ""}.`,
};

/** Apple's card: its state in plain words, and what to do. */
export const APPLE = {
  name: APPLE_NAME,
  sub: `Made by Apple, part of ${SYSTEM_NAME}`,
  points: [`Private: runs on ${THIS_MACHINE_IN_TEXT}`, "Fast", "Needs Apple Intelligence on"],
  state: {
    ready: "Ready",
    off: "Apple Intelligence is turned off",
    preparing: `${SYSTEM_NAME} is still downloading it`,
    cant_run: `${THIS_MACHINE} can’t run it`,
    unchecked: "Couldn’t check",
    no_answer: "Couldn’t check: no answer",
    blocked: `AI Juggler’s helper was blocked by ${SYSTEM_NAME}`,
  } satisfies Record<AppleSetup, string>,
  adopt: "Use Apple’s AI",
  adoptLine: `Bot’s answers are written on ${THIS_MACHINE_IN_TEXT}, with nothing to download.`,
  open: "Open Apple Intelligence settings",
  steps: [
    "Click Open Apple Intelligence settings.",
    "Turn on Apple Intelligence.",
    `Wait while ${SYSTEM_NAME} gets it ready, then come back and click Check again.`,
  ],
  stepsLabel: "How to turn it on",
  preparingLine: `It’s ready by itself once ${SYSTEM_NAME} has finished. Keep your ${MACHINE_WORD} on power and online, then check again.`,
  cantRunLine: `It needs Apple Intelligence, which ${THIS_MACHINE_IN_TEXT}’s chip or ${SYSTEM_NAME} version doesn’t offer.`,
  uncheckedLine: `AI Juggler couldn’t tell whether ${THIS_MACHINE_IN_TEXT} can run it. Check again; if it keeps happening, install AI Juggler again.`,
  noAnswerLine: `AI Juggler’s helper for it didn’t answer. If ${SYSTEM_NAME} is showing a message about AI Juggler, answer it, then check again.`,
  blockedLine: `${SYSTEM_NAME} stopped AI Juggler’s helper before it could check. If ${SYSTEM_NAME} asked about AI Juggler, allow it, then check again. If it keeps happening, install AI Juggler again.`,
};

/** The downloadable AI's card. */
export const DOWNLOADED = {
  sub: (maker: string | null, size: number) => `A small AI${maker ? ` from ${maker}` : ""}, ${gb(size)}, runs only on ${THIS_MACHINE_IN_TEXT}`,
  points: ["Works without Apple Intelligence", `Private: nothing you ask leaves ${THIS_MACHINE_IN_TEXT}`, "Remove it any time"],
  notHere: (free: number) => `Not downloaded yet · ${gb(free)} free on ${THIS_MACHINE_IN_TEXT}`,
  here: `Downloaded, on ${THIS_MACHINE_IN_TEXT}`,
  downloadLine: "Downloads once; Bot’s answers turn on when it’s ready.",
  adopt: (name: string) => `Use ${name}`,
  adoptLine: `It’s already on ${THIS_MACHINE_IN_TEXT}: nothing more to download.`,
  short: (size: number, needed: number, free: number) =>
    `Needs ${gb(needed)} free first: its ${gb(size)} plus 10 GB kept for your work. ${THIS_MACHINE} has ${gb(free)} free.`,
  openStorage: "Open Storage settings",
};

/** Where its files are published, by host. */
const HOSTS: Record<string, string> = { "huggingface.co": "Hugging Face" };

/** The maker and the place a download comes from, read from its
 * licence's address (`https://<host>/<publisher>/…`, the list's own
 * data, never a name in code): `null` when it can't be told. */
export function originOf(licenceUrl: string | null): { maker: string | null; host: string | null } {
  if (!licenceUrl) return { maker: null, host: null };
  try {
    const url = new URL(licenceUrl);
    const org = url.pathname.split("/").filter(Boolean)[0] ?? null;
    return { maker: org ? `the ${org} team` : null, host: HOSTS[url.hostname] ?? url.hostname };
  } catch {
    return { maker: null, host: null };
  }
}

/** The "What is this?" disclosure. */
export const WHAT = {
  summary: "What is this?",
  what: `Bot’s answers are written by a small AI (a language model) that runs on ${THIS_MACHINE_IN_TEXT}. It reads the records Bot finds and writes a short answer that links to each one.`,
  kept: (name: string | null, size: number | null) =>
    `${APPLE_NAME} is part of ${SYSTEM_NAME}.${name && size ? ` ${name} is one ${gb(size)} file in AI Juggler’s own folder on ${THIS_MACHINE_IN_TEXT}.` : ""}`,
  from: (name: string | null, maker: string | null, host: string | null) =>
    `Apple’s comes with ${SYSTEM_NAME}.${name ? ` ${name} is made by ${maker ?? "its publisher"} and downloaded${host ? ` from ${host}` : ""}: the exact file we tested, checked before it’s used.` : ""}`,
  private: `Nothing you ask leaves ${THIS_MACHINE_IN_TEXT}: neither AI sends your questions anywhere.`,
  remove: (name: string | null) =>
    `${name ? `Remove ${name} any time in Settings › AI › Models. ` : ""}${APPLE_NAME} goes when you turn Apple Intelligence off.`,
};

/** No AI can run here: one line on what it needs. */
export const NO_AI = {
  title: "Bot can’t run here yet",
  not_apple_silicon: `Bot’s AI needs a ${MACHINE_WORD} with Apple silicon.`,
  not_enough_memory: (memory: string) => `Bot’s AI needs more memory than ${THIS_MACHINE_IN_TEXT}’s ${memory}.`,
  other: `No AI that writes Bot’s answers fits ${THIS_MACHINE_IN_TEXT} yet.`,
};
