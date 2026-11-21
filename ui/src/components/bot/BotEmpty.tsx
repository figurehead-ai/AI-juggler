/**
 * D-136 point 1, AJ-E3807 — before the first question: what Bot does
 * today (a short can and can't list: it doesn't talk to your agents),
 * three questions to try, and the quiet Beta row where a model passes
 * (D-127: never shown where it can't work). With no model on this Mac:
 * look-ups still work (the box stays), the screens that already answer
 * the common questions, and one offer of the small AI file, with its size,
 * "stays on this Mac" and a link to Settings › AI › Models.
 */
import { useState, type JSX } from "react";
import { IconClose } from "../icons-controls";
import { LogoTile } from "../ToolLogo";
import { IconArrowRight, IconCheckCircle, IconDownload, IconEye, IconHourglass, IconMessage, IconSparkle } from "./bot-icons";
import { BetaPill } from "./BotWritten";
import { Shortcut } from "./BotTurn";
import { useBotNav } from "./bot-nav";
import { WORDS } from "../../lib/bot-words";
import { ask } from "../../lib/bot-store";
import { aiBetaSet, type AiJobInfo } from "../../lib/ipc/ai";
import { aiModelDownloadStart } from "../../lib/ipc/models";
import { useModelsCatalog } from "../../lib/ai-models";
import { gb } from "../../screens/settings/ai/models-words";
import { readKept, keep } from "../../lib/local-store";
import { IconCheck } from "../icons-controls";

const EXAMPLE_ICONS = [IconMessage, IconCheckCircle, IconHourglass];

function Hero({ title, lead }: { title: string; lead: string }): JSX.Element {
  return (
    <div className="flex flex-col gap-2 px-0.5 pt-1.5">
      <span className="text-label font-semibold tracking-[.07em] text-accent-hi uppercase">{WORDS.eyebrow}</span>
      <h3 className="text-xl leading-tight font-semibold tracking-[-.01em] text-ink">{title}</h3>
      <p className="text-body text-muted">{lead}</p>
    </div>
  );
}

