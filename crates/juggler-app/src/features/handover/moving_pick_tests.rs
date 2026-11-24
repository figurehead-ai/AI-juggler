//! AJ-E2317: a move's source by the conversation's holder, its target by
//! the App the user picked (by id), and each profile's folder link checked
//! against its own sessions before the move and before its restarts.
//! Real app state and fixture folders; no OS (nothing runs, nothing is
//! quit or started).

use super::*;
use crate::features::places::handoff::apps_on;

/// Two Accounts, each with two Claude app profiles, and a terminal
/// session on the first whose conversation (`main`) one profile lists.
struct Two {
    _tmp: tempfile::TempDir,
    state: Arc<AppState>,
    root: PathBuf,
    /// Profile id by name: `work` and `team-a` on acct-a, `lab` and
    /// `team-b` on acct-b.
    ids: HashMap<&'static str, String>,
}

impl Two {
    fn id(&self, name: &str) -> String {
        self.ids[name].clone()
    }

    /// `name`'s records list conversation `main` (its app's record of it).
    fn lists_main(&self, name: &str) {
        let folder = self
            .root
            .join("Profiles")
            .join(name)
            .join("claude-code-sessions/acct-a/org");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("local_main.json"),
            format!(
                r#"{{"sessionId":"local_main","cliSessionId":"main","title":"T","cwd":"{}"}}"#,
                self.root.join("work").display()
            ),
        )
        .unwrap();
    }

    /// Its sides, moving to acct-b into the App picked (`target`).
    fn sides(&self, target: Option<&str>) -> Sides {
        let id = target.map(|name| self.id(name));
        sides(&self.state, "s-1", "rec-b", id.as_deref()).unwrap()
    }
}

fn two() -> Two {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    let mut ids = HashMap::new();
    // The one the old pick took on each account is the one seen last.
    for (name, account, seen) in [
        ("work", "acct-a", 10),
        ("team-a", "acct-a", 20),
        ("lab", "acct-b", 10),
        ("team-b", "acct-b", 20),
    ] {
        let (id, _, _) =
            crate::features::places::remember_test_profile(&state, &root, name, Some(account));
        crate::features::places::with_book(&state, |book| {
            book.places.get_mut(&id).unwrap().seen_ms = seen;
        })
        .unwrap();
        ids.insert(name, id);
    }
    let a_code = root.join("code/a");
    code_folders(&a_code, &root.join("code/b"));
    let transcript = a_code.join("projects").join(SLUG).join("main.jsonl");
    std::fs::write(&transcript, "{\"type\":\"user\"}\n").unwrap();
    {
        let storage = crate::commands::lock_storage_arc(&state).unwrap();
        for (account, code, recipe) in [
            ("acct-a", &a_code, "rec-a"),
            ("acct-b", &root.join("code/b"), "rec-b"),
        ] {
            storage
                .accounts()
                .insert(&crate::test_support::account(
                    account,
                    juggler_adapters::claude_code::CLAUDE_CODE_ID,
                    account,
                    code.clone(),
                ))
                .unwrap();
            storage
                .profiles()
                .insert(&crate::test_support::launch_settings(
                    recipe, recipe, account,
                ))
                .unwrap();
        }
        let mut session = crate::test_support::instance(
            "s-1",
            "rec-a",
            juggler_core::model::SessionStatus::Finished,
            root.join("work"),
        );
        session.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript,
            native_id: Some("main".into()),
        });
        storage.sessions().insert(&session).unwrap();
    }
    Two {
        _tmp: tmp,
        state,
        root,
        ids,
    }
}

