/**
 * AJ-E3812 (D-131, D-126 rich mode) — Settings › AI › Models, to the
 * approved mock-up: one page, three groups in order — Recommended for
 * this Mac, Other tested models, Installed — then Each job uses. One
 * answer first, the choices next, management last.
 *
 * AJ-E3823: while Apple's built-in AI is off, getting ready or couldn't
 * be checked, the top group is the engine-setup guide (the same one as
 * "Set up Bot"): it takes the Recommended card's place, and the AI it
 * names leaves Other tested models, so nothing shows twice.
 *
 * Lazy and instant: the pane is its own chunk, its reads are shared and
 * kept, so it paints the last answer at once and reads again behind it.
 * Every word comes from `models-words.ts`.
 */
import type { JSX } from "react";
import { Alert } from "../../../components/Alert";
import { useAiJobs } from "../../../lib/ai-availability";
import { EngineSetup } from "../../../components/ai/EngineSetup";
import { setupView, showsSetup } from "../../../components/ai/engine-setup-view";
import { useModelsCatalogRead, useModelsInstalled } from "../../../lib/ai-models";
import { LoadingState } from "../../../components/LoadingState";
import { useLoadDeadline, type LoadState } from "../../../lib/loading";
import type { ModelsCatalog, ModelsInfo } from "../../../lib/ipc/models";
import { Pane } from "../parts";
import { IconChevronRight } from "../../../components/icons-controls";
import { IconChip, IconOffline } from "./model-icons";
import { useModelActions, type ModelActions } from "./models-actions";
import { fitRows, nameOf, others, recommended, tags, type ModelTag, type Recommended } from "./models-view";
import { HonestyLine, HowWeTest } from "./ModelParts";
import { ApprovalDialog } from "./ModelsApproval";
import { InstalledSection } from "./ModelsInstalled";
import { JobsTable } from "./ModelsJobs";
import { OthersSection } from "./ModelsOthers";
import { AppleTip, NothingCard, RecommendedCard, RecommendedSection } from "./ModelsRecommended";
import { PAGE, chipWords, memoryWords } from "./models-words";

function MacStrip({ catalog }: { catalog: ModelsCatalog | null }): JSX.Element {
  const mac = catalog?.mac;
  const offline = catalog !== null && !catalog.listed && !navigator.onLine;
  const dot = <span aria-hidden="true" className="size-[3px] rounded-full bg-faint" />;
  return (
    <p className="inline-flex shrink-0 flex-wrap items-center gap-2.5 rounded-full bg-surface py-1.5 pr-3 pl-2 text-label whitespace-nowrap text-ink-2 inset-ring-1 inset-ring-hairline/75" data-testid="models-mac">
      <IconChip width={14} height={14} aria-hidden="true" />
      <span className="font-semibold text-ink">{PAGE.thisMac}</span>
      {mac ? (
        <>
          {dot}
          {chipWords(mac.apple_silicon, mac.chip)}
          {dot}
          {memoryWords(mac.memory_bytes, mac.memory_gb)}
          {dot}
          {PAGE.free(mac.free_disk_bytes)}
        </>
      ) : (
        <span className="text-muted">{PAGE.loading}</span>
      )}
      {offline && (
        <>
          {dot}
          <span className="inline-flex items-center gap-1">
            <IconOffline width={13} height={13} aria-hidden="true" />
            {PAGE.offline}
          </span>
        </>
      )}
    </p>
  );
}

function Head({ catalog }: { catalog: ModelsCatalog | null }): JSX.Element {
  return (
    <div className="flex flex-wrap items-end gap-x-4 gap-y-3">
      <div className="flex min-w-0 flex-1 basis-80 flex-col gap-1">
        <span className="flex items-center gap-1.5 text-meta text-muted">
          {PAGE.crumb}
          <IconChevronRight width={12} height={12} aria-hidden="true" />
        </span>
        <h2 className="text-title text-ink">{PAGE.title}</h2>
        <p className="max-w-[62ch] text-meta text-muted">
          {PAGE.lead} <HowWeTest />
        </p>
      </div>
      <MacStrip catalog={catalog} />
    </div>
  );
}

function Skeleton(): JSX.Element {
  return <output aria-label="Loading the AI models" className="block h-56 animate-pulse rounded-2xl bg-skeleton/60" data-testid="models-skeleton" />;
}

/** The Recommended group: the pick, or why there's none; Apple's tip;
 * the honesty line when no other card carries it. */
