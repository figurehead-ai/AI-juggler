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

/// AJ-E805 (round 3): a pass that only read lines appended to known
/// conversations moves their projects' last use and nothing more: it ran
/// the whole projects backfill (every conversation on the Mac placed,
/// every project written) after every pass while sessions worked. A new
/// conversation still brings the backfill, and with it a new folder's
/// project.
#[test]
fn appended_lines_move_a_projects_last_use_and_only_new_conversations_backfill() {
    const C_1: &str = "1c1c1c1c-0000-4000-8000-0000000000a1";
    const C_2: &str = "1c1c1c1c-0000-4000-8000-0000000000a2";
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let app = root.join("Dev/app");
    let other = root.join("Dev/other");
    for repo in [&app, &other] {
        std::fs::create_dir_all(repo.join(".git")).unwrap();
    }
    let config = root.join("setup");
    transcript(&config, &app, C_1, "Fix the login test");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    import_in_place(&state, &config, "Work");
    let project = |name: &str| {
        juggler_app::features::projects::list::projects_list_impl(&state)
            .unwrap()
            .into_iter()
            .find(|p| p.name == name)
    };
    conversations::tick(&state).unwrap();
    let first = project("app").expect("a new conversation's folder is a project");
    // The project goes (as the user's Forget would leave it): only the
    // backfill makes it again.
    {
        let storage = state.storage.lock().unwrap();
        let id = juggler_core::model::ProjectId::new(first.id.clone());
        assert!(storage.projects().delete_if_unused(&id).unwrap());
    }
    // A line appended an hour later.
    let file = config
        .join("projects")
        .join(app.to_string_lossy().replace(['/', '.'], "-"))
        .join(format!("{C_1}.jsonl"));
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str(&format!(
        "{{\"type\":\"assistant\",\"sessionId\":\"{C_1}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T11:00:00.000Z\",\"message\":{{\"role\":\"assistant\",\"content\":\"Done.\"}}}}\n",
        app.display()
    ));
    std::fs::write(&file, text).unwrap();
    assert!(
        conversations::tick(&state).unwrap(),
        "the pass read the line"
    );
    assert!(
        project("app").is_none(),
        "lines appended to a known conversation run no backfill"
    );
    // A new conversation, in a new folder: the backfill runs.
    transcript(&config, &other, C_2, "Tidy imports");
    conversations::tick(&state).unwrap();
    assert!(project("other").is_some(), "a new folder becomes a project");
    let again = project("app").expect("the backfill placed every conversation");
    // A later line moves the project's last use, without a backfill.
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str(&format!(
        "{{\"type\":\"user\",\"sessionId\":\"{C_1}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T12:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"And the logout.\"}}}}\n",
        app.display()
    ));
    std::fs::write(&file, text).unwrap();
    conversations::tick(&state).unwrap();
    let moved = project("app").unwrap();
    assert!(
        moved.history_last_used_ms > again.history_last_used_ms,
        "{:?} after {:?}",
        moved.history_last_used_ms,
        again.history_last_used_ms
    );
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

/// 0.4.5 (AJ-E805): a transcript that grows is read once, as it grows, by
/// one reader that both the conversation list and the usage reading take
/// from. The list's 15 s pass read every changed transcript whole again for
/// its tokens (every pass under 32 MB), so a working session cost more the
/// longer it ran; and the usage reading tailed the same file beside it.
#[test]
fn a_pass_reads_only_what_was_appended() {
    use juggler_adapters::transcript_follow::followed_here;
    const ID: &str = "1c1c1c1c-0000-4000-8000-0000000000b1";
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let app = root.join("Dev/app");
    std::fs::create_dir_all(app.join(".git")).unwrap();
    let config = root.join("setup");
    transcript(&config, &app, ID, "Fix the login test");
    let file = config
        .join("projects")
        .join(app.to_string_lossy().replace(['/', '.'], "-"))
        .join(format!("{ID}.jsonl"));
    let answer = |n: usize| {
        format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"{ID}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:01:00.000Z\",\"requestId\":\"req-{n}\",\"uuid\":\"u-{n}\",\"message\":{{\"id\":\"msg-{n}\",\"role\":\"assistant\",\"content\":\"{}\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":2}}}}}}\n",
            app.display(),
            "x".repeat(400)
        )
    };
    // Big enough for the quick read's two ends (320 KB) and more.
    let mut text = std::fs::read_to_string(&file).unwrap();
    for n in 0..1_000 {
        text.push_str(&answer(n));
    }
    std::fs::write(&file, &text).unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = import_in_place(&state, &config, "Work");
    let tokens = || {
        state
            .conversations
            .all()
            .unwrap()
            .into_iter()
            .find(|c| c.summary.id == ID)
            .map(|c| (c.summary.tokens.input, c.summary.tokens.output))
    };
    let stored = || {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .events_for_account(&account, 0)
            .unwrap()
            .len()
    };
    let start = followed_here();
    conversations::tick(&state).unwrap();
    juggler_app::usage_pipeline::ingest_pass(&state, 4);
    assert_eq!(tokens(), Some((10_000, 2_000)));
    assert_eq!(stored(), 1_000);
    let whole = std::fs::metadata(&file).unwrap().len();
    assert_eq!(
        followed_here() - start,
        whole,
        "the first read reads it once, for both"
    );
    // One more answer: both read only it.
    let line = answer(1_000);
    let mut appended = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap();
    std::io::Write::write_all(&mut appended, line.as_bytes()).unwrap();
    drop(appended);
    let before = followed_here();
    assert!(
        conversations::tick(&state).unwrap(),
        "the pass read the line"
    );
    juggler_app::usage_pipeline::ingest_pass(&state, 8);
    assert_eq!(
        followed_here() - before,
        line.len() as u64,
        "what was appended, and only that"
    );
    assert_eq!(tokens(), Some((10_010, 2_002)));
    assert_eq!(stored(), 1_001);
}

