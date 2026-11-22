/**
 * AJ-E3812 — Recommended for this Mac (AJ-E3811's one pick): the kit's
 * chosen-card look (an accent edge on a tinted surface) holding the
 * view's one filled primary, with a "Why it fits this Mac" checklist
 * beside it built from the facts the pick rests on. Its action says its
 * consequence on the same line: Use it, Download with the size, the
 * download under way (progress, time left, Cancel), the disk refusal
 * with the exact numbers, or a quiet In use once Automatic sends its
 * jobs here.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { Alert } from "../../../components/Alert";
import { Disclosure } from "../../../components/controls";
import { IconCheck } from "../../../components/icons-controls";
import { IconWarn } from "../../../components/icons-menu";
import { Button } from "../../../components/primitives";
import { Pill } from "../../../components/Pill";
import type { DownloadInfo, ModelsCatalog } from "../../../lib/ipc/models";
import { IconDownload, IconRefresh, JobIcon } from "./model-icons";
import { ModelJobRow, ModelFacts, ModelTile, ModelsSection, Verdict } from "./ModelParts";
import { betaJobs, jobsOf, verdictOf, type FitRow, type Recommended } from "./models-view";
import { DISK, DOWNLOAD, FAILED_AFTER, FAILURES, JOB_NAMES, NOTHING, RECOMMENDED, WAITS, listWords, memoryWords } from "./models-words";

export interface RecommendedActions {
  onUse: () => void;
  onDownload: () => void;
  onCancel: () => void;
  onCheckAgain: () => void;
  onOpenStorage: () => void;
}

function FitPanel({ rows, needs }: { rows: FitRow[]; needs: boolean }): JSX.Element {
  return (
    <div
      className="flex flex-col gap-2.5 border-t border-[color-mix(in_srgb,var(--color-accent)_22%,var(--color-divider))] bg-[color-mix(in_srgb,var(--color-surface)_75%,transparent)] px-4 py-4 @min-[640px]:border-t-0 @min-[640px]:border-l"
      data-testid="models-fit"
    >
      <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{needs ? RECOMMENDED.needsTitle : RECOMMENDED.fitTitle}</span>
      <ul className="flex flex-col gap-2.5">
        {rows.map(([ok, key, value]) => (
          <li key={key} className="flex items-start gap-2.5 text-meta leading-snug">
            <span
              aria-hidden="true"
              className={`mt-px flex size-[18px] shrink-0 items-center justify-center rounded-full ${ok ? "bg-success/20 text-success" : "bg-warning/20 text-warning"}`}
            >
              {ok ? <IconCheck width={11} height={11} strokeWidth={3} /> : <IconWarn width={11} height={11} />}
            </span>
            <span className="flex flex-col">
              <span className="text-muted">{key}</span>
              <span className="font-medium text-ink">{value}</span>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** The jobs it passed as chips, then the ones not yet behind a fold. */
function JobsLine({ pick }: { pick: Recommended }): JSX.Element {
  const { passed, notYet } = jobsOf(pick.model);
  return (
    <div className="flex flex-col gap-2">
      <ul className="flex flex-wrap items-center gap-2" aria-label={RECOMMENDED.title}>
        {passed.map((job) => (
          <li key={job} className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-surface pr-2.5 pl-2 text-[12.5px] text-ink inset-ring-1 inset-ring-hairline/75">
            <span className="text-faint">
              <JobIcon job={job} size={14} />
            </span>
            {JOB_NAMES[job]}
            <span aria-hidden="true" className="text-faint">
              ·
            </span>
            <Verdict verdict={verdictOf(pick.model, job).verdict} />
          </li>
        ))}
      </ul>
      {notYet.length > 0 && (
        <Disclosure summary={RECOMMENDED.notYet(notYet.length)} testId="models-recommended-not-yet">
          <ul className="mt-2 flex flex-col gap-2 pl-5">
            {notYet.map((job) => (
              <ModelJobRow key={job} model={pick.model} job={job} quiet />
            ))}
          </ul>
        </Disclosure>
      )}
    </div>
  );
}

function Consequence({ children }: { children: ReactNode }): JSX.Element {
  return <span className="text-[12.5px] leading-snug text-muted">{children}</span>;
}

/** Minutes left from the bytes seen since the first step this screen saw. */
function useMinutesLeft(download: DownloadInfo | null | undefined): number | null {
  const first = useRef<{ at: number; done: number; id: string } | null>(null);
  const [left, setLeft] = useState<number | null>(null);
  useEffect(() => {
    if (!download || download.state !== "running") return;
    const now = Date.now();
    if (first.current?.id !== download.id) first.current = { at: now, done: download.done_bytes, id: download.id };
    const seconds = (now - first.current.at) / 1000;
    const rate = (download.done_bytes - first.current.done) / Math.max(seconds, 1);
    setLeft(seconds >= 3 && rate > 0 ? Math.ceil((download.size_bytes - download.done_bytes) / rate / 60) : null);
  }, [download]);
  return left;
}

