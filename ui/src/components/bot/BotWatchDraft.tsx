/**
 * AJ-E3830 (0.5.2) — Bot pre-fills the Watches form: every request Bot
 * reads as a watch request opens the form with what code verified, and
 * the one line says what was left for the user (in its words, by field);
 * with every field verified and the Watches Beta on, Bot's confirm card
 * instead (`BotWatchConfirm`). Before 0.5.2 (D-149): Bot fills the Watches form. A request to be told
 * ("tell me when the build finishes") gets a draft from two small model
 * calls; the turn opens the Watches form filled in, once, and says so in
 * one line. The form is the one editor (no second one here), and only the
 * user's click in it adds the watch. Something Bot can't watch is said in
 * the request's own words, and the form opens with the nearest it can (or
 * empty). A past turn keeps its line and its way back to the form.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { IconEye } from "./bot-icons";
import { BotWatchDraftWords } from "./BotWatchDraftWords";
import { openWatches, type WatchesIntent } from "../../lib/watches-open";
import { BotReminderDraft } from "./BotReminderDraft";
import { IconClockFace } from "../watches/watch-icons";
import type { BotTurnInfo } from "../../types/generated/BotTurnInfo";
import type { WatchDrafted } from "../../types/generated/WatchDrafted";
import type { WatchPrefill } from "../../types/generated/WatchPrefill";
import { BotWatchConfirm } from "./BotWatchConfirm";

const FIELDS = {
  en: { target: "what to watch", condition: "what to watch for", lifetime: "how long", quiet_hours: "quiet hours" },
  de: { target: "was beobachtet wird", condition: "worauf geachtet wird", lifetime: "wie lange", quiet_hours: "die Ruhezeiten" },
} as const;

function listed(names: string[], and: string): string {
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} ${and} ${names.at(-1)}`;
}

/** The pre-fill's lines, by language (the backend's words, same fields). */
const PREFILL = {
  en: {
    and: "and",
    verified: (card: boolean) => (card ? "Here is the watch. Check it, then click Watch." : "I filled in the Watches form. Check it and click Watch."),
    several: (also: string) => `More than one matches that name, so the Watches form asks which one.${also ? ` Also left for you to pick: ${also}.` : ""}`,
    cant: (asked: string, near: boolean) => `I can't watch for “${asked}”. The Watches form is open with what I could fill in.${near ? " The nearest it can watch is marked." : ""}`,
    noCondition: "Your words don't say what to watch for. The Watches form is open with the rest filled in.",
    none: "I couldn't fill the form from your words. The Watches form is open.",
    left: (asked: string | null, left: string) => `${asked ? `I couldn't read “${asked}”. ` : ""}Left for you to pick: ${left}. The Watches form is open.`,
  },
  de: {
    and: "und",
    verified: (card: boolean) => (card ? "Hier ist die Beobachtung. Prüf sie, dann klick auf Beobachten." : "Ich habe das Beobachtungsformular ausgefüllt. Prüf es und klick auf Beobachten."),
    several: (also: string) => `Mehr als eine passt zu dem Namen, das Formular fragt, welche.${also ? ` Auch noch zu wählen: ${also}.` : ""}`,
    cant: (asked: string, near: boolean) => `Darauf kann ich nicht achten: „${asked}“. Das Formular ist offen, mit dem, was ich ausfüllen konnte.${near ? " Das Nächstliegende ist markiert." : ""}`,
    noCondition: "Deine Worte sagen nicht, worauf geachtet werden soll. Das Formular ist offen, der Rest ausgefüllt.",
    none: "Ich konnte das Formular aus deinen Worten nicht ausfüllen. Es ist offen.",
    left: (asked: string | null, left: string) => `${asked ? `„${asked}“ konnte ich nicht lesen. ` : ""}Noch zu wählen: ${left}. Das Formular ist offen.`,
  },
};

/** The pre-fill's one line, as the backend writes it (same fields). */
function prefillLine(p: WatchPrefill, lang: BotTurnInfo["lang"]): string {
  const l = lang === "de" ? PREFILL.de : PREFILL.en;
  const names = (fields: readonly WatchPrefill["left"][number][]) => listed(fields.map((f) => (lang === "de" ? FIELDS.de : FIELDS.en)[f]), l.and);
  const none = p.left.length === 4 || (p.left.includes("target") && p.left.includes("condition"));
  if (p.why === "verified") return l.verified(p.card);
  if (p.why === "several") return l.several(names(p.left.filter((f) => f !== "target")));
  if (p.why === "cant_watch" && p.asked) return l.cant(p.asked, p.nearest !== null && p.nearest !== undefined);
  if (p.why === "no_condition" && !none) return l.noCondition;
  if (none) return l.none;
  return l.left(p.asked ?? null, names(p.left));
}

