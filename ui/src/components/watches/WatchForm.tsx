/**
 * AJ-E3816 — New watch and Edit watch, one form in the kit's FlowLayout
 * (D-126 rich): four numbered sections (Watch what, For what, Until,
 * Quiet hours), the live preview on the right, the actions at the foot
 * with what the primary does beside it. Only the conditions every chosen
 * target's App can report are offered (D-127), each parameter within its
 * compiled bounds; the echo is the backend's, word for word. Nothing is
 * written until Watch (or Save changes) is pressed.
 */
import { useEffect, useId, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { FlowLayout } from "../FlowLayout";
import { Button } from "../primitives";
import { CheckCard, DateField, Picker, Segmented, TimeField, ToggleRow, type Segment } from "../controls";
import { Alert } from "../Alert";
import { watchCatalogue, watchDraftWords, type CatalogueEntry, type DraftWords, type WatchDraftInfo, type WatchInfo } from "../../lib/ipc/watch-view";
import { errorText } from "../../lib/bridge";
import type { Param } from "../../types/generated/Param";
import type { WatchTarget } from "../../types/generated/WatchTarget";
import { WatchPreview, WatchPreviewStrip } from "./WatchPreview";
import { WatchTargetPicker } from "./WatchTargetPicker";
import { draftFrom, defaultParam, lifeOf, paramSteps, previewFacts, untilMs, type FormState, type LifeKind } from "./watch-form-state";
import { intersect, MAX_CONDITIONS, splitConditions, targetKey, untilEndsOffered, type TargetOption } from "./watches-view";
import { CONDITION_LINE, FORM, LIFE, PARAM } from "./watches-words";
import { CONDITION_ICONS, IconBin, IconCheckCircle, IconChevronD, IconChevronL, IconChevronR, IconEye, IconMoon } from "./watch-icons";
import { pastWhen, whenWords } from "../../lib/time-words";

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

/** The conditions every target can report, as cards. */
function useOffered(targets: readonly WatchTarget[]): { offered: CatalogueEntry[]; loading: boolean } {
  const [lists, setLists] = useState<Record<string, CatalogueEntry[]>>({});
  const keys = targets.map(targetKey);
  const want = keys.join("|");
  useEffect(() => {
    let live = true;
    const missing = targets.filter((t) => lists[targetKey(t)] === undefined);
    if (missing.length > 0)
      void Promise.all(missing.map(async (t) => [targetKey(t), await watchCatalogue(t).catch(() => [])] as const)).then((got) => {
      if (live) setLists((was) => ({ ...was, ...Object.fromEntries(got) }));
    });
    return () => {
      live = false;
    };
    // `want` names the targets; `lists` only fills in.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [want]);
  const loading = keys.some((key) => lists[key] === undefined);
  const offered = useMemo(() => (loading ? [] : intersect(keys.map((key) => lists[key]!))), [loading, lists, want]); // eslint-disable-line react-hooks/exhaustive-deps
  return { offered, loading };
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

/** The line under Until: what the chosen lifetime does. */
function lifeLine(life: LifeKind, at: number | null, past: boolean, names: string[], now: number): string {
  if (life === "once") return LIFE.onceLine;
  if (life === "until_ends") return LIFE.untilEndsLine(names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}` : (names[0] ?? "it"));
  if (life === "today") return LIFE.todayLine;
  if (life === "always") return LIFE.alwaysLine;
  if (past) return LIFE.past;
  return at === null ? LIFE.untilPick : LIFE.untilLine(whenWords(at, now));
}

/** Section 2: the conditions as cards, the four most used first. */
function ConditionCards({ state, offered, loading, onFlip, onParam, labelledBy }: { state: FormState; offered: CatalogueEntry[]; loading: boolean; onFlip: (entry: CatalogueEntry, on: boolean) => void; onParam: (code: string, param: Param | undefined) => void; labelledBy: string }): JSX.Element {
  const [showMore, setShowMore] = useState(false);
  if (state.targets.length === 0) return <p className="text-meta text-muted">{FORM.pickTargetFirst}</p>;
  if (loading)
    return (
      <div aria-hidden="true" className="grid grid-cols-2 gap-2.5">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="h-[82px] animate-pulse rounded-card bg-skeleton/60" />
        ))}
      </div>
    );
  if (offered.length === 0)
    return (
      <p className="text-meta text-muted" data-testid="watch-nothing-offered">
        {FORM.nothingOffered}
      </p>
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
              {on && <ParamControl entry={entry} value={state.params[entry.code]} onChange={(param) => onParam(entry.code, param)} />}
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

/** Section 3: the lifetime, its dates for Until…, and what it does. */
function UntilChoice({ state, set, offered, book, now }: { state: FormState; set: (change: Partial<FormState>) => void; offered: CatalogueEntry[]; book: Map<string, TargetOption>; now: number }): JSX.Element {
  const life = lifeOf(state);
  const segments: Segment<LifeKind>[] = [
    { value: "once", label: LIFE.once },
    ...(untilEndsOffered(state.targets) ? [{ value: "until_ends" as const, label: state.targets.length > 1 ? LIFE.untilTheyEnd : LIFE.untilEnds }] : []),
    { value: "today", label: LIFE.today },
    { value: "until", label: LIFE.until },
    { value: "always", label: LIFE.always },
  ];
  const at = untilMs(state.untilDay, state.untilTime);
  const past = life === "until" && at !== null && at <= now;
  const names = state.targets.flatMap((t) => {
    const name = book.get(targetKey(t))?.name;
    return name ? [`“${name}”`] : [];
  });
  const first = offered.find((e) => e.code === state.codes[0])?.label.toLowerCase();
  return (
    <>
      <div className="flex flex-wrap items-center gap-3">
        <Segmented label={FORM.until} value={life} onChange={(next) => set({ life: next })} segments={segments} testId="watch-lifetime-choice" className="w-fit" />
        {state.life === null && first && <span className="text-label text-muted">{LIFE.suggested(first)}</span>}
      </div>
      {life === "until" && (
        <div className="flex flex-wrap items-center gap-2">
          <DateField value={state.untilDay} onCommit={(day) => set({ untilDay: day })} ariaLabel={LIFE.day} testId="watch-until-day" />
          <TimeField value={state.untilTime} onCommit={(time) => set({ untilTime: time })} ariaLabel={LIFE.time} testId="watch-until-time" />
        </div>
      )}
      <p className={"text-meta " + (past ? "text-danger" : "text-muted")} data-testid="watch-lifetime-line" data-past={past}>
        {lifeLine(life, at, past, names, now)}
      </p>
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
  const { offered, loading } = useOffered(state.targets);
  const draft = useMemo(() => draftFrom(state), [state]);
  const words = useDraftWords(draft, state.targets.length > 0 && state.codes.length > 0);
  // Conditions no longer on offer for these targets leave (D-127: hidden,
  // never shown as unsupported).
  const kept = loading || state.targets.length === 0 ? state.codes : state.codes.filter((code) => offered.some((e) => e.code === code));
  if (kept.length !== state.codes.length) setState((was) => ({ ...was, codes: kept }));
  const set = (change: Partial<FormState>) => setState((was) => ({ ...was, ...change }));
  const flip = (entry: CatalogueEntry, on: boolean) =>
    setState((was) => {
      if (!on) return { ...was, codes: was.codes.filter((c) => c !== entry.code) };
      if (was.codes.length >= MAX_CONDITIONS) return was;
      return { ...was, codes: [...was.codes, entry.code], params: { ...was.params, [entry.code]: was.params[entry.code] ?? defaultParam(entry) } };
    });
  const at = untilMs(state.untilDay, state.untilTime);
  const past = lifeOf(state) === "until" && at !== null && at <= now;
  const was = mode === "edit" && editing && words && words.words !== editing.words ? editing.echo : null;
  const facts = previewFacts({ state, offered, echo: words?.echo ?? null, was, book, quiet, now });
  const atLimit = mode === "new" && props.count >= props.limit;
  const canSave = savable({ loading, past, saving, atLimit, words });
  const save = async () => {
    setSaving(true);
    setFailed(null);
    try {
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
        label={title}
        summary={<WatchPreviewStrip echo={words?.echo ?? null} />}
        preview={<WatchPreview facts={facts} />}
        foot={<FormFoot mode={mode} consequence={mode === "new" ? FORM.startsNow(props.count + 1, props.limit) : FORM.changesNow} canSave={canSave} saving={saving} onCancel={props.onCancel} onRemove={mode === "edit" ? props.onRemove : undefined} onSave={() => void save()} />}
      >
        <FormHead title={title} editing={editing} now={now} onBack={props.onCancel} />
        <div className="flex flex-col gap-[26px]">
          <FormAlerts atLimit={atLimit} limit={props.limit} ended={editing?.state === "ended"} failed={failed} />
          <FormSection n={1} done={many > 0} title={FORM.what} help={FORM.whatHelp} id={`${ids}-what`}>
            <WatchTargetPicker picked={state.targets} book={book} onChange={(targets) => set({ targets })} labelledBy={`${ids}-what`} />
            {many > 1 && <Matching state={state} set={set} />}
          </FormSection>
          <FormSection n={2} done={state.codes.length > 0} title={FORM.forWhat} help={many === 0 ? undefined : many === 2 ? FORM.forWhatBoth : FORM.forWhatHelp(many)} id={`${ids}-for`}>
            <ConditionCards state={state} offered={offered} loading={loading} onFlip={flip} onParam={(code, param) => set({ params: { ...state.params, [code]: param } })} labelledBy={`${ids}-for`} />
          </FormSection>
          <FormSection n={3} done={state.codes.length > 0} title={FORM.until} id={`${ids}-until`}>
            <UntilChoice state={state} set={set} offered={offered} book={book} now={now} />
          </FormSection>
          <FormSection n={4} done={false} title={FORM.quiet} id={`${ids}-quiet`}>
            {quiet ? (
              <div className="rounded-card bg-surface p-1 inset-ring-1 inset-ring-hairline/75">
                <ToggleRow icon={IconMoon} title={FORM.quietTitle} help={FORM.quietHelp(quiet.from, quiet.to)} checked={state.quiet} onChange={(on) => set({ quiet: on })} testId="watch-quiet-toggle" />
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
