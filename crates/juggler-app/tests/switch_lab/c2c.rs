//! D-127 in the live lab (`AIJ_LAB_SUITE=c2c`, `just switch-lab-c2c`):
//! Claude → Claude, Copy and Move, every area checked without a model call
//! on the target.
//!
//! A switch itself spends no tokens (the maintainer, 2026-11-18): it copies
//! or moves records. So the target (B, which may be out of quota) is never
//! asked anything. Only the source (A) spends: a few headless turns, with
//! file checkpoints on (`AIJ_LAB_FILE_HISTORY=1`, as the Claude app's own
//! sessions keep them), that edit files, keep a task list, use a subagent
//! and, with `AIJ_LAB_PROBE`, call the probe MCP server. Then:
//!
//! 1. Copy A → B (the switch): on disk at B, under the id it resumes, the
//!    transcript (every line carries that id; the folder is the project),
//!    its subagents, its task list, its file history (every checkpoint
//!    byte-identical, and every backup the transcript's snapshots name is
//!    there: what `/rewind` reads), the probe server set up on B (`claude
//!    mcp get`, which starts no turn), and the launch: `claude --resume
//!    <id>` in the project. A keeps all of it (a copy).
//! 2. Switch back to A: A's conversation is resumed with what happened
//!    away, and A (the target of this hop, with quota) answers the
//!    codename only the conversation holds; its file history is whole.
//! 3. Move A → B (in place), then Undo: the transcript and the file
//!    history are at B under the same id, byte-identical and gone from A;
//!    Undo puts every byte back on A and leaves none on B.
//!
//! Every set-up is undone and the probe removed from A at the end. The
//! report (real paths: keep it out of the repository) lists every step;
//! the lab's own copies on B and A's lab session stay for the run's
//! cleanup, which names them by the ids in the report.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a test's helpers print what they measured"
)]

use super::*;
use juggler_adapter::moving::NotesChoice;
use juggler_app::features::handover::moving;

/// The transcript of conversation `id` under a Claude config folder (any
/// project folder).
fn transcript_in(config: &Path, id: &str) -> Option<PathBuf> {
    std::fs::read_dir(config.join("projects"))
        .ok()?
        .flatten()
        .map(|dir| dir.path().join(format!("{id}.jsonl")))
        .find(|path| path.is_file())
}

/// Every file under `dir`, by its path inside it, with its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if !entry.file_name().to_string_lossy().starts_with('.') {
                let name = path.strip_prefix(dir).unwrap().display().to_string();
                out.insert(name, std::fs::read(&path).unwrap_or_default());
            }
        }
    }
    out
}

/// `got` holds every file of `want`, byte for byte: what's missing or
/// differs, by name.
fn same_files(want: &BTreeMap<String, Vec<u8>>, got: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    want.iter()
        .filter(|(name, bytes)| got.get(*name) != Some(bytes))
        .map(|(name, _)| name.clone())
        .collect()
}

