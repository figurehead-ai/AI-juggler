//! AJ-E1529: an account's conversations started outside AI Juggler are
//! listed from the tool's own files, folded into their project, can be
//! resumed from a terminal (the adapter's line) and continued in AI
//! Juggler, refused in words when that can't be done safely.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::conversations::{self, commands::list_impl};
use juggler_app::state::AppState;

fn transcript(config: &Path, cwd: &Path, id: &str, ask: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"gitBranch\":\"main\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{ask}\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

#[test]
fn conversations_outside_ai_juggler_are_listed_resumable_and_continued_safely() {
    // rc21: an id in Claude Code's own form (it resumes by no other).
    const C_1: &str = "1c1c1c1c-0000-4000-8000-000000000001";
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let removed = repo.join(".claude/worktrees/gone"); // a removed worktree: only its transcript is left
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, C_1, "Fix the login test");
    transcript(&config, &removed, "c-2", "Tidy imports");

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    let account_id = account.id.to_string();
    juggler_app::features::projects::backfill::projects_backfill_impl(&state).unwrap();

    let listed = list_impl(&state).unwrap();
    assert_eq!(listed.len(), 2);
    let project = juggler_app::features::projects::list::projects_list_impl(&state)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "app")
        .unwrap();
    for conversation in &listed {
        assert_eq!(conversation.account_label, "Work");
        assert_eq!(
            conversation.project_id.as_deref(),
            Some(project.id.as_str()),
            "both fold into the repo"
        );
        assert!(!conversation.live && conversation.session_id.is_none());
    }
    assert_eq!(project.history_sessions, 2);
    let first = listed.iter().find(|c| c.id == C_1).unwrap();
    assert_eq!(first.title.as_deref(), Some("Fix the login test"));

    let line = conversations::resume_command_impl(&state, &account_id, C_1).unwrap();
    assert!(
        line.contains("'--resume' '1c1c1c1c-0000-4000-8000-000000000001'")
            && line.contains(&format!("cd '{}'", repo.display())),
        "{line}"
    );

    // Its folder is gone: refused in words, nothing launched.
    let error = conversations::continue_impl(&state, &account_id, "c-2").unwrap_err();
    assert!(format!("{error}").contains("folder isn't there"), "{error}");
    // Not one of the account's: refused.
    assert!(conversations::continue_impl(&state, &account_id, "nope").is_err());
}

/// QA batch 8: a first pass over a heavy setup took minutes, and nothing
/// read as running until it ended. The newest files are read first, and
/// a long pass reports as it goes (here after every file).
#[test]
fn a_long_pass_reads_the_newest_files_first_and_reports_as_it_goes() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    // Written after the import, so the pass below is the first to read them.
    let now = std::time::SystemTime::now();
    for (id, hours_ago) in [("c-old", 30), ("c-new", 0), ("c-mid", 5)] {
        transcript(&config, &repo, id, "Work on it");
        let file = config
            .join("projects")
            .join(repo.to_string_lossy().replace(['/', '.'], "-"))
            .join(format!("{id}.jsonl"));
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(now - std::time::Duration::from_secs(hours_ago * 3600))
            .unwrap();
    }

    let seen = std::sync::Mutex::new(Vec::new());
    conversations::tick_with(&state, std::time::Duration::ZERO, &|| {
        let mut ids: Vec<String> = state
            .conversations
            .all()
            .unwrap()
            .into_iter()
            .map(|c| c.summary.id)
            .collect();
        ids.sort();
        seen.lock().unwrap().push(ids);
    })
    .unwrap();
    let seen = seen.into_inner().unwrap();
    assert_eq!(
        seen,
        vec![
            vec!["c-new".to_owned()],
            vec!["c-mid".to_owned(), "c-new".to_owned()],
            vec!["c-mid".to_owned(), "c-new".to_owned(), "c-old".to_owned()]
        ],
        "reported after each file, the newest read first, and once all are listed"
    );
    assert_eq!(state.conversations.all().unwrap().len(), 3);
}

