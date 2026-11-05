/**
 * D-62 (AJ-E3008 req. 3, AJ-E3006): the palette lists every candidate,
 * not only the open sessions that can join: sessions other apps run (to
 * add as Watched), Apps, Accounts, Tools, Macs and everyone in scope
 * (their sessions join by themselves), and the sessions that can't join
 * yet, each with its reason in amber and the step that would let it in.
 */
import { useState, type JSX } from "react";

import { Picker } from "../../components/controls";
import { MACHINES_WORD } from "../../lib/machine-words";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { poolJoin } from "../../lib/ipc/pool-model";
import { REACH_WORDS, type Candidate } from "../../lib/new-pool";
import { setPools } from "../../lib/pools";

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
  const rows = lists[kind] ?? [];
  const add = async (row: Candidate) => {
    try {
      setPools(await poolJoin(poolId, row.ref, "member"));
      onAdded();
      showToast({ message: `${row.label} is in the pool`, detail: row.reach === "watched" ? "Watched: it can't act until it's continued in AI Juggler." : row.reach === "later" ? "Its sessions here join by themselves, now and later." : undefined, testId: "canvas-added" });
    } catch (cause) {
      showToast({ message: `Couldn't add ${row.label}`, detail: errorText(cause), tone: "warning" });
    }
  };
  return (
    <div className="flex flex-col gap-1.5 border-t border-hairline px-2 pb-2 pt-2" data-testid="canvas-palette-more">
      <Picker
        choice
        size="sm"
        ariaLabel="More members"
        prefix="More:"
        value={kind}
        options={kinds.map((one) => ({ value: one, label: `${WORDS[one]} ${one === "everyone" ? "" : lists[one].length}`.trim() }))}
        onChange={setKind}
        testId="canvas-palette-kind"
      />
      <ul className="flex flex-col gap-1">
        {rows.slice(0, 30).map((row) => (
          <li key={row.ref} className="flex flex-col gap-0.5 rounded-lg border border-dashed border-hairline bg-surface px-2 py-1.5" data-testid="canvas-palette-more-item">
            <span className="flex items-center gap-2">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className={"truncate text-meta " + (row.blocked ? "text-ink-2" : "text-ink")}>{row.label}</span>
                <span className="truncate text-label text-faint">{row.reach === "later" && !row.blocked && row.kind !== "everyone" ? `${row.detail} · ${REACH_WORDS.later}` : row.detail}</span>
              </span>
              {!row.blocked && (
                <button
                  type="button"
                  onClick={() => void add(row)}
                  aria-label={`${row.reach === "watched" ? "Add as watched" : "Add"}: ${row.label}`}
                  className="shrink-0 rounded-md px-1.5 py-0.5 text-label font-medium text-accent-hi outline-none hover:bg-accent-soft focus-visible:ring-1 focus-visible:ring-accent/60"
                  data-testid="canvas-palette-more-add"
                >
                  {row.reach === "watched" ? "Add as watched" : "Add"}
                </button>
              )}
            </span>
            {row.blocked && (
              <span className="flex flex-wrap items-center gap-1.5 text-label text-warning">
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
