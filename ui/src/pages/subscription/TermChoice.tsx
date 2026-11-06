/**
 * D-62 (AJ-E3202 req. 4) — the one-time choice where an Account's Apps
 * said different things (A9): "Work and Work 2 say different renewal
 * dates: 12 Nov or 3 Dec?", what choosing changes in plain words, and
 * one button per value naming the Apps that say it. One click sets it on
 * the Account for every App; it's never asked again. Two renewal dates:
 * the later one is chosen already, "the newer reading", and ⌘↩ accepts
 * it (the 0.4 review, 18).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { ariaOf, keysOf } from "../../lib/shortcuts";
import { IconWarn } from "../../components/icons-menu";
import { Chip } from "../../components/primitives";
import { RowNote } from "../../screens/settings/parts";
import { cliAppName } from "../../lib/app-name";
import { errorText } from "../../lib/bridge";
import { subscriptionsSetTerm, type SubscriptionInfo, type TermChoiceInfo } from "../../lib/ipc/subscriptions";
import { chooseWords, choiceQuestion, isTerm, termConsequence, termNoun, termValueWords } from "../../lib/account-terms";

/** The value chosen already: of renewal dates (`YYYY-MM-DD`), the
 * later, read after the other renewed; none for a plan or a fee. */
export function preferredValue(choice: TermChoiceInfo): string | null {
  if (choice.term !== "renewal" || choice.options.length < 2) return null;
  return choice.options.map((option) => option.value).reduce((a, b) => (b > a ? b : a));
}

export function TermChoice({ sub, choice, now, onChosen }: { sub: SubscriptionInfo; choice: TermChoiceInfo; now: number; onChosen: () => void }): JSX.Element | null {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const preferred = preferredValue(choice);
  // ⌘↩ accepts the one chosen already, unless a sheet over the page has it.
  const accept = useRef<(() => void) | null>(null);
  useEffect(() => {
    if (preferred === null) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.key !== "Enter" || !(event.metaKey || event.ctrlKey)) return;
      if (document.querySelector("[role='dialog'], [aria-modal='true']")) return;
      event.preventDefault();
      accept.current?.();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [preferred]);
  if (!isTerm(choice.term) || !sub.account_id) return null;
  const term = choice.term;
  const id = sub.account_id;
  const choose = async (value: string) => {
    setBusy(value);
    setError(null);
    try {
      await subscriptionsSetTerm(id, term, value);
      onChosen();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(null);
    }
  };
  accept.current = preferred !== null && busy === null ? () => void choose(preferred) : null;
  const apps = (folders: string[]) => folders.map((folder) => cliAppName(sub.tool, sub.profiles.find((profile) => profile.id === folder)?.label)).join(", ");
  return (
    <li className="flex flex-col gap-2.5 bg-warning/5 px-4 py-3" data-testid="account-term-choice" data-term={term}>
      <span className="flex items-start gap-1.5 text-sm font-semibold text-ink">
        <IconWarn aria-hidden="true" width={15} height={15} className="mt-0.5 shrink-0 text-warning" />
        <span>{choiceQuestion(sub, choice, now)}</span>
      </span>
      <span className="text-xs leading-relaxed text-ink-2">{termConsequence(term)}</span>
      <div role="group" aria-label={`Choose its ${termNoun(term)}`} className="flex flex-wrap gap-2">
        {choice.options.map((option) => {
          const chosen = option.value === preferred;
          return (
            <button
              key={option.value}
              type="button"
              disabled={busy !== null}
              onClick={() => void choose(option.value)}
              aria-keyshortcuts={chosen ? ariaOf("page-action") : undefined}
              className={
                "flex min-w-36 cursor-pointer flex-col items-start gap-0.5 rounded-lg border px-3 py-1.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60 " +
                (chosen ? "border-accent/60 bg-accent-soft hover:border-accent" : "border-hairline bg-surface-2 hover:border-accent/50 hover:bg-surface-3")
              }
              data-testid="account-term-option"
              data-preferred={chosen ? "true" : undefined}
            >
              <span className="flex w-full items-baseline gap-2 text-sm font-medium text-ink">
                <span className="flex-1">{busy === option.value ? "Setting…" : termValueWords(term, option.value, now)}</span>
                {chosen && <span aria-hidden="true" className="text-label font-normal text-muted">{keysOf("page-action")[0]}</span>}
              </span>
              <span className="text-label text-muted">{option.account ? (option.folders.length > 0 ? `on this account, and in ${apps(option.folders)}` : "on this account now") : `in ${apps(option.folders)}`}</span>
              {chosen && <span className="text-label font-medium text-accent-hi">the newer reading</span>}
            </button>
          );
        })}
      </div>
      {error && <RowNote tone="warning">{error}</RowNote>}
    </li>
  );
}

/** The call to choose, where an Account waits for one: on its card and
 * its page's header; it opens its Settings. Nothing when there's none. */
export function ChooseChip({ sub, onOpen }: { sub: SubscriptionInfo; onOpen: () => void }): JSX.Element | null {
  if (!sub.account_id || sub.choices.length === 0) return null;
  return (
    <button type="button" onClick={onOpen} className="shrink-0 rounded-full outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-accent/60" data-testid="account-choose">
      <Chip tone="warning">
        <IconWarn aria-hidden="true" width={12} height={12} className="shrink-0" />
        {chooseWords(sub)}
      </Chip>
    </button>
  );
}
