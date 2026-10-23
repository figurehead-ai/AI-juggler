#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! AJ-E1517's gate: the scrub corpus run over the zip. Secrets, this
//! Mac's home folder and user name, links and tokens are seeded into the
//! settings, the logs and a crash report; the zip is read back and none
//! of them is in it.

use super::*;

/// One preview at a time: the prepared preview is the app's one.
static SERIAL: Mutex<()> = Mutex::new(());

const SK: &str = "sk-ant-api03-Zq7XwVb2LmN4pR8sT1uY6cD9fG3hJ5kQ";
const ENV: &str = "ANTHROPIC_API_KEY=q8Vt3LmZx9RwP2sN";
const BEARER: &str = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.c2VjcmV0.dG9rZW4";
const LINK: &str = "https://example.com/callback?code=one-time-77";

fn home() -> String {
    dirs::home_dir().unwrap().display().to_string()
}

fn user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
}

/// An app whose data folder holds every kind of thing diagnostics reads,
/// each with something that must not leave.
fn seeded() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    let state = Arc::new(
        AppState::init_with_registry(data.clone(), juggler_adapter::AdapterRegistry::new())
            .unwrap(),
    );
    let home = home();
    let user = user();
    {
        let storage = state.storage.lock().unwrap();
        let settings = storage.app_settings();
        settings.set("theme", "dark").unwrap();
        settings
            .set(
                "projects.default_folder",
                &format!("{home}/Dev/acme-secret-project"),
            )
            .unwrap();
        settings
            .set("terminal_note", &format!("ran with {ENV} and {SK}"))
            .unwrap();
        settings
            .set("restore_token", "rt-never-leaves-0042")
            .unwrap();
        settings
            .set("figurehead_device_id", "fh-device-never-leaves")
            .unwrap();
        settings.set("fleet_peer_addr", "10.0.0.42:7766").unwrap();
        settings.set(&format!("credits:{user}"), "12").unwrap();
    }
    let write = |path: &str, text: String| {
        let path = data.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "supervision/supervision.jsonl",
        format!(
            "{{\"event\":\"spawn\",\"program\":\"{home}/.local/bin/claude\",\"cwd\":\"{home}/Dev/acme\"}}\n\
             {{\"event\":\"exit\",\"note\":\"{BEARER}\"}}\n\
             {{\"event\":\"spawn\",\"user\":\"{user}\",\"link\":\"{LINK}\"}}\n"
        ),
    );
    write("stats/upload.log", format!("sent 3 events {LINK}\n"));
    write("last-run", "clean".to_owned());
    write(
        "crashes/queued/abc123.json",
        format!("{{\"message\":\"panicked at {home}/Dev/x.rs with {SK}\"}}"),
    );
    // Never read: an Account's folder and the terminal's output.
    write(
        "accounts/acc-1/.credentials.json",
        format!("{{\"token\":\"{SK}\"}}"),
    );
    write(
        "terminal/sess-1.log",
        "terminal output never leaves".to_owned(),
    );
    (dir, state)
}

fn canaries() -> Vec<String> {
    let mut all = vec![
        SK.to_owned(),
        "q8Vt3LmZx9RwP2sN".to_owned(),
        "eyJhbGciOiJIUzI1NiJ9".to_owned(),
        "example.com".to_owned(),
        "one-time-77".to_owned(),
        "rt-never-leaves-0042".to_owned(),
        "fh-device-never-leaves".to_owned(),
        "10.0.0.42".to_owned(),
        "terminal output never leaves".to_owned(),
        home(),
    ];
    if user().len() >= 3 {
        all.push(user());
    }
    all
}

