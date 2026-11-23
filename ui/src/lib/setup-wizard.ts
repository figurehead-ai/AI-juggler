/**
 * AJ-E1545 — the setup wizard's steps, with no drawing: which show, their
 * names, where Next goes and what each says under its name once set.
 * Pure, so each state is a test of data.
 *
 * 1. Find your apps (first run's discovery and import, in place);
 * 2. Work with your Mac: one card per permission this setup needs, shown
 *    only while one is still missing when the wizard opens (D-127);
 * 3. Meet Bot (the maintainer's amendment: after the permissions, before
 *    Keep it running);
 * 4. Keep it running: opening at login and keeping the Mac awake;
 * 5. Take the tour (AJ-E1546).
 */
import { MACHINE_WORD, SYSTEM_NAME } from "./machine-words";

export type SetupStepId = "apps" | "signin" | "access" | "bot" | "keep" | "tour";

/** Each step's name in the steps beside, and its title on the page (Your
 * apps and Sign in are first run's own, which titles them itself). */
export const SETUP_STEP_WORDS: Record<SetupStepId, { label: string; title: string }> = {
  apps: { label: "Your apps", title: "Which AI apps do you use?" },
  signin: { label: "Sign in", title: "Sign in where needed" },
  access: { label: `${MACHINE_WORD} access`, title: `Let AI Juggler work with your ${MACHINE_WORD}` },
  bot: { label: "Meet Bot", title: "Meet Bot" },
  keep: { label: "Keep running", title: "Keep it running" },
  tour: { label: "Take the tour", title: "You're set up" },
};

const ORDER: readonly SetupStepId[] = ["apps", "signin", "access", "bot", "keep"];

/** The steps beside (the step indicator): the permissions only while one
 * is missing (`null`: not read yet, so the step stays and nothing jumps).
 * The tour is offered after them, on its own last page. */
export function setupSteps(missingPermissions: number | null): SetupStepId[] {
  return ORDER.filter((id) => id !== "access" || missingPermissions === null || missingPermissions > 0);
}

/** The pages in order: the steps, then the tour's offer. */
export function pagesOf(steps: readonly SetupStepId[]): SetupStepId[] {
  return [...steps, "tour"];
}

/** "Next: Meet Bot" — Next names where it goes (WIZ-3); the last step's
 * says it finishes. */
export function nextWords(steps: readonly SetupStepId[], at: SetupStepId): string | null {
  const pages = pagesOf(steps);
  const next = pages[pages.indexOf(at) + 1];
  if (!next) return null;
  return next === "tour" ? "Finish setup" : `Next: ${SETUP_STEP_WORDS[next].label}`;
}

/** "Step 2 of 5", the eyebrow over a step's title; the tour's page is past them. */
export function eyebrow(steps: readonly SetupStepId[], at: SetupStepId): string {
  return at === "tour" ? "All steps done" : `Step ${steps.indexOf(at) + 1} of ${steps.length}`;
}

/** Keep it running's value: what it keeps on. */
export function keepValue(atLogin: boolean | null, awake: boolean | null): string {
  const on = [atLogin ? "Opens at login" : null, awake ? "Stays awake" : null].filter(Boolean);
  return on.length > 0 ? on.join(" · ") : `As ${SYSTEM_NAME} does`;
}
