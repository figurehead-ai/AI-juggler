//! D-51 in the live lab (`AIJ_LAB_SUITE=notes`, `just switch-lab-notes`):
//! does a project note travel?
//!
//! The lab plants one note in Claude account A's notes folder for the
//! lab's throwaway project only (a codename, and its line in `MEMORY.md`),
//! never in the conversation or a project file. Then:
//!
//! 1. A → B (Claude → Claude): the switch check offers the note; the
//!    harness ticks it (standing in for the user's approval in a test) and
//!    applies it, then switches. B's new session is asked the codename (in
//!    a fork that isn't kept). Then the copy is undone, and the note and
//!    its index line must be gone from B.
//! 2. A → Codex (a tool that keeps no notes): the check offers to include
//!    the note in the handover; the harness ticks it, and the tick goes
//!    with the switch. Codex reads the handover and is asked the codename.
//!
//! Each hop is measured (`features::switch_fidelity`, its "Project notes"
//! row) into `<out>.fidelity.md`. At the end the planted note is removed
//! from A. Nothing outside the lab project's notes folders is written by
//! the lab itself; every stop matches the lab's own session ids.

use super::*;
use juggler_adapters::claude_memory::{INDEX, folder_names, memory_folder};
use juggler_app::features::switch_setup::{apply_impl, check_impl, undo_impl};

/// The note's file name, and the codename only it holds.
const NOTE: &str = "lab-codename.md";
const NOTE_CODENAME: &str = "MARSH-WREN";
/// The index line the lab adds for it.
const NOTE_LINE: &str = "- [Lab codename](lab-codename.md) — the lab project's codename";

fn note_fact() -> Fact {
    fidelity::fact(
        "NOTE_CODENAME",
        "the lab project's codename, as this project's notes (your memory of the project) say",
        &[NOTE_CODENAME],
        "Lab A's notes for this project",
        Kind::Note,
    )
}

/// A's transcript of `native`, where Claude Code keeps it for `project`.
fn a_transcript(config: &Path, project: &Path, native: &str) -> Option<PathBuf> {
    folder_names(project)
        .into_iter()
        .map(|name| {
            config
                .join("projects")
                .join(name)
                .join(format!("{native}.jsonl"))
        })
        .find(|path| path.is_file())
}

/// Whether `folder`'s index lists the note.
fn indexed(folder: &Path) -> bool {
    std::fs::read_to_string(folder.join(INDEX)).is_ok_and(|text| text.contains(NOTE))
}

/// The offer for this project's notes in a check, as JSON.
fn notes_offer(check: &Value) -> Option<Value> {
    check["offers"]
        .as_array()?
        .iter()
        .find(|offer| offer["kind"] == "memory")
        .cloned()
}

/// A short turn on A, so there's a conversation to switch; it never
/// names the codename.
fn short_turn_on_a(lab: &Lab, native: &str) -> Value {
    claude_turn(
        lab,
        &lab.claude_a.clone(),
        Some(native),
        None,
        "Reply with just the word OK. Don't change any file.",
    )
}

/// Takes out what the lab planted in A's notes folder for the lab
/// project, and the folder itself when the lab made it and it's empty.
fn unplant(folder: &Path, made: bool) -> bool {
    let _ = std::fs::remove_file(folder.join(NOTE));
    if let Ok(text) = std::fs::read_to_string(folder.join(INDEX)) {
        let kept: Vec<&str> = text.lines().filter(|line| *line != NOTE_LINE).collect();
        if kept.iter().all(|line| line.trim().is_empty()) {
            let _ = std::fs::remove_file(folder.join(INDEX));
        } else {
            let _ = std::fs::write(folder.join(INDEX), kept.join("\n") + "\n");
        }
    }
    if made {
        let _ = std::fs::remove_dir(folder);
    }
    !folder.join(NOTE).exists() && !indexed(folder)
}

