//! The echo (ADR-0036 §2.4 step 5): a watch in plain words, written by
//! code from the typed watch and [`super::words`], in English or German,
//! never a model's text. What the user confirms is what runs.

use super::conditions::Registry;
use super::model::{Condition, Lifetime, Matching, Param, TargetKind, Watch, WatchTarget};
use super::words::{self, Frame, Lang};

/// The names the echo uses: a target's and an agent's, read live by the
/// caller (a session's name is the agent's words: it is quoted, as data).
pub trait Names {
    /// A target's name, when it has one.
    fn target(&self, target: &WatchTarget) -> Option<String>;
    /// An agent's name, when it has one.
    fn agent(&self, id: &str) -> Option<String>;
}

/// No names: each target by its kind ("a session", "an app").
#[derive(Clone, Copy, Debug, Default)]
pub struct Unnamed;

impl Names for Unnamed {
    fn target(&self, _target: &WatchTarget) -> Option<String> {
        None
    }
    fn agent(&self, _id: &str) -> Option<String> {
        None
    }
}

/// A name quoted as data: “api”. Quote marks inside it are plain ones, and
/// it is one line.
fn quoted(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| match c {
            '“' | '”' | '"' => '\'',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    format!("“{}”", clean.trim())
}

fn kind_index(kind: TargetKind) -> usize {
    match kind {
        TargetKind::Session => 0,
        TargetKind::App => 1,
        TargetKind::Project => 2,
        TargetKind::Pool => 3,
        TargetKind::Account => 4,
        TargetKind::Any => 5,
    }
}

/// A condition in words, its parameter filled in, as spans (an agent's
/// name is a name span).
fn condition_spans(
    registry: &Registry,
    condition: &Condition,
    many: bool,
    lang: Lang,
    names: &dyn Names,
) -> Vec<EchoSpan> {
    let Some(rule) = registry.get(&condition.code) else {
        return vec![EchoSpan::plain(condition.code.replace('_', " "))];
    };
    let cw = rule.words();
    let (plain, with) = match lang {
        Lang::En => (cw.en, cw.en_param),
        Lang::De => (cw.de, cw.de_param),
    };
    let pick = |p: words::Phrase| if many { p.many } else { p.one };
    match (&condition.param, with) {
        (Some(param), Some(phrase)) => {
            let text = pick(phrase);
            match param {
                Param::Minutes(n) => vec![EchoSpan::plain(text.replace("{n}", &n.to_string()))],
                Param::Percent(n) => vec![EchoSpan::plain(text.replace("{n}", &n.to_string()))],
                Param::TaskState(state) => vec![EchoSpan::plain(
                    text.replace("{s}", words::task_state(*state, lang)),
                )],
                Param::Agent(id) => {
                    let name = quoted(&names.agent(id).unwrap_or_else(|| id.clone()));
                    let (before, after) = text.split_once("{a}").unwrap_or((text, ""));
                    vec![
                        EchoSpan::plain(before),
                        EchoSpan::name(name),
                        EchoSpan::plain(after),
                    ]
                }
            }
        }
        _ => vec![EchoSpan::plain(pick(plain))],
    }
}

/// Hours and minutes in local time: "17:30".
fn clock(at_ms: i64, offset_hours: i64) -> String {
    let local = at_ms + offset_hours * 3_600_000;
    let minutes = local.div_euclid(60_000).rem_euclid(24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The local day: "20 Nov".
fn day(at_ms: i64, offset_hours: i64, frame: &Frame) -> String {
    let local_days = (at_ms + offset_hours * 3_600_000).div_euclid(86_400_000);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = local_days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let month = usize::try_from(m - 1).unwrap_or(0).min(11);
    format!("{d} {}", frame.months[month])
}

/// The time the user's clock reads, for the "until" words.
#[derive(Clone, Copy, Debug)]
pub struct LocalClock {
    /// Now, epoch ms.
    pub now_ms: i64,
    /// Local time minus UTC, in hours (UTC when unknown).
    pub offset_hours: i64,
}

impl LocalClock {
    /// The next local midnight: the end of "today".
    #[must_use]
    pub const fn next_midnight(self) -> i64 {
        let offset_ms = self.offset_hours * 3_600_000;
        let local = self.now_ms + offset_ms;
        local - local.rem_euclid(86_400_000) - offset_ms + 86_400_000
    }
}

fn lifetime_words(watch: &Watch, frame: &Frame, clock_now: LocalClock) -> String {
    match watch.lifetime {
        Lifetime::Once => frame.once.to_owned(),
        Lifetime::UntilEnds if watch.targets.len() > 1 => frame.until_they_end.to_owned(),
        Lifetime::UntilEnds => frame.until_it_ends.to_owned(),
        Lifetime::Always => frame.always.to_owned(),
        Lifetime::Until { at_ms } if at_ms == clock_now.next_midnight() => frame.today.to_owned(),
        Lifetime::Until { at_ms } => {
            let t = clock(at_ms, clock_now.offset_hours);
            let same_day = day(at_ms, clock_now.offset_hours, frame)
                == day(clock_now.now_ms, clock_now.offset_hours, frame);
            if same_day {
                frame.until_time.replace("{t}", &t)
            } else {
                frame
                    .until_day_time
                    .replace("{d}", &day(at_ms, clock_now.offset_hours, frame))
                    .replace("{t}", &t)
            }
        }
    }
}

/// One piece of the echo's first sentence: a name (a target's or an
/// agent's, quoted as data) or the words around it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EchoSpan {
    /// The text, exactly as it stands in the sentence.
    pub text: String,
    /// It is a name (the view sets it in bold).
    pub name: bool,
}

impl EchoSpan {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            name: false,
        }
    }
    fn name(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            name: true,
        }
    }
}

