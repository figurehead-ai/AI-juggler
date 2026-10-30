/**
 * ADR-0023 — Settings › Advanced › Local cache: what AI Juggler keeps on
 * this Mac to be quick. One line says whether it's on, what it uses and
 * its limit; the limit (Automatic, from the free disk, or a fixed size);
 * Clear cache…, which says first what gets slower; and, folded, each
 * cache with its size and what it speeds up. Nothing in it is sent, and
 * all of it is rebuilt from the tools' files.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Disclosure, Picker, type PickerOption } from "../../components/controls";
import { Modal } from "../../components/form";
import { IconBroom, IconGoTo } from "../../components/icons-actions";
import { byteSize } from "../../lib/compact";
import { errorText } from "../../lib/bridge";
import { CACHE_LIMITS_MB, cacheClear, cacheSetLimit, cacheSetOn, cacheStatus, type CacheInfo, type CacheRowInfo } from "../../lib/ipc/cache";
import { PaneSection, SettingRow, SwitchRow } from "./parts";

const MB = 1_000_000;

/** A size in Finder's units, without a trailing ".0" ("1 GB", "1.3 GB"). */
export function cacheSize(bytes: number): string {
  return byteSize(bytes).replace(/\.0 /, " ");
}

/** The limit picker's choices: Automatic, then the fixed sizes. */
const LIMITS: PickerOption[] = [
  {
    value: "auto",
    label: "Automatic",
    hint: "2% of this computer's free space, between 64 MB and 2 GB",
  },
  ...CACHE_LIMITS_MB.map((mb) => ({
    value: String(mb),
    label: cacheSize(mb * MB),
    hint: "Trims the least recently used first",
  })),
];

/** "a, b and c". */
function listWords(words: string[]): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} and ${words[words.length - 1]}`;
}

/** The one line under "Keep a cache on this computer": what it uses, and
 * what that makes faster (the caches kept now, by name). */
export function cacheSummary(info: CacheInfo): string {
  if (!info.on) return "Off: nothing is kept, so pages read your tools' files each time they open.";
  // Empty (just cleared, or nothing kept yet): nothing is faster yet, and
  // "0 B" is a machine's word.
  if (info.used_bytes === 0) return `Empty for now. It fills as you use AI Juggler, up to ${cacheSize(info.budget_bytes)}.`;
  const kept = info.caches.filter((row) => row.on).map((row, at) => (at === 0 ? row.name : row.name.toLowerCase()));
  // "Subagent lists and app icons open", "Search index opens".
  const one = kept.length === 1 && !kept[0]!.endsWith("s");
  const faster = kept.length > 0 ? `${listWords(kept)} ${one ? "opens" : "open"} without re-reading files.` : "Nothing is kept yet.";
  return `Using ${cacheSize(info.used_bytes)} of ${cacheSize(info.budget_bytes)}. ${faster}`;
}

/** Opens a Settings section (a cache's own switch, elsewhere). */
const openSection = (section: string) => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section } }));

/** What the size limit row says. */
function limitWords(info: CacheInfo): string {
  if (!info.on) return "Turn the cache on to set its limit.";
  if (info.automatic) return `Automatic: ${cacheSize(info.budget_bytes)} now, from this computer's free space.`;
  return `Fixed at ${cacheSize((info.limit_mb ?? 0) * MB)}. The least recently used goes first.`;
}

