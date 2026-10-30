/**
 * How the other account gets something AI Juggler can't copy to it (a
 * skill that comes with the claude.ai account, an OpenAI plugin): the
 * needs in plain lines, then Show me how (the numbered steps), the page
 * that does it, and Check again, which reads the account again. Used by
 * the switch's set-up list and by Settings' reminders, so both read the
 * same. Also the "What does this mean?" disclosure every choice there
 * carries, with its example and the switching guide.
 */
import { useId, useState, type JSX } from "react";
import { SWITCHING_GUIDE_URL, switchSetupOpen, type GetItInfo } from "../lib/ipc/switchSetup";
import { errorText } from "../lib/bridge";
import { trustOpen } from "../lib/ipc/trust";
import { Disclosure } from "./controls";

const BUTTON =
  "rounded border border-hairline px-2 py-0.5 text-meta text-ink hover:bg-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent";

/** "Open claude.ai settings", from the page's own address. */
export function openWords(url: string): string {
  try {
    return `Open ${new URL(url).host} settings`;
  } catch {
    return "Open the settings page";
  }
}

export function TurnOnSteps({
  getIt,
  name,
  onCheckAgain,
}: {
  getIt: GetItInfo;
  /** What it's for ("computer-use"), for the controls' names. */
  name: string;
  /** Reads the account again; resolves to what it found, in words. */
  onCheckAgain?: () => Promise<string>;
}): JSX.Element {
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const panel = useId();
  const page = getIt.open_url;
  const say = (cause: unknown) => setStatus(errorText(cause));
  const checkAgain = async () => {
    if (!onCheckAgain) return;
    setChecking(true);
    try {
      setStatus(await onCheckAgain());
    } catch (cause) {
      say(cause);
    } finally {
      setChecking(false);
    }
  };
  return (
    <div className="flex flex-col gap-1.5" data-testid="turn-on">
      {getIt.needs.length > 0 && (
        <ul className="flex flex-col gap-0.5 text-meta text-ink" data-testid="turn-on-needs">
          {getIt.needs.map((need) => (
            <li key={need}>{need}</li>
          ))}
        </ul>
      )}
      <div className="flex flex-wrap items-center gap-1.5">
        <button
          type="button"
          className={BUTTON}
          aria-expanded={open}
          aria-controls={panel}
          aria-label={`Show me how to turn on ${name}`}
          onClick={() => setOpen((now) => !now)}
        >
          Show me how {open ? "▾" : "▸"}
        </button>
        {page && (
          <button
            type="button"
            className={BUTTON}
            aria-label={`${openWords(page)} (opens in your browser)`}
            onClick={() => void switchSetupOpen(page).catch(say)}
          >
            {openWords(page)} ↗
          </button>
        )}
        {onCheckAgain && (
          <button
            type="button"
            className={BUTTON}
            disabled={checking}
            aria-label={`Check again for ${name}`}
            onClick={() => void checkAgain()}
          >
            {checking ? "Checking…" : "Check again"}
          </button>
        )}
      </div>
      {open && (
        <div id={panel} className="flex flex-col gap-1 rounded border border-hairline p-2" data-testid="turn-on-steps">
          <ol className="list-decimal pl-5 text-body text-ink">
            {getIt.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
          {getIt.then && <p className="text-meta text-ink">{getIt.then}</p>}
        </div>
      )}
      {status && (
        <p className="text-meta text-ink" role="status">
          {status}
        </p>
      )}
    </div>
  );
}

/** "What does this mean?": the choice in a sentence, an example, and the
 * guide's section. */
export function Meaning({ text, example, guide }: { text: string; example: string; guide?: string }): JSX.Element {
  return (
    <Disclosure className="text-meta" testId="setup-meaning" summary="What does this mean?">
      <div className="mt-1 flex flex-col gap-1 text-ink">
        <p>{text}</p>
        <p>For example: {example}</p>
        <p>
          <button
            type="button"
            className="text-accent underline underline-offset-2 hover:no-underline"
            // A section of its own (D-51's notes), or the guide's on what
            // the other account doesn't have.
            onClick={() => void (guide ? trustOpen({ kind: "guide", id: guide }) : switchSetupOpen(SWITCHING_GUIDE_URL))}
          >
            Read more in the switching guide ↗
          </button>
        </p>
      </div>
    </Disclosure>
  );
}
