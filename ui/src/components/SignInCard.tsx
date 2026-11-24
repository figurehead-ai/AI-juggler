/**
 * AJ-E1545 R8 / AJ-E1553 — a sign-in, as a newcomer expects it: the tool's
 * own page, not its terminal. One card for the setup wizard's Sign in step
 * and a sign-in's own page (the rail's Sign-ins open it).
 *
 * - The tool showed a page (its adapter read it): "Open <tool>'s sign-in
 *   page" as the one primary, the device code large with Copy and "Check
 *   the page shows this code, then approve", or a field for a code the
 *   page gives back; a calm "Waiting for you to approve…". The terminal
 *   stays under "Show details", collapsed.
 * - Nothing to read (yet, or ever): the terminal, with one plain line.
 * - While it runs, "Stop signing in" ends it (F29: not only ctrl-c in its
 *   terminal); it then shows as ended, with Try again.
 * - Signed in: "✓ <tool> is signed in".
 * - Ended before it finished: its last lines, Try again, and Dismiss;
 *   never an empty terminal.
 *
 * The page is opened by the backend from what the adapter read (its own
 * hosts only); opening a web page raises no system prompt
 * (docs/design/os-prompts.md).
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { Disclosure } from "./controls";
import { Input } from "./form";
import { Check } from "./StepCheck";
import { providerDisplayName } from "../lib/providers";
import { lazyNamed } from "../lib/lazy";
import { ListSkeleton } from "./ListSkeleton";
import { errorText } from "../lib/bridge";
import { signInsOpenPage, type SignInInfo } from "../lib/ipc/sign-ins";
import { terminalKill, terminalWrite } from "../lib/ipc/terminal-io";

// xterm loads only when a sign-in's terminal is shown; until it is here,
// the frame holds a skeleton, never a blank box (AJ-E1549 R3: the wizard
// has no screen around it to lend its own).
const TerminalView = lazyNamed(() => import("./TerminalView"), "TerminalView", false, () => (
  <div className="p-3">
    <ListSkeleton label="Loading the terminal" rows={2} />
  </div>
));

/** The terminal, at the size each place gives it. */
function Terminal({ id, title, tall }: { id: string; title: string; tall: boolean }): JSX.Element {
  return (
    <div
      className={"overflow-hidden rounded-lg bg-[#05070b] " + (tall ? "h-72" : "h-56")}
      data-testid="sign-in-terminal"
    >
      <TerminalView id={id} title={title} />
    </div>
  );
}

/** Ends a running sign-in (its terminal's process); the backend then
 * lists it as ended, so the card turns to Try again. */
function StopSignIn({ id, onError }: { id: string; onError: (message: string) => void }): JSX.Element {
  const [stopping, setStopping] = useState(false);
  return (
    <Button
      size="sm"
      variant="ghost"
      disabled={stopping}
      onClick={() => {
        setStopping(true);
        void terminalKill(id).catch((cause) => {
          setStopping(false);
          onError(errorText(cause));
        });
      }}
      data-testid="sign-in-stop"
    >
      {stopping ? "Stopping…" : "Stop signing in"}
    </Button>
  );
}

/** The device code, large, with Copy. */
function Code({ code }: { code: string }): JSX.Element {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex flex-wrap items-center gap-3">
      <span className="rounded-lg bg-surface-2 px-3 py-1.5 font-mono text-stat font-semibold tracking-[0.18em] text-ink" data-testid="sign-in-code">
        {code}
      </span>
      <Button
        size="sm"
        variant="secondary"
        onClick={() => {
          void (navigator.clipboard?.writeText(code) ?? Promise.resolve())
            .catch(() => null)
            .then(() => setCopied(true));
        }}
      >
        {copied ? "Copied" : "Copy"}
      </Button>
    </div>
  );
}

/** A code the page gives back, typed into the tool's sign-in for the user. */
function PasteBack({ id, tool }: { id: string; tool: string }): JSX.Element {
  const [value, setValue] = useState("");
  const [sent, setSent] = useState(false);
  return (
    <form
      className="flex flex-col gap-1.5"
      onSubmit={(event) => {
        event.preventDefault();
        const code = value.trim();
        if (!code) return;
        void terminalWrite(id, `${code}\r`);
        setValue("");
        setSent(true);
      }}
    >
      <label htmlFor={`paste-${id}`} className="text-meta text-ink-2">
        If the page gives you a code, paste it here for {tool}:
      </label>
      <div className="flex items-center gap-2">
        <Input
          id={`paste-${id}`}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          autoComplete="off"
          className="min-w-0 flex-1 font-mono"
          data-testid="sign-in-paste"
        />
        <Button size="sm" variant="secondary" type="submit" disabled={!value.trim()}>
          Send
        </Button>
      </div>
      {sent && <span className="text-meta text-muted">Sent to {tool}.</span>}
    </form>
  );
}

