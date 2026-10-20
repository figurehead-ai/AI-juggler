//! AJ-E1009 integration — the restore rehearsal: a fixture Mac exports,
//! a second fixture Mac verifies and restores, and the counts and
//! lineage match the manifest. Credentials, Account folders and the
//! Fleet identity never enter the archive (allowlist plus scan); a
//! restored Mac keeps its own identity; damaged, foreign, newer or
//! unconfirmed restores are refused with the file and the reason.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::backup;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, EdgeActor, EdgeState, Epoch, EpochId, HarnessId, InstanceKind,
    InstanceRecord, LaunchRecipe, MachineId, ProfileId, SessionId, SessionStatus, SettingsOverlay,
    SwitchEdge, SwitchEdgeId, SwitchMechanism, Timestamp,
};

/// A fake key, shaped like the real thing so the scanner would catch it.
const SEEDED_KEY: &str = "sk-ant-api03-SEEDEDFIXTUREKEY0000000000000000000000000000";
const SEEDED_LOGIN: &str = "fixture-oauth-refresh-token-never-in-a-backup";

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap())
}

/// A Mac with two Accounts, Launch settings holding a key in an env
/// value, Instances with history, a switch lineage, a parked pack,
/// heuristics, a process ledger row, sign-in material in the Account
/// folder and a Fleet identity.
fn seed(state: &Arc<AppState>, data: &Path) {
    let account_dir = data.join("accounts").join("acc-work");
    std::fs::create_dir_all(&account_dir).unwrap();
    std::fs::write(account_dir.join(".credentials.json"), SEEDED_LOGIN).unwrap();
    std::fs::create_dir_all(data.join("parked").join("s-quota")).unwrap();
    std::fs::write(
        data.join("parked").join("s-quota").join("PACK.md"),
        "# Pack\n\nThe staged context for the parked Instance.\n",
    )
    .unwrap();
    std::fs::write(
        data.join("heuristics.json"),
        r#"{"stuck_no_output_secs":600,"stuck_cpu_max_pct":5.0,"stuck_transcript_stale_secs":300,"waiting_prompt_secs":120}"#,
    )
    .unwrap();
    juggler_app::fleet::this_machine_fingerprint(state).unwrap(); // creates fleet/
    let storage = state.storage.lock().unwrap();
    let project = storage
        .projects()
        .touch(Path::new("/work/storefront"), None)
        .unwrap()
        .id;
    for (id, label) in [("acc-work", "Work Max"), ("acc-home", "Personal")] {
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new(id),
                harness: HarnessId::new("claude-code"),
                label: label.into(),
                plan: None,
                // A fixed fake path: the committed fixture archive carries
                // no path from the machine that blessed it.
                state_dir: PathBuf::from("/fixture/accounts").join(id),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        let mut overlay = SettingsOverlay::default();
        if id == "acc-work" {
            overlay
                .env
                .insert("ANTHROPIC_API_KEY".into(), SEEDED_KEY.into());
        }
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new(format!("rec-{id}")),
                name: "Main".into(),
                account: AccountId::new(id),
                settings_overlay: overlay,
                default_cwd: None,
            })
            .unwrap();
    }
    for (n, status) in [
        SessionStatus::Finished,
        SessionStatus::Running,
        SessionStatus::Failed,
        SessionStatus::Stopped,
    ]
    .into_iter()
    .enumerate()
    {
        let mut record = InstanceRecord {
            id: SessionId::new(format!("s-{n}")),
            conversation: None,
            profile: ProfileId::new(if n % 2 == 0 {
                "rec-acc-work"
            } else {
                "rec-acc-home"
            }),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.clone()),
            cwd: PathBuf::from("/work/storefront"),
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
        record.status = status;
        storage.sessions().update(&record).unwrap();
    }
    let work = storage
        .switch_graph()
        .create_work(&project, "Invoices")
        .unwrap();
    let epoch = |id: &str, account: &str, session: &str| Epoch {
        id: EpochId::new(id),
        work: work.id.clone(),
        session_ref: Some(SessionId::new(session)),
        native_id: Some(format!("native-{id}")),
        account: AccountId::new(account),
        provider: HarnessId::new("claude-code"),
        resumed_from_epoch: None,
        machine: None,
    };
    storage
        .switch_graph()
        .insert_epoch(&epoch("e-a", "acc-work", "s-0"))
        .unwrap();
    storage
        .switch_graph()
        .insert_epoch(&epoch("e-b", "acc-home", "s-1"))
        .unwrap();
    storage
        .switch_graph()
        .append_edge(&SwitchEdge {
            id: SwitchEdgeId::new("edge-1"),
            work: work.id.clone(),
            from_epoch: EpochId::new("e-a"),
            to_epoch: None,
            from_account: AccountId::new("acc-work"),
            to_account: AccountId::new("acc-home"),
            from_provider: HarnessId::new("claude-code"),
            to_provider: HarnessId::new("claude-code"),
            mechanism: SwitchMechanism::Transplant,
            pack_id: None,
            fidelity_shown: Some("F1".into()),
            manifest_sha256: None,
            state: EdgeState::Staged,
            actor: EdgeActor::User,
            parent_edge_a: None,
            parent_edge_b: None,
            created_at: Timestamp::now(),
            to_machine: None,
        })
        .unwrap();
    storage
        .switch_graph()
        .mark_launched(&SwitchEdgeId::new("edge-1"), &EpochId::new("e-b"))
        .unwrap();
    storage
        .raw_conn()
        .execute(
            "INSERT INTO process_ledger (pid, pgid, started_at, exe_path, cmdline_hash, cmdline_trunc,
                 boot_id, session_id, provider, machine_id, depth, role, state, cwd, first_seen, last_seen)
             VALUES (4242, 4242, 1, '/usr/bin/claude', 'h', ?1, 'boot', 's-1', 'claude-code',
                 'this-mac', 0, 'root', 'running', '/work', 1, 1)",
            [format!("claude --api-key {SEEDED_KEY}")],
        )
        .unwrap();
}

