/**
 * AJ-E3816 — "Watch what": the targets as tokens (the App's own icon, the
 * name, its App), and the picker under them, suggested first, then all
 * (the pickers rule): Open now, Recent (the projects and accounts in
 * use), then All by kind with counts, with search on top and "N picked ·
 * up to 10" at its foot. ↑↓ move, Space picks, Esc closes. "Any session"
 * watches everything, so it never sits beside other targets.
 */
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type JSX } from "react";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { TargetMark } from "./WatchParts";
import { KIND_ORDER, MAX_TARGETS, kindList, optionOf, pickerLists, sameTarget, type TargetOption } from "./watches-view";
import { PICKER } from "./watches-words";
import { IconCheckCircle, IconChevronL, IconChevronR, IconCross, IconFind, IconFolderShut, IconPersonOutline, IconPoolRing, IconTerminal, IconTiles, type WatchIcon } from "./watch-icons";

const KIND_WORDS: Record<WatchTarget["kind"], string> = { session: PICKER.sessions, app: PICKER.apps, project: PICKER.projects, pool: PICKER.pools, account: PICKER.accounts, any: PICKER.any };
const KIND_ICONS: Record<WatchTarget["kind"], WatchIcon> = { session: IconTerminal, app: IconTiles, project: IconFolderShut, pool: IconPoolRing, account: IconPersonOutline, any: IconTiles };

/** The next set of targets once `option` is flipped. */
export function flipTarget(picked: readonly WatchTarget[], option: TargetOption): WatchTarget[] {
  if (picked.some((t) => sameTarget(t, option.target))) return picked.filter((t) => !sameTarget(t, option.target));
  if (option.kind === "any") return [option.target];
  const rest = picked.filter((t) => t.kind !== "any");
  return rest.length >= MAX_TARGETS ? rest : [...rest, option.target];
}

function Token({ option, onRemove }: { option: TargetOption; onRemove: () => void }): JSX.Element {
  const app = option.kind === "session" ? option.line.split(" · ")[0] : null;
  return (
    <span className="inline-flex h-8 max-w-full items-center gap-2 rounded-[9px] bg-surface-2 pl-1 pr-1.5 text-meta text-ink inset-ring-1 inset-ring-hairline/75" data-testid="watch-token">
      <TargetMark option={option} size={24} />
      <span className="truncate font-medium">{option.name}</span>
      {app && <span className="truncate text-muted">{app}</span>}
      <button type="button" onClick={onRemove} aria-label={PICKER.remove(option.name)} className="inline-flex size-5 items-center justify-center rounded text-faint outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-token-remove">
        <IconCross width={13} height={13} aria-hidden="true" />
      </button>
    </span>
  );
}

