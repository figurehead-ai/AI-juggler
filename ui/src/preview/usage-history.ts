/**
 * The preview's usage history (no backend, the visual harness): 120 days
 * of synthetic usage on the preview's profiles, with a weekly rhythm
 * (quiet weekends), heavy days, a few sessions a day per profile, and
 * limit windows read about every 15 minutes while the app is open. It is
 * deterministic for a given clock, so captures don't move.
 */
import { accountsList, projectsList, type AccountInfo, type ProjectInfo, type QuotaDisplayData } from "../lib/bridge";
import { conversationsList, type ConversationInfo } from "../lib/ipc/conversations";
import type { UsageHistoryInfo } from "../types/generated/UsageHistoryInfo";
import type { UsageLimitsInfo } from "../types/generated/UsageLimitsInfo";
import type { UsageQueryInput } from "../types/generated/UsageQueryInput";
import type { UsageSessionDetailInfo } from "../types/generated/UsageSessionDetailInfo";
import type { UsageSessionInfo } from "../types/generated/UsageSessionInfo";
import { pageOf, type Page, type PageRequest } from "../lib/paged";
import type { UsageWindowHistoryInfo } from "../types/generated/UsageWindowHistoryInfo";
import { fold, priceOf, sessionKey, startedPerBucket, type Names, type Row } from "./usage-fold";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const DAYS = 120;

const TITLES = [
  "Refactor the billing module", "Move sign-in to OAuth", "Invoice export for Acme", "Rail grouping by app", "Fix the flaky checkout test",
  "Blog redesign", "Retry failed uploads", "Release notes draft", "Speed up the search index", "Dark mode for settings",
  "Migrate the jobs queue", "Audit the API keys page", "Cache the pricing table", "Split the usage queries", "Onboarding copy pass",
];

/** A small deterministic generator. */
function random(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    return state / 4_294_967_296;
  };
}

function hash(text: string): number {
  let h = 2_166_136_261;
  for (let i = 0; i < text.length; i += 1) h = Math.imul(h ^ text.charCodeAt(i), 16_777_619);
  return h >>> 0;
}

type World = { rows: Row[]; names: Names; starts: Map<string, number>; profiles: AccountInfo[]; now: number };
let world: Promise<World> | null = null;
let worldDay = -1;

function build(profiles: AccountInfo[], projects: ProjectInfo[], conversations: ConversationInfo[], now: number): World {
  const names: Names = { profiles: new Map(), projects: new Map(projects.map((p) => [p.id, p.name])), sessions: new Map() };
  const rows: Row[] = [];
  const starts = new Map<string, number>();
  const end = now - (now % HOUR);
  const firstTool = profiles[0]?.provider;
  profiles.forEach((profile, index) => {
    names.profiles.set(profile.id, { label: profile.label, tool: profile.provider });
    const rnd = random(hash(profile.id));
    const weight = [1.4, 0.8, 0.6, 0.45, 0.7, 0.35, 0.3][index % 7]!;
    const own = conversations.filter((c) => c.account_id === profile.id);
    const models = profile.provider === firstTool ? ["claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5"] : ["gpt-5-codex"];
    for (let d = DAYS; d >= 0; d -= 1) {
      const dayStart = new Date(end - d * DAY);
      dayStart.setHours(0, 0, 0, 0);
      const weekday = (dayStart.getDay() + 6) % 7;
      const weekend = weekday >= 5;
      const heavy = (d + index) % 9 === 0 ? 1.9 : 1;
      const sessions = weekend ? (rnd() < 0.35 ? 1 : 0) : 2 + Math.floor(rnd() * 3);
      for (let s = 0; s < sessions; s += 1) {
        const linked = d < 3 && s === 0 ? own[index % Math.max(1, own.length)] : undefined;
        const id = linked?.id ?? `${profile.id}-${d}-${s}`;
        const key = sessionKey(profile.id, id);
        // Each profile works on a few projects of its own, mostly the first.
        const mine = [index, index + 1, index + 3].map((at) => projects[at % Math.max(1, projects.length)]?.id ?? null);
        const project = linked?.project_id ?? mine[rnd() < 0.6 ? 0 : rnd() < 0.7 ? 1 : 2] ?? null;
        const begin = dayStart.getTime() + (weekend ? 11 : 9 + Math.floor(rnd() * 8)) * HOUR;
        starts.set(key, begin);
        names.sessions.set(key, { project, tool: profile.provider, title: linked?.title ?? TITLES[hash(id) % TITLES.length]!, juggler: linked?.session_id ?? null, started: begin });
        const length = 1 + Math.floor(rnd() * (weekend ? 2 : 4));
        for (let h = 0; h < length; h += 1) {
          const hour = begin + h * HOUR;
          if (hour > end) break;
          const io = Math.round(weight * heavy * (weekend ? 0.5 : 1) * (380_000 + rnd() * 1_400_000));
          const model = models[(s + h) % models.length]!;
          rows.push({ profile: profile.id, hour, session: id, model, input: Math.round(io * 0.82), output: Math.round(io * 0.18), cache: io * 11, messages: 20 + Math.floor(rnd() * 40) });
        }
      }
    }
  });
  return { rows, names, starts, profiles, now };
}

