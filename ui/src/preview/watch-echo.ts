/**
 * AJ-E3816 — the Watches view's stand-in echo, for the preview and the
 * tests only (no backend): the English words of
 * crates/juggler-app/src/features/watches/{echo,words}.rs. It is tested
 * against the backend's own cases (lib/fixtures/watch-echo.json), which
 * the Rust echo is tested against too, so the two can't drift. The app
 * itself always shows the backend's words.
 */
import type { EchoParts } from "../types/generated/EchoParts";
import type { EchoSpan } from "../types/generated/EchoSpan";
import type { Watch } from "../types/generated/Watch";
import type { WatchTarget } from "../types/generated/WatchTarget";

type Phrase = [one: string, many: string];

/** Each condition: its phrase, and with its parameter (`{n}`, `{s}`, `{a}`). */
const CONDITIONS: Record<string, { plain: Phrase; param?: Phrase }> = {
  finishes: { plain: ["finishes", "finish"] },
  ends: { plain: ["ends", "end"] },
  asks_you: { plain: ["asks you", "ask you"], param: ["asks you and waits {n} minutes", "ask you and wait {n} minutes"] },
  fails: { plain: ["fails", "fail"] },
  stuck: { plain: ["looks stuck", "look stuck"], param: ["looks stuck for {n} minutes", "look stuck for {n} minutes"] },
  idle_for: { plain: ["is idle", "are idle"], param: ["is idle for {n} minutes", "are idle for {n} minutes"] },
  hits_limit: { plain: ["hits a usage limit", "hit a usage limit"] },
  runway_below: { plain: ["runs low", "run low"], param: ["runs below {n} %", "run below {n} %"] },
  runway_resets: { plain: ["has room again", "have room again"] },
  leaves_processes: { plain: ["leaves processes running", "leave processes running"] },
  restarted: { plain: ["is restarted after a crash", "are restarted after a crash"] },
  pool_message: { plain: ["gets a message", "get a message"], param: ["gets a message from {a}", "get a message from {a}"] },
  task_changes: { plain: ["has a task moved", "have a task moved"], param: ["has a task moved to {s}", "have a task moved to {s}"] },
  commits: { plain: ["gets a commit", "get a commit"] },
  starts: { plain: ["starts a new session", "start a new session"] },
};

const TASK_STATES: Record<string, string> = { done: "done", blocked: "blocked", in_review: "in review" };
const UNNAMED: Record<WatchTarget["kind"], string> = { session: "a session", app: "an app", project: "a project", pool: "a pool", account: "an account", any: "any session" };
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAY = 86_400_000;

/** The names the echo quotes: a target's (`kind:id`) and an agent's. */
export interface EchoNames {
  target: (target: WatchTarget) => string | null;
  agent: (id: string) => string | null;
}

export interface EchoClock {
  nowMs: number;
  /** Local time minus UTC, hours. */
  offsetHours: number;
}

function quoted(name: string): string {
  // eslint-disable-next-line no-control-regex -- control characters become spaces, as the backend's
  const clean = name.replace(/[“”"]/g, "'").replace(/[\u0000-\u001f\u007f]/g, " ");
  return `“${clean.trim()}”`;
}

export function nextMidnight(clock: EchoClock): number {
  const offset = clock.offsetHours * 3_600_000;
  const local = clock.nowMs + offset;
  return local - (((local % DAY) + DAY) % DAY) - offset + DAY;
}

function hhmm(at: number, offsetHours: number): string {
  const minutes = ((Math.floor((at + offsetHours * 3_600_000) / 60_000) % 1440) + 1440) % 1440;
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

function day(at: number, offsetHours: number): string {
  const d = new Date(at + offsetHours * 3_600_000);
  return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]}`;
}

function lifetimeWords(watch: Watch, clock: EchoClock): string {
  const life = watch.lifetime;
  if (life.kind === "once") return "Once.";
  if (life.kind === "until_ends") return watch.targets.length > 1 ? "Until they end." : "Until it ends.";
  if (life.kind === "always") return "Until you remove it.";
  if (life.atMs === nextMidnight(clock)) return "Until midnight today.";
  const t = hhmm(life.atMs, clock.offsetHours);
  return day(life.atMs, clock.offsetHours) === day(clock.nowMs, clock.offsetHours) ? `Until ${t}.` : `Until ${day(life.atMs, clock.offsetHours)}, ${t}.`;
}

function conditionSpans(condition: Watch["conditions"][number], many: boolean, names: EchoNames): EchoSpan[] {
  const words = CONDITIONS[condition.code];
  if (!words) return [{ text: condition.code.replace(/_/g, " "), name: false }];
  const pick = (phrase: Phrase) => phrase[many ? 1 : 0];
  const param = condition.param;
  if (!param || !words.param) return [{ text: pick(words.plain), name: false }];
  const text = pick(words.param);
  if (param.kind === "minutes" || param.kind === "percent") return [{ text: text.replace("{n}", String(param.value)), name: false }];
  if (param.kind === "task_state") return [{ text: text.replace("{s}", TASK_STATES[param.value] ?? param.value), name: false }];
  const [before, after = ""] = text.split("{a}");
  return [
    { text: before ?? "", name: false },
    { text: quoted(names.agent(param.value) ?? param.value), name: true },
    { text: after, name: false },
  ];
}

function merged(spans: EchoSpan[]): EchoSpan[] {
  const out: EchoSpan[] = [];
  for (const span of spans) {
    if (span.text === "") continue;
    const last = out[out.length - 1];
    if (last && !last.name && !span.name) last.text += span.text;
    else out.push({ ...span });
  }
  return out;
}

/** The watch in its parts, as the backend writes them. */
export function echoParts(watch: Watch, names: EchoNames, clock: EchoClock): EchoParts {
  const all = watch.matching === "all" && watch.targets.length > 1;
  const head: EchoSpan[] = [{ text: "Tell you when ", name: false }];
  if (all) head.push({ text: "all of ", name: false });
  watch.targets.forEach((target, i) => {
    if (i > 0) head.push({ text: i + 1 === watch.targets.length ? (all ? " and " : " or ") : ", ", name: false });
    const name = target.kind === "any" ? null : names.target(target);
    head.push(name === null ? { text: UNNAMED[target.kind], name: false } : { text: quoted(name), name: true });
  });
  head.push({ text: " ", name: false });
  watch.conditions.forEach((condition, i) => {
    if (i > 0) head.push({ text: ", or ", name: false });
    head.push(...conditionSpans(condition, all, names));
  });
  head.push({ text: ".", name: false });
  return { head: merged(head), lifetime: lifetimeWords(watch, clock), quiet: watch.evenInQuietHours ? "Even during quiet hours." : null };
}

/** The parts joined: the echo, word for word. */
export function joined(parts: EchoParts): string {
  const head = parts.head.map((span) => span.text).join("");
  return [head, parts.lifetime, parts.quiet].filter(Boolean).join(" ");
}
