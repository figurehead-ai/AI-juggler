//! ADR-0036 §1.7 (AJ-E3712, AJ-E3817): consolidation, in front of the
//! sink. It changes how many notifications reach the system, never
//! whether a need is told. Pure: the caller hands it what is due and the
//! time; it says what to send now and when to look again.
//!
//! - **A 60-second batch.** The first L2 or L3 item of a burst goes out at
//!   once; further items due within the next 60 seconds are held and sent
//!   at its end as one notification. Calm items (finished, reset,
//!   processes left, a watch on a calm event, the digest) always wait for
//!   the batch.
//! - **Released at once, together.** What quiet hours held (the engine's
//!   items and the watches' alike) and what was replayed on start
//!   ("while AI Juggler was closed") go out as one notification.
//! - **Quiet hours for the watches' alerts.** Held here (the engine holds
//!   its own), unless that watch's own "Even during quiet hours" is on.
//! - **Told once per key.** A key handed over once is never sent again.
//! - **One notification per thing that happened.** A watch's alert on a
//!   need that was just told (or is held in this burst) is folded into it
//!   ("web asks you · you asked to be told"), never a second ping.
//!
//! The words are code's, from names and the kinds' phrases: a model's
//! words never reach a notification.

use std::collections::{HashSet, VecDeque};

use super::engine::{Kind, Target};

/// How long a burst gathers before its held items go out as one.
pub const BATCH_MS: i64 = 60_000;
/// A watch's alert echoing a need told this recently is folded into it.
pub const RECENT_MS: i64 = 5 * 60_000;
/// Keys remembered as told (the oldest go first).
const TOLD_KEPT: usize = 2_000;
/// Names a consolidated body lists before "and N more".
const NAMES_SHOWN: usize = 4;
/// Phrases a consolidated body lists (two lines at most).
const PHRASES_SHOWN: usize = 2;

/// One thing that may become (part of) a notification.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "each mark is its own fact")]
pub struct Item {
    /// Told once per key.
    pub key: String,
    /// What it is about.
    pub kind: Kind,
    /// What a click opens, when it goes out alone.
    pub target: Target,
    /// Its attention level (D-117): 3 broke, 2 needs you, 1 a note.
    pub level: u8,
    /// Whose it is ("web"), or the whole line when it has no phrase.
    pub name: String,
    /// What happened, for one ("asks you"); empty: `name` says it all.
    pub phrase: String,
    /// What happened, for several ("ask you").
    pub phrase_many: String,
    /// Its own title and line, when it goes out alone.
    pub title: String,
    /// See `title`.
    pub body: String,
    /// Held through quiet hours by the engine, now released.
    pub held: bool,
    /// Heard from the journals on start ("while AI Juggler was closed").
    pub replayed: bool,
    /// A watch's alert.
    pub watch: bool,
    /// A watch's alert that goes through quiet hours (its own switch).
    pub breakthrough: bool,
    /// The need or state a watch's alert echoes, if any: told once with it.
    pub echoes: Option<Kind>,
    /// A need the user also asked a watch to tell (folded in).
    pub watched: bool,
}

impl Item {
    /// A plain item: `kind` about `target`, its words and level.
    #[must_use]
    pub fn new(key: impl Into<String>, kind: Kind, target: Target, level: u8) -> Self {
        Self {
            key: key.into(),
            kind,
            target,
            level,
            name: String::new(),
            phrase: String::new(),
            phrase_many: String::new(),
            title: String::new(),
            body: String::new(),
            held: false,
            replayed: false,
            watch: false,
            breakthrough: false,
            echoes: None,
            watched: false,
        }
    }

    /// Its words as one line ("web asks you").
    fn line(&self) -> String {
        if self.phrase.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.phrase)
        }
    }

    /// The need or state this item is about, for folding: its own kind,
    /// or the one a watch's alert echoes (`None`: a watch's alert on
    /// something no other notification tells, never folded).
    fn need(&self) -> Option<Kind> {
        if self.watch {
            self.echoes
        } else {
            Some(self.kind)
        }
    }
}

/// One notification to send: one item, or several consolidated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// What it holds, highest level first, then in the order they came.
    pub items: Vec<Item>,
}

/// Where a consolidated notification's click goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opens {
    /// Its one item's target.
    Target(Target),
    /// The notification centre (a need is among them).
    Centre,
    /// Home's feed, at the newest digest.
    Feed,
}

/// A group's words and click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    /// The title.
    pub title: String,
    /// The line under it.
    pub body: String,
    /// What a click opens.
    pub opens: Opens,
}

