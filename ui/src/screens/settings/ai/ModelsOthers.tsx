/**
 * AJ-E3812 — Other tested models: one card per model this Mac can run
 * (D-127), smallest first. The facts first (download, the Mac it needs,
 * speed on our test Mac), then each job it handles with its verdict in
 * words and the list's line, the jobs not yet folded into one line, at
 * most one tag, its licence, and Download with its size. Short of free
 * disk, the card stays and says what it needs, its Download held.
 */
import type { JSX } from "react";
import { Disclosure } from "../../../components/controls";
import { Button } from "../../../components/primitives";
import { Pill } from "../../../components/Pill";
import { IconUpRight } from "../../../components/icons-places";
import type { CatalogModel, ModelsCatalog } from "../../../lib/ipc/models";
import { trustOpen } from "../../../lib/ipc/trust";
import { IconDisk, IconDownload, IconLock, IconOffline } from "./model-icons";
import { CARD, HonestyLine, JobVerdictRow, ModelFacts, ModelTile, ModelsSection } from "./ModelParts";
import { jobsOf, verdictOf, type ModelTag } from "./models-view";
import { DISK, DOWNLOAD, JOB_NAMES, OTHERS, listWords } from "./models-words";

const TAG_WORDS: Record<ModelTag, string> = { smallest: OTHERS.smallest, fastest: OTHERS.fastest };

function Licence({ model }: { model: CatalogModel }): JSX.Element | null {
  if (!model.licence_name) return null;
  return (
    <button
      type="button"
      className="inline-flex items-center gap-0.5 text-label font-medium text-accent-hi hover:underline"
      onClick={() => void trustOpen({ kind: "licence", id: model.id })}
      aria-label={OTHERS.licence(model.licence_name)}
      title={model.licence_url ?? undefined}
    >
      {model.licence_name}
      <IconUpRight width={11} height={11} aria-hidden="true" />
    </button>
  );
}

export function ModelCard({
  model,
  tag,
  free,
  onDownload,
  busy,
}: {
  model: CatalogModel;
  tag?: ModelTag;
  /** This Mac's free disk, bytes. */
  free: number;
  onDownload: (model: CatalogModel) => void;
  /** Another download is running. */
  busy: boolean;
}): JSX.Element {
  const { passed, notYet } = jobsOf(model);
  const short = free < model.free_disk_needed_bytes;
  return (
    <article className={`flex flex-col gap-3 px-4 pt-4 pb-3.5 ${CARD}`} aria-labelledby={`model-${model.id}`} data-testid="model-card" data-model={model.id}>
      <div className="flex items-center gap-3">
        <ModelTile model={model} />
        <div className="flex min-w-0 flex-1 flex-col gap-px">
          <h4 id={`model-${model.id}`} className="text-section font-semibold text-ink">
            {model.name}
          </h4>
          <Licence model={model} />
        </div>
        {tag && <Pill tone="accent">{TAG_WORDS[tag]}</Pill>}
      </div>
      <ModelFacts model={model} inCard />
      <ul className="flex flex-col gap-2.5">
        {passed.map((job) => {
          const verdict = verdictOf(model, job);
          return <JobVerdictRow key={job} job={job} verdict={verdict.verdict} note={verdict.note} />;
        })}
      </ul>
      {notYet.length > 0 && (
        <Disclosure summary={OTHERS.notYet(listWords(notYet.map((job) => JOB_NAMES[job].toLowerCase())))} className="pl-0.5" testId="model-not-yet">
          <ul className="mt-2 flex flex-col gap-2.5">
            {notYet.map((job) => {
              const verdict = verdictOf(model, job);
              return <JobVerdictRow key={job} job={job} verdict={verdict.verdict} note={verdict.note} quiet />;
            })}
          </ul>
        </Disclosure>
      )}
      <div className="mt-auto flex items-center gap-2 border-t border-divider pt-3">
        <span className="flex min-w-0 flex-1 items-center gap-1.5 text-label text-muted">
          {short ? <IconDisk width={13} height={13} aria-hidden="true" /> : <IconLock width={13} height={13} aria-hidden="true" />}
          {short ? DISK.cardNeeds(model.free_disk_needed_bytes) : OTHERS.stays}
        </span>
        <Button size="sm" disabled={short || busy} onClick={() => onDownload(model)} data-testid="model-download">
          <IconDownload width={13} height={13} aria-hidden="true" />
          {DOWNLOAD.button(model.size_bytes ?? 0)}
        </Button>
      </div>
    </article>
  );
}

/** The list can't be read: only Apple's model, and what still works. */
function Unread({ builtIn }: { builtIn: boolean }): JSX.Element {
  const offline = !navigator.onLine;
  let line = builtIn ? OTHERS.unreadLater : OTHERS.unreadNoBuiltIn;
  if (offline && builtIn) line = OTHERS.unreadOffline;
  return (
    <div className={`flex items-center gap-4 px-5 py-5 ${CARD}`} data-testid="models-unread">
      <span aria-hidden="true" className="flex size-12 shrink-0 items-center justify-center rounded-[13px] bg-surface-3 text-ink-2">
        <IconOffline width={22} height={22} />
      </span>
      <div className="flex min-w-0 flex-col gap-0.5">
        <h4 className="text-section font-semibold text-ink">{OTHERS.unreadTitle}</h4>
        <p className="text-meta text-muted">{line}</p>
      </div>
    </div>
  );
}

export function OthersSection({
  catalog,
  models,
  tags,
  shortOfDisk,
  onDownload,
  busy,
}: {
  catalog: ModelsCatalog;
  models: CatalogModel[];
  tags: Map<string, ModelTag>;
  /** The recommended one is held for disk: these need more too. */
  shortOfDisk: boolean;
  onDownload: (model: CatalogModel) => void;
  busy: boolean;
}): JSX.Element | null {
  if (catalog.listed && models.length === 0) return null;
  let help = OTHERS.help;
  if (shortOfDisk) help = OTHERS.helpDisk;
  else if (catalog.recommendation?.why === "fits" || catalog.recommendation?.why === "installed") help = OTHERS.helpFits;
  return (
    <ModelsSection id="ai-others" title={OTHERS.title} help={catalog.listed ? help : undefined}>
      {catalog.listed ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] items-start gap-3.5">
          {models.map((model) => (
            <ModelCard key={model.id} model={model} tag={tags.get(model.id)} free={catalog.mac?.free_disk_bytes ?? 0} onDownload={onDownload} busy={busy} />
          ))}
        </div>
      ) : (
        <Unread builtIn={catalog.models.some((model) => model.built_in)} />
      )}
      {catalog.listed && <HonestyLine />}
    </ModelsSection>
  );
}