const WORDS = {
  en: {
    filled: "I filled in the Watches form. Check it, then add the watch there.",
    pick: "Two match that name: the Watches form asks which one.",
    cant: (asked: string, near: boolean) => `I can't watch for “${asked}”.` + (near ? " The form is open with the nearest I can." : " The form is open."),
    form: "I couldn't fill the form from your words. The Watches form is open.",
    past: "Bot filled in the Watches form for this.",
    open: "Open the form",
    isOpen: "The Watches form is open.",
    remind: (at: boolean) => (at ? "I drafted the reminder. Check the time, then set it." : "I drafted the reminder. Pick a time, then set it."),
    remindSet: "Bot set a reminder for this.",
    remindNotSet: "Bot drafted a reminder for this; it wasn't set.",
    openReminder: "Open reminder",
  },
  de: {
    filled: "Ich habe das Beobachtungsformular ausgefüllt. Prüf es und füge die Beobachtung dort hinzu.",
    pick: "Zwei passen zu dem Namen: das Formular fragt, welche.",
    cant: (asked: string, near: boolean) => `Darauf kann ich nicht achten: „${asked}“.` + (near ? " Das Formular ist mit dem Nächstliegenden offen." : " Das Formular ist offen."),
    form: "Ich konnte das Formular aus deinen Worten nicht ausfüllen. Es ist offen.",
    past: "Bot hat dafür das Beobachtungsformular ausgefüllt.",
    open: "Formular öffnen",
    isOpen: "Das Beobachtungsformular ist offen.",
    remind: (at: boolean) => (at ? "Ich habe die Erinnerung vorbereitet. Prüf die Zeit und setz sie." : "Ich habe die Erinnerung vorbereitet. Wähl eine Zeit und setz sie."),
    remindSet: "Bot hat dafür eine Erinnerung gesetzt.",
    remindNotSet: "Bot hat dafür eine Erinnerung vorbereitet; sie wurde nicht gesetzt.",
    openReminder: "Erinnerung öffnen",
  },
};

const OPEN = "inline-flex h-7 items-center rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60";

/** 0.5.2 G30 (RW1 G2): a reminder turn read back (its draft is gone after
 * a relaunch) says it set a reminder, with the way to it, or that it
 * drafted one that wasn't set: never "filled in the Watches form". */
function PastReminder({ set, lang }: { set: string | null; lang: BotTurnInfo["lang"] }): JSX.Element {
  const w = WORDS[lang] ?? WORDS.en;
  return (
    <div className="flex flex-col items-start gap-2">
      <p className="flex items-center gap-2 text-body leading-normal text-ink-2" data-testid="bot-reminder-past" data-set={set ? "1" : "0"}>
        <IconClockFace width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
        {set ? w.remindSet : w.remindNotSet}
      </p>
      {set && (
        <button type="button" className={OPEN} onClick={() => openWatches({ kind: "reminder", id: set })} data-testid="bot-reminder-past-open">
          {w.openReminder}
        </button>
      )}
    </div>
  );
}

/** What the form opens on for a draft. */
function intentOf(drafted: WatchDrafted): WatchesIntent {
  if (drafted.kind === "remind") return { kind: "list" };
  if (drafted.kind === "prefilled") return { kind: "new", prefill: drafted.prefill };
  if (drafted.kind === "filled" || drafted.kind === "pick") return { kind: "new", draft: drafted.draft };
  if (drafted.kind === "cant_watch" && drafted.draft) return { kind: "new", draft: drafted.draft };
  return { kind: "new" };
}

function lineOf(drafted: WatchDrafted | null | undefined, lang: BotTurnInfo["lang"]): string {
  const w = WORDS[lang] ?? WORDS.en;
  if (!drafted) return w.past;
  if (drafted.kind === "prefilled") return prefillLine(drafted.prefill, lang);
  if (drafted.kind === "filled") return w.filled;
  if (drafted.kind === "pick") return w.pick;
  if (drafted.kind === "cant_watch") return w.cant(drafted.asked, Boolean(drafted.draft));
  return w.form;
}

export function BotWatchDraft({ turn }: { turn: BotTurnInfo }): JSX.Element {
  const drafted = turn.watch_draft ?? null;
  const opened = useRef(false);
  // 0.5.2 G75 (RW1 Part T): the form opened by itself: the reply says so,
  // and offers "Open the form" only when it didn't open (a turn read back).
  const [selfOpened, setSelfOpened] = useState(false);
  useEffect(() => {
    // The turn just asked opens the form once; a turn read back doesn't,
    // nor Bot's confirm card (its Change… opens the form).
    const card = drafted?.kind === "prefilled" && drafted.prefill.card;
    if (drafted && drafted.kind !== "remind" && !card && turn.live && !opened.current) {
      opened.current = true;
      openWatches(intentOf(drafted));
      setSelfOpened(true);
    }
  }, [drafted, turn.live]);
  const w = WORDS[turn.lang] ?? WORDS.en;
  if (drafted?.kind === "remind") return <BotReminderDraft draft={drafted.reminder} line={w.remind(drafted.reminder.at_ms !== null)} />;
  if (!drafted && turn.past_reminder) return <PastReminder set={turn.past_reminder.set ?? null} lang={turn.lang} />;
  if (drafted?.kind === "prefilled" && drafted.prefill.card)
    return (
      <div className="flex flex-col items-start gap-2">
        <p className="flex items-center gap-2 text-body leading-normal text-ink-2" data-testid="bot-watch-draft">
          <IconEye width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
          {lineOf(drafted, turn.lang)}
        </p>
        <BotWatchConfirm prefill={drafted.prefill} lang={turn.lang === "de" ? "de" : "en"} />
      </div>
    );
  return (
    <div className="flex flex-col items-start gap-2">
      <p className="flex items-center gap-2 text-body leading-normal text-ink-2" data-testid="bot-watch-draft">
        <IconEye width={16} height={16} aria-hidden="true" className="shrink-0 text-accent-hi" />
        {lineOf(drafted, turn.lang)}
      </p>
      <BotWatchDraftWords drafted={drafted} />
      {selfOpened ? (
        !/is open|ist offen/.test(lineOf(drafted, turn.lang)) && (
          <p className="pl-6 text-meta text-muted" data-testid="bot-watch-draft-is-open">
            {w.isOpen}
          </p>
        )
      ) : (
        <button
          type="button"
          className={OPEN}
          onClick={() => openWatches(drafted ? intentOf(drafted) : { kind: "list" })}
          data-testid="bot-watch-draft-open"
        >
          {w.open}
        </button>
      )}
    </div>
  );
}
