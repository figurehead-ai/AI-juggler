//! AJ-E1302: Codex scan + import against fixture homes. The source is
//! hashed file by file before and after every import — byte-identical,
//! never marked. Every `auth.json` is synthetic.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::HarnessAdapter;
use juggler_adapter::environment::{InstallOrigin, LoginProbe};
use juggler_adapters::CodexAdapter;
use juggler_adapters::codex_environment::{
    import_codex_home, import_codex_settings_only, inspect_codex_source, scan_codex_environment,
};

fn logged_in(_: &Path) -> LoginProbe {
    LoginProbe::LoggedIn
}

/// The real probe (field shape of auth.json), for carry-over checks.
fn real_probe(path: &Path) -> LoginProbe {
    match juggler_adapters::codex::codex_login_state(path) {
        juggler_adapters::LoginState::SignedIn => LoginProbe::LoggedIn,
        _ => LoginProbe::LoggedOut,
    }
}

const ROLLOUT: &str = r#"{"timestamp":"2026-10-20T08:00:00.000Z","type":"session_meta","payload":{"id":"019a0000-0000-7000-8000-000000000001","timestamp":"2026-10-20T08:00:00.000Z","cwd":"/work/app","originator":"codex_cli_rs","cli_version":"0.163.0","history_mode":"paginated"}}
"#;

/// A Codex home as 0.163.0 leaves it: config, sign-in, one session,
/// history, plus everything rebuildable that must stay behind.
fn codex_home(root: &Path, name: &str) -> PathBuf {
    let home = root.join(name);
    let day = home.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::write(home.join("config.toml"), "model = \"gpt-5-codex\"\n").unwrap();
    std::fs::write(
        home.join("auth.json"),
        r#"{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"refresh_token":"fixture-refresh","account_id":"acct-fixture"}}"#,
    )
    .unwrap();
    std::fs::write(
        day.join("rollout-2026-10-20T08-00-00-019a0000-0000-7000-8000-000000000001.jsonl"),
        ROLLOUT,
    )
    .unwrap();
    std::fs::write(home.join("history.jsonl"), "{\"text\":\"hi\"}\n").unwrap();
    std::fs::write(home.join("session_index.jsonl"), "{}\n").unwrap();
    std::fs::write(home.join("version.json"), "{}").unwrap();
    // Rebuildable / dangerous to copy:
    std::fs::write(home.join("state_5.sqlite"), "sqlite bytes").unwrap();
    std::fs::write(home.join("state_5.sqlite-wal"), "wal").unwrap();
    std::fs::write(home.join("logs_2.sqlite"), "logs").unwrap();
    std::fs::write(home.join("installation_id"), "per-install").unwrap();
    for dir in [
        ".tmp",
        "log",
        "app-server-daemon",
        "packages/app-server-daemon",
    ] {
        std::fs::create_dir_all(home.join(dir)).unwrap();
        std::fs::write(home.join(dir).join("x"), "noise").unwrap();
    }
    home
}

/// sha256 of every file under `root`, keyed by relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    juggler_adapter::handover::sha256_hex(&bytes),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn files(root: &Path) -> Vec<String> {
    snapshot(root)
        .keys()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn scan_finds_default_override_and_home_patterns() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    codex_home(&home, ".codex");
    codex_home(&home, ".codex-work");
    let elsewhere = codex_home(tmp.path(), "custom-codex");
    // Not a Codex home: no evidence inside.
    std::fs::create_dir_all(home.join(".codex-empty")).unwrap();

    let found = scan_codex_environment(&home, Some(&elsewhere), &logged_in);
    let origins: Vec<(String, InstallOrigin)> = found
        .iter()
        .map(|f| {
            (
                f.path.file_name().unwrap().to_string_lossy().into_owned(),
                f.origin,
            )
        })
        .collect();
    assert_eq!(origins.len(), 3, "{origins:?}");
    assert_eq!(origins[0], (".codex".into(), InstallOrigin::VendorDefault));
    assert!(origins.contains(&("custom-codex".into(), InstallOrigin::EnvOverride)));
    assert!(origins.contains(&(".codex-work".into(), InstallOrigin::HomePattern)));
    assert!(found.iter().all(|f| f.harness.as_str() == "codex"));
    // The size estimate leaves the databases and noise out.
    // Exactly the importable files: config, the rollout, history, the
    // index and version — no sign-in, no databases, no noise.
    let expected = "model = \"gpt-5-codex\"\n".len()
        + ROLLOUT.len()
        + "{\"text\":\"hi\"}\n".len()
        + "{}\n".len()
        + "{}".len();
    assert_eq!(found[0].bytes, expected as u64);
}

