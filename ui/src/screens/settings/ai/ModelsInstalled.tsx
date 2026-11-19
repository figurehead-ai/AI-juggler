/**
 * AJ-E3812 (AJ-E3803's Settings rows) — Installed: the total AI models
 * use with the free disk, a bar of each model's share once two or more
 * are here, then the Settings storage row per model (its size, Remove).
 * Apple's model shows "No disk" and has no Remove. A model the list no
 * longer has keeps working, with ADR-0035 §11.4's note. Remove asks
 * first, saying what it frees, which jobs go back to Automatic, and
 * whether it can be downloaded again; red appears only there.
 */
import { useState, type JSX } from "react";
import { Button } from "../../../components/primitives";
import { IconInfo, IconTrash } from "../../../components/icons-actions";
import type { AiJobInfo } from "../../../lib/ipc/ai";
import type { CatalogModel, InstalledModelInfo, ModelsInfo } from "../../../lib/ipc/models";
import { IconDisk } from "./model-icons";
import { CARD, ModelTile, ModelsSection } from "./ModelParts";
import { dayOf } from "./models-view";
import { INSTALLED, JOB_NAMES, PAGE, gb, listWords } from "./models-words";

/** The jobs that run on `id` now, in words ("Turn reviews, Chat answers"). */
function usedBy(id: string, jobs: readonly AiJobInfo[] | null): string[] {
  return (jobs ?? []).filter((info) => info.model_id === id && (info.state === "ready" || info.state === "waiting")).map((info) => JOB_NAMES[info.job]);
}

/** The jobs picked for `id`: they go back to Automatic when it goes. */
function pickedFor(id: string, jobs: readonly AiJobInfo[] | null): string[] {
  return (jobs ?? []).filter((info) => info.pick === id).map((info) => JOB_NAMES[info.job]);
}

const SHARES = ["bg-accent-fill", "bg-[color-mix(in_srgb,var(--color-accent)_45%,var(--color-surface-3))]", "bg-live", "bg-faint"];