function Downloading({ name, download, onCancel }: { name: string; download: DownloadInfo; onCancel: () => void }): JSX.Element {
  const minutes = useMinutesLeft(download);
  const share = download.size_bytes > 0 ? Math.round((download.done_bytes / download.size_bytes) * 100) : 0;
  const waiting = download.state === "waiting" && download.wait ? WAITS[download.wait.kind](download.wait) : null;
  return (
    <div className="flex flex-col gap-2" data-testid="models-downloading">
      <p className="flex flex-wrap items-center gap-2 text-meta whitespace-nowrap" aria-live="polite">
        <span className="font-semibold text-ink">{download.state === "verifying" ? DOWNLOAD.checking : DOWNLOAD.downloading}</span>
        <span className="text-ink-2 tabular-nums">{DOWNLOAD.done(download.done_bytes, download.size_bytes)}</span>
        {minutes !== null && <span className="text-muted">· {DOWNLOAD.left(minutes)}</span>}
      </p>
      <div className="flex items-center gap-2.5">
        <progress
          className="h-2 min-w-0 flex-1 appearance-none overflow-hidden rounded-full bg-surface-3 [&::-moz-progress-bar]:rounded-full [&::-moz-progress-bar]:bg-accent-fill [&::-webkit-progress-bar]:bg-surface-3 [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-accent-fill"
          aria-label={DOWNLOAD.progressLabel(name)}
          max={100}
          value={share}
        />
        <Button variant="ghost" size="sm" onClick={onCancel} data-testid="models-cancel">
          {DOWNLOAD.cancel}
        </Button>
      </div>
      <Consequence>{waiting ?? DOWNLOAD.how}</Consequence>
    </div>
  );
}

function NoDisk({ pick, catalog, actions }: { pick: Recommended; catalog: ModelsCatalog; actions: RecommendedActions }): JSX.Element {
  const free = catalog.mac?.free_disk_bytes ?? 0;
  return (
    <div className="flex flex-col gap-2.5">
      <Alert
        level="needs-you"
        title={DISK.title}
        testId="models-no-disk"
        below={
          <span className="mt-1.5 flex flex-wrap gap-4">
            <button type="button" className="font-medium text-accent-hi hover:underline" onClick={actions.onOpenStorage}>
              {DISK.openStorage}
            </button>
            <button type="button" className="inline-flex items-center gap-1 font-medium text-accent-hi hover:underline" onClick={actions.onCheckAgain}>
              <IconRefresh width={12} height={12} aria-hidden="true" />
              {DISK.checkAgain}
            </button>
          </span>
        }
      >
        {DISK.body(pick.model.size_bytes ?? 0, pick.model.free_disk_needed_bytes, free)}
      </Alert>
      <div className="flex flex-wrap items-center gap-3 @min-[560px]:flex-nowrap">
        <Button variant="primary" disabled className="shrink-0">
          <IconDownload width={16} height={16} aria-hidden="true" />
          {DOWNLOAD.button(pick.model.size_bytes ?? 0)}
        </Button>
        <Consequence>{DOWNLOAD.readyOnceRoom}</Consequence>
      </div>
    </div>
  );
}

/** The card's action, by what the pick needs now. */
function Action({ pick, catalog, download, actions }: { pick: Recommended; catalog: ModelsCatalog; download: DownloadInfo | null | undefined; actions: RecommendedActions }): JSX.Element {
  const jobs = listWords(betaJobs(pick.model).map((job) => JOB_NAMES[job]));
  if (pick.action === "no-disk") return <NoDisk pick={pick} catalog={catalog} actions={actions} />;
  if (pick.action === "downloading" && download) return <Downloading name={pick.model.name} download={download} onCancel={actions.onCancel} />;
  if (pick.action === "in-use") {
    return (
      <div className="flex flex-wrap items-center gap-3">
        <Pill tone="accent">
          <IconCheck width={12} height={12} strokeWidth={2.6} aria-hidden="true" className="mr-1" />
          {RECOMMENDED.inUse}
        </Pill>
        <Consequence>{RECOMMENDED.inUseLine}</Consequence>
      </div>
    );
  }
  const adopt = pick.action === "use";
  return (
    <div className="flex flex-col gap-2.5">
      {download?.state === "failed" && download.id === pick.model.id && download.failure && (
        <Alert level="broken" testId="models-download-failed" title={FAILURES[download.failure]}>
          {FAILED_AFTER}
        </Alert>
      )}
      <div className="flex flex-wrap items-center gap-3 @min-[560px]:flex-nowrap">
        <Button variant="primary" onClick={adopt ? actions.onUse : actions.onDownload} data-testid="models-recommended-action" className="shrink-0">
          {adopt ? <IconCheck width={15} height={15} strokeWidth={2.2} aria-hidden="true" /> : <IconDownload width={16} height={16} aria-hidden="true" />}
          {adopt ? RECOMMENDED.adopt : DOWNLOAD.button(pick.model.size_bytes ?? 0)}
        </Button>
        <Consequence>{adopt ? RECOMMENDED.adoptLine(jobs) : RECOMMENDED.downloadLine(jobs)}</Consequence>
      </div>
    </div>
  );
}

