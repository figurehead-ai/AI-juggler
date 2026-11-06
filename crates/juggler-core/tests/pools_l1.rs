//! L1 (`docs/qa/0.4-validation-plan.md` §3): the pools' pure logic,
//! enumerated in full and checked against the reference model
//! (`scripts/qa/pool_oracle.py`), which is written from D-62 rather than
//! from this code.
//!
//! - Membership and precedence (AJ-E3004): one session against two pools,
//!   (7 matches × 2 scope × 3 capability × 2 sealed × 2 matched twice)² =
//!   28 224 cases.
//! - Talk (AJ-E3005): 4 × 4 roles × 3 pair rules × 4 overrides × sealed ×
//!   3 reaches × 4 levels × linked = 9 216 cases.
//! - Claude's own channel: 3 pools × 3 states = 27 cases.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;

use juggler_core::model::{ProjectId, SessionId};
use juggler_core::pools::talk::{self, InboundPool, PoolTalk, Reach, Route, Talk};
use juggler_core::pools::{Memberships, SessionFacts, SkipReason};
use juggler_core::storage::{
    MemberKind, MemberRef, Pool, PoolKind, PoolMember, PoolRole, Scope, TalkKey, TalkRule,
};

/// The oracle's table `name`, by case.
fn oracle(name: &str) -> HashMap<String, String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/qa/pool_oracle.py");
    let out = Command::new("python3")
        .arg(&script)
        .arg(name)
        .output()
        .expect("python3 runs the reference model");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let (case, expected) = line.split_once('\t').unwrap();
            (case.to_owned(), expected.to_owned())
        })
        .collect()
}

const KINDS: [&str; 7] = [
    "session", "app", "account", "tool", "mac", "everyone", "none",
];
const CAPABILITIES: [&str; 3] = ["full", "shell", "watched"];

fn member_ref(kind: &str) -> Option<MemberRef> {
    Some(match kind {
        "session" => MemberRef::Session(SessionId::new("s")),
        "app" => MemberRef::App("app-1".to_owned()),
        "account" => MemberRef::Account("acct-1".to_owned()),
        "tool" => MemberRef::Tool("claude-code".to_owned()),
        "mac" => MemberRef::Mac("mac-1".to_owned()),
        "everyone" => MemberRef::Everyone,
        _ => return None,
    })
}

fn kind_word(kind: MemberKind) -> &'static str {
    match kind {
        MemberKind::Everyone => "everyone",
        other => other.as_str(),
    }
}

/// One pool's setting, as the oracle words it.
#[derive(Clone)]
struct Setting {
    kind: &'static str,
    scope_in: bool,
    capability: &'static str,
    sealed: bool,
    twice: bool,
}

