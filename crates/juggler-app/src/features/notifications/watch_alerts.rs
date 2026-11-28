//! AJ-E3817 (ADR-0036 §2.5): a watch's fire becomes a notice of kind
//! `watch` in the one model ("Watches you set"), told once per key, and
//! its alert goes through the same path as every need: the 60-second
//! batch, consolidation, quiet hours held (unless that watch's own "Even
//! during quiet hours" is on), never while AI Juggler is in front, its
//! kind and the user's mutes respected. No second channel; nothing leaves
//! the Mac.
//!
//! The words are the app's own, from the typed watch and the facts: the
//! target's name and what happened ("api finished · your watch"); a click
//! opens the session. A watch on something a need already tells (it asks
//! you, it looks stuck, it finished) is one notification with that need
//! ("web asks you · you asked to be told") and takes its level; on a calm
//! event it is a note (L1), never counted.
//!
//! A reminder (AJ-E2027, ADR-0039 §8) is told the same way ([`reminded`]).

use std::sync::Arc;

use juggler_core::storage::NewNotice;

use super::batch::Item;
use super::centre::{self, LevelFacts, NoticeFacts, NoticeTarget};
use super::engine::{Kind, Target};
use crate::features::watches::conditions::{ConditionRule, Registry};
use crate::features::watches::engine::Fire;
use crate::features::watches::model::WatchTarget;
use crate::state::AppState;

/// The notice kind of a watch's alert.
pub const KIND: &str = "watch";

/// The engine kind a need's notice kind is told as, when there is one: a
/// watch's alert on it is folded into it.
fn echo_of(told_as: &str) -> Option<Kind> {
    Some(match told_as {
        "asks" => Kind::Waiting,
        "stuck" => Kind::Stuck,
        "limit" => Kind::Limit,
        "finished" => Kind::Finished,
        "leftovers" => Kind::LeftBehind,
        "restarted" => Kind::Crashed,
        "reset" => Kind::Reset,
        _ => return None,
    })
}

/// A condition told as a need (it asks, it failed, it looks stuck, a
/// limit): its alert is that need's, shown in Needs you too.
#[must_use]
pub fn is_need(told_as: Option<&str>) -> bool {
    matches!(told_as, Some("asks" | "stuck" | "limit" | "failed"))
}

/// A watch's alert level: a need's own (D-117), else a note.
fn level_of(told_as: Option<&str>) -> u8 {
    match told_as {
        Some(need) if is_need(Some(need)) => centre::attention_level(need, LevelFacts::default()),
        _ => 1,
    }
}

/// What a fire is about, for its words and its click.
struct About {
    name: String,
    target: Target,
    notice: NoticeTarget,
    project_id: Option<String>,
}

fn about(state: &Arc<AppState>, fire: &Fire) -> About {
    about_target(state, fire.session.as_deref(), fire.target.as_ref())
}

