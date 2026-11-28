/**
 * AJ-E3830 (0.5.2, opt-in Beta: Settings › AI › Models, Watches Beta) —
 * Bot's confirm card for a watch request code read whole: every field
 * verified (the target code's own, never the model's pick), the form's own
 * check passed. The watch in the code's plain words (the Watches
 * preview's echo, word for word what runs), quiet hours in one line, how
 * long said as suggested when the request didn't say, and Watch /
 * Change… (the form, filled in) / Cancel. Nothing is written until Watch.
 */
import { useEffect, useState, type JSX } from "react";
import { ConfirmFoot, ConfirmNotes, ConfirmShell, EchoLine, WatchSet, useQuietLine } from "./BotConfirmParts";
import { openWatches } from "../../lib/watches-open";
import { watchesCreate, watchesPreview } from "../../lib/ipc/watches";
import { errorText } from "../../lib/bridge";
import type { WatchPrefill } from "../../types/generated/WatchPrefill";

const WORDS = {
  en: { label: "Confirm a new watch", suggested: "How long wasn't said: this is the suggestion. Change… to pick another.", closed: "Not set. Ask again any time." },
  de: { label: "Neue Beobachtung bestätigen", suggested: "Wie lange, war nicht gesagt: das ist der Vorschlag. Mit Ändern… wählst du anders.", closed: "Nicht gesetzt. Frag jederzeit wieder." },
};

export function BotWatchConfirm({ prefill, lang }: { prefill: WatchPrefill; lang: "en" | "de" }): JSX.Element {
  const w = WORDS[lang] ?? WORDS.en;
  const draft = prefill.draft;
  const quiet = useQuietLine();
  const [words, setWords] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [closed, setClosed] = useState(false);
  useEffect(() => {
    let live = true;
    void watchesPreview(draft).then(
      (preview) => {
        if (!live) return;
        setWords(preview.words);
        setProblem(preview.problem);
      },
      (cause) => live && setError(errorText(cause)),
    );
    return () => {
      live = false;
    };
  }, [draft]);
  if (done) return <WatchSet words={done} />;
  if (closed)
    return (
      <p className="text-meta text-muted" data-testid="bot-watch-confirm-closed">
        {w.closed}
      </p>
    );
  const watch = async () => {
    try {
      const made = await watchesCreate(draft);
      setDone(made.words);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  return (
    <ConfirmShell label={w.label} testId="bot-watch-confirm-card" onClose={() => setClosed(true)}>
      <EchoLine words={words ?? "…"} />
      {draft.lifetime.kind === "unsaid" && (
        <p className="text-meta text-ink-2" data-testid="bot-watch-confirm-suggested">
          {w.suggested}
        </p>
      )}
      <ConfirmNotes problem={problem} quiet={quiet} error={error} />
      <ConfirmFoot canWatch={words !== null && problem === null} onWatch={() => void watch()} onChange={() => openWatches({ kind: "new", prefill })} onCancel={() => setClosed(true)} testPrefix="bot-watch-confirm" />
    </ConfirmShell>
  );
}