/// The echo in its parts, so a view never splits or parses the words:
/// the first sentence (what it tells and when) as spans, the lifetime's
/// sentence, and the quiet-hours sentence when that switch is on.
/// [`EchoParts::joined`] is the echo, word for word.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EchoParts {
    /// "Tell you when “api” finishes." as spans.
    pub head: Vec<EchoSpan>,
    /// "Once.", "Until it ends.", "Until midnight today." …
    pub lifetime: String,
    /// "Even during quiet hours.", when that switch is on.
    pub quiet: Option<String>,
}

impl EchoParts {
    /// The first sentence as one string.
    #[must_use]
    pub fn head_text(&self) -> String {
        self.head.iter().map(|span| span.text.as_str()).collect()
    }

    /// The whole echo: the sentences joined by a space.
    #[must_use]
    pub fn joined(&self) -> String {
        let mut out = format!("{} {}", self.head_text(), self.lifetime);
        if let Some(quiet) = &self.quiet {
            out.push(' ');
            out.push_str(quiet);
        }
        out
    }
}

/// Spans with neighbouring plain text merged and empty ones dropped.
fn merged(spans: Vec<EchoSpan>) -> Vec<EchoSpan> {
    let mut out: Vec<EchoSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if !last.name && !span.name => last.text.push_str(&span.text),
            _ => out.push(span),
        }
    }
    out
}

/// The watch in parts (see [`EchoParts`]).
#[must_use]
pub fn echo_parts(
    registry: &Registry,
    watch: &Watch,
    names: &dyn Names,
    lang: Lang,
    clock_now: LocalClock,
) -> EchoParts {
    let frame = words::frame(lang);
    let all = watch.matching == Matching::All && watch.targets.len() > 1;
    let join = if all { frame.and } else { frame.or };
    let mut head = vec![EchoSpan::plain(frame.head)];
    if all {
        head.push(EchoSpan::plain(frame.all_of));
    }
    let count = watch.targets.len();
    for (i, target) in watch.targets.iter().enumerate() {
        if i > 0 {
            head.push(EchoSpan::plain(if i + 1 == count { join } else { ", " }));
        }
        head.push(match (target, names.target(target)) {
            (WatchTarget::Any, _) | (_, None) => {
                EchoSpan::plain(frame.unnamed[kind_index(target.kind())])
            }
            (_, Some(name)) => EchoSpan::name(quoted(&name)),
        });
    }
    head.push(EchoSpan::plain(" "));
    for (i, condition) in watch.conditions.iter().enumerate() {
        if i > 0 {
            head.push(EchoSpan::plain(frame.cond_or));
        }
        head.extend(condition_spans(registry, condition, all, lang, names));
    }
    head.push(EchoSpan::plain("."));
    EchoParts {
        head: merged(head),
        lifetime: lifetime_words(watch, frame, clock_now),
        quiet: watch.even_in_quiet_hours.then(|| frame.quiet.to_owned()),
    }
}

/// The watch in plain words: "Tell you when “api” or “web” finishes, or
/// looks stuck for 10 minutes. Until midnight today. Even during quiet
/// hours."
#[must_use]
pub fn echo(
    registry: &Registry,
    watch: &Watch,
    names: &dyn Names,
    lang: Lang,
    clock_now: LocalClock,
) -> String {
    echo_parts(registry, watch, names, lang, clock_now).joined()
}
