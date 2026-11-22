//! The rollup's fixtures: reverted, idle, failing, asking and quiet
//! agents, and the round-3 rules (state words follow the facts; never
//! "partly done" on reverted or idle work; never a pass count beside "not
//! run"; "blocked" only where something blocks).

use super::*;

const MIN: i64 = 60_000;

fn agent(id: &str, events: Vec<(i64, Observed)>, turns: Vec<Turn>) -> AgentFacts {
    AgentFacts {
        session: id.into(),
        name: id.into(),
        events,
        turns,
        ..AgentFacts::default()
    }
}

fn state(state: SessionState) -> Observed {
    Observed::StateEntered { state }
}

fn turn(at_ms: i64) -> Turn {
    Turn {
        at_ms,
        outcome: Some(TurnOutcome::Completed),
        ..Turn::default()
    }
}

const WINDOW: Window = Window {
    start_ms: 0,
    end_ms: 30 * MIN,
};

/// Seven agents: one failing, one asking, one stuck, one that reverted
/// its work, one finished with tests, one idle (a turn with no change
/// since), one working; and three quiet ones.
fn busy() -> Vec<AgentFacts> {
    vec![
        agent(
            "web",
            vec![(5 * MIN, state(SessionState::Failed))],
            vec![Turn {
                failure: Some("The build failed: missing module `auth`.".into()),
                outcome: Some(TurnOutcome::Failed),
                ..turn(5 * MIN)
            }],
        ),
        agent(
            "api",
            vec![(6 * MIN, state(SessionState::AsksYou))],
            vec![Turn {
                question: Some("Should I bump the major version?".into()),
                ..turn(6 * MIN)
            }],
        ),
        agent(
            "infra",
            vec![
                (2 * MIN, state(SessionState::Working)),
                (10 * MIN, state(SessionState::Stuck)),
            ],
            Vec::new(),
        ),
        agent(
            "parser",
            Vec::new(),
            vec![Turn {
                words: Some(
                    "The new date library broke two callers, so I reverted the whole change."
                        .into(),
                ),
                caveat: Some(Caveat::Partial),
                ..turn(12 * MIN)
            }],
        ),
        agent(
            "docs",
            vec![
                (20 * MIN, state(SessionState::Finished)),
                (
                    20 * MIN,
                    Observed::Ended {
                        how: EndKind::Clean,
                    },
                ),
            ],
            vec![Turn {
                tests: Tests::Ran {
                    passed: 52,
                    failed: 0,
                },
                caveat: Some(Caveat::NotRun),
                words: Some("Updated the guide. 52 tests pass.".into()),
                ..turn(19 * MIN)
            }],
        ),
        agent(
            "ui",
            vec![(25 * MIN, state(SessionState::Working))],
            Vec::new(),
        ),
        agent("db", Vec::new(), vec![turn(14 * MIN)]),
        // Open, but nothing in the window: quiet.
        agent("cli", Vec::new(), Vec::new()),
    ]
}

#[test]
fn every_agent_is_accounted_for_needs_and_failures_first() {
    let digest = rollup(WINDOW, &busy(), 2);
    let order: Vec<&str> = digest.lines.iter().map(|l| l.name.as_str()).collect();
    // L3 first, then the L2s (the stuck one wants a look first), then
    // what finished, then "on" by newest activity.
    assert_eq!(order, ["web", "infra", "api", "docs", "ui", "db", "parser"]);
    assert_eq!(digest.lines.len(), 7, "every changed agent has a line");
    assert_eq!(digest.shown, 6, "six shown, one behind 'and 1 more'");
    assert_eq!(
        digest.quiet, 3,
        "two quiet given, one open agent with nothing new"
    );
    assert!(!digest.ai, "the rollup is code's");
}

#[test]
fn a_failing_agent_says_its_own_failure_and_level() {
    let digest = rollup(WINDOW, &busy(), 0);
    let web = &digest.lines[0];
    assert_eq!((web.state.as_str(), web.level), ("failed", 3));
    assert_eq!(web.quote_kind.as_deref(), Some("failure"));
    assert_eq!(
        web.quote.as_deref(),
        Some("The build failed: missing module `auth`.")
    );
}

#[test]
fn an_asking_agent_says_its_own_question() {
    let digest = rollup(WINDOW, &busy(), 0);
    let api = digest.lines.iter().find(|l| l.name == "api").unwrap();
    assert_eq!((api.state.as_str(), api.level), ("asks", 2));
    assert_eq!(
        api.quote.as_deref(),
        Some("Should I bump the major version?")
    );
}

#[test]
fn reverted_work_is_said_as_reverted_and_never_partly_done() {
    let digest = rollup(WINDOW, &busy(), 0);
    let parser = digest.lines.iter().find(|l| l.name == "parser").unwrap();
    assert!(parser.reverted);
    assert_eq!(parser.caveat, None, "'partly done' dropped on undone work");
    assert_eq!(parser.state, "idle", "its turn ended cleanly");
}

#[test]
fn never_a_pass_count_beside_not_run() {
    let digest = rollup(WINDOW, &busy(), 0);
    let docs = digest.lines.iter().find(|l| l.name == "docs").unwrap();
    assert_eq!(docs.tests.as_deref(), Some("passed"));
    assert_eq!(docs.passed, Some(52));
    assert_eq!(docs.caveat, None, "'not run' dropped where tests ran");
    assert_eq!(docs.state, "finished");
}

