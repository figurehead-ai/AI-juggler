//! AJ-E3004: the resolver's cases the L1 tables don't vary.
//!
//! - A project's default pool skips the sessions another pool names on
//!   their own, but not the ones a broad member elsewhere matches (a
//!   reviewer pool over everyone doesn't empty the project pools).
//! - An unreadable sign-in is "Account unknown": Account members can't
//!   match, and the session stays in through any other member.
//! - A member's own level comes through; a pool deleted (not listed) holds
//!   nobody.
//! - The budget: 500 sessions against 50 pools resolve in ≤ 10 ms.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::time::Instant;

use juggler_core::model::{ProjectId, SessionId};
use juggler_core::pools::{Memberships, SessionFacts, SkipReason};
use juggler_core::storage::{
    MemberKind, MemberLevel, MemberRef, Pool, PoolKind, PoolMember, PoolRole, Scope,
};

fn pool(id: &str, kind: PoolKind, scope: Scope, created_at: i64) -> Pool {
    Pool {
        id: id.to_owned(),
        name: id.to_owned(),
        kind,
        project: scope.only_project().cloned(),
        created_at,
        spans_macs: false,
        scope,
        sealed: false,
        skip_held: kind == PoolKind::Default,
    }
}

fn row(pool: &str, member: MemberRef, role: PoolRole, joined_at: i64) -> PoolMember {
    PoolMember {
        pool_id: pool.to_owned(),
        member,
        role,
        joined_at,
        level: None,
    }
}

fn facts(id: &str, project: &str, app: &str, account: Option<&str>) -> SessionFacts {
    SessionFacts {
        session: SessionId::new(id),
        work: None,
        project: Some(ProjectId::new(project)),
        app: Some(app.to_owned()),
        account: account.map(str::to_owned),
        tool: Some("claude-code".to_owned()),
        mac: Some("this-mac".to_owned()),
        conversation: None,
    }
}

fn shop() -> Scope {
    Scope::one(Some(&ProjectId::new("shop")))
}

#[test]
fn a_project_pool_skips_sessions_named_elsewhere_but_not_ones_a_broad_member_takes() {
    let pools = [
        pool("pool-default:shop", PoolKind::Default, shop(), 0),
        pool("reviewers", PoolKind::User, Scope::Every, 1),
        pool("team", PoolKind::User, shop(), 2),
    ];
    let members = [
        row(
            "pool-default:shop",
            MemberRef::Everyone,
            PoolRole::Member,
            0,
        ),
        row("reviewers", MemberRef::Everyone, PoolRole::Member, 5),
        row(
            "reviewers",
            MemberRef::Session(SessionId::new("codex-1")),
            PoolRole::Reviewer,
            6,
        ),
        row(
            "team",
            MemberRef::Session(SessionId::new("named")),
            PoolRole::Lead,
            7,
        ),
    ];
    let index = Memberships::new(&pools, &members, HashMap::new());

    // In its project pool and the reviewer pool: two pools (P4).
    let plain = index.resolve(&facts("plain", "shop", "cli:work", Some("acct-w")));
    let ids: Vec<&str> = plain
        .placements
        .iter()
        .map(|place| place.pool_id.as_str())
        .collect();
    assert_eq!(ids, ["reviewers", "pool-default:shop"]);

    // Named in "team": its project pool skips it, saying who holds it.
    let named = index.resolve(&facts("named", "shop", "cli:work", Some("acct-w")));
    let ids: Vec<&str> = named
        .placements
        .iter()
        .map(|place| place.pool_id.as_str())
        .collect();
    assert_eq!(ids, ["team", "reviewers"]);
    assert_eq!(named.primary().unwrap().role, PoolRole::Lead);
    assert_eq!(
        named.skipped[0].reason,
        SkipReason::HeldElsewhere {
            by_pool: "team".to_owned()
        }
    );

    // The reviewer is named in a pool too, so it leaves its project pool.
    let reviewer = index.resolve(&facts("codex-1", "shop", "cli:codex", Some("acct-p")));
    assert_eq!(reviewer.placements.len(), 1);
    assert_eq!(reviewer.primary().unwrap().role, PoolRole::Reviewer);
    assert_eq!(reviewer.primary().unwrap().via(), MemberKind::Session);

    // Once "team" is deleted (no longer listed), named is back in its
    // project pool.
    let index = Memberships::new(&pools[..2], &members, HashMap::new());
    let named = index.resolve(&facts("named", "shop", "cli:work", Some("acct-w")));
    assert!(named.in_pool("pool-default:shop").is_some());
}