function Option({ option, on, onFlip }: { option: TargetOption; on: boolean; onFlip: () => void }): JSX.Element {
  return (
    <button type="button" aria-pressed={on} tabIndex={-1} onClick={onFlip} className={"flex w-full cursor-pointer items-center text-left gap-2.5 rounded-[9px] px-2.5 py-[7px] outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (on ? "bg-accent-soft" : "hover:bg-surface-2")} data-testid="watch-pick-option" data-pick data-key={option.key}>
      <span aria-hidden="true" className={"flex size-[18px] shrink-0 items-center justify-center rounded-[5px] " + (on ? "bg-accent-fill text-white" : "inset-ring-[1.5px] inset-ring-hairline")}>
        {on && (
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
            <path d="M5 12.5l4.5 4.5L19 7.5" />
          </svg>
        )}
      </span>
      <TargetMark option={option} size={24} />
      <span className="flex min-w-0 flex-col">
        <span className="truncate text-body font-medium text-ink">{option.name}</span>
        <span className="truncate text-label text-muted">{option.line}</span>
      </span>
    </button>
  );
}

function Heading({ children }: { children: string }): JSX.Element {
  return <div className="px-2.5 pb-1 pt-2 text-label font-medium uppercase tracking-[0.07em] text-section-label">{children}</div>;
}

export function WatchTargetPicker({ picked, book, onChange, invalid = false, labelledBy }: { picked: readonly WatchTarget[]; book: Map<string, TargetOption>; onChange: (next: WatchTarget[]) => void; invalid?: boolean; labelledBy: string }): JSX.Element {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<WatchTarget["kind"] | null>(null);
  const wrap = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const listId = useId();
  const lists = useMemo(() => pickerLists(book, query), [book, query]);
  const isOn = (option: TargetOption) => picked.some((t) => sameTarget(t, option.target));
  const flip = (option: TargetOption) => onChange(flipTarget(picked, option));
  useEffect(() => {
    if (open) search.current?.focus();
    const outside = (event: MouseEvent) => {
      if (!(event.target instanceof Node) || !wrap.current?.contains(event.target)) setOpen(false);
    };
    if (open) window.addEventListener("mousedown", outside);
    return () => window.removeEventListener("mousedown", outside);
  }, [open]);
  const close = () => {
    setOpen(false);
    setKind(null);
    setQuery("");
  };
  const onKeys = (event: globalThis.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      close();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp" && event.key !== " " && event.key !== "Enter") return;
    const items = [...(list.current?.querySelectorAll<HTMLElement>("[data-pick]") ?? [])];
    const at = items.findIndex((item) => item === document.activeElement);
    if (event.key === " " || event.key === "Enter") {
      if (at < 0) return;
      event.preventDefault();
      items[at]!.click();
      return;
    }
    event.preventDefault();
    const next = event.key === "ArrowDown" ? Math.min(items.length - 1, at + 1) : at - 1;
    if (next < 0) search.current?.focus();
    else items[next]?.focus();
  };
  // ↑↓, Space and Esc inside the field and its list (a listener, not a
  // handler on a plain element).
  const keys = useRef(onKeys);
  useLayoutEffect(() => {
    keys.current = onKeys;
  });
  useEffect(() => {
    const node = wrap.current;
    const listen = (event: globalThis.KeyboardEvent) => keys.current(event);
    node?.addEventListener("keydown", listen);
    return () => node?.removeEventListener("keydown", listen);
  }, []);
  const options = picked.map((target) => optionOf(book, target));
  const shown = (rows: TargetOption[]) => rows.map((option) => <Option key={option.key} option={option} on={isOn(option)} onFlip={() => flip(option)} />);
  return (
    <div ref={wrap} className="relative">
      <div
        className={"flex min-h-12 flex-wrap items-center gap-2 rounded-xl bg-fill px-2.5 py-2 " + (open ? "inset-ring-[1.5px] inset-ring-accent ring-4 ring-accent-soft" : invalid ? "inset-ring-[1.5px] inset-ring-danger" : "inset-ring-1 inset-ring-hairline/75")}
        data-testid="watch-target-field"
      >
        {options.map((option) => (
          <Token key={option.key} option={option} onRemove={() => onChange(picked.filter((t) => !sameTarget(t, option.target)))} />
        ))}
        <button type="button" aria-haspopup="dialog" aria-expanded={open} aria-controls={open ? listId : undefined} aria-describedby={labelledBy} onClick={() => (open ? close() : setOpen(true))} className="min-w-[10rem] flex-1 rounded px-1 py-1 text-left text-meta text-faint outline-none focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-target-add">
          {picked.length === 0 ? PICKER.add : PICKER.addMore}
        </button>
      </div>
      {open && (
        <div className="absolute left-0 top-[calc(100%+6px)] z-30 flex w-[min(452px,100%)] min-w-[320px] flex-col gap-px rounded-card bg-surface p-2 shadow-[0_18px_48px_-12px_rgba(0,0,0,.35),0_2px_6px_rgba(0,0,0,.12)] inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="watch-picker">
          <label className="mx-0.5 mb-1.5 flex h-9 items-center gap-2 rounded-[10px] bg-fill px-2.5 inset-ring-1 inset-ring-hairline/75">
            <IconFind width={15} height={15} aria-hidden="true" className="text-faint" />
            <span className="sr-only">{PICKER.search}</span>
            <input ref={search} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={PICKER.search} className="min-w-0 flex-1 bg-transparent text-body text-ink outline-none placeholder:text-faint" data-testid="watch-picker-search" />
          </label>
          <div ref={list} id={listId} className="flex max-h-[min(420px,calc(100vh-330px))] min-h-40 flex-col gap-px overflow-y-auto">
            {lists.matches !== null ? (
              <>
                <Heading>{PICKER.results}</Heading>
                {lists.matches.length === 0 ? <p className="px-2.5 py-2 text-meta text-muted">{PICKER.none}</p> : shown(lists.matches.slice(0, 30))}
              </>
            ) : kind !== null ? (
              <>
                <button type="button" data-pick onClick={() => setKind(null)} className="flex items-center gap-1.5 rounded-[9px] px-2.5 py-1.5 text-meta font-medium text-accent-hi outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-picker-back">
                  <IconChevronL width={13} height={13} aria-hidden="true" />
                  {PICKER.back}
                </button>
                <Heading>{KIND_WORDS[kind]}</Heading>
                {shown(kindList(book, kind))}
              </>
            ) : (
              <>
                {lists.openNow.length > 0 && (
                  <div>
                    <Heading>{`${PICKER.openNow}  ${lists.openNow.length}`}</Heading>
                    {shown(lists.openNow)}
                  </div>
                )}
                {lists.recent.length > 0 && (
                  <div>
                    <Heading>{PICKER.recent}</Heading>
                    {shown(lists.recent)}
                  </div>
                )}
                <hr className="mx-1.5 my-1 border-t border-divider" />
                <Heading>{PICKER.all}</Heading>
                <div className="grid grid-cols-2 gap-x-1.5">
                  {[...KIND_ORDER, "any" as const].map((each) => {
                    const Icon = KIND_ICONS[each];
                    const anyOption = each === "any" ? optionOf(book, { kind: "any" }) : null;
                    return (
                      <button
                        key={each}
                        type="button"
                        data-pick
                        aria-pressed={anyOption ? isOn(anyOption) : undefined}
                        onClick={() => (anyOption ? flip(anyOption) : setKind(each))}
                        className={"flex items-center gap-2.5 rounded-[9px] px-2.5 py-1.5 text-left text-meta outline-none focus-visible:ring-2 focus-visible:ring-accent/60 " + (anyOption && isOn(anyOption) ? "bg-accent-soft" : "hover:bg-surface-2")}
                        data-testid={`watch-picker-kind-${each}`}
                      >
                        <Icon width={15} height={15} aria-hidden="true" className="text-muted" />
                        <span className="flex-1 font-medium text-ink">{KIND_WORDS[each]}</span>
                        {each !== "any" && <span className="tabular-nums text-muted">{lists.counts[each]}</span>}
                        {anyOption && isOn(anyOption) ? <IconCheckCircle width={13} height={13} aria-hidden="true" className="text-accent-hi" /> : <IconChevronR width={13} height={13} aria-hidden="true" className="text-faint" />}
                      </button>
                    );
                  })}
                </div>
              </>
            )}
          </div>
          <div className="flex items-center gap-2 px-2.5 pb-1 pt-2 text-label text-muted">
            <span className="flex-1" data-testid="watch-picker-count">
              {PICKER.picked(picked.length, MAX_TARGETS)}
            </span>
            <span>{PICKER.keys}</span>
          </div>
        </div>
      )}
    </div>
  );
}