/// QA batch 9: at a start, the conversation list read every file itself
/// while the background pass did too, and didn't answer for 94 s. While a
/// pass is reading, the list answers at once with what's read so far.
#[test]
fn the_list_answers_at_once_while_a_pass_is_reading() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    for id in ["c-1", "c-2", "c-3"] {
        transcript(&config, &repo, id, "Work on it");
    }
    // The newest file isn't a conversation: after reading it the index is
    // still empty, as at a start before the first conversation is read.
    let folder = config
        .join("projects")
        .join(repo.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::write(folder.join("notes.jsonl"), "not a transcript\n").unwrap();
    std::fs::File::options()
        .write(true)
        .open(folder.join("notes.jsonl"))
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
        .unwrap();
    // A pass that stalls after its first file (a slow disk), holding on.
    let (reading, started) = std::sync::mpsc::channel();
    let pass_state = Arc::clone(&state);
    let pass = std::thread::spawn(move || {
        let reading = std::sync::Mutex::new(Some(reading));
        conversations::tick_with(&pass_state, std::time::Duration::ZERO, &|| {
            if let Some(reading) = reading.lock().unwrap().take() {
                reading.send(()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(1_500));
            }
        })
        .unwrap();
    });
    started.recv().unwrap();
    let asked = std::time::Instant::now();
    let listed = list_impl(&state).unwrap();
    assert!(
        asked.elapsed() < std::time::Duration::from_millis(1_000),
        "it didn't wait for the pass: {:?}",
        asked.elapsed()
    );
    assert!(
        listed.is_empty(),
        "nothing read yet, and it didn't read every file itself: {}",
        listed.len()
    );
    pass.join().unwrap();
    assert_eq!(
        list_impl(&state).unwrap().len(),
        3,
        "all, once the pass is done"
    );
}

/// QA batch 10: one Mac's rollouts were 6.6 GB, and reading each whole
/// kept anything from showing for over a minute. A large file is listed
/// first from its start and end (its id, folder, title and last activity,
/// with no tokens yet), and read whole afterwards for its tokens.
#[test]
fn a_large_file_is_listed_from_its_ends_first_and_counted_after() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    // A long session: the ask, then a megabyte of answers, each with its
    // usage, the last one later in the day.
    transcript(&config, &repo, "big", "Port the parser");
    let file = config
        .join("projects")
        .join(repo.to_string_lossy().replace(['/', '.'], "-"))
        .join("big.jsonl");
    let mut text = std::fs::read_to_string(&file).unwrap();
    for n in 0..4_000 {
        text.push_str(&format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"big\",\"timestamp\":\"2026-10-21T{:02}:00:00.000Z\",\"message\":{{\"id\":\"m-{n}\",\"role\":\"assistant\",\"content\":\"{}\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":5}}}}}}\n",
            if n == 3_999 { 18 } else { 11 },
            "x".repeat(200)
        ));
    }
    std::fs::write(&file, text).unwrap();
    assert!(std::fs::metadata(&file).unwrap().len() > 512 * 1024);

    let first = std::sync::Mutex::new(None);
    conversations::tick_with(&state, std::time::Duration::MAX, &|| {
        let mut first = first.lock().unwrap();
        if first.is_none() {
            *first = list_impl(&state).unwrap().into_iter().next();
        }
    })
    .unwrap();
    let quick = first
        .into_inner()
        .unwrap()
        .expect("listed after the quick round");
    assert_eq!(quick.title.as_deref(), Some("Port the parser"));
    assert!(quick.tokens.is_none(), "no tokens until it's read whole");
    let whole = list_impl(&state).unwrap().into_iter().next().unwrap();
    assert_eq!(
        whole.tokens.as_ref().map(|t| (t.input, t.output)),
        Some((40_000, 20_000))
    );
    assert_eq!(
        quick.last_activity_ms, whole.last_activity_ms,
        "the last activity, from its end"
    );
}

/// The go sitting's startup minute (2026-11-07): every start read every
/// transcript again. The index is saved and restored: a file whose size
/// and time are as they were isn't read again (its text here was changed
/// behind its back, same length and time, and the restored index keeps
/// what it read), while a fresh index reads it.
#[test]
fn a_start_reads_only_the_conversation_files_that_changed() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1", "Fix the login test");
    let file = config
        .join("projects")
        .join(repo.to_string_lossy().replace(['/', '.'], "-"))
        .join("c-1.jsonl");
    let start = |data: &Path| {
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
        Arc::new(AppState::init_with_registry(data.to_path_buf(), registry).unwrap())
    };
    let data = root.join("app-data");
    let state = start(&data);
    juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    let titles = |state: &Arc<AppState>| -> Vec<Option<String>> {
        conversations::refresh_impl(state).unwrap();
        list_impl(state)
            .unwrap()
            .into_iter()
            .map(|c| c.title)
            .collect()
    };
    assert_eq!(titles(&state), [Some("Fix the login test".to_owned())]);
    conversations::saved::save(&state);
    assert!(
        data.join(conversations::saved::DIR)
            .join("index.cli")
            .is_file(),
        "saved: {:?}",
        std::fs::read_dir(&data).map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
    );
    drop(state);

    // Changed behind its back: same length, same time (a start can't tell).
    let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("Fix the login test", "Fix the LOGIN test"),
    )
    .unwrap();
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(modified)
        .unwrap();

    let again = start(&data);
    assert_eq!(
        titles(&again),
        [Some("Fix the login test".to_owned())],
        "restored, not read again"
    );
    let fresh = start(&root.join("fresh-data"));
    juggler_app::features::account::import::account_import_mode_impl(
        &fresh,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap();
    assert_eq!(titles(&fresh), [Some("Fix the LOGIN test".to_owned())]);
}