function load(): Promise<World> {
  const now = Date.now();
  const day = Math.floor(now / DAY);
  if (!world || day !== worldDay) {
    worldDay = day;
    world = Promise.all([accountsList(), projectsList(), conversationsList()]).then(([accounts, projects, conversations]) => build(accounts, projects, conversations, now));
  }
  return world;
}

function scoped(w: World, input: Pick<UsageQueryInput, "profile" | "project">): Row[] {
  return w.rows.filter(
    (row) =>
      (input.profile === null || row.profile === input.profile) &&
      (input.project === null || w.names.sessions.get(sessionKey(row.profile, row.session))?.project === input.project),
  );
}

export async function previewHistory(input: UsageQueryInput): Promise<UsageHistoryInfo> {
  const w = await load();
  const rows = scoped(w, input);
  const edges = input.edges_ms;
  const span = edges[edges.length - 1]! - edges[0]!;
  const now = fold(rows, edges, input.group_by, w.names);
  const before = fold(rows, [edges[0]! - span, edges[0]!], "profile", w.names);
  const inScope = new Set(rows.map((row) => sessionKey(row.profile, row.session)));
  const starts = [...w.starts].filter(([key]) => inScope.has(key)).map(([, at]) => at);
  return {
    edges_ms: edges,
    group_by: input.group_by,
    series: now.series,
    totals: now.totals,
    previous: before.totals,
    by_profile: now.by_profile,
    by_tool: now.by_tool,
    by_model: now.by_model,
    by_project: now.by_project,
    top_sessions: now.sessions.slice(0, Math.min(50, input.top ?? 10)),
    session_count: now.sessions.length,
    sessions_per_bucket: now.sessions_per_bucket,
    started_per_bucket: startedPerBucket(edges, starts),
    active_hours: now.active_hours,
    switches: [],
    unmeasured_sessions: input.project === null && input.profile === null ? 3 : 0,
    truncated: false,
    reading: null,
  };
}

export async function previewSessionsPage(input: UsageQueryInput, page: PageRequest): Promise<Page<UsageSessionInfo>> {
  const w = await load();
  return pageOf(fold(scoped(w, input), input.edges_ms, input.group_by, w.names).sessions, page);
}

export async function previewFirst(profile: string | null): Promise<number | null> {
  const w = await load();
  const mine = w.rows.filter((row) => profile === null || row.profile === profile);
  return mine.length ? Math.min(...mine.map((row) => row.hour)) : null;
}

/** Readings every 15 minutes while the app is open (working hours), gaps
 * while it's closed; a weekly window that climbs and resets each Monday. */