export function LocalCacheSection(): JSX.Element {
  const [info, setInfo] = useState<CacheInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void cacheStatus().then(setInfo, (cause) => setError(errorText(cause)));
  }, []);
  const lowDisk =
    info?.on && info.low_disk ? `Your disk is nearly full, so the cache keeps to ${cacheSize(info.budget_bytes)} and adds nothing until there's room.` : null;
  const apply = useCallback((change: Promise<CacheInfo>, done?: (next: CacheInfo) => string | null) => {
    setError(null);
    void change.then(
      (next) => {
        setInfo(next);
        setNote(done?.(next) ?? null);
      },
      (cause) => setError(errorText(cause)),
    );
  }, []);
  return (
    <>
      <PaneSection
        id="local-cache"
        title="Local cache"
        footer="On this computer only, readable by you only, and never sent. Everything in it can be rebuilt from your tools' files."
      >
        {info === null ? (
          <SettingRow title="Keep a cache on this computer" description={error ?? "Reading its size…"} testId="cache-loading" />
        ) : (
          <>
            <SwitchRow
              title="Keep a cache on this computer"
              description={cacheSummary(info)}
              checked={info.on}
              testId="cache-switch-row"
              note={error ?? note ?? lowDisk}
              noteTone={(error ?? note) ? "info" : "warning"}
              onChange={(on) => apply(cacheSetOn(on), (next) => (next.on ? null : "Turned off. Its files were removed."))}
            />
            <SettingRow
              title="Size limit"
              description={limitWords(info)}
              testId="cache-limit-row"
              control={
                <Picker
                  size="sm"
                  value={info.automatic ? "auto" : String(info.limit_mb)}
                  options={LIMITS}
                  ariaLabel="Size limit"
                  heading="Size limit"
                  disabled={!info.on}
                  reason="Turn the cache on to set its limit."
                  testId="cache-limit"
                  onChange={(value) => apply(cacheSetLimit(value === "auto" ? null : Number(value)))}
                />
              }
            />
            <SettingRow
              title="Clear cache"
              description={
                info.used_bytes > 0 ? `Frees ${cacheSize(info.used_bytes)} now. It fills again as you use AI Juggler.` : "Nothing to clear: the cache is empty."
              }
              testId="cache-clear-row"
              control={
                <Button size="sm" disabled={info.used_bytes === 0} onClick={() => setAsking(true)} data-testid="cache-clear">
                  <IconBroom width={14} height={14} aria-hidden="true" />
                  Clear cache…
                </Button>
              }
            />
            <li className="px-4 py-3" data-testid="cache-contents">
              <Disclosure summary="What's in it" testId="cache-contents-toggle">
                <ul className="mt-2 flex flex-col divide-y divide-hairline/60" aria-label="What's in the cache">
                  {info.caches.map((row) => (
                    <CacheRow key={row.key} row={row} />
                  ))}
                </ul>
              </Disclosure>
            </li>
          </>
        )}
      </PaneSection>
      {asking && info && (
        <ClearSheet
          info={info}
          onClose={() => setAsking(false)}
          onClear={() => {
            setAsking(false);
            const freed = info.used_bytes;
            apply(cacheClear(), () => `Cleared ${cacheSize(freed)}.`);
          }}
        />
      )}
    </>
  );
}

/** One cache: its name and size, what it speeds up, and its own switch. */
function CacheRow({ row }: { row: CacheRowInfo }): JSX.Element {
  const size = !row.on
    ? "Off"
    : row.bytes === 0
      ? "Empty"
      : row.whole_only || row.entries === 0
        ? cacheSize(row.bytes)
        : `${cacheSize(row.bytes)} · ${row.entries.toLocaleString("en-US")} ${row.entries === 1 ? "item" : "items"}`;
  return (
    <li className="flex items-start gap-3 py-2.5 first:pt-1 last:pb-0" data-testid="cache-row">
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="text-sm text-ink">{row.name}</span>
        <span className="text-xs leading-relaxed text-muted">{row.speeds_up}</span>
        {row.own_switch && (
          <span className="text-xs leading-relaxed text-muted">
            Kept whole, with its own switch{row.own_switch_pane ? ` in ${row.own_switch_pane}` : ""}: {row.own_switch}.
          </span>
        )}
        {row.own_switch_section && (
          <button
            type="button"
            onClick={() => openSection(row.own_switch_section!)}
            className="mt-0.5 flex w-fit items-center gap-1 rounded-md text-xs font-medium text-accent outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60"
            data-testid="cache-row-door"
          >
            <IconGoTo aria-hidden="true" width={14} height={14} />
            {row.on ? `Change it in ${row.own_switch_pane ?? "Settings"}` : `Turn it on in ${row.own_switch_pane ?? "Settings"}`}
          </button>
        )}
      </div>
      <span className="shrink-0 pt-px text-xs tabular-nums text-muted" data-testid="cache-row-size">
        {size}
      </span>
    </li>
  );
}

/** Clear cache…: what is freed, and what is slower until it fills again. */
function ClearSheet({ info, onClose, onClear }: { info: CacheInfo; onClose: () => void; onClear: () => void }): JSX.Element {
  const kept = info.caches.filter((row) => row.bytes > 0);
  return (
    <Modal
      title="Clear the cache?"
      subtitle={`Frees ${cacheSize(info.used_bytes)} on this computer. Nothing of yours is lost.`}
      onClose={onClose}
      testId="cache-clear-sheet"
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <p className="text-sm text-ink">Until it fills again:</p>
          <ul className="flex flex-col gap-1.5" data-testid="cache-clear-slower">
            {kept.map((row) => (
              <li key={row.key} className="text-xs leading-relaxed text-muted">
                <span className="font-medium text-ink">{row.name}:</span> {row.slower}
              </li>
            ))}
          </ul>
        </div>
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" onClick={onClear} data-autofocus data-testid="cache-clear-go">
            <IconBroom width={14} height={14} aria-hidden="true" />
            Clear cache
          </Button>
        </div>
      </div>
    </Modal>
  );
}
