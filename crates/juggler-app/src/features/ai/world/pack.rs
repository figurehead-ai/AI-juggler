//! AI Juggler's kind pack (ADR-0038 §2): the node kinds, the link kinds
//! with their allowed ends, each kind's walk rules, decay, tiers and
//! reasoning questions. gyrus is generic; this is what makes it Bot's map
//! of the user's work.
//!
//! **Entity nodes are pointers.** A `session`, `app`, `profile`,
//! `project`, `account`, `pool` or `task` node holds the entity's id (its
//! key) and its links, never its state: what it is doing, its name and its
//! usage are read live at assembly from the owning module
//! ([`super::resolve`]). A `memory` node points at an ADR-0037 record by
//! its random subject id; a `turn_summary` at a Chat turn; a `topic` at a
//! Chat topic. None of them holds words: only `fact`, `rule` and
//! `question` nodes carry a label of their own.

use gyrus::{Fold, KindSpec, LinkSpec, Pack, TierRules, WalkRule};

/// The kinds whose node is a pointer to an entity a module owns.
pub const ENTITY_KINDS: &[&str] = &[
    "session", "app", "profile", "project", "account", "pool", "task",
];

/// A tier rule that never archives (what the user stated, entities).
const KEEP: TierRules = TierRules {
    promote_short_after: 2,
    promote_mid_after: 5,
    demote_after_days: 30.0,
    archive_after_days: None,
};

/// ADR-0037 §2's tiers for what was asked: idle 30 days decays, 30 more
/// archives (here: kept, out of walks, rather than deleted).
const ASKED: TierRules = TierRules {
    promote_short_after: 2,
    promote_mid_after: 5,
    demote_after_days: 30.0,
    archive_after_days: Some(60.0),
};

/// A pointer kind: no label in the keyword index, never archived.
fn entity(name: &str, half_life: f64) -> KindSpec {
    KindSpec::new(name)
        .half_life(half_life)
        .tiers(KEEP)
        .not_searchable()
}