function windows(w: World, profile: string, start: number, end: number): { five: UsageWindowHistoryInfo; week: UsageWindowHistoryInfo } {
  const mine = w.rows.filter((row) => row.profile === profile);
  const index = w.profiles.findIndex((p) => p.id === profile);
  // The readings agree with the seeded displays: a profile they show as
  // never measured has no history, and the newest points end where the
  // gauges say. Without seeded displays, the third profile is the
  // never-measured one.
  const seeded = (window as unknown as { __AIJ_QUOTA_DISPLAY__?: QuotaDisplayData[] }).__AIJ_QUOTA_DISPLAY__;
  const shown = seeded?.find((entry) => entry.account === profile);
  const never = seeded ? !shown || (shown.five_hour?.percentage == null && shown.weekly?.percentage == null) : index === 2;
  if (never || mine.length === 0) return { five: { points: [], windows: [], samples: 0 }, week: { points: [], windows: [], samples: 0 } };
  const byHour = new Map<number, number>();
  for (const row of mine) byHour.set(row.hour, (byHour.get(row.hour) ?? 0) + row.input + row.output);
  const scale = index === 0 ? 1.7e-5 : 1.2e-5;
  const five: [number, number][] = [];
  const week: [number, number][] = [];
  const peaks: { first_ms: number; resets_ms: number | null; peak_pct: number }[] = [];
  let windowStart = -1;
  let fiveUsed = 0;
  let weekUsed = 0;
  let weekStart = -1;
  for (let t = start - (start % (15 * 60_000)); t < end; t += 15 * 60_000) {
    const date = new Date(t);
    const open = date.getHours() >= 8 && date.getHours() <= 22;
    const monday = new Date(t);
    monday.setHours(0, 0, 0, 0);
    monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
    if (monday.getTime() !== weekStart) {
      weekStart = monday.getTime();
      weekUsed = 0;
    }
    const used = (byHour.get(t - (t % HOUR)) ?? 0) / 4;
    if (used > 0 && (windowStart < 0 || t >= windowStart + 5 * HOUR)) {
      windowStart = t;
      fiveUsed = 0;
      peaks.push({ first_ms: t, resets_ms: t + 5 * HOUR, peak_pct: 0 });
    }
    fiveUsed = Math.min(100, fiveUsed + used * scale);
    weekUsed = Math.min(100, weekUsed + used * scale * 0.14);
    if (peaks.length) peaks[peaks.length - 1]!.peak_pct = Math.max(peaks[peaks.length - 1]!.peak_pct, Math.round(fiveUsed));
    if (open) {
      five.push([t, Math.round(fiveUsed * 10) / 10]);
      week.push([t, Math.round(weekUsed * 10) / 10]);
    }
  }
  // End where the seeded gauges say.
  const lastWeek = week[week.length - 1]?.[1] ?? 0;
  const wantWeek = shown?.weekly?.percentage;
  if (wantWeek != null && lastWeek > 0) for (const point of week) point[1] = Math.min(100, Math.round((point[1] * wantWeek) / lastWeek));
  const lastPeak = peaks[peaks.length - 1]?.peak_pct ?? 0;
  const wantFive = shown?.five_hour?.state === "reset" ? null : shown?.five_hour?.percentage;
  if (wantFive != null && lastPeak > 0) for (const peak of peaks) peak.peak_pct = Math.min(100, Math.round((peak.peak_pct * wantFive) / lastPeak));
  const thin = (points: [number, number][]) => (points.length <= 400 ? points : points.filter((_, i) => i % Math.ceil(points.length / 400) === 0));
  return {
    five: { points: thin(five), windows: peaks.filter((p) => p.first_ms >= start), samples: five.length },
    week: { points: thin(week), windows: [], samples: week.length },
  };
}

export async function previewLimits(profile: string, start: number, end: number): Promise<UsageLimitsInfo> {
  const w = await load();
  const { five, week } = windows(w, profile, start, end);
  return { profile, five_hour: five, weekly: week };
}

export async function previewSession(profile: string, conversation: string): Promise<UsageSessionDetailInfo> {
  const w = await load();
  const rows = w.rows.filter((row) => row.profile === profile && row.session === conversation);
  const rnd = random(hash(conversation));
  const turns = rows.flatMap((row) =>
    Array.from({ length: Math.min(row.messages, 30) }, (_, k) => {
      const share = (0.4 + rnd()) / Math.min(row.messages, 30);
      const tokens = { input: Math.round(row.input * share), output: Math.round(row.output * share), cache_read: Math.round(row.cache * share), cache_write: 0, reasoning: 0 };
      return { at_ms: row.hour + k * 110_000, model: row.model, tokens, usd: priceOf({ model: row.model, input: tokens.input, output: tokens.output, cache: tokens.cache_read }) };
    }),
  );
  const folded = fold(rows, [0, Number.MAX_SAFE_INTEGER], "model", w.names);
  const io = folded.totals.tokens.input + folded.totals.tokens.output;
  const all = w.rows.filter((row) => row.profile === profile && row.hour >= w.now - 7 * DAY).reduce((sum, row) => sum + row.input + row.output, 0);
  return {
    turns,
    truncated: false,
    totals: folded.totals,
    by_model: folded.by_model,
    windows: [
      { window: "five_hour", since_ms: w.now - 5 * HOUR, basis: "rolling", session_io: Math.round(io * 0.2), profile_io: Math.max(Math.round(io * 0.2), Math.round(all / 20)) },
      { window: "weekly", since_ms: w.now - 7 * DAY, basis: "window", session_io: io, profile_io: Math.max(io, all) },
    ],
  };
}
