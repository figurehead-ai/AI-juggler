/**
 * 0.4.6 (D-126) — the making screens' layout: the steps beside (a flow
 * of four or more; one of two says its step in its head instead), the
 * form filling to 680 px, a live preview of 320–380 px on the right, and
 * the actions at the foot. At most 1180 px wide and left-aligned to the
 * steps, never a narrow column centred on empty space. Under 960 px the
 * preview folds above the form as a summary strip (`summary`).
 *
 * - `FlowSteps`: the steps, in groups, each with its value under its
 *   name; done ones go back, the current one is marked.
 * - `FlowFoot`: Cancel on the left; Back, a secondary, then the view's
 *   one filled primary on the right (D-126: one primary per view).
 */
import { forwardRef, useId, type JSX, type ReactNode, type Ref } from "react";
import { Button } from "./primitives";
import { IconCheck } from "./icons-controls";

export function FlowLayout({
  steps,
  preview,
  summary,
  foot,
  children,
  label,
  testId,
}: {
  /** The steps beside the form (`FlowSteps`). */
  steps?: ReactNode;
  /** The live preview, right of the form, matching it as it changes. */
  preview?: ReactNode;
  /** The preview folded: a strip above the form under 960 px. */
  summary?: ReactNode;
  /** The actions (`FlowFoot`). */
  foot?: ReactNode;
  /** The form. */
  children: ReactNode;
  /** The form's name (a named region; the window has its own main). */
  label?: string;
  testId?: string;
}): JSX.Element {
  return (
    <div className="flex min-h-0 flex-1 flex-col" data-testid={testId}>
      <div className="flex min-h-0 flex-1">
        {steps && (
          <div className="flex w-60 shrink-0 flex-col overflow-y-auto border-r border-divider bg-surface px-3 pb-3 pt-5 min-[1360px]:w-[280px] min-[1360px]:px-4 min-[1360px]:pt-6" data-testid="flow-steps-column">
            {steps}
          </div>
        )}
        <section aria-label={label} className="min-w-0 flex-1 overflow-y-auto bg-canvas">
          <div className="flex max-w-[1180px] items-start gap-10 px-8 pb-6 pt-7 max-[960px]:flex-col max-[960px]:gap-5 max-[960px]:px-6 min-[1360px]:px-12 min-[1360px]:pt-9" data-testid="flow-body">
            {summary && (
              <div className="hidden w-full max-[960px]:block" data-testid="flow-summary">
                {summary}
              </div>
            )}
            <div className="flex min-w-0 max-w-[680px] flex-1 flex-col max-[960px]:w-full" data-testid="flow-form">
              {children}
            </div>
            {preview && (
              <div className="sticky top-0 w-[clamp(320px,28vw,380px)] shrink-0 max-[960px]:hidden" data-testid="flow-preview">
                {preview}
              </div>
            )}
          </div>
        </section>
      </div>
      {foot}
    </div>
  );
}

export type FlowStepState = "done" | "current" | "todo";

export interface FlowStep {
  id: string;
  label: string;
  /** What it is set to, under its name ("web", "Everyone, freely"). */
  value?: string;
  state: FlowStepState;
  /** The group it is in (`groups`' id). */
  group?: string;
  /** Not reachable yet. */
  disabled?: boolean;
}

/** One step's dot: ✓ when done, its number otherwise, the current one
 * filled in the accent. */
function Dot({ state, n, id }: { state: FlowStepState; n: number; id: string }): JSX.Element {
  const look = state === "current" ? "bg-accent-fill text-white shadow-[0_0_0_4px_var(--color-accent-soft)]" : state === "done" ? "bg-accent-soft text-accent-hi" : "bg-surface text-faint inset-ring-[1.5px] inset-ring-hairline";
  return (
    // Hidden on its own, but its number is part of the step's name (WCAG
    // 2.5.3: a step showing "4" is named "4 Who leads …").
    <span aria-hidden="true" id={id} className={`relative z-[1] flex size-6 shrink-0 items-center justify-center rounded-full text-label font-semibold ${look}`}>
      {state === "done" ? <IconCheck width={12} height={12} strokeWidth={2.5} /> : n}
    </span>
  );
}

