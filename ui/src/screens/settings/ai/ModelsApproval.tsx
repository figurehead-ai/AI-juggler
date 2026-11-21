/**
 * AJ-E3812 / AJ-E3813 (D-121) — the second-model approval: the kit's
 * dialog, asked for every extra model and never as a standing yes. It
 * says what the model adds (the jobs it passed), its size, the disk AI
 * models use now and after as two bars, the disk this Mac keeps free, and
 * that it stays on this Mac. Download carries the one-time approval; Not
 * now is remembered; Close (or Esc) decides nothing.
 */
import { useEffect, useId, useRef, type JSX } from "react";
import { Disclosure } from "../../../components/controls";
import { keepTabInside, useDialogFocus } from "../../../components/form";
import { IconClose } from "../../../components/icons-controls";
import { Button } from "../../../components/primitives";
import { Pill } from "../../../components/Pill";
import type { CatalogModel } from "../../../lib/ipc/models";
import { IconDownload, IconLock } from "./model-icons";
import { ModelTile } from "./ModelParts";
import { jobsOf, type ModelTag } from "./models-view";
import { APPROVAL, DOWNLOAD, JOB_NAMES, OTHERS, gb, listWords } from "./models-words";
import { DialogOverlay } from "../../../components/DialogFrame";

const NOW_FILL = "bg-accent-fill";
const ADDED_FILL = "bg-[color-mix(in_srgb,var(--color-accent)_45%,var(--color-surface-3))]";

function Bar({ now, added, most }: { now: number; added: number; most: number }): JSX.Element {
  const share = (bytes: number) => `${(bytes / Math.max(most, 1)) * 100}%`;
  return (
    <div className="flex h-2.5 overflow-hidden rounded-full bg-surface-3" aria-hidden="true">
      <span className={`block h-full ${NOW_FILL}`} style={{ width: share(now) }} />
      {added > 0 && <span className={`ml-0.5 block h-full ${ADDED_FILL}`} style={{ width: share(added) }} />}
    </div>
  );
}

export function ApprovalDialog({
  model,
  have,
  totalNow,
  free,
  tag,
  onDownload,
  onNotNow,
  onClose,
}: {
  model: CatalogModel;
  /** The models already here, by name. */
  have: string[];
  /** What AI models use now, bytes. */
  totalNow: number;
  /** This Mac's free disk, bytes. */
  free: number;
  tag?: ModelTag;
  onDownload: () => void;
  onNotNow: () => void;
  onClose: () => void;
}): JSX.Element {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useDialogFocus(dialog);
  // Esc closes it (deciding nothing); Tab stays inside.
  useEffect(() => {
    const frame = dialog.current;
    if (!frame) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        event.preventDefault();
        onClose();
      }
      keepTabInside(frame, event);
    };
    frame.addEventListener("keydown", onKey);
    return () => frame.removeEventListener("keydown", onKey);
  }, [onClose]);
  const size = model.size_bytes ?? 0;
  const after = totalNow + size;
  const adds = jobsOf(model).passed.map((job) => JOB_NAMES[job]);
  return (
    <DialogOverlay>
      <dialog
        ref={dialog}
        open
        aria-modal="true"
        aria-labelledby={titleId}
        className="relative m-0 flex max-h-[min(92vh,760px,calc(100vh_-_2rem))] w-full max-w-[560px] flex-col overflow-hidden rounded-2xl border-0 bg-surface p-0 text-ink shadow-[0_24px_64px_-12px_rgba(0,0,0,.55)] inset-ring-1 inset-ring-hairline/75"
        data-testid="models-approval"
        data-dialog-frame
      >
        <div className="flex shrink-0 items-center gap-3 px-5 pt-5" data-dialog-part="header">
          <span aria-hidden="true" className="flex size-9 items-center justify-center rounded-[10px] bg-[color-mix(in_srgb,var(--color-accent)_16%,var(--color-surface))] text-accent-hi">
            <IconDownload width={18} height={18} />
          </span>
          <div className="flex min-w-0 flex-1 flex-col gap-px">
            <h2 id={titleId} className="text-section font-semibold text-ink">
              {APPROVAL.title}
            </h2>
            <p className="text-meta text-muted">{APPROVAL.sub(listWords(have))}</p>
          </div>
          <button type="button" onClick={onClose} aria-label={APPROVAL.close} className="rounded-lg p-1 text-faint hover:bg-surface-2 hover:text-ink">
            <IconClose width={18} height={18} aria-hidden="true" />
          </button>
        </div>
        {/* 0.5.0 (DialogFrame.tsx): only the body scrolls; the header and the buttons stay. */}
        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 pt-3.5 pb-5" data-dialog-part="body">
          <div className="flex items-center gap-3 rounded-xl bg-surface-2 px-3.5 py-3">
            <ModelTile model={model} />
            <div className="flex min-w-0 flex-1 flex-col gap-px">
              <span className="text-body font-medium text-ink">{model.name}</span>
              <span className="text-meta text-muted">{APPROVAL.adds(listWords(adds))}</span>
            </div>
            {tag && <Pill tone="accent">{tag === "smallest" ? OTHERS.smallest : OTHERS.fastest}</Pill>}
          </div>
          <div className="flex flex-col gap-2.5">
            <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{APPROVAL.diskTitle}</span>
            <div className="grid grid-cols-[74px_1fr_62px] items-center gap-x-3 gap-y-2 text-meta">
              <span className="text-muted">{APPROVAL.now}</span>
              <Bar now={totalNow} added={0} most={after} />
              <span className="text-right font-semibold text-ink tabular-nums">{gb(totalNow)}</span>
              <span className="text-muted">{APPROVAL.after}</span>
              <Bar now={totalNow} added={size} most={after} />
              <span className="text-right font-semibold text-ink tabular-nums">{gb(after)}</span>
            </div>
            <span className="text-label text-muted">{APPROVAL.addsBytes(size, Math.max(free - size, 0))}</span>
          </div>
          <p className="flex items-start gap-2.5 text-meta text-ink-2">
            <IconLock width={16} height={16} aria-hidden="true" className="mt-px shrink-0" />
            <span>
              <span className="font-semibold text-ink">{APPROVAL.staysTitle}</span> {APPROVAL.stays}
            </span>
          </p>
          <Disclosure summary={APPROVAL.whatChanges} testId="models-approval-changes">
            <p className="mt-1.5 pl-5 text-meta text-muted">{APPROVAL.changes}</p>
          </Disclosure>
        </div>
        <div className="flex shrink-0 items-center gap-2.5 border-t border-divider px-5 py-3.5" data-dialog-part="footer">
          <span className="flex-1 text-label text-faint">{APPROVAL.asked}</span>
          <Button variant="ghost" onClick={onNotNow} data-autofocus data-testid="models-approval-not-now">
            {APPROVAL.notNow}
          </Button>
          <Button variant="primary" onClick={onDownload} data-testid="models-approval-download">
            <IconDownload width={16} height={16} aria-hidden="true" />
            {DOWNLOAD.button(size)}
          </Button>
        </div>
      </dialog>
    </DialogOverlay>
  );
}