/// What a session, else a target, is called and where a click on its
/// notice goes (a watch's fire; a reminder about it, AJ-E2027).
fn about_target(
    state: &Arc<AppState>,
    session: Option<&str>,
    target: Option<&WatchTarget>,
) -> About {
    if let Some(session) = session {
        let about = super::describe(state, session);
        // 0.5.2 G45: a session AI Juggler didn't start opens its page as
        // the centre's other notices about one do (its account and id).
        let notice = crate::features::sessions::seen::parse_source_key(session).map_or_else(
            || NoticeTarget {
                session_id: Some(session.to_owned()),
                ..NoticeTarget::default()
            },
            |(account, id)| NoticeTarget {
                conversation_id: Some(id.to_owned()),
                account_id: Some(account.to_owned()),
                project_id: about.project_id.clone(),
                ..NoticeTarget::default()
            },
        );
        return About {
            name: about.name,
            target: Target::Session(session.to_owned()),
            notice,
            project_id: about.project_id,
        };
    }
    let named = |name: Option<String>, fallback: &str| name.unwrap_or_else(|| fallback.to_owned());
    let storage = state.storage.lock().ok();
    match target {
        Some(WatchTarget::Account(id)) => About {
            name: named(
                storage.as_ref().and_then(|s| {
                    s.accounts()
                        .get(&juggler_core::model::AccountId::new(id.as_str()))
                        .ok()
                        .map(|a| a.label)
                }),
                "An account",
            ),
            target: Target::Account(id.as_str().to_owned()),
            notice: NoticeTarget {
                account_id: Some(id.as_str().to_owned()),
                page: Some("usage".to_owned()),
                ..NoticeTarget::default()
            },
            project_id: None,
        },
        Some(WatchTarget::Pool(id)) => About {
            name: named(
                storage
                    .as_ref()
                    .and_then(|s| s.pools().get(id).ok().map(|p| p.name)),
                "A pool",
            ),
            target: Target::Centre,
            notice: NoticeTarget {
                pool_id: Some(id.clone()),
                ..NoticeTarget::default()
            },
            project_id: None,
        },
        Some(WatchTarget::Project(id)) => About {
            name: named(
                storage.as_ref().and_then(|s| {
                    s.projects()
                        .get(&juggler_core::model::ProjectId::new(id.as_str()))
                        .ok()
                        .map(|p| p.name)
                }),
                "A project",
            ),
            target: Target::Centre,
            notice: NoticeTarget {
                project_id: Some(id.as_str().to_owned()),
                ..NoticeTarget::default()
            },
            project_id: Some(id.as_str().to_owned()),
        },
        _ => About {
            name: "Your watch".to_owned(),
            target: Target::Centre,
            notice: NoticeTarget::default(),
            project_id: None,
        },
    }
}

/// The key a fire is told once by: the watch, what met it, the condition
/// and when it happened.
fn key_of(fire: &Fire) -> String {
    let member = fire
        .session
        .clone()
        .or_else(|| fire.target.as_ref().and_then(|t| t.id().map(str::to_owned)))
        .unwrap_or_else(|| "all".to_owned());
    format!(
        "{KIND}:{}:{member}:{}:{}",
        fire.watch.0, fire.condition, fire.at_ms
    )
}

/// The watches fired: each told once in the model, its alert handed to
/// the notification path. A replayed fire says "while AI Juggler was
/// closed".
pub fn fired(state: &Arc<AppState>, fires: &[Fire]) {
    if fires.is_empty() {
        return;
    }
    let registry = Registry::shipped();
    let now = crate::commands::now_millis();
    let mut items = Vec::new();
    for fire in fires {
        let Some(rule) = registry.get(&fire.condition) else {
            continue;
        };
        if let Some(item) = tell(state, fire, rule, now) {
            items.push(item);
        }
    }
    if !items.is_empty() {
        super::changed(state);
    }
    super::hand_over(state, items);
}