impl Group {
    fn of(mut items: Vec<Item>) -> Option<Self> {
        if items.is_empty() {
            return None;
        }
        // Highest level first; otherwise in the order they came.
        items.sort_by_key(|item| std::cmp::Reverse(item.level));
        Some(Self { items })
    }

    /// Its words, code-composed from names and phrases: one item keeps
    /// today's notification; two or more become one, titled from the
    /// highest level ("web asks you · 3 more"), its body the others'
    /// names in at most two lines ("api, docs and infra finished").
    #[must_use]
    pub fn words(&self) -> Words {
        let replayed = self.items.iter().all(|item| item.replayed);
        let first = &self.items[0];
        if let [one] = self.items.as_slice() {
            let mut body = one.body.clone();
            if one.watched {
                body = join_dot(&body, "you asked to be told");
            }
            if replayed {
                body = join_dot(&body, "while AI Juggler was closed");
            }
            return Words {
                title: one.title.clone(),
                body,
                opens: Opens::Target(one.target.clone()),
            };
        }
        let more = self.items.len() - 1;
        let mut title = format!("{} · {more} more", first.line());
        if replayed {
            title = format!("While AI Juggler was closed: {title}");
        }
        // The others, grouped by what happened, in the order first seen.
        let mut phrases: Vec<(String, String, Vec<String>)> = Vec::new();
        for item in &self.items[1..] {
            let found = phrases.iter_mut().find(|(one, _, _)| *one == item.phrase);
            match found {
                Some((_, _, names)) => names.push(item.name.clone()),
                None => phrases.push((
                    item.phrase.clone(),
                    item.phrase_many.clone(),
                    vec![item.name.clone()],
                )),
            }
        }
        let mut lines: Vec<String> = phrases
            .iter()
            .take(PHRASES_SHOWN)
            .map(|(one, many, names)| {
                let who = names_words(names);
                match (names.len(), one.is_empty()) {
                    (_, true) => who,
                    (1, false) => format!("{who} {one}"),
                    _ => format!("{who} {many}"),
                }
            })
            .collect();
        let left: usize = phrases
            .iter()
            .skip(PHRASES_SHOWN)
            .map(|(_, _, names)| names.len())
            .sum();
        if left > 0 {
            lines.push(format!("and {left} more"));
        }
        let need = self.items.iter().any(|item| item.level >= 2);
        Words {
            title,
            body: lines.join("\n"),
            opens: if need { Opens::Centre } else { Opens::Feed },
        }
    }
}

fn join_dot(head: &str, tail: &str) -> String {
    if head.is_empty() {
        let mut tail = tail.to_owned();
        if let Some(first) = tail.get(..1) {
            tail.replace_range(..1, &first.to_uppercase());
        }
        tail
    } else {
        format!("{head} · {tail}")
    }
}

/// "api", "api and web", "api, docs and infra", "api, docs, web, ui and 3 more".
fn names_words(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        _ if names.len() > NAMES_SHOWN => format!(
            "{} and {} more",
            names[..NAMES_SHOWN].join(", "),
            names.len() - NAMES_SHOWN
        ),
        [head @ .., last] => format!("{} and {last}", head.join(", ")),
    }
}

#[derive(Clone, Debug)]
struct Burst {
    ends_ms: i64,
    loud_sent: bool,
    held: Vec<Item>,
}

/// The batch's state: the open burst, what quiet hours hold, and what was
/// told.
#[derive(Debug)]
pub struct Batcher {
    /// A burst's length: [`BATCH_MS`] (0: each item as it falls due, for
    /// tests of the rules before the batch).
    window_ms: i64,
    burst: Option<Burst>,
    /// The watches' alerts quiet hours hold (the engine holds its own).
    quiet_held: Vec<Item>,
    told: HashSet<String>,
    told_order: VecDeque<String>,
    /// What was sent lately, by target and need: a watch's echo folds in.
    recent: Vec<(Target, Kind, i64)>,
}

impl Default for Batcher {
    fn default() -> Self {
        Self::with_window(BATCH_MS)
    }
}

impl Batcher {
    /// A batch whose bursts last `window_ms` (0: no burst; each item goes
    /// out as it falls due, quiet hours and told-once still kept).
    #[must_use]
    pub fn with_window(window_ms: i64) -> Self {
        Self {
            window_ms,
            burst: None,
            quiet_held: Vec::new(),
            told: HashSet::new(),
            told_order: VecDeque::new(),
            recent: Vec::new(),
        }
    }