impl Setting {
    fn all() -> Vec<Self> {
        let mut out = Vec::new();
        for kind in KINDS {
            for scope_in in [true, false] {
                for capability in CAPABILITIES {
                    for sealed in [false, true] {
                        for twice in [false, true] {
                            out.push(Self {
                                kind,
                                scope_in,
                                capability,
                                sealed,
                                twice,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    fn key(&self) -> String {
        let flag = |on: bool| if on { "1" } else { "0" };
        [
            self.kind,
            if self.scope_in { "in" } else { "out" },
            self.capability,
            flag(self.sealed),
            flag(self.twice),
        ]
        .join(",")
    }

    fn pool(&self, id: &str, created_at: i64) -> Pool {
        let project = ProjectId::new(if self.scope_in { "proj" } else { "elsewhere" });
        Pool {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: PoolKind::User,
            project: Some(project.clone()),
            created_at,
            spans_macs: false,
            scope: Scope::These(BTreeSet::from([Some(project)])),
            sealed: self.sealed,
            skip_held: false,
        }
    }

    fn members(&self, id: &str) -> Vec<PoolMember> {
        let row = |member, role, joined_at| PoolMember {
            pool_id: id.to_owned(),
            member,
            role,
            joined_at,
            level: None,
        };
        let mut out: Vec<PoolMember> = member_ref(self.kind)
            .into_iter()
            .map(|member| row(member, PoolRole::Lead, 10))
            .collect();
        if self.twice && self.kind != "everyone" {
            out.push(row(MemberRef::Everyone, PoolRole::Observer, 20));
        }
        out
    }
}

#[test]
fn membership_matches_the_reference_model_in_all_28_224_cases() {
    let expected = oracle("membership");
    assert_eq!(expected.len(), 28_224);
    let facts = SessionFacts {
        session: SessionId::new("s"),
        work: None,
        project: Some(ProjectId::new("proj")),
        app: Some("app-1".to_owned()),
        account: Some("acct-1".to_owned()),
        tool: Some("claude-code".to_owned()),
        mac: Some("mac-1".to_owned()),
        conversation: None,
    };
    let settings = Setting::all();
    let mut wrong = Vec::new();
    for one in &settings {
        for two in &settings {
            let pools = [one.pool("p1", 1), two.pool("p2", 2)];
            let mut members = one.members("p1");
            members.extend(two.members("p2"));
            let resolved = Memberships::new(&pools, &members, HashMap::new()).resolve(&facts);
            let ins: Vec<String> = resolved
                .placements
                .iter()
                .map(|place| {
                    format!(
                        "{}:{}:{}",
                        place.pool_id,
                        place.role.as_str(),
                        kind_word(place.via())
                    )
                })
                .collect();
            let outs: Vec<String> = resolved
                .skipped
                .iter()
                .map(|skip| {
                    let why = match &skip.reason {
                        SkipReason::OutOfScope => "out_of_scope".to_owned(),
                        SkipReason::SealedElsewhere { sealed_pool } => {
                            format!("sealed_by:{sealed_pool}")
                        }
                        SkipReason::HeldElsewhere { by_pool } => format!("held_by:{by_pool}"),
                    };
                    format!("{}:{why}", skip.pool_id)
                })
                .collect();
            let or_dash = |parts: Vec<String>| {
                if parts.is_empty() {
                    "-".to_owned()
                } else {
                    parts.join(",")
                }
            };
            let got = format!("{}|{}", or_dash(ins), or_dash(outs));
            let case = format!("{}|{}", one.key(), two.key());
            if expected[&case] != got {
                wrong.push(format!("{case}: want {} got {got}", expected[&case]));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 28 224 differ, first: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
}

const ROLES: [PoolRole; 4] = [
    PoolRole::Lead,
    PoolRole::Member,
    PoolRole::Reviewer,
    PoolRole::Observer,
];
const LEVELS: [&str; 4] = ["read", "work_together", "full", "custom"];

fn rule_word(rule: Option<TalkRule>) -> &'static str {
    rule.map_or("none", TalkRule::as_str)
}

#[test]
fn talk_matches_the_reference_model_in_all_9_216_cases() {
    let expected = oracle("talk");
    assert_eq!(expected.len(), 9_216);
    let from = MemberRef::Session(SessionId::new("from"));
    let to = MemberRef::Session(SessionId::new("to"));
    let mut wrong = Vec::new();
    for sender in ROLES {
        for receiver in ROLES {
            for pair in TalkRule::ALL {
                for over in [
                    None,
                    Some(TalkRule::Allowed),
                    Some(TalkRule::AskFirst),
                    Some(TalkRule::Blocked),
                ] {
                    let pair_key = TalkKey::Roles(sender, receiver);
                    let link_key = TalkKey::Connection(from.clone(), to.clone());
                    let mut rows = vec![(&pair_key, pair)];
                    if let Some(over) = over {
                        rows.push((&link_key, over));
                    }
                    let rules = PoolTalk::new(rows);
                    for sealed in [false, true] {
                        for (reach, reach_word) in [
                            (Reach::Tools, "full"),
                            (Reach::Shell, "shell"),
                            (Reach::Watched, "watched"),
                        ] {
                            for level in LEVELS {
                                for linked in [false, true] {
                                    let got = match talk::decide(
                                        &rules,
                                        (&from, sender),
                                        (&to, receiver),
                                        reach,
                                        level == "full",
                                        Route { linked, sealed },
                                    ) {
                                        Talk::Deliver { wake: true } => "deliver:wake",
                                        Talk::Deliver { wake: false } => "deliver",
                                        Talk::Hold(talk::HoldReason::AskFirst) => "hold:ask",
                                        Talk::Hold(talk::HoldReason::Watched) => "hold:watched",
                                        Talk::Refuse(talk::RefuseReason::Blocked) => {
                                            "refuse:blocked"
                                        }
                                        Talk::Refuse(talk::RefuseReason::Sealed) => "refuse:sealed",
                                    };
                                    let flag = |on: bool| if on { "1" } else { "0" };
                                    let case = [
                                        sender.as_str(),
                                        receiver.as_str(),
                                        pair.as_str(),
                                        rule_word(over),
                                        flag(sealed),
                                        reach_word,
                                        level,
                                        flag(linked),
                                    ]
                                    .join(",");
                                    if expected[&case] != got {
                                        wrong.push(format!(
                                            "{case}: want {} got {got}",
                                            expected[&case]
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 9 216 differ, first: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
}

#[test]
fn claudes_own_channel_matches_the_reference_model_in_all_27_cases() {
    let expected = oracle("inbound");
    assert_eq!(expected.len(), 27);
    let states = [
        (
            "sealed",
            InboundPool {
                sealed: true,
                holds: false,
            },
        ),
        (
            "holds",
            InboundPool {
                sealed: false,
                holds: true,
            },
        ),
        (
            "open",
            InboundPool {
                sealed: false,
                holds: false,
            },
        ),
    ];
    for a in states {
        for b in states {
            for c in states {
                let case = [a.0, b.0, c.0].join(",");
                assert_eq!(
                    talk::claude_inbound(&[a.1, b.1, c.1]).as_str(),
                    expected[&case],
                    "{case}"
                );
            }
        }
    }
}
