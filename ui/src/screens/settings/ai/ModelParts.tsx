/**
 * AJ-E3812 — the small parts every Models card is made of, on the D-126
 * kit's tokens: a model's tile (two letters, or a chip for Apple's), its
 * facts strip (download, memory, speed), a verdict in words and colour
 * ("Works well" green, "Beta" accent, "Not yet" muted, never a warning),
 * a job's verdict row, a section with its one-line help, and the honesty
 * line by the verdicts.
 */
import type { JSX, ReactNode } from "react";
import { trustOpen } from "../../../lib/ipc/trust";
import type { CatalogModel } from "../../../lib/ipc/models";
import type { ModelJob } from "../../../types/generated/ModelJob";
import type { ModelSpeed } from "../../../types/generated/ModelSpeed";
import type { ModelVerdict } from "../../../types/generated/ModelVerdict";
import { IconInfo } from "../../../components/icons-actions";
import { IconChip, JobIcon } from "./model-icons";
import { initials } from "./models-view";
import { FACTS, HONESTY, JOB_NAMES, PAGE, SPEED_WORDS, VERDICT_WORDS, gb } from "./models-words";

/** The guide page "How we test models" opens. */
export const HOW_WE_TEST = "ai-models#how-we-test-models";

/** A card's edge: the kit's hairline at three quarters. */
export const CARD = "rounded-[14px] bg-surface inset-ring-1 inset-ring-hairline/75";

const TILE_SIZES = { sm: "size-7 rounded-lg text-[11px]", md: "size-9 rounded-[10px] text-meta", lg: "size-12 rounded-[13px] text-base" } as const;
const CHIP_SIZES = { sm: 15, md: 19, lg: 24 } as const;

/** A model's tile: Apple's a chip, the rest two letters; `on` tinted. */
export function ModelTile({ model, size = "md", on = false }: { model: Pick<CatalogModel, "name" | "built_in">; size?: keyof typeof TILE_SIZES; on?: boolean }): JSX.Element {
  const tone = on ? "bg-[color-mix(in_srgb,var(--color-accent)_16%,var(--color-surface))] text-accent-hi" : "bg-surface-3 text-ink-2";
  return (
    <span aria-hidden="true" className={`flex shrink-0 items-center justify-center font-semibold ${TILE_SIZES[size]} ${tone}`}>
      {model.built_in ? <IconChip width={CHIP_SIZES[size]} height={CHIP_SIZES[size]} /> : initials(model.name)}
    </span>
  );
}

const VERDICT_TONES: Record<ModelVerdict, string> = {
  ships: "font-semibold text-success",
  beta: "font-semibold text-accent-hi",
  not_yet: "font-medium text-muted",
  not_tested: "font-medium text-muted",
};

export function Verdict({ verdict }: { verdict: ModelVerdict }): JSX.Element {
  return <span className={VERDICT_TONES[verdict]}>{VERDICT_WORDS[verdict]}</span>;
}

const SPEED_BARS: Record<ModelSpeed, number> = { quick: 3, moderate: 2, slow: 1 };

function SpeedBars({ speed }: { speed: ModelSpeed }): JSX.Element {
  const on = SPEED_BARS[speed];
  return (
    <span aria-hidden="true" className="inline-flex h-3 items-end gap-0.5">
      {[5, 8, 12].map((height, index) => (
        <i key={height} className={`block w-1 rounded-[1.5px] ${index < on ? "bg-ink-2" : "bg-surface-3"}`} style={{ height }} />
      ))}
    </span>
  );
}

function Fact({ name, value, note }: { name: string; value: ReactNode; note: string }): JSX.Element {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-0.5 px-3 py-2">
      <span className="text-[11.5px] tracking-[.02em] text-muted">{name}</span>
      <span className="flex items-center gap-1.5 whitespace-nowrap text-meta font-semibold text-ink">{value}</span>
      <span className="truncate text-label text-muted">{note}</span>
    </div>
  );
}

/** The facts strip: what it costs to get, the Mac it needs, how fast. */
export function ModelFacts({ model, inCard = false }: { model: CatalogModel; inCard?: boolean }): JSX.Element {
  const look = inCard ? "rounded-xl bg-surface-2" : "rounded-xl bg-surface inset-ring-1 inset-ring-hairline/75";
  return (
    <div className={`flex divide-x divide-divider ${look}`} data-testid="model-facts">
      <Fact name={FACTS.download} value={model.size_bytes ? gb(model.size_bytes) : FACTS.none} note={model.size_bytes ? FACTS.oneTime : FACTS.builtIn} />
      <Fact
        name={FACTS.memory}
        value={model.built_in ? FACTS.shared : FACTS.memoryFloor(model.min_memory_gb)}
        note={model.built_in ? FACTS.withMacos : FACTS.memoryFloorNote}
      />
      {model.speed && (
        <Fact
          name={FACTS.speed}
          value={
            <>
              <SpeedBars speed={model.speed} />
              {SPEED_WORDS[model.speed]}
            </>
          }
          note={FACTS.speedNote}
        />
      )}
    </div>
  );
}

/** A job with its verdict, and the list's line on why under it. */
export function JobVerdictRow({ job, verdict, note, quiet = false }: { job: ModelJob; verdict: ModelVerdict; note: string | null; quiet?: boolean }): JSX.Element {
  return (
    <li className="grid grid-cols-[20px_1fr_auto] items-center gap-x-2 gap-y-px" data-testid={`verdict-${job}`}>
      <span className="text-faint">
        <JobIcon job={job} />
      </span>
      <span className={`text-meta font-medium ${quiet ? "text-ink-2" : "text-ink"}`}>{JOB_NAMES[job]}</span>
      <span className="text-label">
        <Verdict verdict={verdict} />
      </span>
      {note && <span className="col-span-2 col-start-2 text-label leading-snug text-muted">{note}</span>}
    </li>
  );
}

/** A group of the screen: its title, one line of help, then its body;
 * `searchable` (the default) makes it a place Settings' search lands. */
export function ModelsSection({ id, title, help, searchable = true, children }: { id: string; title: string; help?: ReactNode; searchable?: boolean; children: ReactNode }): JSX.Element {
  return (
    <section id={`settings-${id}`} aria-labelledby={`settings-${id}-title`} className="flex scroll-mt-4 flex-col gap-3.5" data-settings-section={searchable ? id : undefined}>
      <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-0.5">
        <h3 id={`settings-${id}-title`} tabIndex={-1} className="rounded-md text-section font-semibold text-ink">
          {title}
        </h3>
        {help && <p className="text-meta text-muted">{help}</p>}
      </div>
      {children}
    </section>
  );
}

/** "How we test models": the guide's page, in the browser. */
export function HowWeTest({ className = "" }: { className?: string }): JSX.Element {
  return (
    <button type="button" className={`font-medium text-accent-hi hover:underline ${className}`} onClick={() => void trustOpen({ kind: "guide", id: HOW_WE_TEST })} data-testid="models-how-we-test">
      {PAGE.howWeTest}
    </button>
  );
}

/** The honesty line, by the verdicts: synthetic tests, not real messages. */
export function HonestyLine(): JSX.Element {
  return (
    <p className="flex items-start gap-2 px-1 text-label text-muted" data-testid="models-honesty">
      <IconInfo width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" />
      <span>
        {HONESTY} <HowWeTest />
      </span>
    </p>
  );
}