    /// What to send now, given what fell due (`quiet`: inside the quiet
    /// hours now). A burst whose minute is up goes out first.
    pub fn offer(&mut self, items: Vec<Item>, quiet: bool, now_ms: i64) -> Vec<Group> {
        self.recent.retain(|(_, _, at)| now_ms - at < RECENT_MS);
        let mut out: Vec<Group> = self.flush(now_ms).into_iter().collect();
        let mut released: Vec<Item> = Vec::new();
        if !quiet {
            released.append(&mut self.quiet_held);
        }
        for item in items {
            if self.told.contains(&item.key) || self.fold(&item, &mut released) {
                continue;
            }
            self.remember(&item.key);
            if item.held || item.replayed {
                released.push(item);
            } else if quiet && !item.breakthrough {
                // Only the watches' alerts reach here in quiet hours (the
                // engine holds its own): held until they end.
                self.quiet_held.push(item);
            } else {
                out.extend(self.live(item, now_ms));
            }
        }
        if let Some(group) = Group::of(released) {
            self.sent(&group, now_ms);
            out.push(group);
        }
        out
    }

    /// The burst's held items as one notification, once its minute is up.
    pub fn flush(&mut self, now_ms: i64) -> Option<Group> {
        if self
            .burst
            .as_ref()
            .is_none_or(|burst| now_ms < burst.ends_ms)
        {
            return None;
        }
        let burst = self.burst.take()?;
        let group = Group::of(burst.held)?;
        self.sent(&group, now_ms);
        Some(group)
    }

    /// When the open burst's minute is up, if one is open.
    #[must_use]
    pub fn next_flush(&self) -> Option<i64> {
        self.burst.as_ref().map(|burst| burst.ends_ms)
    }

    /// Something waits here: a burst to flush by now, or alerts quiet
    /// hours hold.
    #[must_use]
    pub fn waiting(&self, now_ms: i64) -> bool {
        !self.quiet_held.is_empty() || self.burst.as_ref().is_some_and(|b| now_ms >= b.ends_ms)
    }

    /// Drops what waits to be sent (AI Juggler came to the front: the user
    /// sees it there). What was told stays told.
    pub fn drop_waiting(&mut self) {
        if let Some(burst) = self.burst.as_mut() {
            burst.held.clear();
        }
        self.quiet_held.clear();
    }

    fn live(&mut self, item: Item, now_ms: i64) -> Option<Group> {
        if self.window_ms == 0 {
            let group = Group::of(vec![item])?;
            self.sent(&group, now_ms);
            return Some(group);
        }
        let burst = self.burst.get_or_insert(Burst {
            ends_ms: now_ms + self.window_ms,
            loud_sent: false,
            held: Vec::new(),
        });
        if item.level >= 2 && !burst.loud_sent {
            burst.loud_sent = true;
            let group = Group::of(vec![item])?;
            self.sent(&group, now_ms);
            return Some(group);
        }
        burst.held.push(item);
        None
    }

    /// Folds `item` into what was told or waits when it is about the same
    /// thing: a watch's alert on a need just told, or held in this burst,
    /// or a need arriving after a watch's alert on it (the need takes its
    /// place, the watch's mark kept). `true` when it was folded.
    fn fold(&mut self, item: &Item, released: &mut [Item]) -> bool {
        let Some(need) = item.need() else {
            return false;
        };
        if self
            .recent
            .iter()
            .any(|(target, kind, _)| *target == item.target && *kind == need)
        {
            self.remember(&item.key);
            return true;
        }
        let folded = {
            let mut waiting = self
                .burst
                .iter_mut()
                .flat_map(|burst| burst.held.iter_mut())
                .chain(self.quiet_held.iter_mut())
                .chain(released.iter_mut());
            waiting
                .find(|other| other.target == item.target && other.need() == Some(need))
                .map(|other| {
                    let breakthrough = other.breakthrough || item.breakthrough;
                    if item.watch {
                        other.watched = true;
                    } else if other.watch {
                        *other = Item {
                            watched: true,
                            ..item.clone()
                        };
                    }
                    other.breakthrough = breakthrough;
                })
                .is_some()
        };
        if folded {
            self.remember(&item.key);
        }
        folded
    }

    fn sent(&mut self, group: &Group, now_ms: i64) {
        for item in &group.items {
            if let Some(need) = item.need() {
                self.recent.push((item.target.clone(), need, now_ms));
            }
        }
    }

    fn remember(&mut self, key: &str) {
        if self.told.insert(key.to_owned()) {
            self.told_order.push_back(key.to_owned());
            if self.told_order.len() > TOLD_KEPT
                && let Some(oldest) = self.told_order.pop_front()
            {
                self.told.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