function RecommendedGroup({ catalog, read, installed, pick, act }: { catalog: ModelsCatalog | null; read: { state: LoadState; retry: () => void }; installed: ModelsInfo | null; pick: Recommended | null; act: ModelActions }): JSX.Element {
  // AJ-E1552: the skeleton only while the list loads ("Still loading…"
  // after 3 s); the shared read's own deadline turns it into an error
  // with Retry. The list is never empty: no pick is the NothingCard.
  const { slow } = useLoadDeadline(read.state === "loading", "the AI models", { timeoutMs: Number.POSITIVE_INFINITY });
  if (!catalog) {
    return (
      <RecommendedSection>
        <LoadingState state={read.state === "error" ? "error" : "loading"} slow={slow} what="the AI models" skeleton={<Skeleton />} empty={null} onRetry={read.retry}>
          {null}
        </LoadingState>
      </RecommendedSection>
    );
  }
  if (showsSetup(catalog)) {
    return (
      <RecommendedSection>
        <EngineSetup place="models" />
      </RecommendedSection>
    );
  }
  const actions = pick && {
    onUse: () => act.use(pick.model),
    onDownload: () => act.download(pick.model, true),
    onCancel: () => act.cancel(pick.model.id),
    onCheckAgain: act.checkAgain,
    onOpenStorage: act.openStorage,
  };
  return (
    <RecommendedSection>
      {pick && actions ? (
        <RecommendedCard pick={pick} catalog={catalog} fit={fitRows(catalog, pick.model, installed)} download={installed?.download} actions={actions} />
      ) : (
        <NothingCard catalog={catalog} />
      )}
      {catalog.mac?.apple_ai === "off" && !pick?.model.built_in && <AppleTip onOpen={act.openAppleIntelligence} />}
      {!catalog.listed && pick && <HonestyLine />}
    </RecommendedSection>
  );
}

const UNDER_WAY = new Set(["running", "waiting", "verifying"]);

/** The AI the setup guide names, when it leads the page: it isn't
 * listed again below it. */
function namedBySetup(catalog: ModelsCatalog | null, installed: ModelsInfo | null, jobs: Parameters<typeof recommended>[2]): string | null {
  if (!showsSetup(catalog)) return null;
  return setupView(catalog, installed, jobs?.find((job) => job.job === "chat") ?? null).download?.id ?? null;
}

/** What the page shows, worked out once per draw. */
function viewOf(catalog: ModelsCatalog | null, installed: ModelsInfo | null, jobs: Parameters<typeof recommended>[2]) {
  const pick = catalog && recommended(catalog, installed, jobs);
  const builtIn = catalog?.models.find((model) => model.built_in) ?? null;
  const here = installed?.installed ?? [];
  const named = namedBySetup(catalog, installed, jobs);
  return {
    pick,
    builtIn,
    rest: catalog ? others(catalog, installed, named ?? pick?.model.id ?? null) : [],
    tagged: catalog ? tags(catalog) : new Map<string, ModelTag>(),
    free: catalog?.mac?.free_disk_bytes ?? null,
    anyModel: builtIn !== null || here.length > 0,
    busy: UNDER_WAY.has(installed?.download?.state ?? ""),
    automaticName: nameOf(catalog?.recommendation?.model_id, catalog, installed),
    offline: catalog ? !catalog.listed : false,
    have: here.map((model) => model.name),
    totalNow: installed?.total_bytes ?? 0,
  };
}

export function ModelsPane({ anchor }: { anchor?: string }): JSX.Element {
  const catalogRead = useModelsCatalogRead();
  const catalog = catalogRead.data ?? null;
  const installed = useModelsInstalled();
  const jobs = useAiJobs();
  const act = useModelActions(jobs);
  const view = viewOf(catalog, installed, jobs);
  return (
    <Pane id="ai-models" title={PAGE.title} anchor={anchor} head={<Head catalog={catalog} />}>
      <div className="flex flex-col gap-8" data-testid="models-screen">
        {act.note && (
          <Alert level="needs-you" testId="models-note">
            {act.note}
          </Alert>
        )}
        <RecommendedGroup catalog={catalog} read={catalogRead} installed={installed} pick={view.pick} act={act} />
        {catalog && (
          <OthersSection
            catalog={catalog}
            models={view.rest}
            tags={view.tagged}
            shortOfDisk={view.pick?.action === "no-disk"}
            onDownload={(model) => act.download(model, false)}
            busy={view.busy}
          />
        )}
        {installed && (
          <InstalledSection info={installed} builtIn={view.builtIn} jobs={jobs} automaticName={view.automaticName} free={view.free} offline={view.offline} onRemove={act.remove} />
        )}
        {view.anyModel && <JobsTable jobs={jobs} catalog={catalog} installed={installed} actions={{ onBeta: act.beta, onPick: act.pick }} />}
      </div>
      {act.asking && (
        <ApprovalDialog
          model={act.asking}
          have={view.have}
          totalNow={view.totalNow}
          free={view.free ?? 0}
          tag={view.tagged.get(act.asking.id)}
          onDownload={act.approve}
          onNotNow={act.notNow}
          onClose={act.closeAsk}
        />
      )}
    </Pane>
  );
}
