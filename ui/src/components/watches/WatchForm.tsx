/**
 * AJ-E3816 — New watch and Edit watch, one form in the kit's FlowLayout
 * (D-126 rich): four numbered sections (Watch what, For what, Until,
 * Quiet hours), the live preview on the right, the actions at the foot
 * with what the primary does beside it. Only the conditions every chosen
 * target's App can report are offered (D-127), each parameter within its
 * compiled bounds; the echo is the backend's, word for word. Nothing is
 * written until Watch (or Save changes) is pressed.
 */
import { osPromptRequest } from "../../lib/ipc/os-prompts";
import { useEffect, useId, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { FlowLayout } from "../FlowLayout";
import { Button } from "../primitives";
import { CheckCard, Picker, Segmented, ToggleRow } from "../controls";
import { Alert } from "../Alert";
import { watchDraftWords, type CatalogueEntry, type DraftWords, type WatchDraftInfo, type WatchInfo } from "../../lib/ipc/watch-view";
import { errorText } from "../../lib/bridge";
import { useLoadDeadline } from "../../lib/loading";
import { useOffered } from "./use-offered";
import { LoadingState } from "../LoadingState";
import type { Param } from "../../types/generated/Param";
import { WatchPreview, WatchPreviewStrip } from "./WatchPreview";
import { WatchTargetPicker } from "./WatchTargetPicker";
import { botLeft, codesNow, draftFrom, defaultParam, lifeAsked, lifeOf, paramSteps, previewFacts, touched, untilMs, type FormState } from "./watch-form-state";
import { MAX_CONDITIONS, splitConditions, type TargetOption } from "./watches-view";
import { CONDITION_LINE, FORM, PARAM } from "./watches-words";
import { CONDITION_ICONS, IconBin, IconCheckCircle, IconChevronD, IconChevronL, IconChevronR, IconEye, IconMoon } from "./watch-icons";
import { pastWhen } from "../../lib/time-words";
import { UntilChoice } from "./WatchFormUntil";
import { BotFillNote, NearestTag, QuietLeft, consequenceOf, sectionHelps } from "./WatchFormBot";

export interface WatchFormProps {
  mode: "new" | "edit";
  initial: FormState;
  /** The watch being edited. */
  editing: WatchInfo | null;
  book: Map<string, TargetOption>;
  quiet: { from: string; to: string } | null;
  /** Live watches now (watching and paused) and the limit. */
  count: number;
  limit: number;
  now: number;
  onCancel: () => void;
  onSave: (draft: ReturnType<typeof draftFrom>) => Promise<void>;
  onRemove?: () => void;
}

/** A section: its number in the kit's step dot, its title, a line. */
function FormSection({ n, done, title, help, children, id }: { n: number; done: boolean; title: string; help?: ReactNode; children: ReactNode; id: string }): JSX.Element {
  return (
    <section aria-labelledby={id} className="flex flex-col gap-2.5" data-testid={`watch-form-section-${n}`}>
      <div className="flex items-center gap-2.5">
        <span aria-hidden="true" className={"flex size-[22px] shrink-0 items-center justify-center rounded-full text-label font-semibold " + (done ? "bg-accent-soft text-accent-hi" : "bg-surface text-faint inset-ring-[1.5px] inset-ring-hairline")}>
          {n}
        </span>
        <h3 id={id} className="text-section font-semibold text-ink">
          {title}
        </h3>
        {help && <span className="text-meta text-muted">{help}</span>}
      </div>
      <div className="flex flex-col gap-2.5 pl-8">{children}</div>
    </section>
  );
}

/** A number parameter as a small picker within its bounds. */
function ParamControl({ entry, value, onChange }: { entry: CatalogueEntry; value: Param | undefined; onChange: (next: Param | undefined) => void }): JSX.Element | null {
  const spec = entry.param;
  if (!spec) return null;
  if (spec.kind === "minutes" || spec.kind === "percent") {
    const steps = paramSteps(entry);
    const none = spec.required ? [] : [{ value: "none", label: entry.code === "stuck" ? PARAM.asSoon : PARAM.rightAway }];
    const current = value && (value.kind === "minutes" || value.kind === "percent") ? String(value.value) : "none";
    const options = [...none, ...steps.map((n) => ({ value: String(n), label: String(n) }))];
    return (
      <span className="mt-2 inline-flex items-center gap-2 text-meta text-ink-2">
        {spec.kind === "minutes" ? (current === "none" ? "" : FORM.forParam) : FORM.below}
        <Picker
          choice
          size="sm"
          value={current}
          options={options}
          ariaLabel={`${entry.label}: ${spec.kind === "minutes" ? FORM.minutes : FORM.percent}`}
          onChange={(next) => onChange(next === "none" ? undefined : spec.kind === "minutes" ? { kind: "minutes", value: Number(next) } : { kind: "percent", value: Number(next) })}
          testId={`watch-param-${entry.code}`}
        />
        {current === "none" ? "" : spec.kind === "minutes" ? FORM.minutes : FORM.percent}
      </span>
    );
  }
  if (spec.kind === "task_state") {
    const current = value?.kind === "task_state" ? value.value : "done";
    return (
      <span className="mt-2 inline-flex items-center gap-2 text-meta text-ink-2">
        {FORM.to}
        <Picker
          choice
          size="sm"
          value={current}
          options={[
            { value: "done", label: PARAM.done },
            { value: "blocked", label: PARAM.blocked },
            { value: "in_review", label: PARAM.inReview },
          ]}
          ariaLabel={entry.label}
          onChange={(next) => onChange({ kind: "task_state", value: next })}
          testId={`watch-param-${entry.code}`}
        />
      </span>
    );
  }
  return null;
}

/** The echo for the draft, from the backend (one read per pause). */
function useDraftWords(draft: WatchDraftInfo, ready: boolean): DraftWords | null {
  const [words, setWords] = useState<DraftWords | null>(null);
  const asked = useRef(0);
  const key = JSON.stringify(draft);
  useEffect(() => {
    const mine = ++asked.current;
    const timer = window.setTimeout(() => {
      if (!ready) setWords(null);
      else
        void watchDraftWords(draft).then(
          (got) => mine === asked.current && setWords(got),
          () => mine === asked.current && setWords(null),
        );
    }, 80);
    return () => window.clearTimeout(timer);
    // `key` is the draft.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, ready]);
  return words;
}

/** Section 2: the conditions as cards, the four most used first. */
function ConditionCards({ state, catalogue, onFlip, onParam, labelledBy }: { state: FormState; catalogue: ReturnType<typeof useOffered>; onFlip: (entry: CatalogueEntry, on: boolean) => void; onParam: (code: string, param: Param | undefined) => void; labelledBy: string }): JSX.Element {
  const [showMore, setShowMore] = useState(false);
  const { offered, failed, round, retry } = catalogue;
  const loading = catalogue.loading && !failed;
  const { slow } = useLoadDeadline(loading, "the conditions", { timeoutMs: Number.POSITIVE_INFINITY, round });
  if (state.targets.length === 0) return <p className="text-meta text-muted">{FORM.pickTargetFirst}</p>;
  if (loading || failed || offered.length === 0)
    return (
      <LoadingState
        state={loading ? "loading" : failed ? "error" : "empty"}
        slow={slow}
        what="the conditions"
        skeleton={
          <div aria-hidden="true" className="grid grid-cols-2 gap-2.5" data-testid="watch-conditions-skeleton">
            {[0, 1, 2, 3].map((i) => (
              <div key={i} className="h-[82px] animate-pulse rounded-card bg-skeleton/60" />
            ))}
          </div>
        }
        empty={
          <p className="text-meta text-muted" data-testid="watch-nothing-offered">
            {FORM.nothingOffered}
          </p>
        }
        onRetry={retry}
      >
        {null}
      </LoadingState>
    );
  const { top, more } = splitConditions(offered);
  const pickedMore = more.filter((e) => state.codes.includes(e.code));
  const folded = more.filter((e) => !state.codes.includes(e.code));
  const Chevron = showMore ? IconChevronD : IconChevronR;
  return (
    <>
      <fieldset aria-labelledby={labelledBy} className="m-0 grid min-w-0 grid-cols-2 border-0 p-0 gap-2.5 max-[1100px]:grid-cols-1" data-testid="watch-conditions">
        {[...top, ...(showMore ? more : pickedMore)].map((entry) => {
          const Icon = CONDITION_ICONS[entry.code] ?? IconCheckCircle;
          const on = state.codes.includes(entry.code);
          return (
            <CheckCard
              key={entry.code}
              checked={on}
              onChange={(next) => onFlip(entry, next)}
              disabled={!on && state.codes.length >= MAX_CONDITIONS}
              reason={FORM.upTo}
              logo={
                <span aria-hidden="true" className={"flex size-[30px] items-center justify-center rounded-lg " + (on ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")}>
                  <Icon width={16} height={16} />
                </span>
              }
              title={entry.label}
              sub={CONDITION_LINE[entry.code]}
              testId={`watch-condition-${entry.code}`}
            >
              {on ? <ParamControl entry={entry} value={state.params[entry.code]} onChange={(param) => onParam(entry.code, param)} /> : state.bot?.nearest === entry.code && botLeft(state, "condition") ? <NearestTag /> : null}
            </CheckCard>
          );
        })}
      </fieldset>
      {folded.length > 0 || showMore ? (
        <button type="button" aria-expanded={showMore} onClick={() => setShowMore((open) => !open)} className="inline-flex w-fit items-center gap-1.5 rounded text-meta font-medium text-ink-2 outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-conditions-more">
          <Chevron width={14} height={14} aria-hidden="true" />
          {showMore ? FORM.fewer : FORM.more(folded.length, folded.map((e) => e.label.toLowerCase()).join(", "))}
        </button>
      ) : null}
      <p className="text-meta text-muted">{FORM.upTo}</p>
    </>
  );
}

function FormFoot({ mode, consequence, canSave, saving, onCancel, onRemove, onSave }: { mode: "new" | "edit"; consequence: string; canSave: boolean; saving: boolean; onCancel: () => void; onRemove?: () => void; onSave: () => void }): JSX.Element {
  const Mark = mode === "new" ? IconEye : IconCheckCircle;
  return (
    <footer className="flex min-h-[68px] shrink-0 flex-wrap items-center gap-2 border-t border-divider bg-surface px-8 py-3" data-toast-avoid data-testid="watch-form-foot">
      <Button variant="ghost" onClick={onCancel} data-testid="watch-form-cancel">
        {FORM.cancel}
      </Button>
      {onRemove && (
        <Button variant="ghost" className="!text-danger hover:!bg-danger/10" onClick={onRemove} data-testid="watch-form-remove">
          <IconBin width={15} height={15} aria-hidden="true" />
          {FORM.removeWatch}
        </Button>
      )}
      <span className="flex-1" />
      <p className="mr-2 min-w-0 text-meta text-muted" data-testid="watch-form-consequence">
        {consequence}
      </p>
      <Button variant="primary" onClick={onSave} disabled={!canSave} data-testid="watch-form-save">
        <Mark width={15} height={15} aria-hidden="true" />
        {saving ? FORM.saving : mode === "new" ? FORM.watch : FORM.save}
      </Button>
    </footer>
  );
}

/** Watch (or Save changes) can be pressed: the words are in, nothing's wrong. */
function savable(f: { loading: boolean; past: boolean; saving: boolean; atLimit: boolean; words: DraftWords | null }): boolean {
  return !f.loading && !f.past && !f.saving && !f.atLimit && f.words !== null && f.words.problem === null;
}

function FormHead({ title, editing, now, onBack }: { title: string; editing: WatchInfo | null; now: number; onBack: () => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-1 pb-5">
      <button type="button" onClick={onBack} className="inline-flex w-fit items-center gap-1 rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="watch-form-back">
        <IconChevronL width={13} height={13} aria-hidden="true" />
        {FORM.back}
      </button>
      <div className="flex flex-wrap items-baseline gap-x-3">
        <h2 className="text-title text-ink">{title}</h2>
        <span className="text-meta text-muted">{editing ? FORM.editLead(pastWhen(editing.watch.createdMs, now), editing.told) : FORM.newLead}</span>
      </div>
    </div>
  );
}

function FormAlerts({ atLimit, limit, ended, failed }: { atLimit: boolean; limit: number; ended: boolean; failed: string | null }): JSX.Element {
  return (
    <>
      {atLimit && <Alert level="needs-you">{FORM.limitReached(limit)}</Alert>}
      {ended && <Alert level="info">{FORM.endedNote}</Alert>}
      {failed && <Alert level="broken">{failed}</Alert>}
    </>
  );
}

/** "With 2 sessions, tell you  [For each one | When all have]". */
function Matching({ state, set }: { state: FormState; set: (change: Partial<FormState>) => void }): JSX.Element {
  const many = state.targets.length;
  const label = FORM.matchingLabel(many, state.targets.every((t) => t.kind === "session") ? "sessions" : "targets");
  return (
    <div className="flex flex-wrap items-center gap-3 text-meta text-ink-2" data-testid="watch-matching">
      <span>{label}</span>
      <Segmented label={label} value={state.matching} onChange={(matching) => set({ matching })} segments={[{ value: "each", label: FORM.each }, { value: "all", label: FORM.all }]} testId="watch-matching-choice" />
    </div>
  );
}

export function WatchForm(props: WatchFormProps): JSX.Element {
  const { mode, editing, book, quiet, now } = props;
  const [state, setState] = useState<FormState>(props.initial);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const ids = useId().replace(/:/g, "");
  const catalogue = useOffered(state.targets);
  const { offered, loading } = catalogue;
  const draft = useMemo(() => draftFrom(state), [state]);
  const words = useDraftWords(draft, state.targets.length > 0 && state.codes.length > 0);
  // What is no longer on offer leaves; until the user picks, the
  // targets' default follows them (D-127, 0.5.2 G32).
  const held = codesNow(state, offered, loading, book);
  if (held.codes.join("|") !== state.codes.join("|")) setState((was) => ({ ...was, codes: held.codes, params: held.entry && !was.params[held.entry.code] ? { ...was.params, [held.entry.code]: defaultParam(held.entry) } : was.params }));
  // AJ-E3830: a field the user sets is no longer one Bot left.
  const set = (change: Partial<FormState>) => setState((was) => touched(was, change));
  const flip = (entry: CatalogueEntry, on: boolean) =>
    setState((was) => {
      if (!on) return touched(was, { auto: false, codes: was.codes.filter((c) => c !== entry.code) });
      if (was.codes.length >= MAX_CONDITIONS) return was;
      return touched(was, { auto: false, codes: [...was.codes, entry.code], params: { ...was.params, [entry.code]: was.params[entry.code] ?? defaultParam(entry) } });
    });
  const at = untilMs(state.untilDay, state.untilTime);
  const past = lifeOf(state) === "until" && at !== null && at <= now;
  const was = mode === "edit" && editing && words && words.words !== editing.words ? editing.echo : null;
  const facts = previewFacts({ state, offered, echo: words?.echo ?? null, was, book, quiet, now });
  const atLimit = mode === "new" && props.count >= props.limit;
  const asked = lifeAsked(state);
  const canSave = savable({ loading, past, saving, atLimit, words }) && !asked;
  const helps = sectionHelps(state);
  // 0.5.2 G60: Bot left what to watch: the picker opens on what the
  // words matched (kept as opened: the list doesn't move as the user picks).
  const [choices] = useState(() => (props.initial.bot && botLeft(props.initial, "target") ? props.initial.bot.choices : []));
  const save = async () => {
    setSaving(true);
    setFailed(null);
    try {
      // AJ-E1548 #16 (PAT-OSP): a watch tells you by notification, so the
      // first one primes macOS's permission here, at the user's click (our
      // sheet first). Not now, or no answer, still saves the watch.
      if (mode === "new") await osPromptRequest("notifications").catch(() => undefined);
      await props.onSave(draft);
    } catch (error) {
      setFailed(errorText(error));
      setSaving(false);
    }
  };
  const title = mode === "new" ? FORM.newTitle : FORM.editTitle;
  const many = state.targets.length;
  return (
    <div className="-m-5 flex h-[calc(100%+2.5rem)] min-h-0 flex-col" data-testid="watch-form" data-mode={mode}>
      <FlowLayout
        fit="room"
        label={title}
        summary={<WatchPreviewStrip echo={facts.echo} />}
        preview={<WatchPreview facts={facts} />}
        foot={<FormFoot mode={mode} consequence={consequenceOf(mode, asked, props.count, props.limit)} canSave={canSave} saving={saving} onCancel={props.onCancel} onRemove={mode === "edit" ? props.onRemove : undefined} onSave={() => void save()} />}
      >
        <FormHead title={title} editing={editing} now={now} onBack={props.onCancel} />
        <div className="flex flex-col gap-[26px]">
          <FormAlerts atLimit={atLimit} limit={props.limit} ended={editing?.state === "ended"} failed={failed} />
          {state.bot && <BotFillNote bot={state.bot} />}
          <FormSection n={1} done={many > 0} title={FORM.what} help={helps.what} id={`${ids}-what`}>
            <WatchTargetPicker picked={state.targets} book={book} onChange={(targets) => set({ targets })} labelledBy={`${ids}-what`} suggested={choices} startOpen={choices.length > 0} />
            {many > 1 && <Matching state={state} set={set} />}
          </FormSection>
          <FormSection n={2} done={state.codes.length > 0} title={FORM.forWhat} help={helps.forWhat} id={`${ids}-for`}>
            <ConditionCards state={state} catalogue={catalogue} onFlip={flip} onParam={(code, param) => set({ auto: false, params: { ...state.params, [code]: param } })} labelledBy={`${ids}-for`} />
          </FormSection>
          <FormSection n={3} done={state.codes.length > 0 && !asked} title={FORM.until} id={`${ids}-until`}>
            <UntilChoice state={state} set={set} offered={offered} book={book} now={now} />
          </FormSection>
          <FormSection n={4} done={false} title={FORM.quiet} id={`${ids}-quiet`}>
            {quiet ? (
              <div className="rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
                <ToggleRow icon={IconMoon} title={FORM.quietTitle} help={FORM.quietHelp(quiet.from, quiet.to)} checked={state.quiet} onChange={(on) => set({ quiet: on })} testId="watch-quiet-toggle" />
                {botLeft(state, "quiet_hours") && <QuietLeft />}
              </div>
            ) : (
              <p className="text-meta text-muted">{FORM.quietNone}</p>
            )}
          </FormSection>
        </div>
      </FlowLayout>
    </div>
  );
}
