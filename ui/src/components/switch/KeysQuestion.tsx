/**
 * Keys and passwords in the conversation (D-14; the maintainer's walk of
 * 0.1.8's switch, 2026-10-27). Asked only when the preflight says the
 * conversation's words go somewhere new (another tool, and the company
 * behind it): a same-tool switch on this Mac copies the session as it
 * is, and says so in one quiet line. When it applies, it says what was
 * found (kinds and masked previews, never values), who will read it, and
 * what each choice means, with an example. Nothing is preselected.
 */
import type { JSX } from "react";
import type { PreflightInfoData } from "../../lib/bridge";
import { toolSpec } from "../../lib/tools";
import { foundWords, GUIDE, secretWords } from "../../lib/switch-words";
import { Explain } from "./Explain";

export type KeysAnswer = "strip" | "keep";

/** A remembered choice, when there is one (an "ask" policy isn't). */
function remembered(preflight: PreflightInfoData): KeysAnswer | null {
  const effective = preflight.redaction.effective;
  return effective === "strip" || effective === "keep" ? effective : null;
}

/** Whether Switch waits for an answer: the question applies, something
 * was found, no choice is remembered (or it's being changed), and none
 * is given. */
export function keysRequired(preflight: PreflightInfoData, answer: KeysAnswer | null, changing: boolean): boolean {
  if (!preflight.secrets.asks || preflight.redaction_items.length === 0) return false;
  if (remembered(preflight) !== null && !changing) return false;
  return answer === null;
}

/** What the switch sends for this answer: nothing found means nothing to
 * hide (the backend still wants an answer for a text that leaves). */
export function keysChoice(preflight: PreflightInfoData, answer: KeysAnswer | null): KeysAnswer | null {
  if (!preflight.secrets.asks) return null;
  return answer ?? (preflight.redaction_items.length === 0 ? "strip" : null);
}

const radio = "mt-1 size-4 shrink-0 accent-[var(--color-accent)]";

/** "Codex and OpenAI see", "Claude Code sees": the tool and the company
 * behind it, from its spec. */
function whoSees(toolId: string): string {
  const spec = toolSpec(toolId);
  return spec.vendor && spec.vendor !== spec.display_name ? `${spec.display_name} and ${spec.vendor} see` : `${spec.display_name} sees`;
}

export function KeysQuestion({
  preflight,
  answer,
  onAnswer,
  remember,
  onRemember,
  changing,
  onChange,
  unhide,
  onToggle,
}: {
  preflight: PreflightInfoData;
  answer: KeysAnswer | null;
  onAnswer: (answer: KeysAnswer) => void;
  remember: boolean;
  onRemember: (remember: boolean) => void;
  changing: boolean;
  onChange: () => void;
  unhide: ReadonlySet<number>;
  onToggle: (index: number) => void;
}): JSX.Element {
  const tool = toolSpec(preflight.target_tool).display_name;
  const reader = preflight.secrets.reader || tool;
  if (!preflight.secrets.asks) {
    return (
      <p className="text-body text-ink" data-testid="switch-keys-same-tool">
        The conversation is copied as it is, on this computer.
      </p>
    );
  }
  const items = preflight.redaction_items;
  if (items.length === 0) {
    return (
      <p className="text-body text-ink" data-testid="switch-keys-none">
        {foundWords(items)} {reader} will read the conversation to pick up the work.
      </p>
    );
  }
  const kept = remembered(preflight);
  if (kept !== null && !changing) {
    return (
      <p className="flex flex-wrap items-center gap-x-2 text-body text-ink" data-testid="switch-keys-remembered">
        <span>
          Keys and passwords: {kept === "strip" ? "hidden from" : "left in for"} {reader}, as you chose in Settings › Switching.
        </span>
        <button type="button" onClick={onChange} className="min-h-6 rounded text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="switch-keys-change">
          Change for this switch
        </button>
      </p>
    );
  }
  return (
    <fieldset className="flex flex-col gap-3 rounded-lg border border-hairline p-3" data-testid="switch-keys">
      <legend className="px-1 text-section font-semibold text-ink">Keys and passwords in this conversation</legend>
      <p className="text-body text-ink" data-testid="switch-keys-found">
        {foundWords(items)} {reader} will read this conversation to pick up the work.
      </p>
      <label className="flex items-start gap-2.5 text-body text-ink">
        <input type="radio" name="switch-keys" className={radio} checked={answer === "strip"} onChange={() => onAnswer("strip")} data-testid="switch-keys-hide" />
        <span>
          <span className="font-semibold">Hide them</span> <span className="text-label text-success">recommended</span>
          <span className="block text-meta">
            {tool} sees <code className="font-mono">OPENAI_API_KEY=•••</code> instead of the key. If it needs a key, it will ask you.
          </span>
        </span>
      </label>
      <label className="flex items-start gap-2.5 text-body text-ink">
        <input type="radio" name="switch-keys" className={radio} checked={answer === "keep"} onChange={() => onAnswer("keep")} data-testid="switch-keys-leave" />
        <span>
          <span className="font-semibold">Leave them in</span>
          <span className="block text-meta">{whoSees(preflight.target_tool)} the keys exactly as written. Use this only if the work needs them.</span>
        </span>
      </label>
      {answer === "strip" && (
        <div className="flex flex-col gap-1 text-meta text-ink" data-testid="switch-keys-items">
          <p>Show one anyway (this switch only):</p>
          {items.map((item, index) => (
            <label key={`${item.kind}-${index}`} className="flex items-center gap-2">
              <input type="checkbox" checked={unhide.has(index)} onChange={() => onToggle(index)} />
              <span>
                {secretWords(item.kind)} <span className="font-mono">{item.preview}</span>
              </span>
            </label>
          ))}
        </div>
      )}
      <label className="flex items-start gap-2.5 text-body text-ink">
        <input type="checkbox" className="mt-1" checked={remember} onChange={(event) => onRemember(event.target.checked)} data-testid="switch-keys-remember" />
        <span>
          Remember my choice
          <span className="block text-meta">Change it any time in Settings › Switching. Unticked, AI Juggler asks on every switch like this one.</span>
        </span>
      </label>
      <Explain
        guide={GUIDE.keys}
        tips={[
          "A key or password is text that lets someone use an account: an API key, a sign-in token, a private key.",
          `${reader} receives the conversation to continue the work. Hidden keys never leave this computer.`,
          "Hiding replaces only the secret with •••; the rest of the conversation stays as it is.",
          "If the work needs a key, the other tool asks you for it.",
        ]}
      />
    </fieldset>
  );
}
