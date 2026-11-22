/**
 * AJ-E3808, AJ-E3819 — the two sheets that delete: Clear the whole
 * conversation (what goes, what stays: notes and watches, and its own
 * backup line, CLEAR_AND_BACKUPS) and Forget one memory (what goes, that
 * it can't be undone, and FORGET_AND_BACKUPS word for word, as the
 * backend gives it). Each asks first; Esc and Cancel keep everything.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { IconInfo, IconTrash } from "../icons-actions";
import { IconBookmark } from "./bot-icons";
import { CLEAR_AND_BACKUPS, WORDS } from "../../lib/bot-words";
import { clearConversation } from "../../lib/bot-store";
import { FORGET_EVENT, forgetMemory } from "../../lib/bot-memory";
import { botCount, botMemories, type BotCountInfo } from "../../lib/ipc/bot";
import { errorText } from "../../lib/bridge";
import type { BotMemoryInfo } from "../../types/generated/BotMemoryInfo";
import { THIS_MACHINE_IN_TEXT } from "../../lib/machine-words";
import { DialogFrame, DialogOverlay } from "../DialogFrame";

function Sheet({ id, title, body, children, button, onConfirm, onClose }: { id: string; title: string; body: string; children: ReactNode; button: string; onConfirm: () => Promise<void>; onClose: () => void }): JSX.Element {
  const ref = useRef<HTMLDivElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("[data-cancel]")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);
  const run = async () => {
    setBusy(true);
    try {
      await onConfirm();
      onClose();
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <DialogOverlay className="fixed inset-0 z-50 grid place-items-center bg-[rgba(14,18,25,.30)] p-4 [[data-theme=dark]_&]:bg-[rgba(4,6,10,.62)]">
      <DialogFrame
        frameRef={ref}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={`${id}-title`}
        aria-describedby={`${id}-body`}
        className="w-[500px] max-w-full rounded-2xl bg-surface shadow-[0_18px_48px_-12px_rgba(0,0,0,.5)] inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2"
        data-testid={id}
        headerClassName="flex items-start gap-3 px-[22px] pt-[22px]"
        header={
          <>
            <span className="grid size-9 shrink-0 place-items-center rounded-[10px] bg-surface-3 text-danger">
              <IconTrash width={18} height={18} aria-hidden="true" />
            </span>
            <div className="flex flex-col gap-1.5">
              <h2 id={`${id}-title`} className="text-[17px] leading-snug font-semibold text-ink">
                {title}
              </h2>
              <p id={`${id}-body`} className="text-body text-muted">
                {body}
              </p>
            </div>
          </>
        }
        bodyClassName="flex flex-col gap-3.5 px-[22px] pt-3.5 pb-3.5 empty:pb-0"
        footerClassName="flex items-center justify-end gap-2 px-[22px] pt-1 pb-[18px]"
        footer={
          <>
            <button type="button" data-cancel className="inline-flex h-9 items-center rounded-xl px-3.5 text-body font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onClose}>
              {WORDS.cancel}
            </button>
            <button type="button" disabled={busy} className="inline-flex h-9 items-center rounded-xl bg-danger px-3.5 text-body font-medium text-white outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60 [[data-theme=dark]_&]:text-canvas" onClick={() => void run()} data-testid={`${id}-confirm`}>
              {button}
            </button>
          </>
        }
      >
        {children}
        {error && (
          <p className="text-meta text-danger" role="alert">
            {error}
          </p>
        )}
      </DialogFrame>
    </DialogOverlay>
  );
}

function Note({ icon, children }: { icon: "info" | "bookmark"; children: ReactNode }): JSX.Element {
  const Icon = icon === "info" ? IconInfo : IconBookmark;
  return (
    <div className="flex items-start gap-2.5 rounded-xl bg-surface-2 px-3.5 py-3 text-meta leading-snug text-ink-2 [[data-theme=dark]_&]:bg-surface-3">
      <Icon width={16} height={16} aria-hidden="true" className="mt-px shrink-0 text-faint" />
      <span>{children}</span>
    </div>
  );
}

export function BotClearSheet({ onClose, onMemories }: { onClose: () => void; onMemories: () => void }): JSX.Element {
  const [count, setCount] = useState<BotCountInfo | null>(null);
  useEffect(() => {
    void botCount().then(setCount).catch(() => setCount(null));
  }, []);
  return (
    <Sheet id="bot-clear" title={WORDS.clearTitle} body={WORDS.clearBody(count?.questions ?? 0, count?.since_ms ?? null)} button={WORDS.clearButton} onConfirm={clearConversation} onClose={onClose}>
      <Note icon="bookmark">
        <b className="font-semibold text-ink">{WORDS.clearStays(count?.memories ?? 0, count?.watches ?? 0)}</b> Forget them in{" "}
        <button type="button" className="rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => { onClose(); onMemories(); }}>
          {WORDS.clearStaysLink}
        </button>
        .
      </Note>
      <Note icon="info">
        <span data-testid="bot-clear-backups">{CLEAR_AND_BACKUPS}</span>
      </Note>
    </Sheet>
  );
}

function forgetTitle(memory: BotMemoryInfo): string {
  if (memory.slot === "alias" && memory.about) return `Forget that you call “${memory.about}” ${memory.text}?`;
  if (memory.slot === "focus" && memory.about) return `Forget that “${memory.about}” matters most?`;
  return `Forget “${memory.text}”?`;
}

function forgetBody(memory: BotMemoryInfo): string {
  const versions = memory.versions > 0 ? ` with its ${memory.versions} earlier ${memory.versions === 1 ? "version" : "versions"}` : "";
  return `Bot stops using it now. It’s deleted from ${THIS_MACHINE_IN_TEXT}${versions}, and the message where you said it is cleared. You can’t undo this.`;
}

function isMemory(value: unknown): value is BotMemoryInfo {
  return typeof value === "object" && value !== null && typeof Reflect.get(value, "key") === "string" && typeof Reflect.get(value, "text") === "string";
}

/** The forget sheet, opened from anywhere a memory shows (its event). */
export function BotForgetHost(): JSX.Element | null {
  const [memory, setMemory] = useState<BotMemoryInfo | null>(null);
  const [words, setWords] = useState<string | null>(null);
  useEffect(() => {
    const open = (event: Event) => {
      const detail: unknown = event instanceof CustomEvent ? event.detail : null;
      if (isMemory(detail)) setMemory(detail);
    };
    window.addEventListener(FORGET_EVENT, open);
    return () => window.removeEventListener(FORGET_EVENT, open);
  }, []);
  useEffect(() => {
    if (memory && words === null) void botMemories().then((info) => setWords(info.forget_and_backups)).catch(() => setWords(null));
  }, [memory, words]);
  if (!memory) return null;
  return (
    <Sheet id="bot-forget" title={forgetTitle(memory)} body={forgetBody(memory)} button="Forget" onConfirm={() => forgetMemory(memory.key)} onClose={() => setMemory(null)}>
      <Note icon="info">
        <span data-testid="bot-forget-backups">{words ?? ""}</span>
      </Note>
    </Sheet>
  );
}