export function FlowSteps({
  steps,
  groups = [],
  onPick,
  label = "Steps",
  foot,
  testId,
}: {
  steps: readonly FlowStep[];
  /** The groups' names, in order ("The basics", "How they work"), with a
   * tag beside one ("prefilled"). */
  groups?: readonly { id: string; label: string; tag?: string }[];
  onPick: (id: string) => void;
  label?: string;
  /** Under the steps (a guide's link). */
  foot?: ReactNode;
  testId?: string;
}): JSX.Element {
  const base = useId().replace(/:/g, "");
  const order = groups.length > 0 ? groups.map((group) => ({ group, steps: steps.filter((step) => step.group === group.id) })) : [{ group: null, steps }];
  // Numbered in the order shown, across the groups.
  const numberOf = new Map(order.flatMap((part) => part.steps).map((step, at) => [step.id, at + 1]));
  return (
    <nav aria-label={label} className="flex min-h-full flex-col gap-1" data-testid={testId}>
      {order.map(({ group, steps: inGroup }) => (
        <div key={group?.id ?? "steps"} className="flex flex-col gap-1">
          {group && (
            <p className="flex items-center gap-2 px-2.5 pb-1.5 pt-3.5 first:pt-0">
              <span className="section-label">{group.label}</span>
              {group.tag && <span className="rounded-full bg-surface-3 px-2 text-label font-medium text-ink-2">{group.tag}</span>}
            </p>
          )}
          <ol className="flex flex-col gap-1">
            {inGroup.map((step, at) => {
              const id = `${base}-${step.id}`;
              return (
                <li key={step.id} className="relative">
                  {at > 0 && <span aria-hidden="true" className="absolute -top-1.5 left-[21px] h-3.5 w-[1.5px] bg-divider" />}
                  <button
                    type="button"
                    onClick={() => onPick(step.id)}
                    disabled={step.disabled}
                    aria-current={step.state === "current" ? "step" : undefined}
                    // WCAG 2.5.3: its name holds what it shows, the step and
                    // its value ("Who's in, Claude and Codex"), not the step alone.
                    aria-labelledby={`${id}-num ${id}-name ${id}-more`}
                    className={"flex w-full items-start gap-3 rounded-[10px] px-2.5 py-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default " + (step.state === "current" ? "bg-surface-2" : "hover:bg-surface-2 disabled:hover:bg-transparent")}
                    data-testid="flow-step"
                    data-step={step.id}
                    data-state={step.state}
                  >
                    <Dot state={step.state} n={numberOf.get(step.id) ?? at + 1} id={`${id}-num`} />{" "}
                    <span className="flex min-w-0 flex-col">
                      <span id={`${id}-name`} className={"text-meta leading-6 " + (step.state === "current" ? "font-semibold text-ink" : step.state === "todo" ? "font-medium text-muted" : "font-medium text-ink-2")}>
                        {step.label}
                      </span>
                      {/* Word breaks in its text between its number, the
                          step and its value, as in its name: axe's 2.5.3
                          check joins what's drawn with no spaces ("4Who
                          leadsNo lead yet"); a flex box never draws them. */}{" "}
                      <span id={`${id}-more`} className={"-mt-0.5 text-label " + (step.state === "done" ? "text-muted" : "text-faint")}>
                        {step.value}
                        {/* Part of its name: "Where web, done" (a leading space would be dropped). */}
                        {step.state === "done" && <span className="sr-only">{step.value ? ", done" : "done"}</span>}
                      </span>
                    </span>
                  </button>
                </li>
              );
            })}
          </ol>
        </div>
      ))}
      {foot && <div className="mt-auto pt-4">{foot}</div>}
    </nav>
  );
}

export interface FlowAction {
  label: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  testId?: string;
}

/** The foot: Cancel left; Back, a secondary, the one primary right. */
export const FlowFoot = forwardRef(function FlowFoot(
  {
    cancel,
    back,
    secondary,
    primary,
    note,
  }: {
    cancel?: FlowAction;
    back?: FlowAction;
    secondary?: FlowAction;
    /** The view's one filled primary. */
    primary: FlowAction;
    /** A line before the actions ("The rest is prefilled…"). */
    note?: ReactNode;
  },
  primaryRef: Ref<HTMLButtonElement>,
): JSX.Element {
  return (
    <footer className="flex min-h-[68px] shrink-0 flex-wrap items-center gap-2 border-t border-divider bg-surface px-6 py-3" data-toast-avoid data-testid="flow-foot">
      {cancel && (
        <Button variant="ghost" onClick={cancel.onClick} disabled={cancel.disabled} data-testid={cancel.testId}>
          {cancel.label}
        </Button>
      )}
      <span className="flex-1" />
      {note && <p className="mr-3 min-w-0 text-meta text-ink-2">{note}</p>}
      {back && (
        <Button variant="ghost" onClick={back.onClick} disabled={back.disabled} data-testid={back.testId}>
          {back.label}
        </Button>
      )}
      {secondary && (
        <Button variant="secondary" onClick={secondary.onClick} disabled={secondary.disabled} data-testid={secondary.testId}>
          {secondary.label}
        </Button>
      )}
      <Button ref={primaryRef} variant="primary" onClick={primary.onClick} disabled={primary.disabled} data-testid={primary.testId}>
        {primary.label}
      </Button>
    </footer>
  );
});