/// Two profiles on one Account: the source is the one whose records list
/// the conversation, never the one the Account ran last; the target is the
/// App picked, never the Account's other one.
#[test]
fn two_profiles_on_one_account_the_source_by_its_records_the_target_by_the_pick() {
    let two = two();
    two.lists_main("work");
    // Each Account has two Apps that take sessions: a pick by Account
    // (the old one: the running profile, else the one seen last) took
    // the wrong one on both sides.
    assert_eq!(apps_on(&two.state, "acct-a").len(), 2);
    assert_eq!(apps_on(&two.state, "acct-b").len(), 2);
    let sides = two.sides(Some("lab"));
    assert!(sides.stop.is_none(), "{:?}", sides.stop);
    assert_eq!(
        sides.source_place.as_ref().map(|p| &p.0),
        Some(&two.id("work"))
    );
    assert_eq!(
        sides.target_place.as_ref().map(|p| &p.0),
        Some(&two.id("lab"))
    );
    let profile = |name: &str| two.root.join("Profiles").join(name);
    assert_eq!(sides.request.source.host_dir, Some(profile("work")));
    assert_eq!(sides.request.target.host_dir, Some(profile("lab")));
    // The offer restarts exactly those two.
    let offer = offer_of(&two.state, &sides, &Scripted::default());
    assert_eq!(offer.restarts.len(), 2, "{offer:?}");
}

/// Two Apps on the target's Account: the one picked, by its id, whichever
/// it is.
#[test]
fn the_target_is_the_app_picked_by_its_id_when_two_share_an_account() {
    let two = two();
    two.lists_main("work");
    for name in ["lab", "team-b"] {
        let sides = two.sides(Some(name));
        assert_eq!(
            sides.target_place.map(|p| p.0),
            Some(two.id(name)),
            "picked {name}"
        );
    }
    // None picked (a terminal): never one found from the Account; the
    // planner refuses an app → terminal move.
    let sides = two.sides(None);
    assert!(sides.target_place.is_none());
    assert_eq!(sides.request.target.host_dir, None);
    // An App of another Account isn't a target of this one.
    let sides = two.sides(Some("team-a"));
    let stop = sides.stop.expect("refused");
    assert_eq!(stop.kind, juggler_adapter::moving::RefusalKind::Setup);
    assert!(
        stop.words.contains("isn't on acct-b"),
        "the refusal names the Account it isn't on"
    );
}

/// Two profiles listing the conversation can't be told apart: refused, in
/// plain words naming both, before anything is planned or quit.
#[test]
fn two_holders_are_refused_naming_both() {
    let two = two();
    two.lists_main("work");
    two.lists_main("team-a");
    let sides = two.sides(Some("lab"));
    let stop = sides.stop.clone().expect("refused");
    assert_eq!(stop.kind, juggler_adapter::moving::RefusalKind::Setup);
    assert!(
        stop.words.contains(" and ") && stop.words.contains("can't tell which one"),
        "the refusal names both candidates and says it can't tell which one"
    );
    assert!(sides.source_place.is_none() && sides.target_place.is_none());
    let offer = offer_of(&two.state, &sides, &Scripted::default());
    assert!(offer.applies);
    assert_eq!(
        offer.blocks,
        [MoveBlockInfo {
            kind: MoveBlockKind::Setup,
            words: stop.words.clone()
        }]
    );
    let system = Scripted::default();
    let error = run(&two.state, &sides, &system).unwrap_err();
    assert!(
        error.to_string().contains("can't tell which one"),
        "{error}"
    );
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
}

/// One folder spelled two ways is one holder, not two.
#[test]
fn one_profile_remembered_under_two_spellings_is_one_holder() {
    let two = two();
    two.lists_main("work");
    let work =
        crate::features::places::with_book(&two.state, |book| book.places[&two.id("work")].clone())
            .unwrap();
    let mut again = work.clone();
    again.instance_dir = Some(two.root.join("Profiles/./work/"));
    crate::features::places::with_book(&two.state, |book| {
        book.places
            .insert("app:claude-app:work-again".into(), again);
    })
    .unwrap();
    let sides = two.sides(Some("lab"));
    assert!(sides.stop.is_none(), "{:?}", sides.stop);
}

/// A holder AI Juggler has on another Account needs its folder set first
/// (the user's to clear).
#[test]
fn a_holder_on_another_account_needs_its_folder_set() {
    let two = two();
    two.lists_main("lab");
    let stop = two.sides(Some("team-b")).stop.expect("refused");
    assert_eq!(
        stop.kind,
        juggler_adapter::moving::RefusalKind::UserCanClear
    );
    assert!(
        stop.words.contains("doesn't have it on acct-a"),
        "the refusal says the target doesn't have it on the source's Account"
    );
}