/// The transcript's lines, parsed (a torn line is skipped).
fn lines(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The backups the transcript's file-history snapshots name
/// (`snapshot.trackedFileBackups.*.backupFileName`), each once.
fn named_backups(lines: &[Value]) -> Vec<String> {
    let mut names: Vec<String> = lines
        .iter()
        .filter(|line| line["type"] == "file-history-snapshot")
        .filter_map(|line| line["snapshot"]["trackedFileBackups"].as_object())
        .flat_map(|backups| backups.values())
        .filter_map(|backup| backup["backupFileName"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// What a side holds of conversation `id`: its transcript, subagents,
/// task list and file history.
fn holds(config: &Path, id: &str) -> Value {
    let transcript = transcript_in(config, id);
    let parsed = transcript.as_deref().map(lines).unwrap_or_default();
    let history = tree(&config.join("file-history").join(id));
    let named = named_backups(&parsed);
    let unresolved: Vec<&String> = named
        .iter()
        .filter(|name| !history.contains_key(*name))
        .collect();
    json!({
        "transcript": transcript.as_ref().map(|p| p.display().to_string()),
        "lines": parsed.len(),
        "ids_other_than_this": parsed.iter().filter(|l| l["sessionId"].as_str().is_some_and(|s| s != id)).count(),
        "cwds": parsed.iter().filter_map(|l| l["cwd"].as_str()).collect::<std::collections::BTreeSet<_>>(),
        "subagents": files_under(&transcript.as_ref().map(|p| p.with_extension("").join("subagents")).unwrap_or_default()),
        "tasks": files_under(&config.join("tasks").join(id)),
        "file_history": history.len(),
        "snapshot_backups_named": named.len(),
        "snapshot_backups_missing": unresolved,
    })
}

/// The probe server as `config` has it, read by Claude Code's own `mcp
/// get` (it starts no turn).
fn mcp_get(config: &Path, cwd: &Path, name: &str) -> Value {
    let (ok, out, err) = run(
        Command::new("claude")
            .current_dir(cwd)
            .env("CLAUDE_CONFIG_DIR", config)
            .env("DISABLE_AUTOUPDATER", "1")
            .args(["mcp", "get", name]),
        Duration::from_secs(60),
    );
    json!({"ok": ok, "names_server": out.contains("server.py"), "said": out.lines().take(6).collect::<Vec<_>>(), "error": err.lines().take(3).collect::<Vec<_>>()})
}

pub fn journey() {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let (Some(data), Some(project), Some(claude_a), Some(claude_b), Some(out)) = (
        var("AIJ_LAB_DATA"),
        var("AIJ_LAB_PROJECT"),
        var("AIJ_LAB_CLAUDE_A"),
        var("AIJ_LAB_CLAUDE_B"),
        var("AIJ_LAB_OUT"),
    ) else {
        eprintln!("AIJ_LAB_* not set: nothing to do");
        return;
    };
    if std::env::var("AIJ_LAB_SUITE").as_deref() != Ok("c2c") {
        eprintln!("AIJ_LAB_SUITE is not c2c: nothing to do");
        return;
    }
    assert!(
        std::env::var_os("AIJ_LAB_FILE_HISTORY").is_some(),
        "the c2c suite proves file history: run it with AIJ_LAB_FILE_HISTORY=1"
    );
    assert!(
        std::env::var_os("CLAUDE_CONFIG_DIR").is_none() && std::env::var_os("CODEX_HOME").is_none(),
        "run the lab with CLAUDE_CONFIG_DIR and CODEX_HOME unset (env -u …)"
    );
    assert_clean_env();
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_LAB_DATA must be a fresh, empty folder"
    );
    assert!(
        !project.exists(),
        "AIJ_LAB_PROJECT must not exist yet: the lab creates it"
    );
    new_project(&project);
    let project = project.canonicalize().unwrap();

    let home = dirs::home_dir().unwrap();
    let roots: Vec<PathBuf> = vec![
        claude_a.clone(),
        claude_b.clone(),
        home.join(".claude.json"),
        home.join(".agents"),
    ];
    let before = snapshot(&roots);
    let state = Arc::new(AppState::init(data).unwrap());
    let mut lab = Lab {
        project: project.clone(),
        claude_a: claude_a.clone(),
        claude_b: claude_b.clone(),
        codex: PathBuf::new(),
        codex_b: None,
        steps: Vec::new(),
        probe: std::env::var("AIJ_LAB_PROBE").ok(),
        records: Vec::new(),
    };
    let probe_root = project.with_extension("probe");
    let planted = lab.probe.clone().map(|name| {
        std::fs::create_dir_all(probe_root.join("probe")).unwrap();
        std::fs::write(probe_root.join("probe").join("server.py"), PROBE_SERVER).unwrap();
        plant_probe_claude(&probe_root, &claude_a, &name)
    });
    let account_a = import(&state, CLAUDE_CODE_ID, &claude_a, "Lab A");
    let account_b = import(&state, CLAUDE_CODE_ID, &claude_b, "Lab B");
    let (profile_a, profile_b) = (
        profile_of(&state, &account_a),
        profile_of(&state, &account_b),
    );

    // 1. The source on A: the first lab turn (files, a task list, a
    // subagent), a second edit (more checkpoints), and the probe call.
    let native = uuid_v4();
    let turns = source_turns(lab.probe.is_some());
    let picked = [&turns[0], &turns[3], &turns[5]];
    let mut ok = true;
    for (at, prompt) in picked.iter().enumerate() {
        let turn = claude_turn(
            &lab,
            &claude_a,
            (at == 0).then_some(native.as_str()),
            (at > 0).then_some(native.as_str()),
            prompt,
        );
        ok &= turn["is_error"] == false;
    }
    let on_a = holds(&claude_a, &native);
    let a_history = tree(&claude_a.join("file-history").join(&native));
    let a_tasks = tree(&claude_a.join("tasks").join(&native));
    let source_ok = ok
        && tests_pass(&project)
        && on_a["transcript"].is_string()
        && on_a["file_history"].as_u64().unwrap_or(0) > 0
        && on_a["snapshot_backups_missing"]
            .as_array()
            .is_some_and(Vec::is_empty);
    lab.steps.push(json!({"step": "1 source on A (checkpoints on)", "ok": source_ok, "session": native, "holds": on_a}));

    // 2. Copy A → B: nothing is asked of B.
    let source = adopt(&state, &profile_a, &native, &project);
    let preflight = juggler_app::features::switch::preflight::switch_preflight_impl(
        &state, &source, &profile_b,
    );
    let setup = set_up_probe(&state, &mut lab, &source, &profile_b);
    let copied = juggler_app::features::switch::execute::switch_execute_impl(
        &state,
        &source,
        &profile_b,
        "stop-now",
        None,
        &[],
    );
    let mut away: Option<String> = None;
    let step2 = match &copied {
        Ok(done) => {
            let argv = take_launch(&state, &done.session_id, done.pid);
            mark_stopped(&state, &done.session_id);
            let resume_id = transcript_of(&state, &done.session_id)
                .and_then(|r| r.native_id)
                .unwrap_or_else(|| native.clone());
            let on_b = holds(&claude_b, &resume_id);
            let b_history = tree(&claude_b.join("file-history").join(&resume_id));
            let history_differs = same_files(&a_history, &b_history);
            let tasks_differ: Vec<String> = a_tasks
                .keys()
                .filter(|name| !tree(&claude_b.join("tasks").join(&resume_id)).contains_key(*name))
                .cloned()
                .collect();
            let folder = session_cwd(&state, &done.session_id);
            let mcp = lab
                .probe
                .as_ref()
                .map(|name| mcp_get(&claude_b, &project, name));
            let resumes = argv.iter().any(|a| a == "--resume") && argv.contains(&resume_id);
            let a_after = tree(&claude_a.join("file-history").join(&native));
            away = Some(done.session_id.clone());
            let checks = json!({
                "transcript_resume": on_b["transcript"].as_str().is_some_and(|p| Path::new(p).starts_with(&claude_b)) && on_b["ids_other_than_this"] == 0 && on_b["lines"].as_u64() >= on_a["lines"].as_u64() && resumes,
                "cwd": folder == project && on_b["cwds"].as_array().is_some_and(|c| c.iter().all(|d| d.as_str() == Some(project.to_str().unwrap()))),
                "subagents": on_b["subagents"] == on_a["subagents"] && on_a["subagents"].as_u64().unwrap_or(0) > 0,
                "todos": tasks_differ.is_empty() && !a_tasks.is_empty(),
                "file_history": history_differs.is_empty() && !a_history.is_empty() && on_b["snapshot_backups_missing"].as_array().is_some_and(Vec::is_empty),
                "mcp_references": mcp.as_ref().is_none_or(|m| m["ok"] == true && m["names_server"] == true),
                "a_kept_its_copy": same_files(&a_history, &a_after).is_empty(),
            });
            let all = checks.as_object().unwrap().values().all(|v| v == true);
            json!({"step": "2 Copy A→B (no turn on B)", "ok": all, "checks": checks, "resume_id": resume_id,
                   "holds": on_b, "history_missing_or_differs": history_differs, "tasks_missing": tasks_differ,
                   "mcp": mcp, "launch": argv, "setup": setup,
                   "preflight": preflight.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
        Err(error) => {
            json!({"step": "2 Copy A→B (no turn on B)", "ok": false, "error": format!("{error:?}"),
                             "preflight": preflight.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
    };
    lab.steps.push(step2);

    // 3. Back to A: A is this hop's target, and it answers.
    let mut back_session: Option<String> = None;
    if let Some(away) = &away {
        let back = juggler_app::features::switch::back::switch_back_execute_to_impl(
            &state,
            away,
            &[],
            false,
            None,
        );
        let step3 = match &back {
            Ok(done) => {
                let argv = take_launch(&state, &done.session_id, done.pid);
                mark_stopped(&state, &done.session_id);
                back_session = Some(done.session_id.clone());
                let delta = argv.last().cloned().unwrap_or_default();
                let asked = claude_turn(
                    &lab,
                    &claude_a,
                    None,
                    Some(&native),
                    &format!(
                        "{delta}\n\n---\nWithout changing any file: what is this project's codename? Reply with just the codename."
                    ),
                );
                let answer = asked["result"].as_str().unwrap_or_default().to_owned();
                let history = tree(&claude_a.join("file-history").join(&native));
                let whole = same_files(&a_history, &history).is_empty();
                json!({"step": "3 switch back to A (A answers)", "ok": mentions(&answer, CODENAME) && whole,
                       "answer": answer, "file_history_whole_on_a": whole, "launch": argv})
            }
            Err(error) => {
                json!({"step": "3 switch back to A (A answers)", "ok": false, "error": format!("{error:?}")})
            }
        };
        lab.steps.push(step3);
    }

    // 4. Move A → B in place, then Undo.
    if let Some(session) = &back_session {
        let before_move = (
            tree(&claude_a.join("file-history").join(&native)),
            transcript_in(&claude_a, &native).map(|p| std::fs::read(p).unwrap_or_default()),
        );
        let offer = moving::offer(&state, session, &profile_b);
        let step4 = match &offer {
            Ok(offer) if offer.applies && offer.blocks.is_empty() => {
                match moving::execute(&state, session, &profile_b, NotesChoice::KeepTarget) {
                    Ok(done) => {
                        let on_b = holds(&claude_b, &native);
                        let b_history = tree(&claude_b.join("file-history").join(&native));
                        let b_transcript = transcript_in(&claude_b, &native)
                            .map(|p| std::fs::read(p).unwrap_or_default());
                        let moved = same_files(&before_move.0, &b_history).is_empty()
                            && b_transcript == before_move.1
                            && transcript_in(&claude_a, &native).is_none()
                            && tree(&claude_a.join("file-history").join(&native)).is_empty()
                            && on_b["snapshot_backups_missing"]
                                .as_array()
                                .is_some_and(Vec::is_empty);
                        let undone = moving::undo(&state, &done.move_id);
                        let a_back = tree(&claude_a.join("file-history").join(&native));
                        let restored = undone.is_ok()
                            && same_files(&before_move.0, &a_back).is_empty()
                            && transcript_in(&claude_a, &native)
                                .map(|p| std::fs::read(p).unwrap_or_default())
                                == before_move.1
                            && transcript_in(&claude_b, &native).is_none()
                            && tree(&claude_b.join("file-history").join(&native)).is_empty();
                        json!({"step": "4 Move A→B, then Undo", "ok": moved && restored, "moved_whole": moved, "undo_restored": restored,
                           "holds_on_b": on_b, "move_id": done.move_id, "undo": undone.map_err(|e| format!("{e:?}"))})
                    }
                    Err(error) => {
                        json!({"step": "4 Move A→B, then Undo", "ok": false, "error": format!("{error:?}")})
                    }
                }
            }
            Ok(offer) => {
                json!({"step": "4 Move A→B, then Undo", "ok": false, "not_offered": serde_json::to_value(offer).unwrap()})
            }
            Err(error) => {
                json!({"step": "4 Move A→B, then Undo", "ok": false, "error": format!("{error:?}")})
            }
        };
        lab.steps.push(step4);
    }

    // 5. Undo every set-up; take the probe off A.
    let undone: Vec<Value> = lab
        .records
        .iter()
        .rev()
        .map(
            |record| match juggler_app::features::switch_setup::undo_impl(&state, record) {
                Ok(message) => json!({"record": record, "ok": true, "message": message}),
                Err(error) => json!({"record": record, "ok": false, "error": format!("{error:?}")}),
            },
        )
        .collect();
    if lab.probe.is_some() {
        lab.steps.push(json!({"step": "5 undo every set-up", "ok": undone.iter().all(|u| u["ok"] == true), "undone": undone}));
    }
    if let (Some(true), Some(name)) = (planted, lab.probe.clone()) {
        let removed = unplant_probe_claude(&probe_root, &claude_a, &name);
        lab.steps
            .push(json!({"step": "5b the probe removed from A", "ok": removed}));
    } else if planted == Some(false) {
        lab.steps
            .push(json!({"step": "0 the probe planted on A", "ok": false}));
    }

    let after = snapshot(&roots);
    let report = json!({
        "suite": "c2c",
        "tools": {"claude": run(Command::new("claude").arg("--version"), Duration::from_secs(20)).1.trim()},
        "source_session": native,
        "steps": lab.steps,
        "snapshot": snapshot_diff(&before, &after),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("lab report: {}", out.display());
    let failed = report["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["ok"] != true)
        .count();
    assert!(failed == 0, "{failed} lab step(s) failed; see the report");
}
