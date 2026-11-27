/**
 * AJ-E3816 — the create and edit form's state, kept apart from how it
 * draws: the choices as the user makes them, the draft they make (the
 * typed shape the backend checks and puts in words), the parameters'
 * choices within the compiled bounds, and the preview's facts.
 */
import type { CatalogueEntry } from "../../types/generated/CatalogueEntry";
import type { Condition } from "../../types/generated/Condition";
import type { EchoParts } from "../../types/generated/EchoParts";
import type { LifetimeChoice } from "../../types/generated/LifetimeChoice";
import type { Matching } from "../../types/generated/Matching";
import type { Param } from "../../types/generated/Param";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import type { PrefillField } from "../../types/generated/PrefillField";
import type { PrefillWhy } from "../../types/generated/PrefillWhy";
import type { WatchPrefill } from "../../types/generated/WatchPrefill";
import { whenWords } from "../../lib/time-words";
import type { PreviewFacts } from "./WatchPreview";
import { optionOf, suggestedLifetime, type TargetOption } from "./watches-view";
import { BOT_FILL, NOUN, PREVIEW } from "./watches-words";

export type LifeKind = "once" | "until_ends" | "today" | "until" | "always";

export interface FormState {
  targets: WatchTarget[];
  matching: Matching;
  /** The picked conditions, in the order picked. */
  codes: string[];
  /** Each picked condition's parameter (absent: none). */
  params: Record<string, Param | undefined>;
  /** The lifetime chosen; null: the suggested one. */
  life: LifeKind | null;
  /** "Until…": the day (YYYY-MM-DD) and time (HH:MM), local. */
  untilDay: string | null;
  untilTime: string | null;
  quiet: boolean;
  /** 0.5.2 G32: the condition is the form's default, not the user's
   * pick: it follows the targets (a project with no session open starts
   * with "A new session starts", an account with its limit) until the
   * user picks one. */
  auto?: boolean;
  /** AJ-E3830 (0.5.2): Bot pre-filled the form: the fields still left
   * for the user (each leaves once the user sets it), the nearest
   * condition offered beside the cards, and the words it couldn't read. */
  bot?: BotFill;
}

export interface BotFill {
  left: PrefillField[];
  why: PrefillWhy;
  nearest: string | null;
  asked: string | null;
  /** 0.5.2 G60: what the request's names match, the target left: the
   * picker opens on them (none is chosen for the user). */
  choices: WatchTarget[];
}

/** Bot's pre-fill as the form's state: its draft, with what it left. */
export function stateFromPrefill(prefill: WatchPrefill, now: number): FormState {
  return { ...stateFrom(prefill.draft, now), bot: { left: [...prefill.left], why: prefill.why, nearest: prefill.nearest ?? null, asked: prefill.asked ?? null, choices: [...(prefill.choices ?? [])] } };
}

/** The change, with each field it sets no longer left by Bot. */
export function touched(was: FormState, change: Partial<FormState>): FormState {
  const next = { ...was, ...change };
  if (!was.bot) return next;
  const set: PrefillField[] = [];
  if (change.targets) set.push("target");
  if (change.codes || change.params) set.push("condition");
  if (change.life !== undefined || change.untilDay !== undefined || change.untilTime !== undefined) set.push("lifetime");
  if (change.quiet !== undefined) set.push("quiet_hours");
  return { ...next, bot: { ...was.bot, left: was.bot.left.filter((f) => !set.includes(f)) } };
}

/** Bot left `field` and the user hasn't set it yet. */
export function botLeft(state: FormState, field: PrefillField): boolean {
  return state.bot?.left.includes(field) ?? false;
}

/** How long is still to be picked: Bot couldn't read it, so no choice is
 * made for the user (Watch waits for one). */
export function lifeAsked(state: FormState): boolean {
  return state.life === null && botLeft(state, "lifetime");
}

/** 0.5.2 G60 (RW1 R31): Bot left what to watch or what for, and how long
 * is unsaid: the form's suggestion follows them (Once for a session that
 * finishes, Today for a project), so none is shown chosen until both are
 * picked; a chosen-looking "Today" read as Bot's fill and changed under
 * the user once they picked the session. The field still to pick, or
 * null when the suggestion stands. */
export function lifeWaits(state: FormState): "target" | "condition" | null {
  if (state.bot === undefined || state.life !== null || lifeAsked(state)) return null;
  if (state.targets.length === 0) return "target";
  return state.codes.length === 0 ? "condition" : null;
}