#[test]
fn the_zip_holds_what_was_shown_and_nothing_it_shouldnt() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (dir, state) = seeded();
    let began = std::time::Instant::now();
    let preview = preview_impl(&state).unwrap();
    let names: Vec<&str> = preview.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "about.json",
            "app-settings.json",
            "logs/supervision.jsonl",
            "logs/usage-stats-upload.log",
            "logs/last-run",
            "crashes/abc123.json",
        ]
    );
    for key in ["restore_token", "figurehead_device_id", "fleet_peer_addr"] {
        assert!(
            preview.left_out.iter().any(|left| left == key),
            "{key} left out"
        );
    }
    assert!(
        preview.file_name.starts_with("AI Juggler diagnostics ")
            && preview.file_name.ends_with(".zip")
    );

    let dest = dir.path().join("diagnostics.zip");
    let saved = export_impl(&preview.id, &dest).unwrap();
    // Performance budget: under 3 s, shown and saved.
    assert!(
        began.elapsed() < std::time::Duration::from_secs(3),
        "{:?}",
        began.elapsed()
    );
    let bytes = std::fs::read(&dest).unwrap();
    assert_eq!(saved.bytes, bytes.len() as u64);
    let files = zip::read(&bytes);
    // Exactly what was shown.
    assert_eq!(files.len(), preview.files.len());
    for ((name, data), shown) in files.iter().zip(&preview.files) {
        assert_eq!(*name, shown.name);
        assert_eq!(String::from_utf8_lossy(data), shown.text);
    }
    // The scrub corpus: nothing seeded survives anywhere in the zip.
    for (name, data) in &files {
        let text = String::from_utf8_lossy(data);
        for canary in canaries() {
            assert!(
                !text.contains(&canary),
                "{name} still holds {canary:?}:\n{text}"
            );
        }
        assert!(
            juggler_handover::redaction::list_segments(&[text.as_ref()]).is_empty(),
            "{name} still looks like it holds a secret"
        );
    }
    // Home paths read as `~`; the settings that stay are there.
    let log = String::from_utf8_lossy(&files[2].1);
    assert!(log.contains("~/.local/bin/claude"), "{log}");
    let settings: serde_json::Value = serde_json::from_slice(&files[1].1).unwrap();
    assert_eq!(settings["settings"]["theme"], "dark");
    assert_eq!(
        settings["settings"]["projects.default_folder"],
        "~/Dev/acme-secret-project"
    );
    let about: serde_json::Value = serde_json::from_slice(&files[0].1).unwrap();
    assert_eq!(
        about["database"]["schema_on_disk"],
        juggler_core::storage::Storage::schema_version()
    );
    assert_eq!(about["counts"]["sessions"], 0);
    // It never opened an Account's folder or the terminal's output.
    let reads = state.export_reads.lock().unwrap().clone();
    for read in &reads {
        assert!(
            !read.starts_with(state.data_dir.join("accounts")),
            "opened {}",
            read.display()
        );
        assert!(
            !read.starts_with(state.data_dir.join("terminal")),
            "opened {}",
            read.display()
        );
    }
}

#[test]
fn it_saves_only_a_zip_only_the_preview_shown_and_never_a_leak() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (dir, state) = seeded();
    let preview = preview_impl(&state).unwrap();
    let refused = |result: AppResult<DiagnosticsSavedInfo>| match result {
        Err(AppError::Validation { message, .. }) => message,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert!(refused(export_impl(&preview.id, &dir.path().join("d.txt"))).contains(".zip"));
    assert!(
        refused(export_impl("not-the-preview", &dir.path().join("d.zip")))
            .contains("show it again")
    );

    // A file that still holds a secret (as if a scrub missed it): nothing is written.
    if let Ok(mut prepared) = PREPARED.lock() {
        let mut leaky = preview.clone();
        leaky.files[0].text = format!("missed: {SK}");
        *prepared = Some(leaky);
    }
    let dest = dir.path().join("leaky.zip");
    let message = refused(export_impl(&preview.id, &dest));
    assert!(
        message.contains("about.json") && message.contains("nothing was saved"),
        "{message}"
    );
    assert!(!message.contains(SK));
    assert!(!dest.exists() && !dir.path().join("leaky.zip.partial").exists());
}

#[test]
fn a_long_log_comes_as_its_newest_whole_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.log");
    let lines: String = (0..40_000).map(|n| format!("line {n:06}\n")).collect();
    std::fs::write(&path, &lines).unwrap();
    let kept = tail(&path).unwrap();
    assert!(kept.len() <= LOG_TAIL);
    assert!(kept.starts_with("line "), "{}", &kept[..20]);
    assert!(kept.ends_with("line 039999\n"));
}