export function BotEmpty({ job }: { job: AiJobInfo | null }): JSX.Element {
  const offerBeta = job?.state === "beta_off";
  return (
    <div className="flex flex-col gap-[18px]" data-testid="bot-empty">
      <Hero title={WORDS.emptyTitle} lead={WORDS.emptyLead} />
      <ul className="flex flex-col gap-0.5 rounded-xl bg-surface-2 px-1.5 py-2" aria-label="What Bot does today">
        {WORDS.can.map((line) => (
          <li key={line.text} className="flex items-start gap-2.5 px-2 py-[5px] text-[13.5px] leading-snug text-ink-2">
            {line.mark === "yes" && <IconCheck width={15} height={15} aria-label="Does" className="mt-px shrink-0 text-success" />}
            {line.mark === "no" && <IconClose width={15} height={15} aria-label="Doesn’t" className="mt-px shrink-0 text-faint" />}
            {line.mark === "later" && <IconSparkle width={15} height={15} aria-hidden="true" className="mt-px shrink-0 text-accent-hi" />}
            <span className={line.mark === "later" ? "text-muted" : ""}>{line.text}</span>
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-2">
        <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{WORDS.tryOne}</span>
        <div className="flex flex-col gap-1.5">
          {WORDS.examples.map((example, i) => {
            const Icon = EXAMPLE_ICONS[i] ?? IconMessage;
            return (
              <button key={example} type="button" className="flex items-center gap-2.5 rounded-xl bg-surface-2 px-3 py-2.5 text-left text-body text-ink outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void ask(example)} data-testid="bot-example">
                <Icon width={16} height={16} aria-hidden="true" className="shrink-0 text-faint" />
                <span className="grow">{example}</span>
                <IconArrowRight width={14} height={14} aria-hidden="true" className="text-faint" />
              </button>
            );
          })}
        </div>
      </div>
      {offerBeta && (
        <div className="flex items-start gap-3 rounded-[14px] bg-surface px-3.5 py-3 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-beta-row">
          <LogoTile icon={IconSparkle} size={24} on />
          <div className="flex min-w-0 flex-col gap-1">
            <span className="flex items-center gap-1 text-meta font-medium text-ink">
              {WORDS.betaRowTitle} <BetaPill>Beta</BetaPill>
            </span>
            <span className="text-meta text-muted">{WORDS.betaRowLead(job?.model_name ?? "a model")}</span>
            <span className="mt-0.5 flex items-center gap-2.5">
              <button type="button" className="inline-flex items-center gap-1 rounded-md text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void aiBetaSet("chat", true)} data-testid="bot-beta-on">
                {WORDS.turnOn}
                <IconArrowRight width={12} height={12} aria-hidden="true" />
              </button>
              <span className="text-label text-faint">{WORDS.offAgain}</span>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

const DECLINED = "aij.bot.offer.declined";

/** The one offer: only where the recommended download passes Bot's tests
 * on this Mac, fits its memory (never 8 GB) and its free disk. */
function Offer(): JSX.Element | null {
  const nav = useBotNav();
  const catalog = useModelsCatalog();
  const [declined, setDeclined] = useState(() => readKept(DECLINED) === "1");
  const [started, setStarted] = useState(false);
  const id = catalog?.recommendation?.model_id ?? null;
  const model = catalog?.models.find((m) => m.id === id) ?? null;
  const passes = model?.verdicts.some((v) => v.job === "chat" && (v.verdict === "ships" || v.verdict === "beta")) ?? false;
  const mac = catalog?.mac;
  const fits = !!mac && mac.memory_gb > 8 && !!model && mac.free_disk_bytes >= model.free_disk_needed_bytes;
  if (declined || !model || model.size_bytes === null || !passes || !fits) return null;
  const size = gb(model.size_bytes);
  const get = async () => {
    const start = await aiModelDownloadStart(model.id).catch(() => null);
    if (start?.started) setStarted(true);
    else nav.openSettings("ai-models");
  };
  return (
    <div className="flex flex-col gap-2.5 rounded-[14px] bg-surface p-3.5 inset-ring-1 inset-ring-hairline/75 [[data-theme=dark]_&]:bg-surface-2" data-testid="bot-offer">
      <div className="flex items-start gap-3">
        <LogoTile icon={IconSparkle} size={32} on />
        <div className="flex min-w-0 flex-col gap-1">
          <span className="flex items-center gap-1.5 text-body font-semibold text-ink">
            {WORDS.offerTitle} <BetaPill>Beta</BetaPill>
          </span>
          <span className="text-meta text-muted">{WORDS.offerLead(size)}</span>
        </div>
      </div>
      <div className="flex gap-2 pl-11">
        {started ? (
          <output className="text-meta text-ink-2">Downloading. Settings › AI › Models shows how far it is.</output>
        ) : (
          <>
            <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-accent-soft px-2.5 text-meta font-medium text-accent-hi inset-ring-1 inset-ring-accent/30 outline-none hover:bg-accent/20 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void get()} data-testid="bot-offer-get">
              <IconDownload width={14} height={14} aria-hidden="true" />
              {WORDS.offerGet(size)}
            </button>
            <button type="button" className="inline-flex h-7 items-center rounded-lg px-2.5 text-meta font-medium text-ink-2 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => { keep(DECLINED, "1"); setDeclined(true); }} data-testid="bot-offer-decline">
              {WORDS.notNow}
            </button>
          </>
        )}
      </div>
      <span className="pl-11 text-label text-muted">
        {WORDS.offerFoot}{" "}
        <button type="button" className="rounded-md font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => nav.openSettings("ai-models")} data-testid="bot-offer-models">
          {WORDS.offerChoose} ›
        </button>
      </span>
    </div>
  );
}

export function BotNoModel(): JSX.Element {
  return (
    <div className="flex flex-col gap-[18px]" data-testid="bot-no-model">
      <Hero title={WORDS.noModelTitle} lead={WORDS.noModelLead} />
      <div className="flex flex-col gap-2">
        <span className="text-label font-medium tracking-[.07em] text-section-label uppercase">{WORDS.goStraight}</span>
        <Shortcut shortcut="needs_me" />
        <Shortcut shortcut="finished_today" />
        <Shortcut shortcut="runway" />
        <Shortcut shortcut="running" />
        <button type="button" className="inline-flex items-center gap-1.5 self-start rounded-md px-0.5 py-1 text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => void ask("What’s running?")}>
          <IconEye width={14} height={14} aria-hidden="true" />
          {WORDS.watchSession}
        </button>
      </div>
      <Offer />
    </div>
  );
}
