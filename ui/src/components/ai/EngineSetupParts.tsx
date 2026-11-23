/**
 * AJ-E3823 — the engine-setup guide's parts (`EngineSetup.tsx` puts them
 * together): a choice card with its points, Check again, Apple's and the
 * downloadable AI's actions by state, the recommendation line, and "What
 * is this?". Kit parts only: the Models screen's tile, facts, licence,
 * download progress and Download words.
 */
import type { JSX, ReactNode } from "react";
import { Alert } from "../Alert";
import { Disclosure } from "../controls";
import { IconCheck } from "../icons-controls";
import { Button } from "../primitives";
import { Pill } from "../Pill";
import { IconSparkle } from "../bot/bot-icons";
import { useModelsInstalled } from "../../lib/ai-models";
import { Downloading } from "../../screens/settings/ai/ModelsRecommended";
import { IconRefresh } from "../../screens/settings/ai/model-icons";
import type { ModelActions } from "../../screens/settings/ai/models-actions";
import { DOWNLOAD, FAILED_AFTER, FAILURES } from "../../screens/settings/ai/models-words";
import { unknown, type SetupView } from "./engine-setup-view";
import { APPLE, DOWNLOADED, RECOMMEND, SETUP, WHAT, originOf } from "./engine-setup-words";

/** One point of a choice: a benefit (a green check), or what it needs,
 * checked only once it is met (never a check on an unmet need). */
function Point({ children, need = false, met = true }: { children: ReactNode; need?: boolean; met?: boolean }): JSX.Element {
  const mark = !need || met;
  return (
    <li className="flex items-start gap-2 text-meta leading-snug text-ink-2">
      <span aria-hidden="true" className={"mt-px flex size-[16px] shrink-0 items-center justify-center rounded-full " + (mark ? "bg-success/20 text-success" : "bg-surface-3 text-muted")}>
        {mark ? <IconCheck width={10} height={10} strokeWidth={3} /> : <span className="size-1.5 rounded-full bg-current" />}
      </span>
      {children}
    </li>
  );
}

export function Consequence({ children }: { children: ReactNode }): JSX.Element {
  return <span className="text-[12.5px] leading-snug text-muted">{children}</span>;
}

/** A choice: tile, name, sub-line, badge, facts, its state, its points
 * and its action. The recommended one carries the accent edge. */
export function Choice({
  titleId,
  tile,
  name,
  sub,
  recommended,
  state,
  stateTone,
  facts,
  points,
  need,
  children,
  testId,
}: {
  titleId: string;
  tile: ReactNode;
  name: string;
  sub: ReactNode;
  recommended: boolean;
  state: string;
  stateTone: "ok" | "wait" | "warn";
  facts: ReactNode;
  points: readonly string[];
  /** The last point is what it needs; `met` says whether it is. */
  need?: { met: boolean };
  children: ReactNode;
  testId: string;
}): JSX.Element {
  const dot = { ok: "bg-success", wait: "bg-accent", warn: "bg-warning" }[stateTone];
  return (
    <article
      aria-labelledby={titleId}
      className={
        "relative flex min-w-0 flex-col gap-3 rounded-2xl px-4 pt-4 pb-4 " +
        (recommended ? "bg-[color-mix(in_srgb,var(--color-accent)_7%,var(--color-surface))] shadow-card" : "bg-surface inset-ring-1 inset-ring-hairline/75")
      }
      data-testid={testId}
      data-recommended={recommended ? "" : undefined}
    >
      <div className="flex items-start gap-3">
        {tile}
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex flex-wrap items-center gap-2">
            <h4 id={titleId} className="text-section font-semibold text-ink">
              {name}
            </h4>
            {recommended && <Pill tone="accent">{SETUP.recommended}</Pill>}
          </div>
          <span className="text-meta text-muted">{sub}</span>
        </div>
      </div>
      <p className="flex items-start gap-2 text-meta font-medium text-ink" data-testid={`${testId}-state`}>
        <span aria-hidden="true" className={`mt-[5px] size-2 shrink-0 rounded-full ${dot}`} />
        {state}
      </p>
      {facts}
      <ul className="flex flex-col gap-1.5" aria-label={name}>
        {points.map((point, i) => (
          <Point key={point} need={!!need && i === points.length - 1} met={need?.met ?? true}>
            {point}
          </Point>
        ))}
      </ul>
      <div className="mt-auto flex flex-col gap-2.5 pt-1">{children}</div>
      {recommended && <span aria-hidden="true" className="pointer-events-none absolute inset-0 rounded-2xl inset-ring-[1.5px] inset-ring-accent" />}
    </article>
  );
}

