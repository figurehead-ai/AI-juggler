/**
 * D-62 (AJ-E3008 req. 3, AJ-E3006): the palette lists every candidate,
 * not only the open sessions that can join: sessions other apps run
 * (Watched until their app has AI Juggler's connector), Apps, Accounts,
 * Tools, Macs and everyone in scope (their sessions join by
 * themselves), and the sessions that can't join
 * yet, each with its reason and the step that would let it in (D-126: in
 * muted words; a reason is a fact, never amber).
 */
import { useState, type JSX } from "react";

import { Picker } from "../../components/controls";
import { MACHINES_WORD } from "../../lib/machine-words";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { poolJoin } from "../../lib/ipc/pool-model";
import { DesktopListNote, DesktopReachTag } from "./connector/DesktopReach";
import { usePendingAdds, withPendingAdd } from "../../lib/pool-pending";
import { REACH_WORDS, type Candidate } from "../../lib/new-pool";
import { setPools } from "../../lib/pools";
import { keep, keepInPlace, type Kept } from "../../lib/in-place";

export type MoreKind = "watched" | "app" | "account" | "tool" | "mac" | "everyone" | "blocked";

const WORDS: Record<MoreKind, string> = {
  watched: "Sessions in other apps",
  app: "Apps",
  account: "Accounts",
  tool: "Tools",
  mac: MACHINES_WORD,
  everyone: "Everyone",
  blocked: "Can't join yet",
};

export function PaletteMore({
  poolId,
  lists,
  onFix,
  onAdded,
}: {
  poolId: string;
  lists: Record<MoreKind, Candidate[]>;
  onFix: (row: Candidate) => void;
  onAdded: () => void;
}): JSX.Element {
  const kinds = (Object.keys(WORDS) as MoreKind[]).filter((kind) => kind === "everyone" || lists[kind].length > 0);
  const [kind, setKind] = useState<MoreKind>(kinds[0] ?? "everyone");
  const adding = new Set(usePendingAdds(poolId).map((one) => one.ref));
  // Round 5: an added row stays where it was, marked added (no jump).
  const [kept, setKept] = useState<ReadonlyMap<string, Kept<Candidate>>>(new Map());
  const rows = keepInPlace((lists[kind] ?? []).slice(0, 30), kept, (row) => row.ref);
  const add = async (row: Candidate) => {
    try {
      // Round 4 (#5): its card shows at once, pending, until the pool says so.
      setPools(await withPendingAdd({ poolId, ref: row.ref, label: row.label, kind: row.kind }, () => poolJoin(poolId, row.ref, "member")));
      onAdded();
      showToast({ message: `${row.label} is in the pool`, detail: row.reach === "watched" ? "It talks here once its app has AI Juggler's connector: Add is on its card." : row.note ?? (row.reach === "later" ? "Its sessions here join by themselves, now and later." : undefined), testId: "canvas-added" });
    } catch (cause) {
      showToast({ message: `Couldn't add ${row.label}`, detail: errorText(cause), tone: "warning" });
    }
  };
  return (
    <div className="flex flex-col gap-1.5 border-t border-hairline px-2 pb-2 pt-2" data-testid="canvas-palette-more">
      <Picker
        choice
        size="sm"
        // Its name begins with what it shows ("More: Apps 2"), WCAG 2.5.3.
        ariaLabel="More"
        prefix="More:"
        value={kind}
        options={kinds.map((one) => ({ value: one, label: `${WORDS[one]} ${one === "everyone" ? "" : lists[one].length}`.trim() }))}
        onChange={(next) => {
          setKept(new Map());
          setKind(next);
        }}
        testId="canvas-palette-kind"
      />
      {/* Round 7 #9: how a desktop session takes part, once for the list. */}
      <DesktopListNote notes={rows.map(({ row }) => (row.blocked ? undefined : row.note))} testId="canvas-palette-more-note" />
      <ul className="flex flex-col gap-1">
        {rows.map(({ row, added }, at) => (
          <li key={row.ref} className="flex flex-col gap-0.5 rounded-lg border border-dashed border-hairline bg-surface px-2 py-1.5" data-testid="canvas-palette-more-item">
            <span className="flex items-center gap-2">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className={"truncate text-meta " + (row.blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
                <span className="truncate text-label text-faint">{row.reach === "later" && !row.blocked && row.kind !== "everyone" ? `${row.detail} · ${REACH_WORDS.later}` : row.detail}</span>
              </span>
              {!row.blocked && row.note && <DesktopReachTag member={row.ref} testId="canvas-palette-more-reach" />}
              {added && (
                <output className="shrink-0 px-1.5 py-0.5 text-label text-muted" data-testid="canvas-palette-more-added">
                  Added
                </output>
              )}
              {!added && !row.blocked && adding.has(row.ref) && (
                <output className="shrink-0 px-1.5 py-0.5 text-label text-muted" data-testid="canvas-palette-more-adding">
                  Adding…
                </output>
              )}
              {!added && !row.blocked && !adding.has(row.ref) && (
                <button
                  type="button"
                  onClick={() => {
                    setKept((was) => keep(was, row.ref, row, at));
                    void add(row);
                  }}
                  aria-label={`Add: ${row.label}`}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-label font-medium text-accent-hi outline-none hover:bg-accent-soft focus-visible:ring-1 focus-visible:ring-accent/60"
                  data-testid="canvas-palette-more-add"
                >
                  Add
                </button>
              )}
            </span>
            {row.blocked && (
              <span className="flex flex-wrap items-center gap-1.5 text-label text-muted">
                {row.blocked}
                {row.fix && (
                  <button type="button" onClick={() => onFix(row)} className="font-medium text-accent-hi underline-offset-2 hover:underline">
                    {row.fix.label}
                  </button>
                )}
              </span>
            )}
          </li>
        ))}
        {rows.length === 0 && <li className="px-1 text-label text-muted">None here.</li>}
      </ul>
    </div>
  );
}