/** Why this one, in one plain line. */
function reasonOf(pick: Recommended, catalog: ModelsCatalog): string {
  if (pick.model.built_in) return RECOMMENDED.reasonBuiltIn;
  if (catalog.recommendation?.why === "installed") return RECOMMENDED.reasonInstalled;
  const memory = catalog.mac ? memoryWords(catalog.mac.memory_bytes, catalog.mac.memory_gb) : "";
  return pick.smallest ? RECOMMENDED.reasonSmallest(memory) : RECOMMENDED.reasonMost(memory);
}

export function RecommendedCard({
  pick,
  catalog,
  fit,
  download,
  actions,
}: {
  pick: Recommended;
  catalog: ModelsCatalog;
  fit: FitRow[];
  download: DownloadInfo | null | undefined;
  actions: RecommendedActions;
}): JSX.Element {
  return (
    <div className="@container relative overflow-hidden rounded-2xl bg-[color-mix(in_srgb,var(--color-accent)_7%,var(--color-surface))] shadow-card" data-testid="models-recommended">
      <div className="grid grid-cols-1 @min-[640px]:grid-cols-[minmax(0,1fr)_248px]">
        <div className="flex min-w-0 flex-col gap-3.5 px-5 pt-5 pb-4.5">
          <div className="flex items-start gap-3.5">
            <ModelTile model={pick.model} size="lg" on />
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <div className="flex flex-wrap items-center gap-2.5">
                <h4 className="text-[17px] font-semibold text-ink">{pick.model.name}</h4>
                {pick.model.built_in && <Pill tone="neutral">{RECOMMENDED.builtIn}</Pill>}
              </div>
              <p className="text-body text-ink-2">{reasonOf(pick, catalog)}</p>
            </div>
          </div>
          <ModelFacts model={pick.model} />
          <JobsLine pick={pick} />
          <Action pick={pick} catalog={catalog} download={download} actions={actions} />
        </div>
        <FitPanel rows={fit} needs={pick.action === "no-disk"} />
      </div>
      <span aria-hidden="true" className="pointer-events-none absolute inset-0 rounded-2xl inset-ring-[1.5px] inset-ring-accent" />
    </div>
  );
}

/** No pick: why not, in plain words, with no button. */
export function NothingCard({ catalog }: { catalog: ModelsCatalog }): JSX.Element {
  const mac = catalog.mac;
  const memory = mac ? memoryWords(mac.memory_bytes, mac.memory_gb) : "";
  const why = catalog.recommendation?.nothing;
  let line = NOTHING.nothingListed;
  if (!catalog.listed) line = NOTHING.offline;
  else if (why === "not_apple_silicon") line = NOTHING.notAppleSilicon;
  else if (why === "not_enough_memory") line = NOTHING.notEnoughMemory(memory);
  return (
    <div className="flex flex-col gap-1 rounded-2xl bg-surface px-5 py-4.5 inset-ring-1 inset-ring-hairline/75" data-testid="models-nothing">
      <h4 className="text-section font-semibold text-ink">{NOTHING.title}</h4>
      <p className="text-body text-ink-2">{line}</p>
    </div>
  );
}

/** Turning on Apple Intelligence would make Apple's model the pick. */
export function AppleTip({ onOpen }: { onOpen: () => void }): JSX.Element {
  return (
    <Alert level="info" testId="models-apple-tip">
      {RECOMMENDED.tipAppleLead} <span className="font-semibold text-ink">{RECOMMENDED.tipAppleTitle}</span>
      {RECOMMENDED.tipAppleRest}{" "}
      <button type="button" className="font-medium text-accent-hi hover:underline" onClick={onOpen}>
        {RECOMMENDED.openSystemSettings}
      </button>
    </Alert>
  );
}

export function RecommendedSection({ children }: { children: ReactNode }): JSX.Element {
  return (
    <ModelsSection id="ai-recommended" title={RECOMMENDED.title} help={RECOMMENDED.help}>
      {children}
    </ModelsSection>
  );
}