/// The pack.
///
/// # Errors
///
/// A pack that doesn't hold together (a test builds it, so a build that
/// ships has one that does).
#[expect(
    clippy::too_many_lines,
    reason = "the pack is one declaration, kind by kind"
)]
pub fn pack() -> gyrus::Result<Pack> {
    let sessions_of = WalkRule::inward("part_of", 2, 6).decay(0.6);
    Pack::builder()
        .kind(
            KindSpec::new("user")
                .half_life(365.0)
                .tiers(KEEP)
                .not_searchable()
                .walk(WalkRule::out("prefers", 1, 6))
                .walk(WalkRule::out("works_on", 2, 4))
                .walk(WalkRule::out("behaves", 1, 3)),
        )
        .kind(
            KindSpec::new("preference")
                .half_life(180.0)
                .tiers(KEEP)
                .not_searchable()
                .walk(WalkRule::out("about", 1, 3).decay(0.9))
                .question("Does this apply to what the user is asking about now?"),
        )
        .kind(
            KindSpec::new("behaviour")
                .half_life(90.0)
                .walk(WalkRule::out("about", 1, 2)),
        )
        .kind(
            entity("project", 60.0)
                .walk(sessions_of.clone())
                .walk(WalkRule::inward("about", 2, 4))
                .question("Which of its sessions are running now? Read their live state."),
        )
        .kind(
            entity("task", 14.0)
                .walk(WalkRule::out("part_of", 2, 2))
                .walk(WalkRule::inward("about", 1, 3))
                .question("Is it still open? Read its live state."),
        )
        .kind(
            entity("app", 90.0)
                .walk(WalkRule::inward("runs_in", 1, 6).decay(0.6))
                .walk(WalkRule::inward("profile_of", 1, 4))
                .walk(WalkRule::out("uses_account", 1, 2)),
        )
        .kind(
            entity("profile", 90.0)
                .walk(WalkRule::out("profile_of", 1, 1))
                .walk(WalkRule::out("uses_account", 1, 2))
                .walk(WalkRule::inward("runs_in", 1, 4))
                .question("Is this the active profile right now?"),
        )
        .kind(
            entity("account", 90.0)
                .walk(WalkRule::inward("uses_account", 1, 4))
                .question("How much room is left? Read its live usage, never a remembered figure."),
        )
        .kind(
            entity("session", 7.0)
                .walk(WalkRule::out("part_of", 2, 2).decay(0.8))
                .walk(WalkRule::out("runs_in", 1, 1).decay(0.8))
                .walk(WalkRule::out("member_of", 1, 2))
                .walk(WalkRule::out("uses_account", 1, 1))
                .walk(WalkRule::inward("about", 2, 4))
                .walk(WalkRule::inward("asked_about", 1, 3))
                .question(
                    "What is it doing right now? Read its live state, never a remembered one.",
                ),
        )
        .kind(
            entity("pool", 30.0)
                .walk(WalkRule::inward("member_of", 1, 6))
                .walk(WalkRule::out("part_of", 1, 1)),
        )
        .kind(
            KindSpec::new("memory")
                .half_life(90.0)
                .tiers(ASKED)
                .not_searchable()
                .mergeable()
                .walk(WalkRule::out("about", 2, 4).decay(0.8))
                .walk(WalkRule::out("derived_from", 1, 2))
                .max_links("about", 16)
                .question("When was this said, and does it still hold?"),
        )
        .kind(
            KindSpec::new("fact")
                .half_life(30.0)
                .tiers(ASKED)
                .mergeable()
                .walk(WalkRule::out("about", 2, 3))
                .walk(WalkRule::both("contradicts", 1, 2))
                .question("Is this still current? Check its source's live state."),
        )
        .kind(KindSpec::new("rule").half_life(365.0).tiers(KEEP))
        .kind(
            KindSpec::new("question")
                .half_life(14.0)
                .tiers(ASKED)
                .walk(WalkRule::out("about", 1, 3)),
        )
        .kind(
            KindSpec::new("topic")
                .half_life(30.0)
                .tiers(KEEP)
                .not_searchable()
                .walk(WalkRule::inward("part_of", 1, 6).decay(0.8)),
        )
        .kind(
            KindSpec::new("conversation")
                .half_life(30.0)
                .tiers(KEEP)
                .not_searchable()
                .walk(WalkRule::inward("part_of", 1, 4)),
        )
        .kind(
            KindSpec::new("turn_summary")
                .half_life(14.0)
                .tiers(ASKED)
                .not_searchable()
                .walk(WalkRule::out("asked_about", 1, 4).decay(0.8))
                .walk(WalkRule::out("part_of", 1, 1)),
        )
        .link(LinkSpec::new("prefers", &["user"], &["preference"]))
        .link(LinkSpec::new("behaves", &["user"], &["behaviour"]))
        .link(LinkSpec::new("works_on", &["user"], &["project", "task"]))
        .link(LinkSpec::new(
            "about",
            &[
                "memory",
                "fact",
                "preference",
                "behaviour",
                "question",
                "rule",
            ],
            &[],
        ))
        .link(LinkSpec::new(
            "part_of",
            &["session", "task", "pool", "turn_summary", "conversation"],
            &["project", "pool", "topic", "conversation"],
        ))
        .link(LinkSpec::new("runs_in", &["session"], &["app", "profile"]))
        .link(LinkSpec::new("profile_of", &["profile"], &["app"]))
        .link(LinkSpec::new(
            "uses_account",
            &["app", "profile", "session"],
            &["account"],
        ))
        .link(LinkSpec::new("member_of", &["session"], &["pool"]))
        .link(LinkSpec::new(
            "asked_about",
            &["turn_summary", "question"],
            &[
                "session", "app", "profile", "project", "account", "pool", "task", "memory",
            ],
        ))
        .link(LinkSpec::new(
            "derived_from",
            &["fact", "memory"],
            &["memory", "turn_summary"],
        ))
        .link(LinkSpec::new(
            "contradicts",
            &["fact", "memory"],
            &["fact", "memory"],
        ))
        .link(LinkSpec::new(
            "supersedes",
            &["fact", "memory"],
            &["fact", "memory"],
        ))
        .link(LinkSpec::new("governs", &["rule"], &[]))
        .link(LinkSpec::new(
            "same_as",
            &["memory", "fact"],
            &["memory", "fact"],
        ))
        .rules("rule", "governs")
        .same_as("same_as")
        .fold(Fold {
            child: "turn_summary".into(),
            link: "part_of".into(),
            parent: "topic".into(),
            after_days: 14.0,
        })
        .build()
}
