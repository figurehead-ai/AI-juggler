//! Every word a watch is shown in, English and German (ADR-0036 §2.4 step
//! 5): code composes the echo from the typed watch with these, never a
//! model's words. A condition's phrases sit here beside the frame's, so
//! the plain-words lint and a translator read one file. `{n}` is the
//! condition's number, `{s}` a task state's word, `{a}` an agent's name.

/// A language the words cover (English and German in 0.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum Lang {
    /// English.
    #[default]
    En,
    /// German.
    De,
}

/// One phrase, for one target and for several together ("all of").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phrase {
    /// "api finishes".
    pub one: &'static str,
    /// "api and web finish".
    pub many: &'static str,
}

/// A condition's words: without its parameter, and with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CondWords {
    /// English.
    pub en: Phrase,
    /// English, with the parameter.
    pub en_param: Option<Phrase>,
    /// German.
    pub de: Phrase,
    /// German, with the parameter.
    pub de_param: Option<Phrase>,
    /// The catalogue's label ("Finishes"), English.
    pub label_en: &'static str,
    /// The catalogue's label, German.
    pub label_de: &'static str,
    /// AJ-E3817: what happened, said after the name in an alert ("api
    /// finished"), English.
    pub told_en: &'static str,
    /// The same, German.
    pub told_de: &'static str,
}

const fn p(one: &'static str, many: &'static str) -> Phrase {
    Phrase { one, many }
}

/// `finishes`.
pub const FINISHES: CondWords = CondWords {
    en: p("finishes", "finish"),
    en_param: None,
    de: p("fertig ist", "fertig sind"),
    de_param: None,
    label_en: "Finishes",
    label_de: "Ist fertig",
    told_en: "finished",
    told_de: "ist fertig",
};

/// `ends`.
pub const ENDS: CondWords = CondWords {
    en: p("ends", "end"),
    en_param: None,
    de: p("endet", "enden"),
    de_param: None,
    label_en: "Ends",
    label_de: "Endet",
    told_en: "ended",
    told_de: "ist beendet",
};

/// `asks_you`.
pub const ASKS_YOU: CondWords = CondWords {
    en: p("asks you", "ask you"),
    en_param: Some(p(
        "asks you and waits {n} minutes",
        "ask you and wait {n} minutes",
    )),
    de: p("dich etwas fragt", "dich etwas fragen"),
    de_param: Some(p(
        "dich etwas fragt und {n} Minuten wartet",
        "dich etwas fragen und {n} Minuten warten",
    )),
    label_en: "Asks you",
    label_de: "Fragt dich",
    told_en: "asks you",
    told_de: "fragt dich etwas",
};

/// `fails`.
pub const FAILS: CondWords = CondWords {
    en: p("fails", "fail"),
    en_param: None,
    de: p("fehlschlägt", "fehlschlagen"),
    de_param: None,
    label_en: "Fails",
    label_de: "Schlägt fehl",
    told_en: "failed",
    told_de: "ist fehlgeschlagen",
};

/// `stuck`.
pub const STUCK: CondWords = CondWords {
    en: p("looks stuck", "look stuck"),
    en_param: Some(p(
        "looks stuck for {n} minutes",
        "look stuck for {n} minutes",
    )),
    de: p("festzuhängen scheint", "festzuhängen scheinen"),
    de_param: Some(p(
        "seit {n} Minuten festzuhängen scheint",
        "seit {n} Minuten festzuhängen scheinen",
    )),
    label_en: "Looks stuck",
    label_de: "Scheint festzuhängen",
    told_en: "looks stuck",
    told_de: "scheint festzuhängen",
};

/// `idle_for` (its parameter is required).
pub const IDLE_FOR: CondWords = CondWords {
    en: p("is idle", "are idle"),
    en_param: Some(p("is idle for {n} minutes", "are idle for {n} minutes")),
    de: p("untätig ist", "untätig sind"),
    de_param: Some(p("{n} Minuten untätig ist", "{n} Minuten untätig sind")),
    label_en: "Is idle",
    label_de: "Ist untätig",
    told_en: "has been idle",
    told_de: "ist untätig",
};

/// `hits_limit`.
pub const HITS_LIMIT: CondWords = CondWords {
    en: p("hits a usage limit", "hit a usage limit"),
    en_param: None,
    de: p("ein Nutzungslimit erreicht", "ein Nutzungslimit erreichen"),
    de_param: None,
    label_en: "Hits a usage limit",
    label_de: "Erreicht ein Nutzungslimit",
    told_en: "hit a usage limit",
    told_de: "hat ein Nutzungslimit erreicht",
};

