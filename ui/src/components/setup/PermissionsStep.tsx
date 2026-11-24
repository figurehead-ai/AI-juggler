/**
 * AJ-E1545 step "Let AI Juggler work with your Mac", on AJ-E1547's
 * registry and gate (PAT-OSP): one card per system prompt this setup
 * needs, each showing a picture of the dialog the system will show and
 * the button to click, why, what happens if declined, where it can be
 * changed later, and a short clip of the effect. Allow primes it through
 * the gate (`osPromptPrime`), which makes the call; the card reads the
 * system's answer again while shown and when the window comes back, and
 * says Allowed only when the system does. Skip for now on every card.
 *
 * The login item and Bot's helper are primed on their own steps (Keep
 * running, Meet Bot), so they aren't repeated here.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "../primitives";
import { Clip } from "../Clip";
import { OsPromptPicture } from "../OsPromptPicture";
import { IconCheck } from "../icons-controls";
import { IconWindow } from "../icons-places";
import { IconGoTo } from "../icons-actions";
import { IconShield } from "../icons";
import { permissionClip } from "../../lib/clips";
import { watchAccess } from "../../lib/osAccess";
import { same } from "../../lib/same";
import { errorText } from "../../lib/bridge";
import { SYSTEM_NAME } from "../../lib/machine-words";
import { osPromptPrime, osPromptsList, type OsPromptInfo } from "../../lib/ipc/os-prompts";

/** Primed on their own steps. */
const ELSEWHERE = new Set(["login-item", "helper", "helper-changed"]);

/** The prompts this step offers: the ones this setup needs, not primed elsewhere. */
export function setupPrompts(all: readonly OsPromptInfo[]): OsPromptInfo[] {
  return all.filter((prompt) => prompt.needed && !ELSEWHERE.has(prompt.id));
}

/** The step's value: "1 of 2 allowed", "All allowed". */
export function promptsValue(prompts: readonly OsPromptInfo[]): string {
  const allowed = prompts.filter((prompt) => prompt.status === "allowed").length;
  return prompts.length > 0 && allowed === prompts.length ? "All allowed" : `${allowed} of ${prompts.length} allowed`;
}

// One meaning per icon (ICN-1): Go to it brings a window forward; a window is what the preview shows.
const ICONS: Record<string, typeof IconWindow> = { accessibility: IconGoTo, "screen-recording": IconWindow };

function Fact({ label, children }: { label: string; children: string }): JSX.Element {
  return (
    <span className="flex flex-col gap-0.5">
      <span className="text-label text-muted">{label}</span>
      <span className="text-meta text-ink">{children}</span>
    </span>
  );
}