#[test]
fn partly_done_never_on_idle_work_and_blocked_only_where_something_blocks() {
    let idle = agent("idle", vec![(MIN, state(SessionState::Idle))], Vec::new());
    let line = line_of(&idle).unwrap();
    assert_eq!(line.state, "idle");
    // A caveat with no turn can't be said at all (no words to hold one);
    // with a turn and nothing blocking, "blocked" goes.
    let unblocked = agent(
        "calm",
        Vec::new(),
        vec![Turn {
            caveat: Some(Caveat::Blocked),
            words: Some("Moved the handler into its own module.".into()),
            ..turn(MIN)
        }],
    );
    assert_eq!(line_of(&unblocked).unwrap().caveat, None);
    let blocked = agent(
        "waits",
        Vec::new(),
        vec![Turn {
            caveat: Some(Caveat::Blocked),
            words: Some("I'm waiting on the staging key before I can deploy.".into()),
            ..turn(MIN)
        }],
    );
    assert_eq!(line_of(&blocked).unwrap().caveat, Some(Caveat::Blocked));
    assert!(
        !caveat_holds(Caveat::Partial, &[], Tests::Unknown, false, false),
        "idle: nothing was done"
    );
    assert!(caveat_holds(
        Caveat::Partial,
        &[turn(MIN)],
        Tests::Unknown,
        false,
        false
    ));
    assert!(!caveat_holds(
        Caveat::NotDone,
        &[turn(MIN)],
        Tests::Unknown,
        true,
        false
    ));
}

#[test]
fn state_words_follow_the_newest_fact() {
    // Failed, then started again and working: working, not failed.
    let recovered = agent(
        "svc",
        vec![
            (MIN, state(SessionState::Failed)),
            (2 * MIN, Observed::Restarted),
            (3 * MIN, state(SessionState::Working)),
        ],
        Vec::new(),
    );
    let line = line_of(&recovered).unwrap();
    assert_eq!((line.state.as_str(), line.level), ("working", 1));
    assert!(line.restarted && line.notable);
    // Asked, then answered: no longer asks.
    let answered = agent(
        "q",
        vec![
            (MIN, state(SessionState::AsksYou)),
            (2 * MIN, Observed::AskResolved),
        ],
        Vec::new(),
    );
    assert_eq!(line_of(&answered).unwrap().state, "working");
    // A limit's ending keeps the limit.
    let limit = agent(
        "l",
        vec![
            (MIN, state(SessionState::LimitHit)),
            (
                MIN,
                Observed::Ended {
                    how: EndKind::Error,
                },
            ),
        ],
        Vec::new(),
    );
    assert_eq!(line_of(&limit).unwrap().state, "limit");
    // Stuck since minute 10, the window's newest fact at 25: 15 minutes.
    let stuck = agent(
        "s",
        vec![
            (10 * MIN, state(SessionState::Stuck)),
            (25 * MIN, Observed::ProcessesLeft { count: 2 }),
        ],
        Vec::new(),
    );
    let line = line_of(&stuck).unwrap();
    assert_eq!((line.stuck_min, line.left_processes), (Some(15), Some(2)));
}

#[test]
fn the_same_facts_give_the_same_digest() {
    let mut shuffled = busy();
    shuffled.reverse();
    assert_eq!(rollup(WINDOW, &busy(), 2), rollup(WINDOW, &shuffled, 2));
}

#[test]
fn undo_words_are_read_unless_negated() {
    assert!(says_undone("Tests broke, so I rolled it back."));
    assert!(says_undone("I threw the experiment away."));
    assert!(says_undone("Scrapped the whole refactor."));
    assert!(says_undone("Ich habe die Änderung zurückgenommen."));
    assert!(!says_undone("I did not revert anything."));
    assert!(!says_undone("Dropped the logging level to info."));
    assert!(!says_undone("Added the retry backoff."));
}

#[test]
fn the_notification_carries_only_the_calm_news() {
    let digest = rollup(WINDOW, &busy(), 0);
    let (title, body) = notice_words(&digest).unwrap();
    assert_eq!(title, "Digest · 7 agents");
    assert_eq!(
        body, "docs finished",
        "the failure and the asks keep their own path"
    );
    let mut many = busy();
    for name in ["a", "b", "c"] {
        many.push(agent(
            name,
            vec![(MIN, state(SessionState::Finished))],
            vec![Turn {
                tests: Tests::Ran {
                    passed: 3,
                    failed: if name == "b" { 1 } else { 0 },
                },
                ..turn(MIN)
            }],
        ));
    }
    let (_, body) = notice_words(&rollup(WINDOW, &many, 0)).unwrap();
    assert_eq!(body, "4 agents finished · tests failed on b");
    let working = vec![agent(
        "ui",
        vec![(MIN, state(SessionState::Working))],
        Vec::new(),
    )];
    assert_eq!(
        notice_words(&rollup(WINDOW, &working, 0)),
        None,
        "nothing calm worth a ping"
    );
}

#[test]
fn a_quote_is_capped_at_a_word() {
    let long = "word ".repeat(40);
    let capped = cap(&long, QUOTE_MAX);
    assert!(capped.chars().count() <= QUOTE_MAX);
    assert!(capped.ends_with("word…"));
}