/// A profile whose own sessions use another folder than its link: the
/// offer says so, naming both folders, and a move refuses before anything
/// quits.
#[test]
fn a_contradicted_link_refuses_up_front_naming_both_folders() {
    let lab = lab();
    let elsewhere = lab._tmp.path().join("code/elsewhere");
    let system = running_both(&lab);
    system
        .mismatched
        .lock()
        .unwrap()
        .insert(source_id(&lab), elsewhere.clone());
    let offer = offer_of(&lab.state, &lab.sides, &system);
    let block = offer
        .blocks
        .iter()
        .find(|b| b.words.contains("sessions use"));
    assert!(block.is_some(), "{:?}", offer.blocks);
    let block = block.unwrap();
    assert_eq!(block.kind, MoveBlockKind::UserCanClear);
    assert!(
        block.words.contains(&elsewhere.display().to_string())
            && block.words.contains(&lab.a_code.display().to_string())
            && block.words.ends_with("Move becomes available"),
        "{}",
        block.words
    );
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    assert!(error.to_string().contains("sessions use"), "{error}");
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
    assert!(unchanged(&lab));
}

/// A link contradicted by the time the move restarts the profiles: that
/// profile stays closed, and the move says why; the other starts.
#[test]
fn a_profile_is_never_restarted_on_a_folder_its_sessions_contradict() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let elsewhere = lab._tmp.path().join("code/elsewhere");
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    let system = Scripted {
        mismatch_on_quit: Mutex::new(Some((target.clone(), elsewhere))),
        ..running_both(&lab)
    };
    let done = run(&lab.state, &lab.sides, &system).unwrap();
    assert_eq!(done.opened.state, "failed", "{}", done.opened.detail);
    assert!(
        done.opened.detail.contains("left it closed"),
        "{}",
        done.opened.detail
    );
    let starts = system.starts.lock().unwrap();
    assert!(!starts.contains(&target), "{starts:?}");
    assert!(starts.contains(&source_id(&lab)), "{starts:?}");
    assert!(system.sent.lock().unwrap().is_empty(), "no link sent");
}

/// The real check is AJ-E2327's evidence (`places::link::folder_mismatch`):
/// a profile whose own records' conversations are in another folder than
/// the one it's linked to is refused up front, naming both folders.
#[test]
fn the_real_check_is_the_profiles_own_evidence() {
    let two = two();
    two.lists_main("work");
    let code = |name: &str| two.root.join("code").join(name);
    let installs = ["a", "b"]
        .iter()
        .map(|name| crate::features::places::fakes::install(code(name)))
        .collect();
    let _ = two
        .state
        .places
        .made_up
        .set(crate::features::places::MadeUpMac {
            table: Table::default(),
            home: two.root.clone(),
            installs: Some(installs),
        });
    let sides = two.sides(Some("lab"));
    let (id, work) = sides.source_place.clone().unwrap();
    let real = Real::quiet();
    // Its records' conversation is in the folder it's linked to: agreed.
    assert_eq!(real.link_mismatch(&two.state, &id, &work, &code("a")), None);
    assert!(
        !offer_of(&two.state, &sides, &real)
            .blocks
            .iter()
            .any(|b| b.words.contains("sessions use"))
    );
    // Its history is in acct-b's folder now: the link is contradicted.
    let at = |name: &str| code(name).join("projects").join(SLUG).join("main.jsonl");
    std::fs::rename(at("a"), at("b")).unwrap();
    let uses = real.link_mismatch(&two.state, &id, &work, &code("a"));
    assert_eq!(uses.as_deref(), Some(code("b").as_path()));
    let offer = offer_of(&two.state, &sides, &real);
    let block = offer
        .blocks
        .iter()
        .find(|b| b.words.contains("sessions use"));
    assert!(block.is_some(), "{:?}", offer.blocks);
    let words = &block.unwrap().words;
    assert!(
        words.contains(&code("b").display().to_string())
            && words.contains(&code("a").display().to_string()),
        "{words}"
    );
}
