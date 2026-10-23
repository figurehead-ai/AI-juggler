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
use juggler_app::commands;
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
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/app");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let removed = repo.join(".claude/worktrees/gone"); // a removed worktree: only its transcript is left
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, "c-1", "Fix the login test");
    transcript(&config, &removed, "c-2", "Tidy imports");

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    let account_id = account.id.to_string();
    commands::projects_backfill_impl(&state).unwrap();

    let listed = list_impl(&state).unwrap();
    assert_eq!(listed.len(), 2);
    let project = commands::projects_list_impl(&state)
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
    let first = listed.iter().find(|c| c.id == "c-1").unwrap();
    assert_eq!(first.title.as_deref(), Some("Fix the login test"));

    let line = conversations::resume_command_impl(&state, &account_id, "c-1").unwrap();
    assert!(
        line.contains("'--resume' 'c-1'") && line.contains(&format!("cd '{}'", repo.display())),
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
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
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
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
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
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
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
