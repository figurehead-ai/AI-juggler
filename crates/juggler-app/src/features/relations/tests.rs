//! AJ-E3102: the counts of a set of rows, two parts answering as one,
//! and the property that every change event makes stale exactly what it
//! should (and nothing else).

use std::collections::BTreeSet;

use super::stamps::{Change, SOURCES, Source, Stamps, moved};
use super::*;

fn row(project: Option<&str>, folder: &str, app: &str, live: bool, archived: bool) -> Row {
    let mut of: [Option<String>; KINDS] = Default::default();
    of[EntityKind::All.index()] = Some(String::new());
    of[EntityKind::Project.index()] = project.map(Into::into);
    of[EntityKind::Folder.index()] = Some(folder.into());
    of[EntityKind::App.index()] = Some(app.into());
    of[EntityKind::Tool.index()] = Some("claude-code".into());
    Row {
        of,
        live,
        archived,
        more_pools: Vec::new(),
    }
}

fn entity(kind: EntityKind, id: &str) -> EntityRef {
    EntityRef {
        kind,
        id: id.into(),
    }
}

#[test]
fn an_entity_counts_its_sessions_by_state_and_what_they_are_on() {
    let own = Part::of(
        [
            row(Some("p-1"), "work", "cli:work", true, false),
            row(Some("p-1"), "work", "cli:work", false, false),
            row(Some("p-2"), "work", "cli:work", false, true),
            row(None, "home", "cli:home", false, false),
        ],
        Snapshot::default(),
        StoredStamps::default(),
    );
    let outside = Part::of(
        [
            row(Some("p-1"), "home", "app:claude:/Work", true, false),
            row(Some("p-3"), "work", "cli:work", false, false),
        ],
        Snapshot::default(),
        StoredStamps::default(),
    );
    let parts = [Arc::new(own), Arc::new(outside)];
    let p1 = answer(&parts, &entity(EntityKind::Project, "p-1"));
    assert_eq!(
        (
            p1.sessions.all,
            p1.sessions.open,
            p1.sessions.ended,
            p1.sessions.archived
        ),
        (3, 2, 1, 0)
    );
    assert_eq!((p1.folders, p1.apps, p1.tools, p1.projects), (2, 2, 1, 1));
    let work = answer(&parts, &entity(EntityKind::Folder, "work"));
    assert_eq!(work.sessions.all, 4);
    assert_eq!(work.sessions.archived, 1);
    assert_eq!(work.projects, 3, "p-1 and p-2 here, p-3 outside, as one");
    let all = answer(&parts, &entity(EntityKind::All, "ignored"));
    assert_eq!(all.sessions.all, 6);
    assert_eq!(all.projects, 3, "a session with no project is in none");
    assert_eq!(all.id, "ignored", "the answer names what was asked");
    let none = answer(&parts, &entity(EntityKind::Project, "p-9"));
    assert_eq!(none.sessions.all, 0);
    assert_eq!(none.apps, 0);
}

/// What each change must make stale, written from the task (AJ-E3102),
/// not from [`Change::invalidates`].
fn expected(change: Change) -> BTreeSet<Source> {
    use Source::{Conversations, Folders, Places, Pools, Projects, Sessions};
    match change {
        Change::SessionsUpdated => [Sessions, Projects].into(),
        Change::ConversationsIndex => [Conversations, Projects].into(),
        Change::ConversationsLive => [Conversations].into(),
        Change::PlacesChanged => [Places].into(),
        Change::AccountsChanged => [Folders].into(),
        Change::ProjectsChanged => [Projects].into(),
        Change::PoolsChanged => [Pools].into(),
        Change::SessionsResources
        | Change::SessionsQuota
        | Change::TasksChanged
        | Change::MessagesChanged
        | Change::ProcessesChanged
        | Change::NoticesChanged
        | Change::RuntimeChanged
        | Change::ThisMac => BTreeSet::new(),
    }
}

/// Which parts a set of stale sources must rebuild.
fn stale_parts(sources: &BTreeSet<Source>) -> Vec<PartKind> {
    PartKind::ALL
        .into_iter()
        .filter(|kind| kind.deps().iter().any(|dep| sources.contains(dep)))
        .collect()
}

#[test]
fn every_change_event_invalidates_exactly_what_it_should() {
    // Every kind, alone: only its sources move, and only the parts made
    // from them go stale.
    for change in Change::ALL {
        let stamps = Stamps::default();
        let before = stamps.snapshot();
        stamps.bump(change);
        let after = stamps.snapshot();
        let moved_sources: BTreeSet<Source> = Source::ALL
            .into_iter()
            .filter(|source| before[source.index()] != after[source.index()])
            .collect();
        assert_eq!(moved_sources, expected(change), "{change:?}");
        let stale: Vec<PartKind> = PartKind::ALL
            .into_iter()
            .filter(|kind| moved(kind.deps(), &before, &after))
            .collect();
        assert_eq!(stale, stale_parts(&expected(change)), "{change:?}");
    }
    // Every pair, in either order: what goes stale is what either one
    // invalidates (no change hides another's).
    for first in Change::ALL {
        for second in Change::ALL {
            let stamps = Stamps::default();
            let before = stamps.snapshot();
            stamps.bump(first);
            stamps.bump(second);
            let after = stamps.snapshot();
            let union: BTreeSet<Source> =
                expected(first).union(&expected(second)).copied().collect();
            for kind in PartKind::ALL {
                assert_eq!(
                    moved(kind.deps(), &before, &after),
                    stale_parts(&union).contains(&kind),
                    "{first:?} then {second:?}: {kind:?}"
                );
            }
        }
    }
    // Every source is moved by some change, and every one is read by
    // some part: none is dead.
    for source in Source::ALL {
        assert!(
            Change::ALL
                .iter()
                .any(|c| c.invalidates().contains(&source))
        );
        assert!(PartKind::ALL.iter().any(|k| k.deps().contains(&source)));
    }
    assert_eq!(Source::ALL.len(), SOURCES);
}