/** 0.5.2 G32: the default condition for these targets, among what they
 * can all report (`offered`): what the first target's kind is most often
 * watched for, and for an App, a project or any session with none open,
 * a session starting (nothing in it can end a turn or be idle yet). */
export function defaultCode(targets: readonly WatchTarget[], offered: readonly CatalogueEntry[], book: Map<string, TargetOption>): string | null {
  const first = targets[0];
  const option = first ? optionOf(book, first) : null;
  const group = option !== null && (option.kind === "app" || option.kind === "project" || option.kind === "any");
  const order =
    option?.kind === "account" ? ["hits_limit", "runway_below", "runway_resets"] : option?.kind === "pool" ? ["finishes", "pool_message", "asks_you"] : group && (option.openSessions ?? 0) === 0 ? ["starts", "finishes"] : ["finishes", "asks_you"];
  return order.find((code) => offered.some((entry) => entry.code === code)) ?? offered[0]?.code ?? null;
}

/** The conditions the form holds now: what is still on offer for its
 * targets (D-127: one no longer offered leaves, never shown as
 * unsupported), or, until the user picks one, the targets' default
 * (`auto`, 0.5.2 G32) with that card's entry for its parameter. */
export function codesNow(state: FormState, offered: readonly CatalogueEntry[], loading: boolean, book: Map<string, TargetOption>): { codes: string[]; entry: CatalogueEntry | undefined } {
  const ready = !loading && state.targets.length > 0;
  if (state.auto === true) {
    if (!ready) return { codes: state.codes, entry: undefined };
    const code = defaultCode(state.targets, offered, book);
    return { codes: code ? [code] : [], entry: offered.find((e) => e.code === code) };
  }
  return { codes: ready ? state.codes.filter((code) => offered.some((e) => e.code === code)) : state.codes, entry: undefined };
}

/** A parameter's default when its card is picked (the mock's, and none
 * where "right away" is the common choice). */
export function defaultParam(entry: CatalogueEntry): Param | undefined {
  const spec = entry.param;
  if (!spec) return undefined;
  const fit = (n: number) => Math.min(Math.max(n, spec.min ?? n), spec.max ?? n);
  if (spec.kind === "minutes") {
    if (entry.code === "stuck") return { kind: "minutes", value: fit(15) };
    if (entry.code === "idle_for") return { kind: "minutes", value: fit(30) };
    return spec.required ? { kind: "minutes", value: fit(10) } : undefined;
  }
  if (spec.kind === "percent") return { kind: "percent", value: fit(20) };
  if (spec.kind === "task_state") return { kind: "task_state", value: "done" };
  return undefined;
}

const MINUTE_STEPS = [5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 240, 360, 480];
const PERCENT_STEPS = [5, 10, 20, 25, 30, 40, 50, 75, 90, 95];

/** The choices a number parameter offers: steps within its bounds. */
export function paramSteps(entry: CatalogueEntry): number[] {
  const spec = entry.param;
  if (!spec || (spec.kind !== "minutes" && spec.kind !== "percent")) return [];
  const steps = spec.kind === "minutes" ? MINUTE_STEPS : PERCENT_STEPS;
  return steps.filter((n) => n >= Math.max(spec.min ?? 0, 1) && n <= (spec.max ?? n));
}

/** "YYYY-MM-DD" and "HH:MM" for a local time. */
function dayAndTime(at: number): { day: string; time: string } {
  const d = new Date(at);
  const two = (n: number) => String(n).padStart(2, "0");
  return { day: `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`, time: `${two(d.getHours())}:${two(d.getMinutes())}` };
}

/** A local day and time as epoch ms; null when either is missing. */
export function untilMs(day: string | null, time: string | null): number | null {
  if (!day || !time) return null;
  const [y, m, d] = day.split("-").map(Number);
  const [h, min] = time.split(":").map(Number);
  if ([y, m, d, h, min].some((n) => n === undefined || Number.isNaN(n))) return null;
  return new Date(y ?? 0, (m ?? 1) - 1, d ?? 1, h ?? 0, min ?? 0, 0, 0).getTime();
}

/** The lifetime in force: the chosen one, else the suggested. */
export function lifeOf(state: FormState): LifeKind {
  if (state.life === "until_ends" && !state.targets.every((t) => t.kind === "session")) return suggestedLifetime(state.targets, state.codes);
  return state.life ?? suggestedLifetime(state.targets, state.codes);
}

function lifetimeChoice(state: FormState): LifetimeChoice {
  const life = lifeOf(state);
  if (life === "until") {
    const at = untilMs(state.untilDay, state.untilTime);
    return at === null ? { kind: "today" } : { kind: "until", atMs: at };
  }
  return { kind: life };
}