function PromptCard({ prompt, onAllow }: { prompt: OsPromptInfo; onAllow: () => void }): JSX.Element {
  const [skipped, setSkipped] = useState(false);
  const [asked, setAsked] = useState(false);
  const allowed = prompt.status === "allowed";
  const Icon = ICONS[prompt.id] ?? (prompt.id.startsWith("automation") ? IconGoTo : IconShield);
  const clip = permissionClip(prompt.id);
  return (
    // 0.5.1 F46 (PAT-SURF): every card the same quiet edge; Allowed is its
    // ✓ and word, never a green ring.
    <li className="flex flex-col gap-4 rounded-[14px] bg-surface p-5 inset-ring-1 inset-ring-hairline" data-testid={`setup-permission-${prompt.id}`} data-state={allowed ? "allowed" : skipped ? "skipped" : prompt.status}>
      <div className="flex flex-wrap items-start gap-3">
        <span aria-hidden="true" className={"flex size-9 shrink-0 items-center justify-center rounded-[10px] " + (allowed ? "bg-accent-soft text-accent-hi" : "bg-surface-3 text-ink-2")}>
          <Icon width={18} height={18} />
        </span>
        <span className="flex min-w-0 flex-1 basis-60 flex-col gap-0.5">
          <span className="text-section font-semibold text-ink">{prompt.title}</span>
          <span className="text-body text-ink-2">{prompt.why}</span>
        </span>
        {allowed ? (
          <output className="flex h-9 items-center gap-1.5 text-body font-medium text-success" data-testid={`setup-permission-${prompt.id}-allowed`}>
            <IconCheck width={16} height={16} strokeWidth={2.4} aria-hidden="true" />
            Allowed
          </output>
        ) : (
          <span className="flex flex-wrap items-center gap-2">
            {!skipped && (
              <Button variant="ghost" onClick={() => setSkipped(true)} data-testid={`setup-permission-${prompt.id}-skip`}>
                Skip for now <span className="sr-only">{prompt.title}</span>
              </Button>
            )}
            <Button
              variant="accent"
              onClick={() => {
                setAsked(true);
                onAllow();
              }}
              data-testid={`setup-permission-${prompt.id}-allow`}
            >
              Allow <span className="sr-only">{prompt.title}</span>
            </Button>
          </span>
        )}
      </div>
      {asked && !allowed && (
        <output className="block text-body text-ink" data-testid={`setup-permission-${prompt.id}-asked`}>
          {prompt.status === "asked"
            ? `Asked. ${SYSTEM_NAME} doesn't say what you answered; if you allowed it, it's on.`
            : `${SYSTEM_NAME}'s dialog is open. Don't see it? It can open behind this window: press ⌘Tab to find it. This card turns to Allowed as soon as you say yes.`}
        </output>
      )}
      {!allowed && (
        <div className="grid gap-4 sm:grid-cols-[minmax(0,240px)_minmax(0,1fr)]">
          {clip && <Clip id={clip} label={prompt.title} />}
          <div className="flex flex-col gap-3 rounded-xl bg-surface-2 p-4" data-testid={`setup-permission-${prompt.id}-prompt`}>
            {prompt.dialog ? <OsPromptPicture dialog={prompt.dialog} /> : <Fact label={`${SYSTEM_NAME} will show`}>{prompt.system_says}</Fact>}
            <Fact label="Click">{prompt.click}</Fact>
            <Fact label="If you say no">{prompt.if_declined}</Fact>
          </div>
        </div>
      )}
      <p className="text-meta text-muted">
        {/* 0.5.1 critique: once allowed, macOS has asked; the line says
            only where to change it. */}
        {skipped && !allowed
          ? "Skipped for now. Allow it here or later in Settings."
          : allowed
            ? `Change it in ${prompt.change_later ?? "Settings"}`
            : `${SYSTEM_NAME} will ask you · optional · change it later${prompt.change_later ? ` in ${prompt.change_later}` : " in Settings"}`}
      </p>
    </li>
  );
}

export function PermissionsStep({ prompts, setPrompts }: { prompts: OsPromptInfo[]; setPrompts: (next: OsPromptInfo[]) => void }): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  // Read again while shown and when the window is back: a grant shows at once.
  const shown = useRef(prompts);
  useEffect(() => {
    shown.current = prompts;
  }, [prompts]);
  useEffect(
    () =>
      watchAccess(
        () =>
          void osPromptsList().then(
            (next) => {
              const mine = setupPrompts(next);
              if (!same(shown.current, mine)) setPrompts(mine);
            },
            () => null,
          ),
      ),
    [setPrompts],
  );
  const allow = (id: string) => {
    setError(null);
    void osPromptPrime(id)
      .then(() => osPromptsList())
      .then((next) => setPrompts(setupPrompts(next)), (cause) => setError(errorText(cause)));
  };
  return (
    <div className="flex flex-col gap-4" data-testid="setup-permissions">
      <p className="text-body text-ink-2">
        Each one is optional: AI Juggler works without them. Each card shows the dialog {SYSTEM_NAME} will show and what to click; Allow opens it, and this page sees your answer as soon as you give it.
      </p>
      <ul className="flex flex-col gap-4">
        {prompts.map((prompt) => (
          <PromptCard key={prompt.id} prompt={prompt} onAllow={() => allow(prompt.id)} />
        ))}
      </ul>
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