/// What the session page's Conversation panel is sent.
struct Kept;

impl conversations::feed::DeltaSink for Kept {
    fn send(&self, _: conversations::feed::dto::ConversationDeltaInfo) -> bool {
        true
    }
}

/// The turns the panel shows for `account`'s conversation `id`, as
/// `conversations_follow` reads them: its file from the index, then the
/// feed.
fn panel_turns(state: &Arc<AppState>, account: &str, id: &str) -> (String, Vec<String>) {
    let path = state.conversations.path_of(account, id);
    assert!(path.is_some(), "the index names the conversation's file");
    let feeds = Arc::new(conversations::feed::Feeds::default());
    let snapshot = conversations::feed::follow_impl(
        state,
        &feeds,
        conversations::feed::FeedKey {
            account: account.to_owned(),
            conversation: id.to_owned(),
            subagent: None,
        },
        path,
        Some(20),
        Arc::new(Kept),
    );
    if let Some(subscription) = snapshot.subscription {
        conversations::feed::unfollow_impl(&feeds, subscription);
    }
    let said = snapshot
        .turns
        .iter()
        .flat_map(|turn| &turn.blocks)
        .map(|block| block.text.clone())
        .collect();
    (snapshot.state, said)
}

/// 0.4.5: the session page's Conversation panel reads a Claude Code
/// conversation's turns, written as the lab's replica writes them (a
/// title line, then turns), and a Codex thread's (the coordinator's walk:
/// the panel's placeholders never filled on a build with the transcript
/// readers).
#[test]
fn a_conversations_turns_reach_the_session_pages_panel() {
    const ID: &str = "6a9fa8e6-0f05-4268-8408-e5f7c82c78b9";
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let app = root.join("repos/api");
    std::fs::create_dir_all(app.join(".git")).unwrap();
    let config = root.join("setup");
    let dir = config
        .join("projects")
        .join(app.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let line = |kind: &str, uuid: &str, parent: &str, at: &str, message: &str| {
        format!(
            r#"{{"parentUuid": {parent}, "isSidechain": false, "userType": "external", "cwd": "{}", "sessionId": "{ID}", "version": "2.1.296", "gitBranch": "main", "uuid": "{uuid}", "timestamp": "2026-11-10T20:{at}.000Z", "type": "{kind}", "message": {message}}}"#,
            app.display()
        )
    };
    let reply = |id: &str, text: &str| {
        format!(
            r#"{{"id": "msg_{id}", "type": "message", "role": "assistant", "model": "claude-sonnet-4-5", "content": [{{"type": "text", "text": "{text}"}}], "stop_reason": "end_turn", "usage": {{"input_tokens": 120, "cache_creation_input_tokens": 300, "cache_read_input_tokens": 1200, "output_tokens": 80, "service_tier": "standard"}}}}"#
        )
    };
    let lines = [
        r#"{"type": "summary", "summary": "Fix the login flow in the api", "leafUuid": "leaf"}"#
            .to_owned(),
        line(
            "user",
            "u1",
            "null",
            "00:00",
            r#"{"role": "user", "content": "Fix the login flow in the api"}"#,
        ),
        line(
            "assistant",
            "a1",
            "\"u1\"",
            "01:00",
            &reply("a1", "The session cookie expires too early."),
        ),
        line(
            "user",
            "u2",
            "\"a1\"",
            "05:00",
            r#"{"role": "user", "content": "Keep going."}"#,
        ),
        line(
            "assistant",
            "a2",
            "\"u2\"",
            "06:00",
            &reply("a2", "Done: the change is in and the tests pass."),
        ),
    ];
    std::fs::write(dir.join(format!("{ID}.jsonl")), lines.join("\n") + "\n").unwrap();
    // A Codex home with one thread (the adapters' paginated fixture).
    let codex = root.join("codex-home");
    let day = codex.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::write(codex.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
    std::fs::write(
        codex.join("auth.json"),
        r#"{"auth_mode":"chatgpt","tokens":{"refresh_token":"fixture"}}"#,
    )
    .unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../juggler-adapters/tests/fixtures/codex/rollout-paginated.jsonl"),
        day.join("rollout-2026-10-20T08-00-00-019a1b2c-0000-7000-8000-00000000c0de.jsonl"),
    )
    .unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    registry.register(Arc::new(
        juggler_adapters::CodexAdapter::with_bin("/bin/true").with_home(&codex),
    ));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let claude = import_in_place(&state, &config, "Work");
    let codex_account = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        "codex",
        codex.to_str().unwrap(),
        "Codex work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            origin: Some("home-pattern".into()),
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    conversations::tick(&state).unwrap();
    let (shown, said) = panel_turns(&state, &claude, ID);
    assert_eq!(shown, "shown");
    for words in [
        "Fix the login flow in the api",
        "The session cookie expires too early.",
        "Done: the change is in and the tests pass.",
    ] {
        assert!(
            said.iter().any(|text| text.contains(words)),
            "{words:?} in {said:?}"
        );
    }
    let (shown, said) = panel_turns(
        &state,
        &codex_account,
        "019a1b2c-0000-7000-8000-00000000c0de",
    );
    assert_eq!(shown, "shown");
    assert!(!said.is_empty(), "the thread's turns");
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