/// Rewrites one archive entry: (path, body) → (path, body).
type Edit = dyn Fn(&str, Vec<u8>) -> (String, Vec<u8>);

fn entries(archive: &Path) -> Vec<(String, Vec<u8>)> {
    let file = std::fs::File::open(archive).unwrap();
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.entries()
        .unwrap()
        .map(|entry| {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_string_lossy().into_owned();
            let mut body = Vec::new();
            entry.read_to_end(&mut body).unwrap();
            (name, body)
        })
        .collect()
}

#[test]
fn rehearsal_export_on_one_mac_restore_on_another_counts_and_lineage_match() {
    let work = tempfile::tempdir().unwrap();
    let (mac_a, mac_b) = (work.path().join("mac-a"), work.path().join("mac-b"));
    let a = app_state(&mac_a);
    seed(&a, &mac_a);
    let archive = work.path().join("move.ajbackup");

    let mut updates = Vec::new();
    let started = Instant::now();
    let exported =
        backup::backup_export_impl(&a, &archive, &mut |done, total| updates.push((done, total)))
            .unwrap();
    eprintln!(
        "[perf] export: {:?} for {} bytes",
        started.elapsed(),
        exported.bytes
    );
    assert!(
        updates.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "progress only moves forward"
    );
    assert_eq!(updates.last().map(|(d, t)| d == t), Some(true));

    // Allowlist: nothing else is in the archive.
    let contents = entries(&archive);
    let names: Vec<&str> = contents.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names[0], "manifest.json");
    assert!(names.contains(&"db/juggler.sqlite"));
    assert!(names.contains(&"settings/heuristics.json"));
    assert!(names.contains(&"parked/s-quota/PACK.md"));
    assert!(
        names
            .iter()
            .all(|name| !name.starts_with("fleet") && !name.starts_with("accounts")),
        "{names:?}"
    );
    // No credential, by construction: not the env value, not the login
    // file, not the key in a process command line, not the Fleet key.
    let fleet_key = std::fs::read_dir(mac_a.join("fleet"))
        .unwrap()
        .flatten()
        .map(|entry| std::fs::read(entry.path()).unwrap_or_default())
        .collect::<Vec<_>>();
    for (name, body) in &contents {
        let text = String::from_utf8_lossy(body);
        assert!(!text.contains(SEEDED_KEY), "{name} carries the key");
        assert!(
            !text.contains(SEEDED_LOGIN),
            "{name} carries sign-in material"
        );
        assert!(
            juggler_handover::redaction::list_segments(&[text.as_ref()]).is_empty(),
            "{name} has something the scanner flags"
        );
        for key in &fleet_key {
            if key.len() >= 16 {
                assert!(
                    !body.windows(key.len()).any(|window| window == &key[..]),
                    "{name} carries the Fleet identity"
                );
            }
        }
    }
    let manifest = &exported.manifest;
    assert_eq!(manifest.format_version, backup::FORMAT_VERSION);
    assert!(
        manifest
            .withheld
            .iter()
            .any(|line| line.contains("ANTHROPIC_API_KEY"))
    );
    assert_eq!(manifest.counts["instances"], 4);
    assert_eq!(manifest.counts["switch_edges"], 1);
    assert_eq!(manifest.counts["epochs"], 2);

    // Mac B: its own identity, verifies, restores.
    let b = app_state(&mac_b);
    let b_identity = juggler_app::fleet::this_machine_fingerprint(&b).unwrap();
    let started = Instant::now();
    let plan = backup::backup_verify_impl(&b, &archive).unwrap();
    eprintln!("[perf] verify: {:?}", started.elapsed());
    assert!(plan.other_machine);
    assert!(
        plan.notes[0].contains("never its identity"),
        "{:?}",
        plan.notes
    );
    assert!(
        plan.notes
            .iter()
            .any(|note| note.contains("ANTHROPIC_API_KEY"))
    );
    assert!(backup::backup_restore_impl(&b, &archive, "guess").is_err());
    let plan = backup::backup_verify_impl(&b, &archive).unwrap();
    let restored = backup::backup_restore_impl(&b, &archive, &plan.token).unwrap();
    eprintln!("[rehearsal] manifest counts {:?}", manifest.counts);
    eprintln!("[rehearsal] restored counts {:?}", restored.counts);
    eprintln!("[rehearsal] archive entries {names:?}");
    eprintln!("[rehearsal] withheld {:?}", manifest.withheld);
    assert_eq!(
        restored.counts, manifest.counts,
        "counts match the manifest"
    );
    assert!(
        restored
            .next_steps
            .iter()
            .any(|step| step.contains("Sign in"))
    );

    // Lineage intact; live-on-A is stopped here; withheld stays blank;
    // B keeps its own identity; the before-restore copy exists.
    let storage = b.storage.lock().unwrap();
    let edges = storage.switch_graph().edges_for_session("s-0").unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges[0].0.to_epoch.as_ref().map(EpochId::as_str),
        Some("e-b")
    );
    assert_eq!(
        storage
            .sessions()
            .get(&SessionId::new("s-1"))
            .unwrap()
            .status,
        SessionStatus::Stopped
    );
    let recipe = storage
        .profiles()
        .get(&ProfileId::new("rec-acc-work"))
        .unwrap();
    assert_eq!(recipe.settings_overlay.env["ANTHROPIC_API_KEY"], "");
    let ledger: i64 = storage
        .raw_conn()
        .query_row("SELECT COUNT(*) FROM process_ledger", [], |row| row.get(0))
        .unwrap();
    assert_eq!(ledger, 0, "A's process ids never land on B");
    drop(storage);
    assert_eq!(
        juggler_app::fleet::this_machine_fingerprint(&b).unwrap(),
        b_identity
    );
    assert!(
        restored
            .kept_before
            .iter()
            .any(|path| Path::new(path).exists())
    );
    assert!(
        mac_b
            .join("parked")
            .join("s-quota")
            .join("PACK.md")
            .exists()
    );
    assert!(
        !mac_b
            .join("accounts")
            .join("acc-work")
            .join(".credentials.json")
            .exists()
    );
}

