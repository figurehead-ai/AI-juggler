/**
 * AJ-E3814 — watches, as Bot's confirm card calls them (AJ-E3815): what
 * a target can be watched for (only what its Apps report, D-127), a
 * draft in plain words (nothing written), and the watch created on the
 * user's click. The list feeds the Bot page's "Watches Bot set".
 *
 * The preview and tests set `window.__AIJ_WATCHES__`: the catalogue, the
 * watches, and the calls made (a create is recorded, never run).
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { CatalogueEntry } from "../../types/generated/CatalogueEntry";
import type { WatchDraftInfo } from "../../types/generated/WatchDraftInfo";
import type { WatchInfo } from "../../types/generated/WatchInfo";
import type { WatchPreviewInfo } from "../../types/generated/WatchPreviewInfo";
import type { WatchTarget } from "../../types/generated/WatchTarget";

export type { CatalogueEntry, WatchDraftInfo, WatchInfo, WatchPreviewInfo, WatchTarget };

export interface WatchesSeam {
  catalogue?: CatalogueEntry[];
  watches?: WatchLine[];
  /** The targets' names, by id (the preview's echo). */
  names?: Record<string, string>;
  calls?: [string, unknown][];
}

declare global {
  interface Window {
    __AIJ_WATCHES__?: WatchesSeam;
  }
}

const PREVIEW_VERBS: Record<string, string> = { finishes: "finishes", asks_you: "asks you", stuck: "looks stuck", fails: "fails" };

/** The preview's echo, as the backend's words module says it. */
function previewWords(draft: WatchDraftInfo): string {
  const target = draft.targets[0];
  const name = target && target.kind !== "any" ? (seam().names?.[target.id] ?? "it") : "it";
  const verbs = draft.conditions.map((c) => PREVIEW_VERBS[c.code] ?? c.code);
  const said = verbs.length > 1 ? `${verbs.slice(0, -1).join(", ")} or ${verbs.at(-1)}` : (verbs[0] ?? "");
  const until = { once: " Once.", until_ends: " Until it ends.", today: " Until midnight today.", always: "", unsaid: " Once.", until: "." }[draft.lifetime.kind];
  return `Tell you when “${name}” ${said}.${until}`.replace("..", ".");
}

function seam(): WatchesSeam {
  window.__AIJ_WATCHES__ ??= {};
  return window.__AIJ_WATCHES__;
}

/** The alert's word and whether it shows in Needs you (AJ-E3816's fields). */
const told = (word: string, need: boolean) => ({ told: word, need });

const PREVIEW_CATALOGUE: CatalogueEntry[] = [
  { code: "finishes", label: "Finishes", param: null, ...told("finished", false) },
  { code: "asks_you", label: "Asks you", param: null, ...told("asks you", true) },
  { code: "stuck", label: "Looks stuck", param: null, ...told("looks stuck", true) },
  { code: "fails", label: "Fails", param: null, ...told("failed", true) },
];

/** What `target` can be watched for. */
export async function watchesCatalogue(target: WatchTarget): Promise<CatalogueEntry[]> {
  if (!hasLiveBackend()) return seam().catalogue ?? PREVIEW_CATALOGUE;
  return call("watches_catalogue", { target, lang: "en" });
}

/** A draft in plain words; nothing is written. */
export async function watchesPreview(draft: WatchDraftInfo): Promise<Pick<WatchPreviewInfo, "words" | "problem">> {
  if (!hasLiveBackend()) return { words: previewWords(draft), problem: null };
  const preview = await call("watches_preview", { draft, lang: "en" });
  return { words: preview.words, problem: preview.problem };
}

/** Creates the watch the user confirmed (their click, never Bot's). */
export async function watchesCreate(draft: WatchDraftInfo): Promise<{ words: string }> {
  if (!hasLiveBackend()) {
    (seam().calls ??= []).push(["watches_create", draft]);
    return { words: previewWords(draft) };
  }
  const info = await call("watches_create", { draft, lang: "en" });
  return { words: info.words };
}

/** A watch as the Bot page's side lists it: its id and its words. */
export interface WatchLine {
  id: string;
  words: string;
}

/** The newest watches, in plain words (one page). */
export async function watchesNewest(limit = 5): Promise<WatchLine[]> {
  if (!hasLiveBackend()) return (seam().watches ?? []).slice(0, limit);
  const page = await call("watches_list", { request: { limit, cursor: null }, lang: "en" });
  return page.items.map((info: WatchInfo) => ({ id: info.watch.id, words: info.words }));
}