function CheckAgain({ act, primary }: { act: ModelActions; primary: boolean }): JSX.Element {
  const words = act.checking ? SETUP.checkingAgain : SETUP.checkAgain;
  if (primary) {
    return (
      <Button variant="primary" onClick={act.checkAgain} disabled={act.checking} className="self-start" data-testid="setup-check-again">
        <IconRefresh width={15} height={15} aria-hidden="true" />
        {words}
      </Button>
    );
  }
  return (
    <button type="button" onClick={act.checkAgain} disabled={act.checking} className="inline-flex items-center gap-1 self-start rounded-md text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="setup-check-again">
      <IconRefresh width={12} height={12} aria-hidden="true" />
      {words}
    </button>
  );
}

/** Apple's card's body: what to do in its state. */
export function AppleAction({ view, act, onAdopt, notSetUp = false }: { view: SetupView; act: ModelActions; onAdopt: () => void; notSetUp?: boolean }): JSX.Element | null {
  // AJ-E1547: never checked because its helper isn't set up yet: the
  // check is the helper's first start, told first through the gate.
  if (view.apple === "unchecked" && notSetUp) {
    return (
      <div className="flex flex-col gap-2.5">
        <p className="text-meta leading-snug text-ink-2" data-testid="setup-apple-why">
          {APPLE.setUpLine}
        </p>
        <Button variant="primary" onClick={act.checkAgain} disabled={act.checking} className="self-start" data-testid="setup-apple-set-up">
          {act.checking ? SETUP.checkingAgain : APPLE.setUp}
        </Button>
      </div>
    );
  }
  switch (view.apple) {
    case "ready":
      return view.appleInUse ? (
        <InUse />
      ) : (
        <div className="flex flex-col gap-1.5">
          <Button variant="primary" onClick={onAdopt} className="self-start" data-testid="setup-use-apple">
            <IconCheck width={15} height={15} strokeWidth={2.2} aria-hidden="true" />
            {APPLE.adopt}
          </Button>
          <Consequence>{APPLE.adoptLine}</Consequence>
        </div>
      );
    case "off":
      return (
        <div className="flex flex-col gap-2.5">
          <div className="flex flex-col gap-1">
            <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{APPLE.stepsLabel}</span>
            <ol className="flex list-decimal flex-col gap-1 pl-5 text-meta leading-snug text-ink-2 marker:text-muted" data-testid="setup-apple-steps">
              {APPLE.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button variant="primary" onClick={act.openAppleIntelligence} data-testid="setup-open-apple">
              {APPLE.open}
            </Button>
            <CheckAgain act={act} primary={false} />
          </div>
        </div>
      );
    case "preparing":
      return (
        <div className="flex flex-col gap-2">
          <Consequence>{APPLE.preparingLine}</Consequence>
          <CheckAgain act={act} primary={false} />
        </div>
      );
    case "unchecked":
    case "no_answer":
    case "blocked": {
      const line = { unchecked: APPLE.uncheckedLine, no_answer: APPLE.noAnswerLine, blocked: APPLE.blockedLine }[view.apple];
      return (
        <div className="flex flex-col gap-2.5">
          <p className="text-meta leading-snug text-ink-2" data-testid="setup-apple-why">
            {line}
          </p>
          <CheckAgain act={act} primary />
        </div>
      );
    }
    default:
      return null;
  }
}

function InUse(): JSX.Element {
  return (
    <span className="self-start">
      <Pill tone="accent">
        <IconCheck width={12} height={12} strokeWidth={2.6} aria-hidden="true" className="mr-1" />
        {SETUP.inUse}
      </Pill>
    </span>
  );
}

/** The downloadable AI's card body. */
export function DownloadAction({ view, act, onAdopt, onDownload }: { view: SetupView; act: ModelActions; onAdopt: () => void; onDownload: () => void }): JSX.Element | null {
  const model = view.download;
  const installed = useModelsInstalled();
  if (!model) return null;
  const size = model.size_bytes ?? 0;
  const primary = view.recommended === "download";
  const download = installed?.download;
  const failed = download?.state === "failed" && download.id === model.id && download.failure;
  switch (view.downloadAction) {
    case "in-use":
      return <InUse />;
    case "use":
      return (
        <div className="flex flex-col gap-1.5">
          <Button variant={primary ? "primary" : "secondary"} onClick={onAdopt} className="self-start" data-testid="setup-use-download">
            {DOWNLOADED.adopt(model.name)}
          </Button>
          <Consequence>{DOWNLOADED.adoptLine}</Consequence>
        </div>
      );
    case "downloading":
      return download ? <Downloading name={model.name} download={download} onCancel={() => act.cancel(model.id)} /> : null;
    case "short":
      return (
        <div className="flex flex-col gap-2" data-testid="setup-short">
          <p className="text-meta leading-snug text-ink-2">{DOWNLOADED.short(size, model.free_disk_needed_bytes, view.free)}</p>
          <span className="flex flex-wrap gap-4">
            <button type="button" className="text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={act.openStorage}>
              {DOWNLOADED.openStorage}
            </button>
            <CheckAgain act={act} primary={false} />
          </span>
        </div>
      );
    default:
      return (
        <div className="flex flex-col gap-1.5">
          {failed && (
            <Alert level="broken" title={FAILURES[failed]}>
              {FAILED_AFTER}
            </Alert>
          )}
          <Button variant={primary ? "primary" : "secondary"} onClick={onDownload} className="self-start" data-testid="setup-download">
            {DOWNLOAD.button(model.name, size)}
          </Button>
          <Consequence>{DOWNLOADED.downloadLine}</Consequence>
        </div>
      );
  }
}

/** The line above the choices: the recommendation, or why there's none. */
export function Recommendation({ view }: { view: SetupView }): JSX.Element | null {
  const name = view.download?.name ?? null;
  let words: string | null = null;
  if (view.apple === "ready") words = RECOMMEND.ready;
  else if (view.apple === "off") words = RECOMMEND.off;
  else if (view.apple === "preparing") words = RECOMMEND.preparing;
  else if (view.apple === "cant_run" && name) words = RECOMMEND.download(name);
  else if (unknown(view.apple)) words = RECOMMEND.unknown(name);
  if (!words) return null;
  const known = !unknown(view.apple);
  return (
    <p
      className={"flex items-start gap-2.5 rounded-xl px-3.5 py-2.5 text-body leading-snug " + (known ? "bg-accent-soft text-ink" : "bg-surface-2 text-ink-2")}
      data-testid="setup-recommendation"
      data-known={known ? "" : undefined}
    >
      <IconSparkle width={16} height={16} aria-hidden="true" className={"mt-0.5 shrink-0 " + (known ? "text-accent-hi" : "text-faint")} />
      <span>{words}</span>
    </p>
  );
}

export function What({ view }: { view: SetupView }): JSX.Element {
  const model = view.download;
  const origin = originOf(model?.licence_url ?? null);
  const name = model?.name ?? null;
  return (
    <Disclosure summary={WHAT.summary} testId="setup-what">
      <ul className="mt-2 flex flex-col gap-1.5 pl-5 text-meta leading-snug text-ink-2" data-testid="setup-what-body">
        <li>{WHAT.what}</li>
        <li>{WHAT.kept(name, model?.size_bytes ?? null)}</li>
        <li>{WHAT.from(name, origin.maker, origin.host)}</li>
        <li>{WHAT.private}</li>
        <li>{WHAT.remove(name)}</li>
      </ul>
    </Disclosure>
  );
}