#[test]
fn damaged_newer_and_unsafe_archives_are_refused_with_the_reason() {
    let work = tempfile::tempdir().unwrap();
    let mac_a = work.path().join("mac-a");
    let a = app_state(&mac_a);
    seed(&a, &mac_a);
    let archive = work.path().join("a.ajbackup");
    backup::backup_export_impl(&a, &archive, &mut |_, _| {}).unwrap();
    assert!(
        backup::backup_export_impl(&a, &archive, &mut |_, _| {}).is_err(),
        "never overwrites a file"
    );
    let b = app_state(&work.path().join("mac-b"));

    let rewrite = |name: &str, edit: &Edit| {
        let out = work.path().join(name);
        let file = std::fs::File::create(&out).unwrap();
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::fast(),
        ));
        for (path, body) in entries(&archive) {
            let (path, body) = edit(&path, body);
            // Raw header bytes: the tar crate refuses to *write* `..`,
            // but a hostile archive can carry it.
            let mut header = tar::Header::new_old();
            header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
            header.set_size(body.len() as u64);
            header.set_mode(0o600);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder.append(&header, &body[..]).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
        out
    };
    let damaged = rewrite("damaged.ajbackup", &|path, mut body| {
        if path == "settings/heuristics.json" {
            body[2] ^= 1;
        }
        (path.to_owned(), body)
    });
    let refused = backup::backup_verify_impl(&b, &damaged)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("damaged.ajbackup")
            && refused.contains("heuristics.json")
            && refused.contains("checksum"),
        "{refused}"
    );

    let newer = rewrite("newer.ajbackup", &|path, body| {
        if path == "manifest.json" {
            let mut manifest: serde_json::Value = serde_json::from_slice(&body).unwrap();
            manifest["schema_version"] = serde_json::json!(9_999);
            return (path.to_owned(), serde_json::to_vec(&manifest).unwrap());
        }
        (path.to_owned(), body)
    });
    let refused = backup::backup_verify_impl(&b, &newer)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("newer AI Juggler"), "{refused}");

    let escaping = rewrite("escape.ajbackup", &|path, body| {
        if path == "settings/heuristics.json" {
            return ("settings/../../outside.txt".to_owned(), body);
        }
        (path.to_owned(), body)
    });
    assert!(backup::backup_verify_impl(&b, &escaping).is_err());
    assert!(
        !work.path().join("outside.txt").exists(),
        "nothing written outside staging"
    );

    // A live Instance blocks the restore.
    let plan = backup::backup_verify_impl(&b, &archive).unwrap();
    {
        let storage = b.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-b"),
                harness: HarnessId::new("claude-code"),
                label: "B".into(),
                plan: None,
                state_dir: work.path().join("b-state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-b"),
                name: "Main".into(),
                account: AccountId::new("acc-b"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new("b-live"),
                conversation: None,
                profile: ProfileId::new("rec-b"),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: work.path().to_path_buf(),
                status: SessionStatus::Running,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
    }
    let refused = backup::backup_restore_impl(&b, &archive, &plan.token)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("stop every running session"), "{refused}");
}