/// A first start (no saved list) builds the conversation list from every
/// transcript, which takes long on a big history: the window shows a
/// quiet line while `building` says so. It clears after a whole pass,
/// and a start with a saved list never shows it. In the app the reading
/// is the background pass's (after the first paint, at the lowest
/// priority): a command finding the list empty doesn't read it itself.
#[test]
fn a_first_start_says_its_list_is_building_until_a_pass_reads_it() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1", "Fix the login test");
    let start = |data: &Path, import: bool| {
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
        let state = Arc::new(AppState::init_with_registry(data.to_path_buf(), registry).unwrap());
        if !import {
            return state;
        }
        juggler_app::features::account::import::account_import_mode_impl(
            &state,
            CLAUDE_CODE_ID,
            config.to_str().unwrap(),
            "Work",
            juggler_app::features::account::import::ImportRequest {
                in_place: true,
                ..juggler_app::features::account::import::ImportRequest::default()
            },
        )
        .unwrap();
        state
    };
    let data = root.join("app-data");
    let state = start(&data, true);
    assert!(
        conversations::building(&state),
        "a first start builds its list"
    );
    // The app's background pass reads: a command answers at once, empty.
    state.conversations.read_in_background();
    assert!(
        list_impl(&state).unwrap().is_empty(),
        "the command didn't read"
    );
    assert!(conversations::building(&state));
    conversations::refresh_impl(&state).unwrap();
    assert!(!conversations::building(&state), "built after a whole pass");
    assert_eq!(list_impl(&state).unwrap().len(), 1);
    conversations::saved::save(&state);
    drop(state);
    let again = start(&data, false);
    assert!(
        !conversations::building(&again),
        "a start with a saved list isn't building"
    );
}

