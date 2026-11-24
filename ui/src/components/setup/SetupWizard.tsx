/**
 * AJ-E1545 — the setup wizard the Welcome's Get started opens (and a new
 * install opens by itself): rich mode, a full-window sheet with a sticky
 * header (the app's real icon and the title), the steps beside with each
 * one's value, the step's own eyebrow and title, and a sticky foot (Finish
 * later; Back; Skip this step; the one primary that names where it goes,
 * last). Nobody is left on their own: every step can wait, none is
 * required. 0.5.1 F50: each way out says what it does: Finish later
 * leaves setup (it opens again where it was left), Skip this step goes
 * on without this step, and a sign-in row's "Sign in to this app later"
 * leaves that one app.
 *
 * 1. Find your apps: first run's discovery and import, in place (its own
 *    primary, Bring in, is the step's, in the foot's end slot after Skip
 *    this step; its links in the foot's middle).
 * 2. Let AI Juggler work with your Mac: one card per permission.
 * 3. Meet Bot. 4. Keep it running. 5. The tour (AJ-E1546).
 *
 * Esc and Finish later close it (a bring-in under way finishes first); Tab
 * stays inside; a new step opens at its top with the keyboard on its
 * title.
 */
import { useCallback, useEffect, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { FlowFoot, FlowLayout, FlowSteps, type FlowStep } from "../FlowLayout";
import { keepTabInside } from "../form";
import { AppMark } from "../AppMark";
import { IconChevronRight } from "../icons-controls";
import { FirstRun } from "../../screens/FirstRun";
import { PermissionsStep, promptsValue, setupPrompts } from "./PermissionsStep";
import { MeetBotStep, botStanding } from "./MeetBotStep";
import { KeepRunningStep } from "./KeepRunningStep";
import { TourOfferStep } from "./TourOfferStep";
import { eyebrow, keepValue, nextWords, pagesOf, SETUP_STEP_WORDS, setupSteps, type SetupStepId } from "../../lib/setup-wizard";
import { osPromptsList, type OsPromptInfo } from "../../lib/ipc/os-prompts";
import { useAiJob } from "../../lib/ai-availability";
import { startTour, TOUR } from "../../lib/tour";

const AT_KEY = "aij.setup.at";
const ORDER_INDEX: Record<SetupStepId, number> = { apps: 0, signin: 1, access: 2, bot: 3, keep: 4, tour: 5 };

function isStep(id: string | null): id is SetupStepId {
  return id !== null && id in ORDER_INDEX;
}

/** The step the wizard was left at (Finish later), if any. */
function leftAt(): SetupStepId | null {
  try {
    const at = localStorage.getItem(AT_KEY);
    return isStep(at) ? at : null;
  } catch {
    return null;
  }
}

function rememberAt(at: SetupStepId | null): void {
  try {
    if (at && at !== "apps" && at !== "signin") localStorage.setItem(AT_KEY, at);
    else localStorage.removeItem(AT_KEY);
  } catch {
    // A private window: it starts at the beginning next time.
  }
}

export function SetupWizard({
  resumeAtAccess = false,
  openLoginTerminal,
  onAccountsChanged,
  onClose,
}: {
  resumeAtAccess?: boolean;
  openLoginTerminal: (id: string, title: string, options?: { select?: boolean; account?: string }) => void;
  onAccountsChanged?: () => void;
  /** The wizard closed (finished, or Finish later). */
  onClose: () => void;
}): JSX.Element {
  const [prompts, setPrompts] = useState<OsPromptInfo[] | null>(null);
  // Whether the permissions step shows is settled once, when it is read:
  // a grant on the step never takes the step away under the user.
  const [missing, setMissing] = useState<number | null>(null);
  useEffect(() => {
    void osPromptsList().then(
      (all) => {
        const mine = setupPrompts(all);
        setPrompts(mine);
        setMissing(mine.filter((prompt) => prompt.status !== "allowed").length);
      },
      () => {
        setPrompts([]);
        setMissing(0);
      },
    );
  }, []);
  const steps = setupSteps(missing);
  const pages = pagesOf(steps);
  // Left with Finish later, it opens again where it was left (PAT-USE (4)).
  const [wanted, setAt] = useState<SetupStepId>(() => (resumeAtAccess ? "access" : (leftAt() ?? "apps")));
  // Opened at the permissions (after Reopen) and none is missing now: on.
  const at: SetupStepId = pages.includes(wanted) ? wanted : wanted === "access" ? "bot" : "apps";
  const [furthest, setFurthest] = useState(() => ORDER_INDEX[wanted]);
  const [appsValue, setAppsValue] = useState<string | undefined>(undefined);
  const [signinValue, setSigninValue] = useState<string | undefined>(undefined);
  const [keep, setKeep] = useState<{ login: boolean | null; awake: boolean | null }>({ login: null, awake: null });
  const [busy, setBusy] = useState(false);
  // First run's links sit in this sticky foot's middle, and its primary
  // (Bring in N, Sign in to all N) at its end, after Skip this step.
  const [footSlot, setFootSlot] = useState<HTMLElement | null>(null);
  const [footEnd, setFootEnd] = useState<HTMLElement | null>(null);
  // Sign in's own skip (it counts each sign-in left as skipped).
  const skipSignin = useRef<(() => void) | null>(null);
  const chat = useAiJob("chat");
  const onKeepValue = useCallback((login: boolean | null, awake: boolean | null) => setKeep({ login, awake }), []);

  const go = (id: SetupStepId) => {
    setAt(id);
    setFurthest((now) => Math.max(now, pages.indexOf(id)));
  };
  const index = pages.indexOf(at);
  const next = pages[index + 1];
  // Your apps and Sign in are first run's, one flow: Next from either
  // goes past both; Back to them starts at Your apps.
  const afterFirstRun = pages[pages.indexOf("signin") + 1]!;
  const goNext = () => (at === "apps" || at === "signin" ? go(afterFirstRun) : next ? go(next) : finish());
  const before = pages[index - 1];
  const back = before ? () => go(before === "signin" ? "apps" : before) : undefined;
  // First run says which of its steps it's on.
  const onFirstRunStep = useCallback((step: string) => {
    if (step === "signin") {
      setAt("signin");
      setFurthest((now) => Math.max(now, ORDER_INDEX.signin));
    }
    // Back from its sign-in to its list: Your apps again (never from another step).
    else if (step === "found" || step === "fresh" || step === "folder") setAt((now) => (now === "signin" ? "apps" : now));
  }, []);

  const leave = () => {
    rememberAt(at);
    onClose();
  };
  const finish = () => {
    rememberAt(null);
    onClose();
  };
  const frame = useRef<HTMLDialogElement | null>(null);
  // A new step puts the keyboard on its title (first run's own, on its steps).
  const focusTitle = (node: HTMLElement | null) => {
    const title = node?.querySelector<HTMLElement>("h1");
    if (!title) return;
    if (!title.hasAttribute("tabindex")) title.setAttribute("tabindex", "-1");
    title.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    // A popover inside that took the Escape keeps the wizard open; a
    // bring-in under way finishes first (its Stop is on the step).
    if (event.key === "Escape" && !event.defaultPrevented && !busy) {
      event.stopPropagation();
      leave();
    }
    keepTabInside(frame.current, event);
  };

  const valueOf = (id: SetupStepId): string | undefined => {
    if (id === "apps") return appsValue;
    if (id === "signin") return signinValue;
    if (id === "access" && prompts) return promptsValue(prompts);
    if (id === "bot") {
      const standing = botStanding(chat);
      return standing === "ready" ? "Ready" : steps.indexOf("bot") < furthest ? "Later" : undefined;
    }
    if (id === "keep" && steps.indexOf("keep") <= furthest && (keep.login !== null || keep.awake !== null)) return keepValue(keep.login, keep.awake);
    return undefined;
  };
  const flowSteps: FlowStep[] = steps.map((id, n) => ({
    id,
    label: SETUP_STEP_WORDS[id].label,
    value: valueOf(id),
    state: id === at ? "current" : n <= furthest ? "done" : "todo",
    // Sign in is part of Your apps' flow: reached from there, not here.
    disabled: (n > furthest || busy || id === "signin") && id !== at,
    reason: busy ? "Bringing your apps in first" : id === "signin" ? "Comes after Your apps" : n > furthest ? "Not reached yet" : undefined,
  }));

  const close = { label: "Finish later", onClick: leave, disabled: busy, testId: "setup-not-now" };
  const nextAction = next
    ? {
        label: (
          <>
            {nextWords(steps, at)}
            <IconChevronRight aria-hidden="true" width={16} height={16} className="ml-1" />
          </>
        ),
        onClick: goNext,
        testId: "setup-next",
      }
    : undefined;
  const foot = (() => {
    if (at === "apps" || at === "signin") {
      return (
        <FlowFoot
          cancel={close}
          secondary={{
            label: "Skip this step",
            onClick: () => {
              if (at === "apps") setAppsValue((now) => now ?? "Skipped for now");
              setSigninValue((now) => now ?? (at === "signin" ? "Later" : undefined));
              if (at === "signin" && skipSignin.current) skipSignin.current();
              else goNext();
            },
            disabled: busy,
            testId: "setup-skip",
          }}
          middle={<div ref={setFootSlot} className="@container flex min-w-0 flex-1 px-2" data-testid="setup-foot-slot" />}
          end={<div ref={setFootEnd} className="flex items-center empty:hidden" data-testid="setup-foot-end" />}
        />
      );
    }
    if (at === "bot") {
      const ready = botStanding(chat) === "ready";
      // Skip for now is the engine setup's own, beside its choices.
      return <FlowFoot cancel={close} back={back && { label: "Back", onClick: back, testId: "setup-back" }} primary={ready ? nextAction : undefined} secondary={ready ? undefined : nextAction && { ...nextAction, testId: "setup-next" }} />;
    }
    if (at === "tour") {
      return (
        <FlowFoot
          cancel={close}
          back={back && { label: "Back", onClick: back, testId: "setup-back" }}
          secondary={{ label: "Finish without it", onClick: finish, testId: "setup-finish" }}
          primary={{
            label: "Take the tour",
            onClick: () => {
              finish();
              startTour();
            },
            testId: "setup-take-tour",
          }}
        />
      );
    }
    return <FlowFoot cancel={close} back={back && { label: "Back", onClick: back, testId: "setup-back" }} primary={nextAction} />;
  })();

  const words = SETUP_STEP_WORDS[at];
  return createPortal(
    <dialog
      ref={frame}
      open
      aria-modal="true"
      aria-label="Set up AI Juggler"
      className="window-sheet fixed inset-0 z-40 m-0 flex h-full max-h-none w-full max-w-none flex-col border-0 bg-canvas p-0 text-ink"
      data-testid="setup-wizard"
      data-step={at}
      onKeyDown={onKeyDown}
    >
      {/* The window's traffic lights sit over the top 28 px; the bar moves the window. */}
      <header data-tauri-drag-region className="flex items-center gap-3 border-b border-divider bg-surface px-6 pb-3 pt-8">
        <AppMark size={32} />
        <h2 className="type-section">Set up AI Juggler</h2>
      </header>
      <FlowLayout
        label={words.title}
        at={at}
        steps={<FlowSteps steps={flowSteps} onPick={(id) => {
              if (isStep(id)) go(id);
            }} label="Setup steps" testId="setup-steps" />}
        foot={foot}
        testId="setup-flow"
      >
        {/* A new page opens fresh, but Your apps and Sign in are first
            run's one flow (0.5.1 F48): keyed apart, moving to Sign in
            remounted it and dropped its sign-ins, back on Your apps. */}
        <div key={at === "signin" ? "apps" : at} ref={focusTitle} className="flex flex-col gap-6">
          <div className="flex flex-col gap-1.5">
            <p className="section-label text-accent-hi" data-testid="setup-eyebrow">
              {eyebrow(steps, at)}
            </p>
            {/* Your apps and Sign in: first run titles its own steps. */}
            {at !== "apps" && at !== "signin" && (
              <h1 tabIndex={-1} className="type-title outline-none" data-testid="setup-title">
                {words.title}
              </h1>
            )}
          </div>
          {(at === "apps" || at === "signin") && (
            <FirstRun
              embedded
              onStep={onFirstRunStep}
              footSlot={footSlot}
              footEnd={footEnd}
              skipSignin={skipSignin}
              openLoginTerminal={openLoginTerminal}
              onAccountsChanged={() => {
                setAppsValue("Apps brought in");
                onAccountsChanged?.();
              }}
              onBusy={setBusy}
              onDone={() => {
                setAppsValue((now) => now ?? "Done");
                setSigninValue((now) => now ?? (at === "signin" ? "Done" : "None needed"));
                goNext();
              }}
            />
          )}
          {at === "access" && prompts && <PermissionsStep prompts={prompts} setPrompts={setPrompts} />}
          {at === "bot" && <MeetBotStep onSkip={goNext} onChosen={goNext} />}
          {at === "keep" && <KeepRunningStep onValue={onKeepValue} />}
          {at === "tour" && <TourOfferStep stops={TOUR.length} />}
        </div>
      </FlowLayout>
    </dialog>,
    document.body,
  );
}