#[test]
fn something_that_looks_like_a_secret_stops_the_export() {
    let work = tempfile::tempdir().unwrap();
    let mac_a = work.path().join("mac-a");
    let a = app_state(&mac_a);
    seed(&a, &mac_a);
    // A key somewhere the scrub does not know about: a project name.
    a.storage
        .lock()
        .unwrap()
        .projects()
        .touch(Path::new(&format!("/work/{SEEDED_KEY}")), None)
        .unwrap();
    let archive = work.path().join("stopped.ajbackup");
    let refused = backup::backup_export_impl(&a, &archive, &mut |_, _| {})
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("looks like a secret") && refused.contains("db/juggler.sqlite"),
        "{refused}"
    );
    assert!(!refused.contains(SEEDED_KEY), "the value is masked");
    assert!(!archive.exists(), "nothing was written");
}

#[test]
fn export_throughput_keeps_a_gigabyte_inside_a_minute() {
    let work = tempfile::tempdir().unwrap();
    let mac_a = work.path().join("mac-a");
    let a = app_state(&mac_a);
    seed(&a, &mac_a);
    // 64 MB of pack data (mixed text, the shape of real packs).
    let line = "turn 0042 · assistant: refactored the invoice exporter and added tests for the retry path\n";
    let body: String = line.repeat(64 * 1024 * 1024 / line.len());
    std::fs::write(
        mac_a.join("parked").join("s-quota").join("transcript.md"),
        &body,
    )
    .unwrap();
    let archive = work.path().join("big.ajbackup");
    let started = Instant::now();
    backup::backup_export_impl(&a, &archive, &mut |_, _| {}).unwrap();
    let export = started.elapsed();
    let b = app_state(&work.path().join("mac-b"));
    let started = Instant::now();
    backup::backup_verify_impl(&b, &archive).unwrap();
    let verify = started.elapsed();
    let per_gb = |took: Duration| took.as_secs_f64() * 1024.0 / 64.0;
    eprintln!(
        "[perf] 64 MB: export {export:?} (≈{:.1} s per GB), verify {verify:?} (≈{:.1} s per GB)",
        per_gb(export),
        per_gb(verify)
    );
    assert!(
        per_gb(export) < 60.0,
        "export ≈{:.1} s per GB",
        per_gb(export)
    );
    assert!(
        per_gb(verify) < 30.0,
        "verify ≈{:.1} s per GB",
        per_gb(verify)
    );
}