/** The draft the form makes. */
export function draftFrom(state: FormState): WatchDraftInfo {
  const conditions: Condition[] = state.codes.map((code) => {
    const param = state.params[code];
    return param ? { code, param } : { code };
  });
  return { targets: state.targets, matching: state.targets.length > 1 ? state.matching : "each", conditions, lifetime: lifetimeChoice(state), evenInQuietHours: state.quiet };
}

/** A draft as the form's state (editing, a starter, a door). */
export function stateFrom(draft: WatchDraftInfo, now: number): FormState {
  const life = draft.lifetime;
  const until = life.kind === "until" ? dayAndTime(life.atMs) : dayAndTime(now + 3 * 3_600_000 - (now % 3_600_000));
  return {
    targets: [...draft.targets],
    matching: draft.matching,
    codes: draft.conditions.map((c) => c.code),
    params: Object.fromEntries(draft.conditions.map((c) => [c.code, c.param])),
    life: life.kind === "unsaid" ? null : life.kind,
    untilDay: until.day,
    untilTime: until.time,
    quiet: draft.evenInQuietHours,
  };
}

/** The parts joined: the whole echo, as the backend joins it. */
export function joinedWords(parts: EchoParts): string {
  return [parts.head.map((span) => span.text).join(""), parts.lifetime, parts.quiet].filter(Boolean).join(" ");
}

function capitalised(words: string): string {
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** What the preview shows beside the form. */
export function previewFacts(input: { state: FormState; offered: readonly CatalogueEntry[]; echo: EchoParts | null; was: EchoParts | null; book: Map<string, TargetOption>; quiet: { from: string; to: string } | null; now: number }): PreviewFacts {
  const { state, offered } = input;
  const picked = state.codes.map((code) => offered.find((entry) => entry.code === code)).filter((e): e is CatalogueEntry => e !== undefined);
  const first = state.targets[0];
  const option = first ? optionOf(input.book, first) : null;
  const name = option ? (option.kind === "any" ? "A session" : `“${option.name}”`) : null;
  const told = picked[0]?.told ?? null;
  const title = name && told ? `${name} ${told} · your watch` : null;
  const line = option && option.kind !== "any" ? [option.line.split(" · ").slice(0, 2).join(" · "), PREVIEW.openIt].join(" · ") : option ? PREVIEW.openIt : null;
  const calm = picked.filter((e) => !e.need).map((e) => NOUN[e.code] ?? e.label);
  const needs = picked.filter((e) => e.need).map((e) => NOUN[e.code] ?? e.label);
  const shows = [
    ...(calm.length > 0 ? [PREVIEW.calmNote(calm.slice(0, 2).join(" or "))] : []),
    ...(needs.length > 0 ? [PREVIEW.needNote(capitalised(needs.slice(0, 2).join(" or ").toLowerCase()))] : []),
  ];
  const quiet = input.quiet ? (state.quiet ? PREVIEW.quietWakes(input.quiet.from, input.quiet.to) : PREVIEW.quietWaits(input.quiet.from, input.quiet.to)) : PREVIEW.noQuiet;
  const life = lifeOf(state);
  const at = life === "until" ? untilMs(state.untilDay, state.untilTime) : null;
  // AJ-E2314 L4: Always said "Ends at midnight tonight." (no "Until…" time
  // read as no time at all); only an "Until…" without one ends tonight.
  const ends =
    life === "once" ? PREVIEW.onceEnds : life === "until_ends" ? PREVIEW.untilEnds(state.targets.length) : life === "until" && at !== null ? PREVIEW.untilEndsAt(whenWords(at, input.now)) : life === "today" || life === "until" ? PREVIEW.todayEnds : PREVIEW.alwaysEnds;
  // AJ-E3830: how long is the user's to pick (Bot couldn't read it): the
  // preview says no lifetime for them.
  if (lifeAsked(state)) return { echo: input.echo ? { ...input.echo, lifetime: "" } : null, was: input.was, title, line, shows, quiet, quietWakes: state.quiet, ends: BOT_FILL.lifePick, endsKind: life };
  const waits = lifeWaits(state);
  if (waits) return { echo: input.echo, was: input.was, title, line, shows, quiet, quietWakes: state.quiet, ends: BOT_FILL.lifeFollows(waits), endsKind: life };
  return { echo: input.echo, was: input.was, title, line, shows, quiet, quietWakes: state.quiet, ends, endsKind: life };
}
