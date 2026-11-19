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

/// A watch's alert level: a need's own (D-117), else a note.
fn level_of(told_as: Option<&str>) -> u8 {
    match told_as {
        Some(need @ ("asks" | "stuck" | "limit" | "failed")) => {
            centre::attention_level(need, LevelFacts::default())
        }
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
    if let Some(session) = fire.session.as_deref() {
        let about = super::describe(state, session);
        return About {
            name: about.name,
            target: Target::Session(session.to_owned()),
            notice: NoticeTarget {
                session_id: Some(session.to_owned()),
                ..NoticeTarget::default()
            },
            project_id: about.project_id,
        };
    }
    let named = |name: Option<String>, fallback: &str| name.unwrap_or_else(|| fallback.to_owned());
    let storage = state.storage.lock().ok();
    match &fire.target {
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
