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

fn target_words(target: &WatchTarget, names: &dyn Names, frame: &Frame) -> String {
    match (target, names.target(target)) {
        (WatchTarget::Any, _) | (_, None) => frame.unnamed[kind_index(target.kind())].to_owned(),
        (_, Some(name)) => quoted(&name),
    }
}

/// A condition in words, its parameter filled in.
fn condition_words(
    registry: &Registry,
    condition: &Condition,
    many: bool,
    lang: Lang,
    names: &dyn Names,
) -> String {
    let Some(rule) = registry.get(&condition.code) else {
        return condition.code.replace('_', " ");
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
                Param::Minutes(n) => text.replace("{n}", &n.to_string()),
                Param::Percent(n) => text.replace("{n}", &n.to_string()),
                Param::TaskState(state) => text.replace("{s}", words::task_state(*state, lang)),
                Param::Agent(id) => text.replace(
                    "{a}",
                    &quoted(&names.agent(id).unwrap_or_else(|| id.clone())),
                ),
            }
        }
        _ => pick(plain).to_owned(),
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
    let frame = words::frame(lang);
    let all = watch.matching == Matching::All && watch.targets.len() > 1;
    let target_list: Vec<String> = watch
        .targets
        .iter()
        .map(|t| target_words(t, names, frame))
        .collect();
    let join = if all { frame.and } else { frame.or };
    let targets = match target_list.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{}{join}{last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    };
    let conditions: Vec<String> = watch
        .conditions
        .iter()
        .map(|c| condition_words(registry, c, all, lang, names))
        .collect();
    let mut out = format!(
        "{}{}{targets} {}. {}",
        frame.head,
        if all { frame.all_of } else { "" },
        conditions.join(frame.cond_or),
        lifetime_words(watch, frame, clock_now)
    );
    if watch.even_in_quiet_hours {
        out.push(' ');
        out.push_str(frame.quiet);
    }
    out
}