function DiskBar({ models, total }: { models: InstalledModelInfo[]; total: number }): JSX.Element {
  return (
    <>
      <div className="flex h-2.5 gap-0.5 overflow-hidden rounded-full" aria-hidden="true">
        {models.map((model, index) => (
          <span key={model.id} className={`block h-full ${SHARES[index % SHARES.length]}`} style={{ width: `${(model.size_bytes / Math.max(total, 1)) * 100}%` }} />
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        <ul className="flex min-w-0 flex-1 flex-wrap gap-x-4 gap-y-1 text-label text-muted">
          {models.map((model, index) => (
            <li key={model.id} className="inline-flex items-center gap-1.5">
              <span aria-hidden="true" className={`inline-block size-[9px] rounded-[3px] ${SHARES[index % SHARES.length]}`} />
              {model.name} · {gb(model.size_bytes)}
            </li>
          ))}
        </ul>
        <span className="text-label text-faint">{INSTALLED.neverCleared}</span>
      </div>
    </>
  );
}

function Confirm({ model, jobs, automaticName, onKeep, onRemove }: { model: InstalledModelInfo; jobs: readonly AiJobInfo[] | null; automaticName: string | null; onKeep: () => void; onRemove: () => void }): JSX.Element {
  const back = pickedFor(model.id, jobs);
  const gone = model.standing === "withdrawn";
  return (
    <section className="mt-1 mb-0.5 ml-[52px] flex flex-col gap-2.5 rounded-xl bg-surface-2 px-3.5 py-3 text-meta text-ink-2" aria-labelledby={`remove-${model.id}`} data-testid="models-remove-confirm">
      <p>
        <span id={`remove-${model.id}`} className="font-semibold text-ink">
          {INSTALLED.confirmTitle(model.name)}
        </span>{" "}
        {INSTALLED.confirmFrees(model.size_bytes)} {back.length > 0 ? INSTALLED.confirmBack(listWords(back), automaticName) : INSTALLED.confirmHides}{" "}
        {gone ? INSTALLED.confirmGone : INSTALLED.confirmAgain}
      </p>
      <span className="flex gap-2">
        <Button size="sm" onClick={onKeep} data-autofocus>
          {INSTALLED.keep}
        </Button>
        <Button size="sm" variant="danger" onClick={onRemove} data-testid="models-remove-confirmed">
          <IconTrash width={13} height={13} aria-hidden="true" />
          {INSTALLED.removeSized(model.size_bytes)}
        </Button>
      </span>
    </section>
  );
}

function Row({ tile, name, line, right, children }: { tile: JSX.Element; name: string; line: string; right: JSX.Element; children?: JSX.Element | false }): JSX.Element {
  return (
    <div className="flex items-center gap-3 px-3 py-2.5" data-testid="models-installed-row">
      {tile}
      <div className="flex min-w-0 flex-1 flex-col gap-px">
        <span className="text-body font-medium text-ink">{name}</span>
        <span className="text-label text-muted">{line}</span>
        {children}
      </div>
      {right}
    </div>
  );
}

export function InstalledSection({
  info,
  builtIn,
  jobs,
  automaticName,
  free,
  offline,
  onRemove,
}: {
  info: ModelsInfo;
  /** Apple's model, when this Mac shows it. */
  builtIn: CatalogModel | null;
  jobs: readonly AiJobInfo[] | null;
  /** The model Automatic would use instead (the confirm's words). */
  automaticName: string | null;
  free: number | null;
  offline: boolean;
  onRemove: (id: string) => void;
}): JSX.Element {
  const [asking, setAsking] = useState<string | null>(null);
  const count = info.installed.length;
  let help = count === 0 ? INSTALLED.helpNone : INSTALLED.helpCount(count);
  if (count > 0 && offline) help = `${help} · ${INSTALLED.helpOffline}`;
  else if (count > 1 && builtIn) help = `${INSTALLED.helpCount(count)}, ${INSTALLED.helpWithBuiltIn}`;
  let legend = builtIn ? INSTALLED.legendNone : INSTALLED.legendOne;
  if (count === 1 && info.installed[0]) legend = offline ? INSTALLED.legendOffline(info.installed[0].name) : INSTALLED.legendOne;
  const builtInUse = builtIn ? usedBy(builtIn.id, jobs) : [];
  return (
    <ModelsSection id="ai-installed" title={INSTALLED.title} help={help}>
      <div className={`overflow-hidden ${CARD}`} data-testid="models-installed">
        <div className="flex flex-col gap-2.5 border-b border-divider px-4.5 pt-4 pb-3.5">
          <p className="flex flex-wrap items-baseline gap-2.5">
            <span className="text-meta text-muted">{INSTALLED.use}</span>
            <span className="text-lg font-semibold text-ink tabular-nums" data-testid="models-total">
              {gb(info.total_bytes)}
            </span>
            <span className="text-meta text-muted">{INSTALLED.onThisMac}</span>
            {free !== null && <span className="ml-auto text-label text-muted">{PAGE.free(free)}</span>}
          </p>
          {count >= 2 ? <DiskBar models={info.installed} total={info.total_bytes} /> : <span className="text-label text-muted">{legend}</span>}
        </div>
        <ul className="flex flex-col divide-y divide-divider p-1.5">
          {builtIn && (
            <li>
            <Row
              tile={<ModelTile model={builtIn} size="sm" />}
              name={builtIn.name}
              line={`${INSTALLED.builtIn} · ${builtInUse.length > 0 ? INSTALLED.usedBy(listWords(builtInUse)) : INSTALLED.readyToUse}`}
              right={<span className="w-[150px] text-meta text-muted">{INSTALLED.noDisk}</span>}
            />
            </li>
          )}
          {!builtIn && count === 0 && (
            <li>
            <Row
              tile={
                <span aria-hidden="true" className="flex size-7 items-center justify-center rounded-lg bg-surface-3 text-ink-2">
                  <IconDisk width={15} height={15} />
                </span>
              }
              name={INSTALLED.noModelsTitle}
              line={INSTALLED.noModelsLine}
              right={<span />}
            />
            </li>
          )}
          {info.installed.map((model) => {
            const use = usedBy(model.id, jobs);
            return (
              <li key={model.id} className="flex flex-col">
                <Row
                  tile={<ModelTile model={{ name: model.name, built_in: false }} size="sm" />}
                  name={model.name}
                  line={`${INSTALLED.downloaded(dayOf(model.installed_at_ms))} · ${use.length > 0 ? INSTALLED.usedBy(listWords(use)) : INSTALLED.usedByNothing}`}
                  right={
                    <span className="flex items-center gap-3">
                      <span className="min-w-14 text-right text-meta font-semibold text-ink tabular-nums">{gb(model.size_bytes)}</span>
                      <Button size="sm" onClick={() => setAsking(asking === model.id ? null : model.id)} aria-expanded={asking === model.id} data-testid="models-remove">
                        <IconTrash width={13} height={13} aria-hidden="true" />
                        {INSTALLED.remove}
                      </Button>
                    </span>
                  }
                >
                  {model.standing === "withdrawn" && (
                    <span className="mt-1 flex items-center gap-1.5 text-label text-ink-2" data-testid="models-withdrawn">
                      <IconInfo width={13} height={13} aria-hidden="true" />
                      {INSTALLED.withdrawn}
                    </span>
                  )}
                </Row>
                {asking === model.id && (
                  <Confirm
                    model={model}
                    jobs={jobs}
                    automaticName={automaticName}
                    onKeep={() => setAsking(null)}
                    onRemove={() => {
                      setAsking(null);
                      onRemove(model.id);
                    }}
                  />
                )}
              </li>
            );
          })}
        </ul>
      </div>
    </ModelsSection>
  );
}
