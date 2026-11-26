/**
 * AJ-E3830 (D-149) — Bot fills the Watches form. A request to be told
 * ("tell me when the build finishes") gets a draft from two small model
 * calls; the turn opens the Watches form filled in, once, and says so in
 * one line. The form is the one editor (no second one here), and only the
 * user's click in it adds the watch. Something Bot can't watch is said in
 * the request's own words, and the form opens with the nearest it can (or
 * empty). A past turn keeps its line and its way back to the form.
 */
import { useEffect, useRef, type JSX } from "react";
import { IconEye } from "./bot-icons";
import { BotWatchDraftWords } from "./BotWatchDraftWords";
import { openWatches, type WatchesIntent } from "../../lib/watches-open";
import { BotReminderDraft } from "./BotReminderDraft";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";
import type { WatchDrafted } from "../../types/generated/WatchDrafted";

const WORDS = {
  en: {
    filled: "I filled in the Watches form. Check it, then add the watch there.",
    pick: "Two match that name: the Watches form asks which one.",
    cant: (asked: string, near: boolean) => `I can't watch for “${asked}”.` + (near ? " The form is open with the nearest I can." : " The form is open."),
    form: "I couldn't fill the form from your words. The Watches form is open.",
    past: "Bot filled in the Watches form for this.",
    open: "Open the form",
    remind: (at: boolean) => (at ? "I drafted the reminder. Check the time, then set it." : "I drafted the reminder. Pick a time, then set it."),
  },
  de: {
    filled: "Ich habe das Beobachtungsformular ausgefüllt. Prüf es und füge die Beobachtung dort hinzu.",
    pick: "Zwei passen zu dem Namen: das Formular fragt, welche.",
    cant: (asked: string, near: boolean) => `Darauf kann ich nicht achten: „${asked}“.` + (near ? " Das Formular ist mit dem Nächstliegenden offen." : " Das Formular ist offen."),
    form: "Ich konnte das Formular aus deinen Worten nicht ausfüllen. Es ist offen.",
    past: "Bot hat dafür das Beobachtungsformular ausgefüllt.",
    open: "Formular öffnen",
    remind: (at: boolean) => (at ? "Ich habe die Erinnerung vorbereitet. Prüf die Zeit und setz sie." : "Ich habe die Erinnerung vorbereitet. Wähl eine Zeit und setz sie."),
  },
};

/** What the form opens on for a draft. */
function intentOf(drafted: WatchDrafted): WatchesIntent {
  if (drafted.kind === "remind") return { kind: "list" };
  if (drafted.kind === "filled" || drafted.kind === "pick") return { kind: "new", draft: drafted.draft };
  if (drafted.kind === "cant_watch" && drafted.draft) return { kind: "new", draft: drafted.draft };
  return { kind: "new" };
}

function lineOf(drafted: WatchDrafted | null | undefined, lang: BotTurnInfo["lang"]): string {
  const w = WORDS[lang] ?? WORDS.en;
  if (!drafted) return w.past;
  if (drafted.kind === "filled") return w.filled;
  if (drafted.kind === "pick") return w.pick;
  if (drafted.kind === "cant_watch") return w.cant(drafted.asked, Boolean(drafted.draft));
  return w.form;
}

export function BotWatchDraft({ turn }: { turn: BotTurnInfo }): JSX.Element {
  const drafted = turn.watch_draft ?? null;
  const opened = useRef(false);
  useEffect(() => {
    // The turn just asked opens the form once; a turn read back doesn't.
    if (drafted && drafted.kind !== "remind" && turn.live && !opened.current) {
      opened.current = true;
      openWatches(intentOf(drafted));
    }
  }, [drafted, turn.live]);
  const w = WORDS[turn.lang] ?? WORDS.en;
  if (drafted?.kind === "remind") return <BotReminderDraft draft={drafted.reminder} line={w.remind(drafted.reminder.at_ms !== null)} />;
  return (
    <div className="flex flex-col items-start gap-2">
      <p className="flex items-center gap-2 text-body leading-normal text-ink-2" data-testid="bot-watch-draft">
        <IconEye width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
        {lineOf(drafted, turn.lang)}
      </p>
      <BotWatchDraftWords drafted={drafted} />
      <button
        type="button"
        className="inline-flex h-7 items-center rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
        onClick={() => openWatches(drafted ? intentOf(drafted) : { kind: "list" })}
        data-testid="bot-watch-draft-open"
      >
        {w.open}
      </button>
    </div>
  );
}