/// The committed v1 archive (`tests/fixtures/backup-v1.ajbackup`) keeps
/// restoring on every later build: format 1 stays readable, counts and
/// lineage match its manifest. `AIJ_BLESS=1` rewrites it (origin set to
/// fixture values, so it carries no host name or fingerprint).
#[test]
fn the_committed_v1_fixture_archive_restores() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/backup-v1.ajbackup");
    let work = tempfile::tempdir().unwrap();
    if std::env::var("AIJ_BLESS").is_ok() {
        let mac_a = work.path().join("mac-a");
        let a = app_state(&mac_a);
        seed(&a, &mac_a);
        let raw = work.path().join("raw.ajbackup");
        backup::backup_export_impl(&a, &raw, &mut |_, _| {}).unwrap();
        std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
        let file = std::fs::File::create(&fixture).unwrap();
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::best(),
        ));
        for (path, body) in entries(&raw) {
            let body = if path == "manifest.json" {
                let mut manifest: serde_json::Value = serde_json::from_slice(&body).unwrap();
                manifest["origin"] =
                    serde_json::json!({"fingerprint": "fixture-mac", "label": "Fixture Mac"});
                manifest["created_at_ms"] = serde_json::json!(1_792_000_000_000_i64);
                serde_json::to_vec_pretty(&manifest).unwrap()
            } else {
                body
            };
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o600);
            header.set_mtime(0);
            header.set_cksum();
            builder.append_data(&mut header, &path, &body[..]).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
    }
    let archive = work.path().join("backup-v1.ajbackup");
    std::fs::copy(&fixture, &archive)
        .expect("tests/fixtures/backup-v1.ajbackup (AIJ_BLESS=1 writes it)");
    let mac = work.path().join("restore-here");
    let state = app_state(&mac);
    let plan = backup::backup_verify_impl(&state, &archive).unwrap();
    assert_eq!(plan.manifest.format_version, 1);
    assert_eq!(plan.manifest.origin.label, "Fixture Mac");
    assert!(plan.other_machine);
    let restored = backup::backup_restore_impl(&state, &archive, &plan.token).unwrap();
    assert_eq!(restored.counts, plan.manifest.counts);
    let storage = state.storage.lock().unwrap();
    assert_eq!(
        storage
            .switch_graph()
            .edges_for_session("s-0")
            .unwrap()
            .len(),
        1
    );
}