/// The suite: see the module's header.
pub fn journey() {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let (Some(data), Some(project), Some(claude_a), Some(claude_b), Some(codex), Some(out)) = (
        var("AIJ_LAB_DATA"),
        var("AIJ_LAB_PROJECT"),
        var("AIJ_LAB_CLAUDE_A"),
        var("AIJ_LAB_CLAUDE_B"),
        var("AIJ_LAB_CODEX"),
        var("AIJ_LAB_OUT"),
    ) else {
        eprintln!("AIJ_LAB_* not set: nothing to do");
        return;
    };
    if std::env::var("AIJ_LAB_SUITE").as_deref() != Ok("notes") {
        eprintln!("AIJ_LAB_SUITE is not notes: nothing to do");
        return;
    }
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
        codex.clone(),
        home.join(".claude.json"),
        home.join(".agents"),
    ];
    let before = snapshot(&roots);

    let state = Arc::new(AppState::init(data).unwrap());
    // The user's policy, chosen before any switch (D-14): strip secrets.
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let lab = Lab {
        project: project.clone(),
        claude_a: claude_a.clone(),
        claude_b: claude_b.clone(),
        codex: codex.clone(),
        codex_b: None,
        steps: Vec::new(),
        probe: None,
        records: Vec::new(),
    };
    let mut steps: Vec<Value> = Vec::new();
    let account_a = import(&state, CLAUDE_CODE_ID, &claude_a, "Lab A");
    let account_b = import(&state, CLAUDE_CODE_ID, &claude_b, "Lab B");
    let account_x = import(&state, CODEX_ID, &codex, "Lab Codex");
    let (profile_a, profile_b, profile_x) = (
        profile_of(&state, &account_a),
        profile_of(&state, &account_b),
        profile_of(&state, &account_x),
    );
    let mut book = fidelity::Book::new(&state);
    let claude = tool_name(&state, CLAUDE_CODE_ID);
    let codex_tool = tool_name(&state, CODEX_ID);
    let (label_a, label_b, label_x) = (
        format!("Lab A ({claude})"),
        format!("Lab B ({claude})"),
        format!("Lab Codex ({codex_tool})"),
    );
    let fact = note_fact();
    let question = fidelity::facts::question(std::slice::from_ref(&fact));

    // 0. The note, in A's notes folder for the lab project only.
    let a_folder = memory_folder(&claude_a, &project);
    let made = !a_folder.exists();
    std::fs::create_dir_all(&a_folder).unwrap();
    std::fs::write(
        a_folder.join(NOTE),
        format!(
            "---\nname: Lab codename\ndescription: The lab project's codename\ntype: project\n---\n\nThe lab project's codename is {NOTE_CODENAME}.\n"
        ),
    )
    .unwrap();
    let mut index = std::fs::read_to_string(a_folder.join(INDEX)).unwrap_or_default();
    index.push_str(NOTE_LINE);
    index.push('\n');
    std::fs::write(a_folder.join(INDEX), index).unwrap();
    steps.push(
        json!({"step": "0 a note planted on A, for the lab project only",
                      "ok": made && a_folder.join(NOTE).is_file(), "folder_was_new": made}),
    );

    // 1. A → B with the note copied (ticked by the harness).
    let b_folder = memory_folder(&claude_b, &project);
    let native1 = uuid_v4();
    let turn = short_turn_on_a(&lab, &native1);
    let s1 = adopt(&state, &profile_a, &native1, &project);
    let check_b = check_impl(&state, &s1, &profile_b)
        .map(|check| serde_json::to_value(check).unwrap())
        .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
    let offer_b = notes_offer(&check_b);
    let offered_b = offer_b
        .as_ref()
        .is_some_and(|o| o["verdict"] == "copy" && o["count"] == 1 && o["suggested"] == false);
    let applied = offer_b
        .as_ref()
        .and_then(|offer| offer["id"].as_str())
        .map(|id| apply_impl(&state, &s1, &profile_b, &[id.to_owned()]))
        .and_then(Result::ok)
        .unwrap_or_default();
    let record = applied.first().and_then(|done| done.record.clone());
    let copied = b_folder.join(NOTE).is_file() && indexed(&b_folder);
    let source_a = a_transcript(&claude_a, &project, &native1).and_then(|path| {
        fidelity::source_at(&state, &account_a, &native1, &path, &project, &label_a)
    });
    let in_conversation = source_a
        .as_ref()
        .is_some_and(|source| mentions(&source.side.history.all_text(), NOTE_CODENAME));
    let to_b = commands::switch_execute_with(&state, &s1, &profile_b, "stop-now", None, &[], &[]);
    let step1 = match &to_b {
        Ok(done) => {
            let argv = take_launch(&state, &done.session_id, done.pid);
            let link = transcript_of(&state, &done.session_id);
            let resume_id = link
                .as_ref()
                .and_then(|r| r.native_id.clone())
                .unwrap_or_else(|| native1.clone());
            let folder = session_cwd(&state, &done.session_id);
            let target = link.as_ref().map(|link| {
                fidelity::native(
                    &state, &account_b, &link.path, &resume_id, &folder, &label_b,
                )
            });
            let probe = claude_ask(&lab, &claude_b, &project, &resume_id, &question);
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let probed = fidelity::probe(std::slice::from_ref(&fact), &answer, &project);
            let answered = probed.iter().all(|p| p.score == Score::Answered);
            let grade = match (source_a.clone(), target) {
                (Some(source), Some(target)) => {
                    book.hop("A → B (notes)", "transplant", source, target, probed)
                }
                _ => {
                    book.missing("A → B (notes)", "a transcript wasn't found");
                    "NOT MEASURED"
                }
            };
            json!({"step": "1 A → B, the note copied with the tick", "ok": turn["is_error"] == false
                       && offered_b && copied && !in_conversation && answered && grade == "PASS",
                   "offer": offer_b, "applied": serde_json::to_value(&applied).unwrap(),
                   "copied": copied, "codename_in_conversation": in_conversation,
                   "probe": answer, "fidelity": grade, "session": done.session_id, "launch": argv})
        }
        Err(error) => json!({"step": "1 A → B, the note copied with the tick", "ok": false,
                             "offer": offer_b, "error": format!("{error:?}")}),
    };
    steps.push(step1);

    // 1b. Undo the copy: the note and its index line are gone from B.
    let undone = record
        .as_deref()
        .map(|record| undo_impl(&state, record))
        .map(|result| result.map_err(|error| format!("{error:?}")));
    let gone = !b_folder.join(NOTE).exists() && !indexed(&b_folder);
    steps.push(
        json!({"step": "1b undo: the copied note and its index line are gone from B",
                      "ok": matches!(undone, Some(Ok(_))) && gone,
                      "undo": format!("{undone:?}"), "gone": gone}),
    );

    // 2. A → Codex with the note quoted in the handover (ticked).
    let native2 = uuid_v4();
    let turn2 = short_turn_on_a(&lab, &native2);
    let s2 = adopt(&state, &profile_a, &native2, &project);
    let check_x = check_impl(&state, &s2, &profile_x)
        .map(|check| serde_json::to_value(check).unwrap())
        .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
    let offer_x = notes_offer(&check_x);
    let offered_x = offer_x
        .as_ref()
        .is_some_and(|o| o["verdict"] == "handover" && o["count"] == 1 && o["suggested"] == false);
    let approved: Vec<String> = offer_x
        .as_ref()
        .and_then(|offer| offer["id"].as_str())
        .map(str::to_owned)
        .into_iter()
        .collect();
    // Set up beforehand: nothing (the switch itself carries it).
    let set_up = apply_impl(&state, &s2, &profile_x, &approved).unwrap_or_default();
    let source_a2 = a_transcript(&claude_a, &project, &native2).and_then(|path| {
        fidelity::source_at(&state, &account_a, &native2, &path, &project, &label_a)
    });
    let to_x = commands::switch_execute_with(
        &state,
        &s2,
        &profile_x,
        "stop-now",
        Some("strip"),
        &[],
        &approved,
    );
    let step2 = match &to_x {
        Ok(done) => {
            let argv = take_launch(&state, &done.session_id, done.pid);
            let brief = argv
                .iter()
                .position(|a| a == "--")
                .and_then(|at| argv.get(at + 1))
                .cloned()
                .unwrap_or_default();
            let handover = handover_of(&brief);
            let file = handover["path"].as_str().map(PathBuf::from);
            let text = file
                .as_deref()
                .and_then(|file| std::fs::read_to_string(file).ok())
                .unwrap_or_default();
            let quoted = text.contains("This project's notes from Lab A")
                && text.contains(&format!("{NOTE}:\n> "))
                && text.contains(NOTE_CODENAME);
            let folder = session_cwd(&state, &done.session_id);
            let target = file
                .as_deref()
                .map(|file| fidelity::handover(&state, &account_x, file, &folder, &label_x));
            let (thread, answer, ok) = codex_turn(
                &lab,
                None,
                &format!(
                    "{brief}\n\n---\nFirst read the handover file named above. Then: {question}"
                ),
            );
            let probed = fidelity::probe(std::slice::from_ref(&fact), &answer, &project);
            let answered = probed.iter().all(|p| p.score == Score::Answered);
            let grade = match (source_a2, target) {
                (Some(source), Some(target)) => {
                    book.hop("A → Codex (notes)", "replay", source, target, probed)
                }
                _ => {
                    book.missing(
                        "A → Codex (notes)",
                        "the source's transcript or the handover wasn't found",
                    );
                    "NOT MEASURED"
                }
            };
            json!({"step": "2 A → Codex, the note quoted in the handover with the tick",
                   "ok": turn2["is_error"] == false && offered_x && set_up.is_empty() && quoted
                       && ok && answered && grade == "PASS",
                   "offer": offer_x, "set_up_beforehand": set_up.len(), "quoted": quoted,
                   "probe": answer, "thread": thread, "fidelity": grade,
                   "handover": handover, "session": done.session_id})
        }
        Err(error) => json!({"step": "2 A → Codex, the note quoted in the handover with the tick",
                             "ok": false, "offer": offer_x, "error": format!("{error:?}")}),
    };
    steps.push(step2);

    // 3. Clean up: the planted note off A; nothing of it left on B.
    let removed = unplant(&a_folder, made);
    let b_clean = !b_folder.join(NOTE).exists() && !indexed(&b_folder);
    if b_clean && b_folder.is_dir() && std::fs::read_dir(&b_folder).unwrap().next().is_none() {
        let _ = std::fs::remove_dir(&b_folder);
    }
    steps.push(
        json!({"step": "3 the planted note removed from A, none left on B",
                      "ok": removed && b_clean,
                      "a_folder_left": a_folder.exists(), "b_folder_left": b_folder.exists()}),
    );

    let after = snapshot(&roots);
    let report = json!({
        "tools": {
            "claude": run(Command::new("claude").arg("--version"), Duration::from_secs(20)).1.trim(),
            "codex": run(Command::new("codex").arg("--version"), Duration::from_secs(20)).1.trim(),
        },
        "steps": steps,
        "fidelity": book.hops,
        "snapshot": snapshot_diff(&before, &after),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("lab report: {}", out.display());
    eprintln!("fidelity report: {}", book.write(&out).display());
    let failed = report["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["ok"] != true)
        .count();
    assert!(failed == 0, "{failed} lab step(s) failed; see the report");
}
