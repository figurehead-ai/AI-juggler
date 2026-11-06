/**
 * The 0.4 validation plan's L3 "all pairs" (docs/qa/0.4-validation-plan.md
 * §3): a deterministic covering array in which every pair of values of
 * any two parameters appears together in at least one row, with the
 * combinations that can't exist left out (`allowed`). Greedy: each row is
 * the candidate that covers the most pairs still uncovered, candidates
 * tried in a fixed order, so the same parameters always give the same
 * rows. Small parameter sets only (the product is enumerated).
 */

export type Params = Record<string, readonly string[]>;
export type Case<P extends Params> = { [K in keyof P]: P[K][number] };

function product<P extends Params>(params: P): Case<P>[] {
  let rows: Record<string, string>[] = [{}];
  for (const [name, values] of Object.entries(params)) {
    rows = rows.flatMap((row) => values.map((value) => ({ ...row, [name]: value })));
  }
  return rows as Case<P>[];
}

/** Every pair a row covers, as `a=x|b=y` keys. */
function pairsOf(row: Record<string, string>, names: string[]): string[] {
  const out: string[] = [];
  for (let i = 0; i < names.length; i++) for (let j = i + 1; j < names.length; j++) out.push(`${names[i]}=${row[names[i]!]}|${names[j]}=${row[names[j]!]}`);
  return out;
}

export function allPairs<P extends Params>(params: P, allowed: (row: Case<P>) => boolean = () => true): Case<P>[] {
  const names = Object.keys(params);
  const candidates = product(params).filter(allowed);
  // The pairs that some allowed row can cover.
  const uncovered = new Set(candidates.flatMap((row) => pairsOf(row as Record<string, string>, names)));
  const rows: Case<P>[] = [];
  while (uncovered.size > 0) {
    let best: Case<P> | null = null;
    let gain = 0;
    for (const row of candidates) {
      const covers = pairsOf(row as Record<string, string>, names).filter((pair) => uncovered.has(pair)).length;
      if (covers > gain) {
        best = row;
        gain = covers;
      }
    }
    if (!best) break;
    rows.push(best);
    for (const pair of pairsOf(best as Record<string, string>, names)) uncovered.delete(pair);
  }
  return rows;
}