#[test]
fn each_event_is_named_as_the_app_emits_it() {
    let names: BTreeSet<&str> = Change::ALL.iter().filter_map(|c| c.event()).collect();
    for event in [
        crate::EVENT_SESSIONS_UPDATED,
        crate::EVENT_SESSIONS_RESOURCES,
        crate::EVENT_SESSIONS_QUOTA,
        crate::EVENT_THIS_MAC,
        crate::features::conversations::EVENT_CHANGED,
        crate::features::places::EVENT_CHANGED,
        crate::features::tasks::EVENT_CHANGED,
        crate::features::messages::EVENT_CHANGED,
        crate::features::processes::EVENT_CHANGED,
        crate::features::notifications::views::EVENT_CHANGED,
        crate::features::runtime::EVENT_CHANGED,
    ] {
        assert!(names.contains(event), "{event} has no change kind");
    }
}

#[test]
fn a_part_is_fresh_until_one_of_its_own_sources_moves() {
    let stamps = Stamps::default();
    let built = stamps.snapshot();
    let stored = StoredStamps {
        sessions: 4,
        folders: 2,
        projects: 7,
        pools: 3,
    };
    let own = Part::of([], built, stored);
    let outside = Part::of([], built, stored);
    assert!(own.fresh(PartKind::Own, &built, &stored));
    // A project added: only the conversations' part reads projects.
    let more = StoredStamps {
        projects: 8,
        ..stored
    };
    assert!(own.fresh(PartKind::Own, &built, &more));
    assert!(!outside.fresh(PartKind::Outside, &built, &more));
    // A write nothing announced (the database's stamp moved alone).
    let written = StoredStamps {
        sessions: 5,
        ..stored
    };
    assert!(!own.fresh(PartKind::Own, &built, &written));
    assert!(!outside.fresh(PartKind::Outside, &built, &written));
    // A session moved to another pool (AJ-E3104): only AI Juggler's
    // sessions are in pools.
    let moved = StoredStamps { pools: 4, ..stored };
    assert!(!own.fresh(PartKind::Own, &built, &moved));
    assert!(outside.fresh(PartKind::Outside, &built, &moved));
    // An event alone.
    stamps.bump(Change::ConversationsLive);
    let now = stamps.snapshot();
    assert!(own.fresh(PartKind::Own, &now, &stored));
    assert!(!outside.fresh(PartKind::Outside, &now, &stored));
}

#[test]
fn a_session_in_two_pools_counts_in_each_d62() {
    let mut first = row(Some("p-1"), "work", "cli:work", true, false);
    first.of[EntityKind::Pool.index()] = Some("pool-reviews".into());
    first.more_pools = vec!["pool-default:p-1".into()];
    let own = Part::of([first], Snapshot::default(), StoredStamps::default());
    let parts = [Arc::new(own), Arc::new(Part::default())];
    for pool in ["pool-reviews", "pool-default:p-1"] {
        assert_eq!(
            answer(&parts, &entity(EntityKind::Pool, pool)).sessions.all,
            1,
            "{pool}"
        );
    }
    assert_eq!(answer(&parts, &entity(EntityKind::Project, "p-1")).pools, 2);
    assert_eq!(
        answer(&parts, &entity(EntityKind::All, "x")).sessions.all,
        1,
        "one session, not two"
    );
}

/// Walk K1: a project's Apps tab counts each App's sessions there, from
/// the cache, over both halves (it counted a 100-session sample).
#[test]
fn a_related_id_carries_how_many_sessions_it_shares() {
    let own = Part::of(
        [
            row(Some("p-1"), "work", "cli:work", false, false),
            row(Some("p-1"), "work", "cli:work", false, false),
            row(Some("p-1"), "home", "cli:home", false, false),
            row(Some("p-2"), "work", "cli:work", false, false),
        ],
        Snapshot::default(),
        StoredStamps::default(),
    );
    let outside = Part::of(
        [row(Some("p-1"), "work", "cli:work", true, false)],
        Snapshot::default(),
        StoredStamps::default(),
    );
    let parts = [Arc::new(own), Arc::new(outside)];
    let apps = related_in(&parts, &entity(EntityKind::Project, "p-1"), EntityKind::App);
    assert_eq!(apps.ids, ["cli:home", "cli:work"]);
    assert_eq!(apps.sessions, [1, 3]);
    assert!(!apps.more);
}
