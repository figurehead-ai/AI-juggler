/**
 * The cost table, a page at a time (0.1.5 review: Usage drew every
 * session × model × day row at once). 0.1.7: a row is one session's day,
 * named as the rail names it, its models folded together with each one's
 * part alongside (`commands::CostRowInfo`); the totals stay with
 * `costRollups`.
 */
import { invoke } from "./transport";
import { hasLiveBackend, type ConfidenceWire, type CostRollupRow } from "../bridge";
import { pageOf, type Page, type PageRequest } from "../paged";

/** One row of the cost table: a session's UTC day. */
export interface CostRow {
  /** The tool's id for the session ("" when the usage names none): for
   * the tooltip and the CSV, never shown as the name. */
  session: string;
  /** The user's label, else the conversation's title. */
  title: string | null;
  /** Its project's name. */
  project: string | null;
  /** UTC `YYYY-MM-DD`. */
  day: string;
  tokens: CostRollupRow["tokens"];
  /** USD at list prices of the models that have one; null when none has. */
  usd: number | null;
  credits: number | null;
  confidence: ConfidenceWire;
  /** Each model's part, most tokens first. */
  models: CostRollupRow[];
}

/** A session as the rest of the app names it: its label or title, else
 * "Untitled session" (Usage's Top sessions rule); never its id. */
export function costSessionName(row: Pick<CostRow, "session" | "title">): string {
  if (row.title?.trim()) return row.title.trim();
  return row.session ? "Untitled session" : "No session recorded";
}

/** A rollup row in the design preview's seam, with the names the app
 * would give it. */
type SeededRow = CostRollupRow & { title?: string | null; project?: string | null };

const RANK: Record<string, number> = { estimated: 0, computed: 1, measured: 2 };

/** The design preview's rollup rows as the backend pages them: one row
 * per session and day, newest day first (fixtures only). */
export function sessionDays(rows: SeededRow[]): CostRow[] {
  const byKey = new Map<string, CostRow>();
  for (const row of rows) {
    const day = row.day ?? "";
    const id = `${day}\u001f${row.key}`;
    const at =
      byKey.get(id) ??
      byKey
        .set(id, {
          session: row.key,
          title: row.title ?? null,
          project: row.project ?? null,
          day,
          tokens: { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 },
          usd: null,
          credits: null,
          confidence: row.confidence,
          models: [],
        })
        .get(id)!;
    at.models.push(row);
    for (const k of ["input", "output", "cache_read", "cache_write", "reasoning"] as const) at.tokens[k] += row.tokens[k];
    if (row.usd.usd != null) at.usd = (at.usd ?? 0) + row.usd.usd;
    if (row.credits != null) at.credits = (at.credits ?? 0) + row.credits;
    if ((RANK[row.confidence.toLowerCase()] ?? 0) < (RANK[at.confidence.toLowerCase()] ?? 0)) at.confidence = row.confidence;
  }
  const io = (row: CostRollupRow) => row.tokens.input + row.tokens.output;
  return [...byKey.values()]
    .map((row) => ({ ...row, models: row.models.sort((a, b) => io(b) - io(a)) }))
    .sort((a, b) => (a.day === b.day ? a.session.localeCompare(b.session) : a.day < b.day ? 1 : -1));
}

/** One page of the cost table, newest day first; `account` narrows it to
 * one account. Read it with `usePagedList(costRowsKey(account), …)`. */
export async function costRowsPage(account: string | null, page: PageRequest): Promise<Page<CostRow>> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_COST_ROLLUPS__?: { rows: SeededRow[] } }).__AIJ_COST_ROLLUPS__;
    return pageOf(sessionDays(seeded?.rows ?? []), page);
  }
  return invoke<Page<CostRow>>("cost_rows_page", { account, page });
}

/** The paged list's key for one account's (or every account's) table. */
export function costRowsKey(account: string | null): string {
  return `costs:${account ?? "ALL"}`;
}