/// `runway_below` (its parameter is required).
pub const RUNWAY_BELOW: CondWords = CondWords {
    en: p("runs low", "run low"),
    en_param: Some(p("runs below {n} %", "run below {n} %")),
    de: p("knapp wird", "knapp werden"),
    de_param: Some(p(
        "unter {n} % Spielraum fällt",
        "unter {n} % Spielraum fallen",
    )),
    label_en: "Runs low",
    label_de: "Wird knapp",
    told_en: "is running low on room",
    told_de: "wird knapp",
};

/// `runway_resets`.
pub const RUNWAY_RESETS: CondWords = CondWords {
    en: p("has room again", "have room again"),
    en_param: None,
    de: p("wieder Spielraum hat", "wieder Spielraum haben"),
    de_param: None,
    label_en: "Has room again",
    label_de: "Hat wieder Spielraum",
    told_en: "has room again",
    told_de: "hat wieder Spielraum",
};

/// `leaves_processes`.
pub const LEAVES_PROCESSES: CondWords = CondWords {
    en: p("leaves processes running", "leave processes running"),
    en_param: None,
    de: p("Prozesse laufen lässt", "Prozesse laufen lassen"),
    de_param: None,
    label_en: "Leaves processes running",
    label_de: "Lässt Prozesse laufen",
    told_en: "left processes running",
    told_de: "hat Prozesse laufen lassen",
};

/// `restarted`.
pub const RESTARTED: CondWords = CondWords {
    en: p("is restarted after a crash", "are restarted after a crash"),
    en_param: None,
    de: p(
        "nach einem Absturz neu startet",
        "nach einem Absturz neu starten",
    ),
    de_param: None,
    label_en: "Restarts after a crash",
    label_de: "Startet nach einem Absturz neu",
    told_en: "was restarted after a crash",
    told_de: "wurde nach einem Absturz neu gestartet",
};

/// `pool_message`.
pub const POOL_MESSAGE: CondWords = CondWords {
    en: p("gets a message", "get a message"),
    en_param: Some(p("gets a message from {a}", "get a message from {a}")),
    de: p("eine Nachricht bekommt", "eine Nachricht bekommen"),
    de_param: Some(p(
        "eine Nachricht von {a} bekommt",
        "eine Nachricht von {a} bekommen",
    )),
    label_en: "Gets a message",
    label_de: "Bekommt eine Nachricht",
    told_en: "got a message",
    told_de: "hat eine Nachricht bekommen",
};

/// `task_changes` (its parameter is required).
pub const TASK_CHANGES: CondWords = CondWords {
    en: p("has a task moved", "have a task moved"),
    en_param: Some(p("has a task moved to {s}", "have a task moved to {s}")),
    de: p(
        "eine Aufgabe verschoben hat",
        "eine Aufgabe verschoben haben",
    ),
    de_param: Some(p(
        "eine Aufgabe auf „{s}“ verschoben hat",
        "eine Aufgabe auf „{s}“ verschoben haben",
    )),
    label_en: "A task moves",
    label_de: "Eine Aufgabe wird verschoben",
    told_en: "had a task moved",
    told_de: "hat eine Aufgabe verschoben",
};

/// `commits`.
pub const COMMITS: CondWords = CondWords {
    en: p("gets a commit", "get a commit"),
    en_param: None,
    de: p("einen Commit bekommt", "einen Commit bekommen"),
    de_param: None,
    label_en: "Gets a commit",
    label_de: "Bekommt einen Commit",
    told_en: "got a commit",
    told_de: "hat einen Commit bekommen",
};

/// `starts`.
pub const STARTS: CondWords = CondWords {
    en: p("starts a new session", "start a new session"),
    en_param: None,
    de: p("eine neue Sitzung startet", "eine neue Sitzung starten"),
    de_param: None,
    label_en: "Starts a session",
    label_de: "Startet eine Sitzung",
    told_en: "started a new session",
    told_de: "hat eine neue Sitzung gestartet",
};

