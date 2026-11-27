/**
 * AJ-E3830 (D-149 amendment 2) — the drafted watch in plain words, its
 * target included, under Bot's line, so a wrong draft is seen before Add:
 * the Watches preview's own words for the draft ("Tell you when “api”
 * finishes. Once."). Nothing shows for a draft with no target (the form
 * asks which one).
 */
import { useEffect, useState, type JSX } from "react";
import { watchesPreview } from "../../lib/ipc/watches";
import type { WatchDrafted } from "../../types/generated/WatchDrafted";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";

function draftOf(drafted: WatchDrafted | null): WatchDraftInfo | null {
  if (!drafted) return null;
  if (drafted.kind === "filled" || drafted.kind === "pick") return drafted.draft;
  // AJ-E3830: the pre-fill's words only when every field was read (a
  // partial draft's echo would say a default as if it were asked).
  if (drafted.kind === "prefilled") return drafted.prefill.left.length === 0 && !drafted.prefill.card ? drafted.prefill.draft : null;
  if (drafted.kind === "cant_watch") return drafted.draft ?? null;
  return null;
}

export function BotWatchDraftWords({ drafted }: { drafted: WatchDrafted | null }): JSX.Element | null {
  const draft = draftOf(drafted);
  const shown = draft && draft.targets.length > 0 ? draft : null;
  // The words are kept with the draft they describe; a stale answer for
  // another draft is never shown.
  const [said, setSaid] = useState<{ draft: WatchDraftInfo; words: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    if (shown)
      void watchesPreview(shown).then(
        (preview) => live && setSaid({ draft: shown, words: preview.problem ? null : preview.words }),
        () => live && setSaid({ draft: shown, words: null }),
      );
    return () => {
      live = false;
    };
  }, [shown]);
  const words = shown && said?.draft === shown ? said.words : null;
  if (!words) return null;
  return (
    <p className="rounded-[10px] bg-surface px-3 py-2 text-body leading-normal text-ink inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-watch-draft-words">
      {words}
    </p>
  );
}