#[test]
fn import_copies_config_and_history_and_leaves_the_source_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex");
    let before = snapshot(&source);
    let dest = tmp.path().join("managed/codex/acct");

    let outcome = import_codex_home(&source, &dest, &real_probe).unwrap();

    assert_eq!(
        snapshot(&source),
        before,
        "the source must be byte-identical"
    );
    assert!(!source.join(".ai-juggler-managed").exists(), "never marked");
    let copied = files(&dest);
    for kept in [
        "config.toml",
        "history.jsonl",
        "session_index.jsonl",
        "version.json",
        "sessions/2026/10/20/rollout-2026-10-20T08-00-00-019a0000-0000-7000-8000-000000000001.jsonl",
        ".ai-juggler-managed",
    ] {
        assert!(
            copied.contains(&kept.to_string()),
            "{kept} missing: {copied:?}"
        );
    }
    for dropped in [
        "auth.json",
        "installation_id",
        "state_5.sqlite",
        "state_5.sqlite-wal",
        "logs_2.sqlite",
    ] {
        assert!(!copied.contains(&dropped.to_string()), "{dropped} copied");
    }
    assert!(copied.iter().all(|f| !f.starts_with(".tmp")
        && !f.starts_with("log/")
        && !f.starts_with("app-server-daemon")
        && !f.starts_with("packages")));
    assert!(
        outcome.skipped.iter().any(|s| s.contains("state_5.sqlite")),
        "{:?}",
        outcome.skipped
    );
    // Copied bytes are identical to the source's.
    let after = snapshot(&dest);
    assert_eq!(
        after[Path::new("config.toml")],
        before[Path::new("config.toml")]
    );
    // The sign-in stays with the original (a ChatGPT refresh token is
    // single-use: two homes sharing one sign each other out) — listed,
    // and the copy signs in once.
    assert!(
        outcome
            .skipped
            .iter()
            .any(|s| s.starts_with("auth.json (your sign-in")),
        "{:?}",
        outcome.skipped
    );
    assert_eq!(outcome.login, LoginProbe::LoggedOut);
    let note = outcome.login_note.unwrap();
    assert!(note.contains("stays with the original"), "{note}");
    assert!(note.contains("Sign in once"), "{note}");
}

#[test]
fn settings_only_import_leaves_history_behind_and_lists_it() {
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex");
    let before = snapshot(&source);
    let dest = tmp.path().join("managed/codex/light");

    let outcome = import_codex_settings_only(&source, &dest, &real_probe).unwrap();

    assert_eq!(snapshot(&source), before);
    assert!(outcome.degraded);
    let copied = files(&dest);
    assert!(copied.contains(&"config.toml".to_string()));
    assert!(!copied.contains(&"auth.json".to_string()));
    assert!(copied.iter().all(|f| !f.starts_with("sessions")));
    assert!(!copied.contains(&"history.jsonl".to_string()));
    assert!(!copied.contains(&"session_index.jsonl".to_string()));
    for name in ["sessions/", "history.jsonl", "session_index.jsonl"] {
        assert!(
            outcome.skipped.iter().any(|s| s.starts_with(name)),
            "{name} not listed: {:?}",
            outcome.skipped
        );
    }
}

#[test]
fn import_refuses_what_it_must_not_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let not_codex = tmp.path().join("random");
    std::fs::create_dir_all(&not_codex).unwrap();
    std::fs::write(not_codex.join("notes.txt"), "x").unwrap();
    let err = import_codex_home(&not_codex, &tmp.path().join("d1"), &logged_in).unwrap_err();
    assert!(
        err.to_string().contains("Codex home"),
        "the refusal says it is not a Codex home"
    );

    // An existing destination is never overwritten.
    let source = codex_home(tmp.path(), ".codex");
    let taken = tmp.path().join("taken");
    std::fs::create_dir_all(&taken).unwrap();
    assert!(import_codex_home(&source, &taken, &logged_in).is_err());

    // An AI Juggler-managed home is launched, not re-imported.
    let managed = tmp.path().join("managed-home");
    CodexAdapter::new().prepare_account(&managed).unwrap();
    std::fs::write(managed.join("config.toml"), "").unwrap();
    assert!(import_codex_home(&managed, &tmp.path().join("d2"), &logged_in).is_err());
}

#[test]
fn inspect_reports_sizes_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex");
    let before = snapshot(&source);
    let inspect = inspect_codex_source(&source, &logged_in).unwrap();
    assert_eq!(snapshot(&source), before);
    assert!(inspect.is_config);
    assert!(!inspect.already_managed);
    let names: Vec<&str> = inspect.folders.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"sessions/"), "{names:?}");
    assert!(!names.iter().any(|n| n.contains("sqlite")), "{names:?}");
}