/// Imports the Claude Code folder `config` in place as `label`.
fn import_in_place(state: &Arc<AppState>, config: &Path, label: &str) -> String {
    std::fs::create_dir_all(config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    juggler_app::features::account::import::account_import_mode_impl(
        state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        label,
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string()
}

/// AJ-E3006 R2, rc19: Continue in AI Juggler on a conversation in the
/// second of two Apps (the Lab App first, aij-test second, one
/// subscription) resumes it in aij-test's own folder, in its folder, by
/// Claude's own id for it; a copy an older build forked as
/// `<id>-fork-<n>-0` (Claude Code shows its Resume picker for that) is
/// refused in words before anything starts.
#[test]
fn continue_resumes_in_the_conversations_own_app_by_its_own_id() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let web = root.join("lab/web");
    std::fs::create_dir_all(&web).unwrap();
    let lab = root.join("ClaudeCodeProfiles/lab");
    let aij_test = root.join("ClaudeCodeProfiles/aij-test");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let lab_id = import_in_place(&state, &lab, "Lab");
    let aij_id = import_in_place(&state, &aij_test, "aij-test");
    assert_ne!(lab_id, aij_id);
    let original = "a44c2a28-4542-45c3-a4bb-dadfce0e8b3a";
    let old_fork = format!("{original}-fork-1a1bf34281e-0");
    let new_fork = "0b9e4c1d-77aa-7f00-8123-4c5d6e7f8a9b";
    transcript(&lab, &web, original, "echo lab");
    transcript(&aij_test, &web, original, "echo lab");
    transcript(&aij_test, &web, &old_fork, "echo lab");
    transcript(&aij_test, &web, new_fork, "echo lab");
    conversations::tick_with(&state, std::time::Duration::ZERO, &|| {}).unwrap();

    let (target, spec) = conversations::continue_launch(&state, &aij_id, new_fork).unwrap();
    assert_eq!(target.account.id.as_str(), aij_id);
    assert_eq!(target.account.state_dir, aij_test);
    assert_eq!(
        spec.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(aij_test.to_str().unwrap()),
        "aij-test's own folder, not the Lab App's"
    );
    let at = spec.args.iter().position(|arg| arg == "--resume").unwrap();
    assert_eq!(spec.args[at + 1], new_fork);
    assert_eq!(spec.cwd, web);
    // The Lab App's own copy resumes in the Lab App's folder.
    let (_, lab_spec) = conversations::continue_launch(&state, &lab_id, original).unwrap();
    assert_eq!(
        lab_spec.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(lab.to_str().unwrap())
    );

    // rc19's id: refused in words, no session row made.
    let error = conversations::continue_impl(&state, &aij_id, &old_fork).unwrap_err();
    assert!(
        format!("{error}").contains("can't open this copy"),
        "{error}"
    );
    let storage = state.storage.lock().unwrap();
    assert!(storage.sessions().list(None).unwrap().is_empty());
}

/// rc21: aij-test's copy `<id>-fork-…` (an older build's id) is refused
/// by the one check every door that resumes asks, in its tool's words:
/// Continue in AI Juggler, Copy the resume command (a conversation's and
/// a session's: no command handed out), an ended session's Resume (no
/// session row made), and every resume launch built (an epoch's Resume,
/// a restart, Run in AI Juggler, a switch back). The rows carry the same
/// words, with the conversation it was copied from as the switch
/// recorded it, or how to find it.
#[test]
fn every_door_that_resumes_refuses_an_id_its_tool_wont_open() {
    use juggler_app::features::launch::resume_gate;
    use juggler_core::model::{
        AccountId, ConversationId, EdgeActor, EdgeState, Epoch, EpochId, HarnessId, InstanceKind,
        InstanceRecord, MachineId, SessionId, SessionStatus, SwitchEdge, SwitchEdgeId,
        SwitchMechanism, Timestamp,
    };

    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let web = root.join("lab/web");
    std::fs::create_dir_all(&web).unwrap();
    let lab = root.join("ClaudeCodeProfiles/lab");
    let aij_test = root.join("ClaudeCodeProfiles/aij-test");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let lab_id = import_in_place(&state, &lab, "Lab");
    let aij_id = import_in_place(&state, &aij_test, "aij-test");
    let original = "a44c2a28-4542-45c3-a4bb-dadfce0e8b3a";
    let old_fork = format!("{original}-fork-1a1bf34281e-0");
    let other_old = format!("{original}-fork-1a1bf34281f-0");
    let new_fork = "0b9e4c1d-77aa-7f00-8123-4c5d6e7f8a9b";
    transcript(&lab, &web, original, "echo lab");
    transcript(&aij_test, &web, original, "echo lab");
    transcript(&aij_test, &web, &old_fork, "echo lab");
    transcript(&aij_test, &web, &other_old, "echo lab");
    transcript(&aij_test, &web, new_fork, "echo lab");
    conversations::tick_with(&state, std::time::Duration::ZERO, &|| {}).unwrap();

    // The switch that made `old_fork` (the Lab App's conversation copied
    // into aij-test), and the session Continue in AI Juggler ran on it.
    let (account, profile) = {
        let storage = state.storage.lock().unwrap();
        let account = storage
            .accounts()
            .get(&AccountId::new(aij_id.as_str()))
            .unwrap();
        let profile = storage
            .profiles()
            .list_by_account(&account.id)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let project = storage.projects().touch(&web, None).unwrap();
        let mut record = InstanceRecord {
            id: SessionId::new("sess-old-copy"),
            conversation: Some(ConversationId::new(old_fork.as_str())),
            profile: profile.id.clone(),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: web.clone(),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        };
        storage.sessions().insert(&record).unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
        let graph = storage.switch_graph();
        let item = graph.create_work(&project.id, "web").unwrap();
        let epoch = |id: &str, account: &str, native: &str| Epoch {
            id: EpochId::new(id),
            work: item.id.clone(),
            session_ref: None,
            native_id: Some(native.to_owned()),
            account: AccountId::new(account),
            provider: HarnessId::new(CLAUDE_CODE_ID),
            resumed_from_epoch: None,
            machine: None,
        };
        graph
            .insert_epoch(&epoch("e-lab", &lab_id, original))
            .unwrap();
        graph
            .insert_epoch(&epoch("e-aij", &aij_id, &old_fork))
            .unwrap();
        graph
            .append_edge(&SwitchEdge {
                id: SwitchEdgeId::new("edge-copy"),
                work: item.id.clone(),
                from_epoch: EpochId::new("e-lab"),
                to_epoch: None,
                from_account: AccountId::new(lab_id.as_str()),
                to_account: AccountId::new(aij_id.as_str()),
                from_provider: HarnessId::new(CLAUDE_CODE_ID),
                to_provider: HarnessId::new(CLAUDE_CODE_ID),
                mechanism: SwitchMechanism::Transplant,
                pack_id: None,
                fidelity_shown: None,
                manifest_sha256: None,
                state: EdgeState::Staged,
                actor: EdgeActor::User,
                parent_edge_a: None,
                parent_edge_b: None,
                created_at: Timestamp::now(),
                to_machine: None,
            })
            .unwrap();
        graph
            .mark_launched(&SwitchEdgeId::new("edge-copy"), &EpochId::new("e-aij"))
            .unwrap();
        (account, profile)
    };
    let refused = |what: &str, error: juggler_app::error::AppError| {
        assert!(
            format!("{error}").contains("can't open this copy by its id"),
            "{what}: {error}"
        );
    };

    // Continue in AI Juggler.
    refused(
        "continue",
        conversations::continue_impl(&state, &aij_id, &old_fork).unwrap_err(),
    );
    // Copy the resume command: a conversation's, a session's.
    refused(
        "conversation's command",
        conversations::resume_command_impl(&state, &aij_id, &old_fork).unwrap_err(),
    );
    refused(
        "session's command",
        juggler_app::features::sessions::resume_command_impl(&state, "sess-old-copy").unwrap_err(),
    );
    // An ended session's Resume.
    refused(
        "resume",
        juggler_app::features::switch::resume::session_resume_impl(&state, "sess-old-copy")
            .unwrap_err(),
    );
    // Every resume launch the app builds (an epoch's Resume, a restart,
    // Run in AI Juggler, a switch back): one gate.
    let adapter = state.registry.get(&HarnessId::new(CLAUDE_CODE_ID)).unwrap();
    let resume = juggler_adapter::LaunchMode::Resume {
        session_id: old_fork.clone(),
        delta_prompt: None,
    };
    refused(
        "launch",
        resume_gate::launch_spec(adapter.as_ref(), &account, &profile, &web, &resume).unwrap_err(),
    );
    let fresh = juggler_adapter::LaunchMode::Resume {
        session_id: new_fork.to_owned(),
        delta_prompt: None,
    };
    assert!(resume_gate::launch_spec(adapter.as_ref(), &account, &profile, &web, &fresh).is_ok());
    assert!(conversations::resume_command_impl(&state, &aij_id, new_fork).is_ok());
    {
        let storage = state.storage.lock().unwrap();
        assert_eq!(
            storage.sessions().list(None).unwrap().len(),
            1,
            "no session started"
        );
    }

    // The rows say it, with the way out.
    let rows = list_impl(&state).unwrap();
    let row = |id: &str| {
        rows.iter()
            .find(|row| row.id == id && row.account_id == aij_id)
            .unwrap()
    };
    let copy = row(&old_fork)
        .resume_refusal
        .clone()
        .expect("the copy says why");
    assert!(
        copy.why.contains("can't open this copy by its id"),
        "{}",
        copy.why
    );
    let from = copy
        .original
        .expect("the switch recorded where it came from");
    assert_eq!(
        (
            from.account_id.as_str(),
            from.account_label.as_str(),
            from.id.as_str()
        ),
        (lab_id.as_str(), "Lab", original)
    );
    assert_eq!(copy.find, None);
    let unknown = row(&other_old)
        .resume_refusal
        .clone()
        .expect("an unrecorded copy says why");
    assert_eq!(unknown.original, None);
    assert!(unknown.find.unwrap().contains("History"));
    assert_eq!(row(new_fork).resume_refusal, None);
    assert_eq!(row(original).resume_refusal, None);
    let session = juggler_app::features::sessions::list::instances_shared_snapshot(&state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == "sess-old-copy")
        .expect("the ended session's row");
    let words = session.resume_refusal.expect("the session's row says why");
    assert_eq!(words.why, copy.why);
    assert_eq!(words.original.map(|o| o.id), Some(original.to_owned()));
}