/// One fire: its notice (told once per key) and, when new, its alert.
fn tell(
    state: &Arc<AppState>,
    fire: &Fire,
    rule: &'static dyn ConditionRule,
    now: i64,
) -> Option<Item> {
    let about = about(state, fire);
    let told = rule.words().told_en;
    let key = key_of(fire);
    let mut detail = vec!["Your watch".to_owned()];
    if fire.replayed {
        detail.push("while AI Juggler was closed".to_owned());
    }
    let notice = NewNotice {
        key: key.clone(),
        kind: KIND.to_owned(),
        title: format!("“{}” {told}", about.name),
        detail: detail.join(" · "),
        project_id: about.project_id.clone(),
        data: centre::data_of(
            &about.notice,
            &NoticeFacts {
                told: Some(told.to_owned()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("open".to_owned()),
        ..NewNotice::default()
    };
    // Told once per key: a fire already in the model isn't told again.
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten())?;
    let told_as = rule.told_as();
    let body = format!("{} · your watch", capitalised(told));
    Some(Item {
        name: about.name.clone(),
        phrase: told.to_owned(),
        phrase_many: told.to_owned(),
        title: about.name,
        body,
        watch: true,
        breakthrough: fire.even_in_quiet_hours,
        replayed: fire.replayed,
        echoes: told_as.and_then(echo_of),
        ..Item::new(key, Kind::Watch, about.target, level_of(told_as))
    })
}

/// AJ-E2027 (ADR-0039 §8): a reminder came due. Told once per schedule
/// and due time in the one model (beside the watches the user set), and
/// its alert handed to the notification path as something the user asked
/// for at that time: never held by quiet hours (they picked the time), a
/// click opens what it is about. A late one (asleep or closed) says so.
/// Returns whether it was told now (not already).
pub fn reminded(
    state: &Arc<AppState>,
    schedule_id: &str,
    due_ms: i64,
    title: &str,
    about: Option<&WatchTarget>,
    late: bool,
) -> bool {
    let session = match about {
        Some(WatchTarget::Session(id)) => Some(id.as_str()),
        _ => None,
    };
    let target = about.filter(|t| !matches!(t, WatchTarget::Any));
    let named = about_target(state, session, target);
    let key = format!("reminder:{schedule_id}:{due_ms}");
    let mut detail = vec!["Your reminder".to_owned()];
    if target.is_some() {
        detail.push(format!("about {}", named.name));
    }
    if late {
        detail.push("while AI Juggler was closed or asleep".to_owned());
    }
    let notice = NewNotice {
        key: key.clone(),
        kind: KIND.to_owned(),
        title: title.to_owned(),
        detail: detail.join(" · "),
        project_id: named.project_id.clone(),
        data: centre::data_of(
            &named.notice,
            &NoticeFacts {
                told: Some("reminder".to_owned()),
                ..NoticeFacts::default()
            },
        ),
        action: target.map(|_| "open".to_owned()),
        ..NewNotice::default()
    };
    let now = crate::commands::now_millis();
    let told = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten());
    if told.is_none() {
        return false;
    }
    super::changed(state);
    let body = detail.join(" · ");
    super::hand_over(
        state,
        vec![Item {
            name: title.to_owned(),
            phrase: "reminder".to_owned(),
            phrase_many: "reminders".to_owned(),
            title: title.to_owned(),
            body,
            watch: true,
            breakthrough: true,
            replayed: late,
            ..Item::new(key, Kind::Watch, named.target, 2)
        }],
    );
    true
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::watches::model::WatchId;

    fn fire(condition: &str) -> Fire {
        Fire {
            watch: WatchId("w-1".into()),
            condition: condition.into(),
            target: Some(WatchTarget::Session("s1".into())),
            session: Some("s1".into()),
            at_ms: 1_000,
            cause: "state_entered",
            even_in_quiet_hours: false,
            replayed: false,
        }
    }

    #[test]
    fn a_watch_on_a_need_takes_its_level_and_echoes_it_a_calm_one_is_a_note() {
        let registry = Registry::shipped();
        let told = |code: &str| registry.get(code).unwrap().told_as();
        assert_eq!(level_of(told("asks_you")), 2);
        assert_eq!(level_of(told("fails")), 3);
        assert_eq!(level_of(told("finishes")), 1);
        assert_eq!(level_of(told("ends")), 1);
        assert_eq!(told("asks_you").and_then(echo_of), Some(Kind::Waiting));
        assert_eq!(
            told("fails").and_then(echo_of),
            None,
            "only the watch tells a failure"
        );
        assert_eq!(told("idle_for"), None);
    }

    #[test]
    fn every_condition_has_its_alert_words() {
        for rule in Registry::shipped().iter() {
            let words = rule.words();
            assert!(
                !words.told_en.is_empty() && !words.told_de.is_empty(),
                "{}",
                rule.code()
            );
            assert!(
                !words.told_en.contains('{'),
                "{}: no parameter in an alert",
                rule.code()
            );
        }
    }

    #[test]
    fn a_fire_is_told_once_by_its_key() {
        let a = fire("finishes");
        let mut later = a.clone();
        later.at_ms = 2_000;
        assert_ne!(key_of(&a), key_of(&later), "a new entry is its own news");
        assert_eq!(key_of(&a), key_of(&a.clone()));
        assert_eq!(key_of(&a), "watch:w-1:s1:finishes:1000");
    }
}