/// A task state, in words.
#[must_use]
pub const fn task_state(state: crate::features::observe::TaskState, lang: Lang) -> &'static str {
    use crate::features::observe::TaskState;
    match (state, lang) {
        (TaskState::Done, Lang::En) => "done",
        (TaskState::Blocked, Lang::En) => "blocked",
        (TaskState::InReview, Lang::En) => "in review",
        (TaskState::Done, Lang::De) => "erledigt",
        (TaskState::Blocked, Lang::De) => "blockiert",
        (TaskState::InReview, Lang::De) => "in Prüfung",
    }
}

/// The frame of the echo.
pub struct Frame {
    /// Before the targets: "Tell you when ".
    pub head: &'static str,
    /// Before several targets that must all meet it: "all of ".
    pub all_of: &'static str,
    /// Between targets, each: " or ".
    pub or: &'static str,
    /// Between targets, all: " and ".
    pub and: &'static str,
    /// Between conditions: ", or ".
    pub cond_or: &'static str,
    /// Once.
    pub once: &'static str,
    /// Until one target ends.
    pub until_it_ends: &'static str,
    /// Until several end.
    pub until_they_end: &'static str,
    /// Until local midnight today.
    pub today: &'static str,
    /// Until a time today: `{t}`.
    pub until_time: &'static str,
    /// Until a time on another day: `{d}`, `{t}`.
    pub until_day_time: &'static str,
    /// Always.
    pub always: &'static str,
    /// The quiet-hours switch, when on.
    pub quiet: &'static str,
    /// A target with no name: one session, App, project, pool, account, any.
    pub unnamed: [&'static str; 6],
    /// Months, for a day ("20 Nov").
    pub months: [&'static str; 12],
}

/// English.
pub const EN: Frame = Frame {
    head: "Tell you when ",
    all_of: "all of ",
    or: " or ",
    and: " and ",
    cond_or: ", or ",
    once: "Once.",
    until_it_ends: "Until it ends.",
    until_they_end: "Until they end.",
    today: "Until midnight today.",
    until_time: "Until {t}.",
    until_day_time: "Until {d}, {t}.",
    always: "Until you remove it.",
    quiet: "Even during quiet hours.",
    unnamed: [
        "a session",
        "an app",
        "a project",
        "a pool",
        "an account",
        "any session",
    ],
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
};

/// German.
pub const DE: Frame = Frame {
    head: "Ich sage dir Bescheid, wenn ",
    all_of: "alle: ",
    or: " oder ",
    and: " und ",
    cond_or: ", oder ",
    once: "Einmal.",
    until_it_ends: "Bis sie endet.",
    until_they_end: "Bis sie enden.",
    today: "Bis heute Mitternacht.",
    until_time: "Bis {t}.",
    until_day_time: "Bis {d}, {t}.",
    always: "Bis du sie entfernst.",
    quiet: "Auch während der Ruhezeit.",
    unnamed: [
        "eine Sitzung",
        "eine App",
        "ein Projekt",
        "ein Pool",
        "ein Konto",
        "irgendeine Sitzung",
    ],
    months: [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sep.", "Okt.", "Nov.",
        "Dez.",
    ],
};

/// The frame for a language.
#[must_use]
pub const fn frame(lang: Lang) -> &'static Frame {
    match lang {
        Lang::En => &EN,
        Lang::De => &DE,
    }
}

/// Why a watch ended, in plain words (the Watches view's note: never a
/// silent failure).
#[must_use]
pub const fn ended(reason: super::engine::EndReason, lang: Lang) -> &'static str {
    use super::engine::EndReason;
    match (reason, lang) {
        (EndReason::FirstFire, Lang::En) => "Ended: it told you once, as asked.",
        (EndReason::TargetsEnded, Lang::En) => "Ended: what it watched has ended.",
        (EndReason::TimeReached, Lang::En) => "Ended: its time was up.",
        (EndReason::TargetGone, Lang::En) => "Ended: what it watched was removed.",
        (EndReason::NoLongerOffered, Lang::En) => "Ended: AI Juggler can no longer watch for this.",
        (EndReason::FirstFire, Lang::De) => "Beendet: Sie hat dir einmal Bescheid gesagt.",
        (EndReason::TargetsEnded, Lang::De) => "Beendet: Was sie beobachtet hat, ist zu Ende.",
        (EndReason::TimeReached, Lang::De) => "Beendet: Ihre Zeit ist abgelaufen.",
        (EndReason::TargetGone, Lang::De) => "Beendet: Was sie beobachtet hat, wurde entfernt.",
        (EndReason::NoLongerOffered, Lang::De) => {
            "Beendet: AI Juggler kann darauf nicht mehr achten."
        }
    }
}