#[test]
fn the_adapter_routes_scan_and_import_through_codex() {
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex-side");
    let dest = tmp.path().join("acct");
    let adapter = CodexAdapter::new();
    let outcome = adapter.import_account(&source, &dest).unwrap();
    assert!(outcome.files_copied >= 5);
    assert!(adapter.inspect_source(&source).unwrap().is_config);
    let light = adapter
        .import_account_degraded(&source, &tmp.path().join("acct2"))
        .unwrap();
    assert!(light.degraded);
}

#[cfg(unix)]
#[test]
fn links_follow_the_shared_rule_and_the_sign_in_stays_back() {
    // #73's rule, shared with Claude: a linked folder is kept as a link
    // (never followed or copied through), a linked file is copied by
    // content, a broken link is left out with a note. Codex's sign-in
    // stays with the original even when it is a link.
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex");
    std::fs::remove_file(source.join("auth.json")).unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(outside.join("dir")).unwrap();
    std::fs::write(outside.join("dir/x"), "outside dir content").unwrap();
    std::fs::write(outside.join("notes.md"), "my notes").unwrap();
    std::fs::write(
        outside.join("auth.json"),
        r#"{"tokens":{"refresh_token":"fixture"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(source.join("skills/real")).unwrap();
    std::fs::write(source.join("skills/real/SKILL.md"), "a skill").unwrap();
    symlink("real", source.join("skills/inside-link")).unwrap();
    symlink(outside.join("dir"), source.join("skills/outside-dir")).unwrap();
    symlink(outside.join("notes.md"), source.join("AGENTS.md")).unwrap();
    symlink(outside.join("auth.json"), source.join("auth.json")).unwrap();
    symlink(tmp.path().join("nowhere"), source.join("skills/dangling")).unwrap();
    let before = snapshot_no_links(&source);
    let dest = tmp.path().join("acct");

    let outcome = import_codex_home(&source, &dest, &logged_in).unwrap();

    let is_link = |rel: &str| {
        std::fs::symlink_metadata(dest.join(rel)).is_ok_and(|meta| meta.file_type().is_symlink())
    };
    assert!(
        is_link("skills/inside-link"),
        "a linked folder stays a link"
    );
    assert_eq!(
        std::fs::read_link(dest.join("skills/inside-link")).unwrap(),
        Path::new("real"),
        "a relative link inside keeps pointing inside the copy"
    );
    assert!(is_link("skills/outside-dir"));
    assert_eq!(
        std::fs::canonicalize(dest.join("skills/outside-dir")).unwrap(),
        std::fs::canonicalize(outside.join("dir")).unwrap()
    );
    // A linked file: a plain copy, not shared.
    assert!(!is_link("AGENTS.md"));
    assert_eq!(
        std::fs::read_to_string(dest.join("AGENTS.md")).unwrap(),
        "my notes"
    );
    // The sign-in never travels, linked or not.
    assert!(std::fs::symlink_metadata(dest.join("auth.json")).is_err());
    assert!(
        outcome
            .skipped
            .iter()
            .any(|s| s.starts_with("auth.json (your sign-in"))
    );
    // A broken link is left out, with a note; the import succeeds.
    assert!(std::fs::symlink_metadata(dest.join("skills/dangling")).is_err());
    assert!(
        outcome
            .skipped
            .iter()
            .any(|s| s.contains("skills/dangling (a broken link"))
    );
    assert_eq!(
        snapshot_no_links(&source),
        before,
        "the source is untouched"
    );
}

#[test]
fn skip_names_apply_to_the_top_level_only() {
    let tmp = tempfile::tempdir().unwrap();
    let source = codex_home(tmp.path(), ".codex");
    // The user's own content that happens to share a skipped name.
    for nested in [
        "skills/x/packages/p.json",
        "skills/x/log/notes.md",
        "skills/x/tmp/t.txt",
    ] {
        let path = source.join(nested);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "user content").unwrap();
    }
    std::fs::write(source.join("skills/x/data.sqlite"), "a skill's own db").unwrap();
    let dest = tmp.path().join("acct");
    import_codex_home(&source, &dest, &logged_in).unwrap();
    let copied = files(&dest);
    for nested in [
        "skills/x/packages/p.json",
        "skills/x/log/notes.md",
        "skills/x/tmp/t.txt",
        "skills/x/data.sqlite",
    ] {
        assert!(copied.contains(&nested.to_string()), "{nested} dropped");
    }
    assert!(
        !copied
            .iter()
            .any(|f| f.starts_with("packages/") || f.starts_with("log/"))
    );
}

/// [`snapshot`] that skips links (they cannot be hashed by content).
fn snapshot_no_links(root: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            if kind.is_symlink() {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    format!("link→{}", std::fs::read_link(&path).unwrap().display()),
                );
            } else if kind.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    juggler_adapter::handover::sha256_hex(&std::fs::read(&path).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