#[test]
fn an_unknown_sign_in_stays_in_through_any_other_member_and_says_so() {
    let pools = [
        pool("personal", PoolKind::User, Scope::Every, 1),
        pool("apps", PoolKind::User, Scope::Every, 2),
    ];
    let read = MemberLevel {
        level: "read".to_owned(),
        custom_actions: String::new(),
    };
    let members = [
        PoolMember {
            level: Some(read.clone()),
            ..row(
                "personal",
                MemberRef::Account("acct-p".to_owned()),
                PoolRole::Member,
                1,
            )
        },
        row(
            "apps",
            MemberRef::App("cli:home".to_owned()),
            PoolRole::Member,
            2,
        ),
    ];
    let index = Memberships::new(&pools, &members, HashMap::new());

    // Known: in through its Account, at the member's own level (P5).
    let known = index.resolve(&facts("s-1", "blog", "cli:home", Some("acct-p")));
    let personal = known.in_pool("personal").unwrap();
    assert_eq!(personal.via(), MemberKind::Account);
    assert_eq!(personal.level.as_ref(), Some(&read));
    assert!(!known.account_unknown);

    // Unknown: the Account can't match; the App member keeps it in one.
    let unknown = index.resolve(&facts("s-1", "blog", "cli:home", None));
    assert!(unknown.account_unknown);
    assert!(unknown.in_pool("personal").is_none());
    assert_eq!(unknown.primary().unwrap().pool_id, "apps");
}

#[test]
fn a_watched_conversation_becomes_the_session_that_continues_it() {
    let pools = [pool("team", PoolKind::User, shop(), 1)];
    let watched = MemberRef::parse("conversation:acc-work/conv-9").unwrap();
    assert!(watched.is_watched());
    assert_eq!(watched.to_wire(), "conversation:acc-work/conv-9");
    let members = [row("team", watched, PoolRole::Reviewer, 1)];
    let index = Memberships::new(&pools, &members, HashMap::new());
    // Before it's continued, no AI Juggler session is it.
    assert!(
        index
            .resolve(&facts("s-1", "shop", "cli:work", Some("acct-w")))
            .placements
            .is_empty()
    );
    // Continued in AI Juggler: its session is the member, named on its own.
    let continued = SessionFacts {
        conversation: Some(("acc-work".to_owned(), "conv-9".to_owned())),
        ..facts("s-1", "shop", "cli:work", Some("acct-w"))
    };
    let resolved = index.resolve(&continued);
    let place = resolved.in_pool("team").unwrap();
    assert_eq!(
        (place.role, place.via()),
        (PoolRole::Reviewer, MemberKind::Session)
    );
}

#[test]
fn five_hundred_sessions_against_fifty_pools_resolve_within_ten_ms() {
    let mut pools = Vec::new();
    let mut members = Vec::new();
    for n in 0..50 {
        let id = format!("pool-{n}");
        pools.push(pool(
            &id,
            PoolKind::User,
            if n % 2 == 0 { Scope::Every } else { shop() },
            n,
        ));
        members.push(row(
            &id,
            MemberRef::App(format!("cli:{}", n % 7)),
            PoolRole::Member,
            n,
        ));
        members.push(row(
            &id,
            MemberRef::Account(format!("acct-{}", n % 3)),
            PoolRole::Member,
            n,
        ));
        members.push(row(
            &id,
            MemberRef::Session(SessionId::new(format!("s-{n}"))),
            PoolRole::Lead,
            n,
        ));
        if n % 5 == 0 {
            members.push(row(&id, MemberRef::Everyone, PoolRole::Observer, n));
        }
    }
    let sessions: Vec<SessionFacts> = (0..500)
        .map(|n| {
            facts(
                &format!("s-{n}"),
                if n % 2 == 0 { "shop" } else { "blog" },
                &format!("cli:{}", n % 7),
                Some(&format!("acct-{}", n % 3)),
            )
        })
        .collect();
    // A membership change: the index is rebuilt, then every session read.
    let mut best = u128::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        let index = Memberships::new(&pools, &members, HashMap::new());
        let placed: usize = sessions
            .iter()
            .map(|facts| index.resolve(facts).placements.len())
            .sum();
        best = best.min(started.elapsed().as_micros());
        assert!(placed > 500);
    }
    // Debug builds are several times slower than release; the budget
    // holds there with room.
    assert!(best <= 10_000, "500 sessions × 50 pools took {best} µs");
}