export function SignInCard({
  signIn,
  terminalId,
  tool,
  title,
  tall = false,
  onTryAgain,
  onDismiss,
}: {
  /** The backend's entry (null until it is listed: shown as starting). */
  signIn: SignInInfo | null;
  terminalId: string;
  /** The App's tool (its words come from its spec). */
  tool: string;
  title: string;
  /** A sign-in's own page gives the terminal more room. */
  tall?: boolean;
  onTryAgain: () => void;
  /** Takes an ended (or finished) sign-in off the list. */
  onDismiss?: () => void;
}): JSX.Element {
  const name = providerDisplayName(tool);
  const [error, setError] = useState<string | null>(null);
  const state = signIn?.state ?? "running";

  if (state === "signed_in") {
    return (
      <div className="flex flex-wrap items-center gap-3" data-testid="sign-in-card" data-state="signed_in">
        <output className="flex items-center gap-1.5 text-body font-medium text-success">
          <Check />
          {name} is signed in
        </output>
        {onDismiss && (
          <Button size="sm" variant="secondary" onClick={onDismiss}>
            Done
          </Button>
        )}
      </div>
    );
  }

  if (state === "ended") {
    const lines = signIn?.last_lines ?? [];
    return (
      <div className="flex flex-col gap-3" data-testid="sign-in-card" data-state="ended">
        <output className="block text-body font-medium text-ink">
          Sign-in ended before it finished
        </output>
        {lines.length > 0 ? (
          <div className="flex flex-col gap-1">
            <span className="text-meta text-muted">Its last lines:</span>
            <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded-lg bg-surface-2 px-3 py-2 font-mono text-meta leading-relaxed text-ink-2" data-testid="sign-in-last-lines">
              {lines.join("\n")}
            </pre>
          </div>
        ) : (
          <p className="text-meta text-muted">{name} said nothing before it ended.</p>
        )}
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="primary" onClick={onTryAgain}>
            Try again
          </Button>
          {onDismiss && (
            <Button variant="ghost" onClick={onDismiss}>
              Dismiss
            </Button>
          )}
        </div>
      </div>
    );
  }

  const hasPage = Boolean(signIn?.page_host);
  if (!hasPage) {
    // Nothing read from its output (yet, or a sign-in that can't be read):
    // its terminal, with one plain line.
    return (
      <div className="flex flex-col gap-2" data-testid="sign-in-card" data-state="terminal">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-meta leading-relaxed text-ink-2">
            {name}'s sign-in runs below: follow its steps there. AI Juggler notices when you're done.
          </p>
          <StopSignIn id={terminalId} onError={setError} />
        </div>
        <Terminal id={terminalId} title={title} tall={tall} />
        {error && (
          <p className="text-meta text-danger" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }

  const code = signIn?.code ?? null;
  return (
    <div className="flex flex-col gap-3" data-testid="sign-in-card" data-state="page">
      <div className="flex flex-wrap items-center gap-3">
        <Button
          variant="primary"
          onClick={() => {
            setError(null);
            void signInsOpenPage(terminalId).catch((cause) => setError(errorText(cause)));
          }}
        >
          Open {name}'s sign-in page
        </Button>
        <span className="text-meta text-muted">{signIn?.page_host}</span>
      </div>
      {code && (
        <div className="flex flex-col gap-1.5">
          <Code code={code} />
          <p className="text-meta text-ink-2">Check the page shows this code, then approve.</p>
        </div>
      )}
      {!code && signIn?.pastes_code && <PasteBack id={terminalId} tool={name} />}
      {!code && !signIn?.pastes_code && <p className="text-meta text-ink-2">Sign in on the page, then approve.</p>}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <output className="flex items-center gap-2 text-meta text-muted">
          <span aria-hidden="true" className="dot-live size-1.5 rounded-full bg-live" />
          Waiting for you to approve…
        </output>
        <StopSignIn id={terminalId} onError={setError} />
      </div>
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
      <Disclosure summary="Show details" testId="sign-in-details">
        <div className="pt-2">
          <Terminal id={terminalId} title={title} tall={tall} />
        </div>
      </Disclosure>
    </div>
  );
}
