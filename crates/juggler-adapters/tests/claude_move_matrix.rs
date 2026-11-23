//! ADR-0031 (D-93): the acceptance matrix of a switch by move, on small
//! synthetic Claude app profiles and Claude Code folders: the Claude
//! planner (`claude_move`) and AI Juggler's engine
//! (`juggler-handover::moving`) together, one test per row of the
//! research note's matrix (`docs/research/claude-move-switch.md`).
//! Fixtures only: no real profile, no sign-in, no model prompt.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    reason = "a test: a failed step fails the test"
)]

use std::path::{Path, PathBuf};

use juggler_adapter::moving::{
    HoldWhy, InPlaceMove, MemberSide, MovePlan, MoveRequest, MoveSide, MoveStep, NotesChoice,
    RefusalKind,
};
use juggler_adapter::transfer::file_id;
use juggler_adapters::claude_move::ClaudeMove;
use juggler_handover::moving::{Phase, Recovery, Stopped, Transaction};

const SLUG: &str = "-w-proj";
const ORG: &str = "org-1";

/// One account: a Claude app profile and its Claude Code folder.
struct Account {
    host: PathBuf,
    code: PathBuf,
    folder: PathBuf,
}

struct Lab {
    tmp: tempfile::TempDir,
    moves: PathBuf,
    work: PathBuf,
}

fn clones() -> bool {
    let tmp = tempfile::tempdir().unwrap();
    juggler_adapter::transfer::clones_supported(tmp.path()).unwrap_or(false)
}

impl Lab {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let moves = tmp.path().join("aij/moves");
        let work = tmp.path().join("work/proj");
        std::fs::create_dir_all(&work).unwrap();
        Self { tmp, moves, work }
    }

    fn account(&self, name: &str) -> Account {
        let host = self.tmp.path().join(format!("profiles/{name}"));
        let code = self.tmp.path().join(format!("code/{name}"));
        let folder = host.join(format!("claude-code-sessions/acct-{name}/{ORG}"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(code.join("projects").join(SLUG)).unwrap();
        std::fs::write(
            code.join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"acct-{name}","organizationUuid":"{ORG}"}}}}"#
            ),
        )
        .unwrap();
        Account { host, code, folder }
    }

    fn request(&self, from: &Account, to: &Account, conversation: &str) -> MoveRequest {
        let side = |a: &Account| MoveSide {
            tool_dir: a.code.clone(),
            host_dir: Some(a.host.clone()),
            version: Some("5.11492.5".into()),
        };
        MoveRequest {
            conversation: conversation.into(),
            source: side(from),
            target: side(to),
            notes: NotesChoice::KeepTarget,
        }
    }

    fn cwd(&self) -> String {
        self.work.display().to_string()
    }
}

/// A record of `cli` as the app writes one.
fn record(
    account: &Account,
    local: &str,
    cli: &str,
    parent: Option<&str>,
    archived: bool,
    cwd: &str,
) {
    let parent = parent.map_or_else(String::new, |p| {
        format!(r#","spawnedFrom":{{"sessionId":"{p}","sideSession":true,"name":"n"}}"#)
    });
    std::fs::write(
        account.folder.join(format!("{local}.json")),
        format!(
            r#"{{"sessionId":"{local}","cliSessionId":"{cli}","title":"T {cli}","isArchived":{archived},"cwd":"{cwd}","model":"m","lastActivityAt":1000,"error":"limit","promptAppendSnapshot":"x","unknownFutureField":{{"kept":true}}{parent}}}"#
        ),
    )
    .unwrap();
}

/// A history with `turns` lines, and its folder: nested agents with their
/// sidecars, outputs, rewind.
fn history(account: &Account, slug: &str, cli: &str, turns: usize, seed: &str) -> PathBuf {
    let project = account.code.join("projects").join(slug);
    std::fs::create_dir_all(project.join(cli).join("subagents")).unwrap();
    std::fs::create_dir_all(project.join(cli).join("tool-results")).unwrap();
    let mut body = String::new();
    for n in 0..turns {
        body.push_str(&format!(
            "{{\"type\":\"{}\",\"sessionId\":\"{cli}\",\"n\":{n},\"seed\":\"{seed}\"}}\n",
            if n % 2 == 0 { "user" } else { "assistant" }
        ));
    }
    let path = project.join(format!("{cli}.jsonl"));
    std::fs::write(&path, body).unwrap();
    std::fs::write(
        project.join(cli).join("subagents/agent-a.jsonl"),
        b"{\"type\":\"user\"}\n",
    )
    .unwrap();
    std::fs::write(
        project.join(cli).join("subagents/agent-a.meta.json"),
        b"{\"agentType\":\"x\"}",
    )
    .unwrap();
    std::fs::write(
        project.join(cli).join("tool-results/out-1.txt"),
        format!("output of {seed}"),
    )
    .unwrap();
    let rewind = account.code.join("file-history").join(cli);
    std::fs::create_dir_all(&rewind).unwrap();
    std::fs::write(rewind.join("v1"), format!("rewind {seed}")).unwrap();
    path
}

fn read(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// Plans, backs up, applies and checks before the apps start; then the
/// app's start (a line of bookkeeping appended to each moved history) and
/// the checks after. Returns the committed move.
fn run(lab: &Lab, request: &MoveRequest, id: &str) -> (Transaction, MovePlan) {
    let plan = ClaudeMove.plan(request).unwrap();
    assert!(plan.can_move(), "refused: {:?}", plan.refusals);
    let mut txn = Transaction::begin(
        &lab.moves,
        id,
        "claude-code",
        request.clone(),
        plan.clone(),
        None,
        1,
    )
    .unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let report = txn.verify_cold(ClaudeMove.verify(request, &plan)).unwrap();
    assert!(report.passed(), "cold: {:?}", report.failures());
    for member in &plan.members {
        if let Some(to) = &member.transcript_to {
            let mut bytes = std::fs::read(to).unwrap();
            bytes.extend_from_slice(b"{\"type\":\"custom-title\",\"customTitle\":\"t\"}\n");
            std::fs::write(to, bytes).unwrap();
        }
    }
    let report = txn
        .verify_opened(
            &|line| ClaudeMove.appended(line),
            ClaudeMove.verify(request, &plan),
        )
        .unwrap();
    assert!(report.passed(), "after: {:?}", report.failures());
    assert_eq!(report.added.turns, 0, "a move writes no turn");
    assert_eq!(txn.manifest.phase, Phase::Committed);
    (txn, plan)
}

macro_rules! needs_clones {
    () => {
        if !clones() {
            eprintln!("skipped: this disk makes no clones");
            return;
        }
    };
}

/// Row 1: main, two children, a grandchild and nested agents' sidecars all
/// move; an unrelated conversation stays as it was.
#[test]
fn row01_simple_family() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_c1", "c1", Some("local_main"), false, &cwd);
    record(&a, "local_c2", "c2", Some("local_main"), false, &cwd);
    record(&a, "local_g1", "g1", Some("local_c1"), false, &cwd);
    record(&a, "local_other", "other", None, false, &cwd);
    let main = history(&a, SLUG, "main", 6, "a");
    for c in ["c1", "c2", "g1", "other"] {
        history(&a, SLUG, c, 2, c);
    }
    let main_id = file_id(&main).unwrap();
    let other_before =
        std::fs::read(a.code.join("projects").join(SLUG).join("other.jsonl")).unwrap();
    let (_txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row01");
    assert_eq!(plan.members.len(), 4);
    let moved = b.code.join("projects").join(SLUG).join("main.jsonl");
    assert_eq!(file_id(&moved).unwrap(), main_id, "the same file, renamed");
    for c in ["main", "c1", "c2", "g1"] {
        let dir = b.code.join("projects").join(SLUG).join(c);
        assert!(
            dir.join("subagents/agent-a.jsonl").is_file(),
            "{c} nested agent"
        );
        assert!(
            dir.join("subagents/agent-a.meta.json").is_file(),
            "{c} sidecar"
        );
        assert!(
            b.code.join("file-history").join(c).join("v1").is_file(),
            "{c} rewind"
        );
        assert!(
            b.folder.join(format!("local_{c}.json")).is_file(),
            "{c} record"
        );
        assert!(
            !a.folder.join(format!("local_{c}.json")).exists(),
            "{c} retired"
        );
    }
    assert_eq!(
        read(&b.folder.join("local_g1.json"))["spawnedFrom"]["sessionId"],
        "local_c1"
    );
    let main_record = read(&b.folder.join("local_main.json"));
    assert_eq!(
        main_record["unknownFutureField"]["kept"], true,
        "unknown fields kept"
    );
    assert!(
        main_record.get("error").is_none(),
        "stale runtime state cleared"
    );
    // Untouched.
    assert!(a.folder.join("local_other.json").is_file());
    assert_eq!(
        std::fs::read(a.code.join("projects").join(SLUG).join("other.jsonl")).unwrap(),
        other_before
    );
}

/// Row 2: a child's app id differs from its conversation id; its history
/// still opens.
#[test]
fn row02_distinct_ids() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(
        &a,
        "local_sidekick",
        "conv-floor",
        Some("local_main"),
        false,
        &cwd,
    );
    history(&a, SLUG, "main", 2, "m");
    history(&a, SLUG, "conv-floor", 4, "floor");
    let (_txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row02");
    let child = plan
        .members
        .iter()
        .find(|m| m.conversation == "conv-floor")
        .unwrap();
    assert_eq!(child.record_id.as_deref(), Some("local_sidekick"));
    assert!(
        b.code
            .join("projects")
            .join(SLUG)
            .join("conv-floor.jsonl")
            .is_file()
    );
    let rec = read(&b.folder.join("local_sidekick.json"));
    assert_eq!(rec["cliSessionId"], "conv-floor");
    assert!(
        !b.folder.join("local_conv-floor.json").exists(),
        "no id made up from the other"
    );
}

/// Row 3: the target already had an older record (an alias) of the main
/// conversation with archived children: they re-link to the moved one and
/// stay archived; the alias is put aside.
#[test]
fn row03_existing_alias() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&b, "local_oldalias", "main", None, false, &cwd);
    for n in 0..3 {
        record(
            &b,
            &format!("local_arch{n}"),
            &format!("arch{n}"),
            Some("local_oldalias"),
            true,
            &cwd,
        );
        history(&b, SLUG, &format!("arch{n}"), 2, "arch");
    }
    history(&a, SLUG, "main", 4, "a");
    let (txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row03");
    assert!(
        !b.folder.join("local_oldalias.json").exists(),
        "alias put aside"
    );
    for n in 0..3 {
        let rec = read(&b.folder.join(format!("local_arch{n}.json")));
        assert_eq!(rec["spawnedFrom"]["sessionId"], "local_main");
        assert_eq!(rec["isArchived"], true);
        assert!(
            rec.get("promptAppendSnapshot").is_none(),
            "routing snapshot regenerated"
        );
        assert_eq!(rec["cliSessionId"], format!("arch{n}"));
    }
    assert_eq!(
        plan.members
            .iter()
            .filter(|m| m.side == MemberSide::Target)
            .count(),
        3
    );
    // The alias is in the move's holding place, whole.
    let held = std::fs::read_dir(txn.dir().join("holding"))
        .unwrap()
        .count();
    assert!(held >= 1);
}

/// Row 4: the target has its own different version of the conversation:
/// both kept, never joined, one owner.
#[test]
fn row04_divergent_destination() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&b, "local_main", "main", None, false, &cwd);
    let ours = std::fs::read(history(&a, SLUG, "main", 8, "ours")).unwrap();
    let theirs = std::fs::read(history(&b, SLUG, "main", 5, "theirs")).unwrap();
    let (txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row04");
    assert!(plan.notes.iter().any(|n| n.contains("different version")));
    let now = std::fs::read(b.code.join("projects").join(SLUG).join("main.jsonl")).unwrap();
    assert!(now.starts_with(&ours), "the moved branch, not a join");
    assert!(
        !now.windows(6).any(|w| w == b"theirs"),
        "never concatenated"
    );
    let mut kept = false;
    for entry in std::fs::read_dir(txn.dir().join("holding"))
        .unwrap()
        .flatten()
    {
        if entry.path().is_file() && std::fs::read(entry.path()).unwrap() == theirs {
            kept = true;
        }
    }
    assert!(kept, "the target's branch is kept whole");
    let claims = std::fs::read_dir(&b.folder)
        .unwrap()
        .flatten()
        .filter(|e| read(&e.path())["cliSessionId"] == "main")
        .count();
    assert_eq!(claims, 1, "one active owner");
}

/// Row 5: histories under other project folders (worktrees) are found; a
/// working folder that's gone is named, its history kept.
#[test]
fn row05_worktree_roots() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(
        &a,
        "local_wt",
        "wt",
        Some("local_main"),
        true,
        "/gone/worktree",
    );
    history(&a, SLUG, "main", 2, "m");
    let slug = "-w-proj--claude-worktrees-x";
    std::fs::create_dir_all(a.code.join("projects").join(slug)).unwrap();
    history(&a, slug, "wt", 3, "wt");
    let (_txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row05");
    assert!(
        b.code
            .join("projects")
            .join(slug)
            .join("wt.jsonl")
            .is_file()
    );
    assert!(plan.notes.iter().any(|n| n.contains("folder that's gone")));
    assert!(!Path::new("/gone/worktree").exists(), "nothing recreated");
}

/// Row 6: out-of-line outputs stay where old messages name them, on both
/// sides, and the move lists them for cleanup.
#[test]
fn row06_output_references() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&b, "local_main", "main", None, false, &cwd);
    history(&a, SLUG, "main", 2, "a");
    history(&b, SLUG, "main", 2, "b");
    let (txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "row06");
    let source_out = a
        .code
        .join("projects")
        .join(SLUG)
        .join("main/tool-results/out-1.txt");
    let target_out = b
        .code
        .join("projects")
        .join(SLUG)
        .join("main/tool-results/out-1.txt");
    assert_eq!(std::fs::read_to_string(&source_out).unwrap(), "output of a");
    assert_eq!(std::fs::read_to_string(&target_out).unwrap(), "output of b");
    assert!(
        plan.retained
            .iter()
            .any(|r| r.path.ends_with("main/tool-results"))
    );
    assert!(
        txn.dependencies()
            .iter()
            .any(|p| p.ends_with("main/tool-results"))
    );
}

/// Row 7: the project's notes differ: kept by default (nothing
/// project-wide changes); taken by choice, with the target's kept aside
/// and restored by Undo.
#[test]
fn row07_shared_notes_conflict() {
    needs_clones!();
    for choice in [NotesChoice::KeepTarget, NotesChoice::TakeSource] {
        let lab = Lab::new();
        let (a, b) = (lab.account("a"), lab.account("b"));
        let cwd = lab.cwd();
        record(&a, "local_main", "main", None, false, &cwd);
        history(&a, SLUG, "main", 2, "a");
        for (account, text) in [(&a, "source notes"), (&b, "target notes")] {
            let memory = account.code.join("projects").join(SLUG).join("memory");
            std::fs::create_dir_all(&memory).unwrap();
            std::fs::write(memory.join("MEMORY.md"), text).unwrap();
        }
        let mut request = lab.request(&a, &b, "main");
        request.notes = choice;
        let plan = ClaudeMove.plan(&request).unwrap();
        assert!(plan.notes_conflict.is_some());
        let (mut txn, _) = run(&lab, &request, "row07");
        let memory = b.code.join("projects").join(SLUG).join("memory/MEMORY.md");
        let expected = if choice == NotesChoice::KeepTarget {
            "target notes"
        } else {
            "source notes"
        };
        assert_eq!(std::fs::read_to_string(&memory).unwrap(), expected);
        assert_eq!(
            std::fs::read_to_string(a.code.join("projects").join(SLUG).join("memory/MEMORY.md"))
                .unwrap(),
            "source notes",
            "the source's notes are cloned, never moved"
        );
        txn.undo(&Stopped::checked(), 2).unwrap();
        assert_eq!(std::fs::read_to_string(&memory).unwrap(), "target notes");
    }
}

/// Row 9: what a move can't do is refused before anything, as a refusal
/// the user can't clear (D-127: Move isn't shown): a records shape it
/// doesn't know (D-128, on any version), a link, a hard-linked history,
/// app to terminal, another disk.
#[test]
fn row09_unsupported_shape_link_and_device() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    let main = history(&a, SLUG, "main", 2, "a");
    // A records layout it doesn't know, on the version that was proven:
    // refused, and named for the gate watch.
    std::fs::write(b.folder.join("sessions.db"), b"").unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        plan.refusals.iter().any(|r| r.kind == RefusalKind::Setup),
        "{:?}",
        plan.refusals
    );
    assert!(
        plan.shape.iter().any(|what| what.contains("sessions.db")),
        "{:?}",
        plan.shape
    );
    std::fs::remove_file(b.folder.join("sessions.db")).unwrap();
    // App to terminal.
    let mut request = lab.request(&a, &b, "main");
    request.target.host_dir = None;
    let plan = ClaudeMove.plan(&request).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.words.contains("terminal") && r.kind == RefusalKind::Setup),
        "{:?}",
        plan.refusals
    );
    // A link inside the family's folder.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            "/etc/hosts",
            main.with_extension("").join("subagents/linked.jsonl"),
        )
        .unwrap();
        let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
        assert!(!plan.can_move(), "a link refuses: {:?}", plan.refusals);
        std::fs::remove_file(main.with_extension("").join("subagents/linked.jsonl")).unwrap();
        // A hard-linked history.
        std::fs::hard_link(&main, lab.tmp.path().join("second-name")).unwrap();
        let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
        assert!(
            plan.refusals.iter().any(|r| r.words.contains("hard link")),
            "{:?}",
            plan.refusals
        );
        std::fs::remove_file(lab.tmp.path().join("second-name")).unwrap();
    }
    // Another disk for the move's backups: refused, nothing written.
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    let elsewhere = Path::new("/dev/ai-juggler-move-test/moves");
    let refusals = juggler_handover::moving::preflight(&plan, elsewhere);
    assert!(
        refusals
            .iter()
            .any(|r| r.words.contains("another disk") || r.words.contains("clone")),
        "{refusals:?}"
    );
    assert!(main.is_file(), "nothing moved");
}

/// Row 10: the app adds its bookkeeping when it starts: accepted, every
/// original byte checked.
#[test]
fn row10_restart_append() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    record(&a, "local_main", "main", None, false, &lab.cwd());
    history(&a, SLUG, "main", 4, "a");
    // `run` appends a title line and checks the prefix.
    run(&lab, &lab.request(&a, &b, "main"), "row10");
}

/// Row 11: a history-only system notice (a background command that
/// stopped) is told apart from a typed turn.
#[test]
fn row11_system_task_notification() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    record(&a, "local_main", "main", None, false, &lab.cwd());
    history(&a, SLUG, "main", 2, "a");
    let request = lab.request(&a, &b, "main");
    let plan = ClaudeMove.plan(&request).unwrap();
    let mut txn = Transaction::begin(
        &lab.moves,
        "row11",
        "claude-code",
        request.clone(),
        plan.clone(),
        None,
        1,
    )
    .unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    assert!(
        txn.verify_cold(ClaudeMove.verify(&request, &plan))
            .unwrap()
            .passed()
    );
    let moved = b.code.join("projects").join(SLUG).join("main.jsonl");
    let mut bytes = std::fs::read(&moved).unwrap();
    bytes.extend_from_slice(br#"{"type":"user","promptSource":"system","origin":{"kind":"task-notification"},"queueTranscriptOnly":true}"#);
    bytes.push(b'\n');
    std::fs::write(&moved, &bytes).unwrap();
    let report = txn
        .verify_opened(&|l| ClaudeMove.appended(l), Vec::new())
        .unwrap();
    assert!(report.passed());
    assert_eq!((report.added.notices, report.added.turns), (1, 0));
    // A typed turn is a turn.
    bytes.extend_from_slice(b"{\"type\":\"user\",\"message\":{\"content\":\"go on\"}}\n");
    std::fs::write(&moved, &bytes).unwrap();
    let report = txn
        .verify_opened(&|l| ClaudeMove.appended(l), Vec::new())
        .unwrap();
    assert_eq!(report.added.turns, 1);
}

/// Row 12: a damaged backup refuses Undo before anything changes; an
/// interrupted move is read back from its journal and finished, or undone.
#[test]
fn row12_corrupt_backup_and_interrupted_operation() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_c1", "c1", Some("local_main"), false, &cwd);
    history(&a, SLUG, "main", 2, "a");
    history(&a, SLUG, "c1", 2, "c");
    let request = lab.request(&a, &b, "main");
    let plan = ClaudeMove.plan(&request).unwrap();
    let mut txn = Transaction::begin(
        &lab.moves,
        "row12",
        "claude-code",
        request.clone(),
        plan.clone(),
        None,
        1,
    )
    .unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    // Interrupted: the journal says applying; nothing was done yet.
    let dir = txn.dir().to_path_buf();
    txn.manifest.phase = Phase::Applying;
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&txn.manifest).unwrap(),
    )
    .unwrap();
    let mut back = Transaction::open(&dir).unwrap();
    assert_eq!(back.recover().unwrap(), Recovery::Applying { next: 0 });
    back.apply(&stopped, 0).unwrap();
    assert!(
        back.verify_cold(ClaudeMove.verify(&request, &plan))
            .unwrap()
            .passed()
    );
    // A damaged backup: Undo refuses, the moved family stays as it is.
    let entry = back
        .manifest
        .entries
        .iter()
        .find(|e| e.backup.is_some())
        .unwrap()
        .clone();
    let backup = dir.join("backups").join(entry.backup.unwrap());
    let original = std::fs::read(&backup).ok();
    if backup.is_file() {
        std::fs::write(&backup, b"damaged").unwrap();
    } else {
        std::fs::write(backup.join("injected"), b"x").unwrap();
    }
    assert!(back.undo(&stopped, 3).is_err());
    assert!(b.folder.join("local_main.json").is_file(), "nothing undone");
    // Repaired, Undo puts both accounts back exactly.
    if let Some(bytes) = original {
        std::fs::write(&backup, bytes).unwrap();
    } else {
        std::fs::remove_file(backup.join("injected")).unwrap();
    }
    back.undo(&stopped, 4).unwrap();
    assert!(a.folder.join("local_main.json").is_file());
    assert!(a.folder.join("local_c1.json").is_file());
    assert!(!b.folder.join("local_main.json").exists());
    assert!(
        a.code
            .join("projects")
            .join(SLUG)
            .join("main.jsonl")
            .is_file()
    );
}

/// Row 13: Undo after more work: the original branches come back, the
/// newer work of the parent and a child is kept apart, whole.
#[test]
fn row13_undo_after_more_work() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_c1", "c1", Some("local_main"), false, &cwd);
    record(&b, "local_main", "main", None, false, &cwd);
    let a_main = std::fs::read(history(&a, SLUG, "main", 4, "a")).unwrap();
    let a_child = std::fs::read(history(&a, SLUG, "c1", 2, "c")).unwrap();
    let b_main = std::fs::read(history(&b, SLUG, "main", 3, "b")).unwrap();
    let (mut txn, _) = run(&lab, &lab.request(&a, &b, "main"), "row13");
    let moved_main = b.code.join("projects").join(SLUG).join("main.jsonl");
    let moved_child = b.code.join("projects").join(SLUG).join("c1.jsonl");
    for path in [&moved_main, &moved_child] {
        let mut bytes = std::fs::read(path).unwrap();
        bytes.extend_from_slice(b"{\"type\":\"assistant\",\"later\":true}\n");
        std::fs::write(path, bytes).unwrap();
    }
    let later_main = std::fs::read(&moved_main).unwrap();
    let later_child = std::fs::read(&moved_child).unwrap();
    let kept = txn.undo(&Stopped::checked(), 7).unwrap();
    assert_eq!(
        std::fs::read(a.code.join("projects").join(SLUG).join("main.jsonl")).unwrap(),
        a_main
    );
    assert_eq!(
        std::fs::read(a.code.join("projects").join(SLUG).join("c1.jsonl")).unwrap(),
        a_child
    );
    assert_eq!(
        std::fs::read(&moved_main).unwrap(),
        b_main,
        "the target's own branch is back"
    );
    let mut found = (false, false);
    for path in files(&kept) {
        let bytes = std::fs::read(&path).unwrap();
        found.0 |= bytes == later_main;
        found.1 |= bytes == later_child;
    }
    assert_eq!(found, (true, true), "the newer work is kept apart");
}

/// Row 14: moved on to a third account, then back to the first: the
/// current owner and its whole family stay authoritative each time; old
/// aliases orphan nothing.
#[test]
fn row14_repeated_switch_and_third_account() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b, c) = (lab.account("a"), lab.account("b"), lab.account("c"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_c1", "c1", Some("local_main"), false, &cwd);
    record(&b, "local_alias", "main", None, false, &cwd);
    record(&b, "local_arch", "arch", Some("local_alias"), true, &cwd);
    history(&a, SLUG, "main", 4, "a");
    history(&a, SLUG, "c1", 2, "c");
    history(&b, SLUG, "arch", 2, "arch");
    run(&lab, &lab.request(&a, &b, "main"), "row14-ab");
    // Work on B, then on to C.
    let on_b = b.code.join("projects").join(SLUG).join("main.jsonl");
    let mut bytes = std::fs::read(&on_b).unwrap();
    bytes.extend_from_slice(b"{\"type\":\"assistant\",\"on\":\"b\"}\n");
    std::fs::write(&on_b, &bytes).unwrap();
    let (_, plan) = run(&lab, &lab.request(&b, &c, "main"), "row14-bc");
    let names: Vec<&str> = plan
        .members
        .iter()
        .map(|m| m.conversation.as_str())
        .collect();
    assert!(
        names.contains(&"c1") && names.contains(&"arch"),
        "the whole current family: {names:?}"
    );
    assert!(
        std::fs::read(c.code.join("projects").join(SLUG).join("main.jsonl"))
            .unwrap()
            .starts_with(&bytes)
    );
    assert_eq!(
        read(&c.folder.join("local_arch.json"))["spawnedFrom"]["sessionId"],
        "local_main"
    );
    assert_eq!(read(&c.folder.join("local_arch.json"))["isArchived"], true);
    // And back to A: A owns it again, with everything.
    let (_, plan) = run(&lab, &lab.request(&c, &a, "main"), "row14-ca");
    assert_eq!(plan.members.len(), 3);
    for id in ["local_main", "local_c1", "local_arch"] {
        assert!(a.folder.join(format!("{id}.json")).is_file(), "{id} on A");
        assert!(!c.folder.join(format!("{id}.json")).exists(), "{id} left C");
        assert!(
            !b.folder.join(format!("{id}.json")).exists(),
            "{id} not on B"
        );
    }
}

/// The CLI row: Claude Code folder to folder, no app.
#[test]
fn cli_folder_to_folder() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let main = history(&a, SLUG, "main", 4, "a");
    let id = file_id(&main).unwrap();
    let mut request = lab.request(&a, &b, "main");
    for side in [&mut request.source, &mut request.target] {
        side.host_dir = None;
        side.version = Some("2.1.317".into());
    }
    let (_, plan) = run(&lab, &request, "cli");
    assert_eq!(plan.members.len(), 1);
    assert_eq!(
        file_id(&b.code.join("projects").join(SLUG).join("main.jsonl")).unwrap(),
        id
    );
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(files(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// Review fixes: the target's copy of a history in another project folder
/// is put aside (one history per conversation on the target).
#[test]
fn a_target_copy_in_another_project_folder_is_put_aside() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    record(&a, "local_main", "main", None, false, &lab.cwd());
    history(&a, SLUG, "main", 2, "a");
    let other = "-w-proj--claude-worktrees-old";
    std::fs::create_dir_all(b.code.join("projects").join(other)).unwrap();
    history(&b, other, "main", 3, "stale");
    run(&lab, &lab.request(&a, &b, "main"), "other-slug");
    assert!(
        !b.code
            .join("projects")
            .join(other)
            .join("main.jsonl")
            .exists()
    );
    assert!(
        b.code
            .join("projects")
            .join(SLUG)
            .join("main.jsonl")
            .is_file()
    );
}

/// Review fixes: a record AI Juggler can't read that names the family
/// refuses the move (it might be a member it would leave behind).
#[test]
fn an_unreadable_record_naming_the_family_refuses() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    record(&a, "local_main", "main", None, false, &lab.cwd());
    history(&a, SLUG, "main", 2, "a");
    std::fs::write(
        a.folder.join("local_odd.json"),
        r#"{"spawnedFrom":{"sessionId":"local_main"},"#,
    )
    .unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.words.contains("can't be read")),
        "{:?}",
        plan.refusals
    );
    // One that names nothing of the family is only noted: it adds no
    // refusal to the ones the same family has without it.
    std::fs::remove_file(a.folder.join("local_odd.json")).unwrap();
    let without = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    std::fs::write(a.folder.join("local_odd.json"), "{torn").unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert_eq!(plan.refusals, without.refusals);
    assert!(
        plan.notes.iter().any(|n| n.contains("doesn't know")),
        "{:?}",
        plan.notes
    );
    // A disk without clones refuses every move for its Undo backups
    // (Linux CI's ext4); only where it clones is that the whole list.
    if clones() {
        assert!(plan.can_move(), "{:?}", plan.refusals);
    }
}

/// D-128: no version number gates a move. The Claude app 5.x's records
/// (2026-11-18): a side session carries a start chain naming the first
/// record of its chain, and an archived-sessions hint sits beside the
/// records. On a new major whose shape fits, the family moves: the
/// moved conversation's own links to a parent left behind go (its start
/// chain too), a target child's chain is re-pointed at the moved record,
/// and both folders' hints are put aside for the app to make again.
#[test]
fn a_new_major_whose_records_fit_moves_with_its_start_chain_and_hint() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    // The moved conversation was itself started from one that stays.
    record(&a, "local_top", "top", None, false, &cwd);
    std::fs::write(
        a.folder.join("local_main.json"),
        format!(
            r#"{{"sessionId":"local_main","cliSessionId":"main","cwd":"{cwd}","isArchived":false,"lastActivityAt":1,"spawnedFrom":{{"sessionId":"local_top","taskId":"t0","sideSession":true}},"startChain":{{"depth":1,"firstSessionId":"local_top"}}}}"#
        ),
    )
    .unwrap();
    std::fs::write(
        a.folder.join("local_kid.json"),
        format!(
            r#"{{"sessionId":"local_kid","cliSessionId":"kid","cwd":"{cwd}","isArchived":true,"lastActivityAt":2,"spawnedFrom":{{"sessionId":"local_main","taskId":"t1","sideSession":true}},"startChain":{{"depth":1,"firstSessionId":"local_main"}}}}"#
        ),
    )
    .unwrap();
    // The target has an older record of the same conversation (an alias)
    // and a child whose chain starts at it.
    record(&b, "local_old", "main", None, false, &cwd);
    std::fs::write(
        b.folder.join("local_bkid.json"),
        format!(
            r#"{{"sessionId":"local_bkid","cliSessionId":"bkid","cwd":"{cwd}","isArchived":true,"lastActivityAt":3,"spawnedFrom":{{"sessionId":"local_old","taskId":"t2","sideSession":true}},"startChain":{{"depth":1,"firstSessionId":"local_old"}}}}"#
        ),
    )
    .unwrap();
    for account in [&a, &b] {
        std::fs::write(
            account.folder.join("archived-sessions.idx"),
            r#"{"v":1,"archived":["local_kid"]}"#,
        )
        .unwrap();
        std::fs::write(
            account.folder.join("scheduled-tasks.json"),
            r#"{"scheduledTasks":[]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(account.folder.join("backlog")).unwrap();
    }
    history(&a, SLUG, "main", 2, "a");
    history(&a, SLUG, "kid", 2, "k");
    let mut request = lab.request(&a, &b, "main");
    request.source.version = Some("5.11492.5".into());
    request.target.version = Some("5.11492.5".into());
    let plan = ClaudeMove.plan(&request).unwrap();
    if !clones() {
        return; // Linux CI's ext4: the clone refusal is the whole list.
    }
    assert!(plan.can_move(), "{:?} {:?}", plan.refusals, plan.shape);
    assert!(plan.shape.is_empty(), "{:?}", plan.shape);
    let written = |id: &str| {
        plan.steps
            .iter()
            .find_map(|step| match step {
                MoveStep::WriteRecord { path, value } if path.ends_with(format!("{id}.json")) => {
                    Some(value.clone())
                }
                _ => None,
            })
            .expect("each record is written")
    };
    let main = written("local_main");
    assert!(
        main.get("spawnedFrom").is_none() && main.get("startChain").is_none(),
        "{main}"
    );
    assert_eq!(
        written("local_kid")["startChain"]["firstSessionId"],
        "local_main"
    );
    assert_eq!(
        written("local_bkid")["startChain"]["firstSessionId"],
        "local_main"
    );
    let hints: Vec<&PathBuf> = plan
        .steps
        .iter()
        .filter_map(|step| match step {
            MoveStep::Hold {
                path,
                why: HoldWhy::AppHint,
            } => Some(path),
            _ => None,
        })
        .collect();
    assert_eq!(hints.len(), 2, "{hints:?}");
    let mut txn = Transaction::begin(
        &lab.moves,
        "m-5x",
        "claude-code",
        request.clone(),
        plan.clone(),
        None,
        1,
    )
    .unwrap();
    txn.back_up(&Stopped::checked()).unwrap();
    txn.apply(&Stopped::checked(), 0).unwrap();
    let report = txn.verify_cold(ClaudeMove.verify(&request, &plan)).unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    assert!(!a.folder.join("archived-sessions.idx").exists());
    assert!(!b.folder.join("archived-sessions.idx").exists());
    txn.undo(&Stopped::checked(), 3).unwrap();
    assert!(
        a.folder.join("archived-sessions.idx").is_file(),
        "Undo brings the hints back"
    );
}

/// D-128: a record that names another under a field the move doesn't know
/// could tie the family together in a way the plan would cut (here a child
/// hangs from the moved conversation by a new field, not `spawnedFrom`):
/// refused, as one the user can't clear, and named, on the version that
/// was proven too.
#[test]
fn a_record_linked_by_a_field_the_move_doesnt_know_refuses() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    std::fs::write(
        a.folder.join("local_kid.json"),
        format!(
            r#"{{"sessionId":"local_kid","cliSessionId":"kid","cwd":"{cwd}","isArchived":false,"lastActivityAt":2,"family":{{"parentRecord":"local_main"}}}}"#
        ),
    )
    .unwrap();
    history(&a, SLUG, "main", 2, "a");
    history(&a, SLUG, "kid", 2, "k");
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(!plan.can_move(), "the child would be left behind");
    assert!(
        plan.refusals.iter().all(|r| r.kind == RefusalKind::Setup),
        "{:?}",
        plan.refusals
    );
    assert!(
        plan.shape
            .iter()
            .any(|what| what.contains("family.parentRecord")),
        "{:?}",
        plan.shape
    );
}

/// D-127: a target profile not signed in yet is the user's to clear, and
/// says how.
#[test]
fn a_target_not_signed_in_is_the_users_to_clear() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    record(&a, "local_main", "main", None, false, &lab.cwd());
    history(&a, SLUG, "main", 2, "a");
    std::fs::remove_file(b.code.join(".claude.json")).unwrap();
    std::fs::remove_dir_all(b.host.join("claude-code-sessions")).unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    let waits: Vec<_> = plan
        .refusals
        .iter()
        .filter(|r| r.kind == RefusalKind::UserCanClear)
        .collect();
    assert!(
        waits
            .iter()
            .any(|r| r.words.contains("Move becomes available once you sign in")),
        "{:?}",
        plan.refusals
    );
}

/// 5.x (2026-11-18): a member whose messages wait to be sent in the app
/// (`waiting-input/`) holds the move back, as the user's to clear; the
/// folder itself (empty, or another conversation's) doesn't.
#[test]
fn a_member_with_messages_waiting_to_be_sent_waits() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_other", "other", None, false, &cwd);
    history(&a, SLUG, "main", 2, "a");
    std::fs::create_dir_all(a.folder.join("waiting-input")).unwrap();
    std::fs::write(a.folder.join("waiting-input/local_other.json"), "{}").unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        !plan
            .refusals
            .iter()
            .any(|r| r.words.contains("waiting to be sent")),
        "{:?}",
        plan.refusals
    );
    assert!(plan.shape.is_empty(), "{:?}", plan.shape);
    std::fs::write(a.folder.join("waiting-input/local_main.json"), "{}").unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.kind == RefusalKind::UserCanClear
                && r.words.ends_with("Move becomes available once it's sent")),
        "{:?}",
        plan.refusals
    );
}

/// Review fixes: a member also started from a conversation outside the
/// family would keep a link to a record left behind: refused.
#[test]
fn a_second_parent_outside_the_family_refuses() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    record(&a, "local_other", "other", None, false, &cwd);
    std::fs::write(
        a.folder.join("local_kid.json"),
        r#"{"sessionId":"local_kid","cliSessionId":"kid","spawnedFrom":{"sessionId":"local_main"},"forkedFromSessionId":"local_other"}"#,
    )
    .unwrap();
    history(&a, SLUG, "main", 2, "a");
    history(&a, SLUG, "kid", 2, "k");
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.words.contains("isn't part of this family")),
        "{:?}",
        plan.refusals
    );
}

// ---- AJ-E2328: a move carries the worktree's ownership ------------------

/// A worktree of the lab's repo, made as git makes one: `<repo>/.git/
/// worktrees/<name>/HEAD` on `branch`, and the worktree's `.git` file
/// naming it. Returns the worktree's folder.
fn worktree(lab: &Lab, name: &str, branch: &str) -> PathBuf {
    let repo = lab.tmp.path().join("work/repo");
    let git = repo.join(".git/worktrees").join(name);
    std::fs::create_dir_all(&git).unwrap();
    std::fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap();
    let folder = repo.join(".claude/worktrees").join(name);
    std::fs::create_dir_all(folder.join("src")).unwrap();
    std::fs::write(folder.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
    std::fs::write(folder.join("src/work.rs"), b"fn main() {}\n").unwrap();
    folder
}

/// Points `folder`'s HEAD at `branch` (the work moved on, as git does).
fn checkout(lab: &Lab, name: &str, branch: &str) {
    let git = lab.tmp.path().join("work/repo/.git/worktrees").join(name);
    std::fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap();
}

/// A record of `cli` working in worktree `folder` on `branch`, granted
/// `grants` and with `shell` folders.
fn worktree_record(
    account: &Account,
    local: &str,
    cli: &str,
    parent: Option<&str>,
    folder: &Path,
    branch: &str,
    grants: &[String],
) {
    let mut value = serde_json::json!({
        "sessionId": local, "cliSessionId": cli, "title": format!("T {cli}"),
        "isArchived": false, "cwd": folder.display().to_string(), "lastActivityAt": 1000,
        "worktreeName": folder.file_name().unwrap().to_string_lossy(),
        "worktreePath": folder.display().to_string(), "branch": branch,
        "sessionPermissionUpdates": [{"type": "addDirectories", "destination": "session", "directories": grants}],
        "shellFolders": [folder.join("src").display().to_string()],
    });
    if let Some(parent) = parent {
        value["spawnedFrom"] = serde_json::json!({"sessionId": parent, "sideSession": true});
    }
    std::fs::write(
        account.folder.join(format!("{local}.json")),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

/// `host`'s worktree file, owning `leases` (name, folder, record, branch)
/// and having seen `seen`; with fields the move must keep as they are.
fn worktree_file(host: &Path, leases: &[(&str, &Path, &str, &str)], seen: &[&Path]) {
    let mut worktrees = serde_json::Map::new();
    for (name, folder, by, branch) in leases {
        worktrees.insert(
            (*name).to_owned(),
            serde_json::json!({
                "name": name, "path": folder.display().to_string(), "leasedBy": by,
                "branch": branch, "sourceBranch": "main", "createdAt": 1,
                "anchors": [{"gitRoot": "/Users/someone/repo"}], "autoTrusted": true,
                "futureLeaseField": {"kept": 1},
            }),
        );
    }
    let root = seen
        .first()
        .and_then(|p| p.parent())
        .map(|p| p.display().to_string());
    let mut roots = serde_json::Map::new();
    if let Some(root) = root {
        roots.insert(root, serde_json::json!(5));
    }
    let sightings: serde_json::Map<String, serde_json::Value> = seen
        .iter()
        .map(|p| (p.display().to_string(), serde_json::json!(7)))
        .collect();
    std::fs::write(
        host.join("git-worktrees.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 2, "originUrls": {"x": "y"}, "originPins": {},
            "worktrees": worktrees,
            "untrackedDirGc": {"sightings": sightings, "roots": roots, "cwds": {"/Users/someone/repo": 3}},
            "pendingTombstones": [],
        }))
        .unwrap(),
    )
    .unwrap();
}

/// What the app's cleanup of `host` would remove of `folder`: one it has
/// seen in a folder it looks after, without owning it (the rule its logs
/// show: "Removed untracked worktree dir").
fn sweep_would_remove(host: &Path, folder: &Path) -> bool {
    let file = read(&host.join("git-worktrees.json"));
    let key = folder.display().to_string();
    let seen = file["untrackedDirGc"]["sightings"].get(&key).is_some();
    let owned = file["worktrees"]
        .as_object()
        .is_some_and(|w| w.values().any(|e| e["path"] == key.as_str()));
    seen && !owned
}

fn hash_tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    files(dir)
        .into_iter()
        .map(|rel| {
            let bytes = std::fs::read(dir.join(&rel)).unwrap();
            (rel, bytes)
        })
        .collect()
}

/// R8: a session in its own worktree moves with its ownership: the target
/// owns the worktree (every unknown field kept), the source forgets it and
/// its cleanup's sighting of it, so the old profile's sweep removes
/// nothing; the new profile resumes in the same tree on the branch the
/// worktree is on; nothing inside the worktree changes. Undo restores both
/// profiles' files byte for byte.
#[test]
fn e2328_a_worktree_moves_with_its_session_and_the_old_sweep_removes_nothing() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-1", "claude/wt-1");
    let other = worktree(&lab, "wt-other", "claude/other");
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-1", &[]);
    history(&a, SLUG, "main", 2, "a");
    worktree_file(
        &a.host,
        &[
            ("wt-1", &wt, "local_main", "claude/wt-1"),
            ("wt-other", &other, "local_someone_else", "claude/other"),
        ],
        &[&wt],
    );
    let a_before = std::fs::read(a.host.join("git-worktrees.json")).unwrap();
    let tree_before = hash_tree(&wt);
    let (mut txn, plan) = run(&lab, &lab.request(&a, &b, "main"), "e2328-a");
    assert!(
        plan.carried.iter().any(|c| matches!(c,
            juggler_adapter::moving::Carried::Worktree { name, branch }
                if name == "wt-1" && branch.as_deref() == Some("claude/wt-1"))),
        "{:?}",
        plan.carried
    );
    let theirs = read(&b.host.join("git-worktrees.json"));
    assert_eq!(theirs["worktrees"]["wt-1"]["leasedBy"], "local_main");
    assert_eq!(theirs["worktrees"]["wt-1"]["futureLeaseField"]["kept"], 1);
    assert_eq!(theirs["schemaVersion"], 2);
    let ours = read(&a.host.join("git-worktrees.json"));
    assert!(
        ours["worktrees"].get("wt-1").is_none(),
        "the source owns it no more"
    );
    assert_eq!(
        ours["worktrees"]["wt-other"]["leasedBy"], "local_someone_else",
        "others stay"
    );
    assert_eq!(
        ours["originUrls"]["x"], "y",
        "unknown top-level fields kept"
    );
    assert!(ours["pendingTombstones"].is_array());
    assert!(
        !sweep_would_remove(&a.host, &wt),
        "the old profile's sweep removes nothing"
    );
    // The new profile resumes in the same tree: it owns it, on its branch.
    let record = read(&b.folder.join("local_main.json"));
    assert_eq!(record["worktreePath"], theirs["worktrees"]["wt-1"]["path"]);
    assert_eq!(record["branch"], theirs["worktrees"]["wt-1"]["branch"]);
    assert_eq!(
        hash_tree(&wt),
        tree_before,
        "nothing inside the worktree changed"
    );
    // Undo: both files back byte for byte, and no lease left in each.
    txn.undo(&Stopped::checked(), 9).unwrap();
    assert_eq!(
        std::fs::read(a.host.join("git-worktrees.json")).unwrap(),
        a_before
    );
    assert!(
        !b.host.join("git-worktrees.json").exists(),
        "the target had none; Undo leaves none"
    );
    assert_eq!(hash_tree(&wt), tree_before);
}

/// R8: the worktree's HEAD moved on from the branch the app had: the
/// carried entry and the record name the real branch (never a checkout),
/// and the plan says so; a worktree on no branch waits for the user.
#[test]
fn e2328_a_diverged_head_is_recorded_never_checked_out() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-2", "claude/wt-2");
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-2", &[]);
    history(&a, SLUG, "main", 2, "a");
    worktree_file(&b.host, &[], &[]);
    worktree_file(&a.host, &[("wt-2", &wt, "local_main", "claude/wt-2")], &[]);
    checkout(&lab, "wt-2", "release/0.5.1");
    let head = lab.tmp.path().join("work/repo/.git/worktrees/wt-2/HEAD");
    let head_before = std::fs::read(&head).unwrap();
    let request = lab.request(&a, &b, "main");
    let plan = ClaudeMove.plan(&request).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("release/0.5.1") && n.contains("nothing is checked out")),
        "{:?}",
        plan.notes
    );
    run(&lab, &request, "e2328-b");
    let theirs = read(&b.host.join("git-worktrees.json"));
    assert_eq!(theirs["worktrees"]["wt-2"]["branch"], "release/0.5.1");
    assert_eq!(
        read(&b.folder.join("local_main.json"))["branch"],
        "release/0.5.1"
    );
    assert_eq!(
        std::fs::read(&head).unwrap(),
        head_before,
        "HEAD never moved"
    );

    // On no branch: the user's to clear, nothing planned to change it.
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-3", "claude/wt-3");
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-3", &[]);
    history(&a, SLUG, "main", 2, "a");
    worktree_file(&a.host, &[("wt-3", &wt, "local_main", "claude/wt-3")], &[]);
    std::fs::write(
        lab.tmp.path().join("work/repo/.git/worktrees/wt-3/HEAD"),
        format!("{}\n", "c".repeat(40)),
    )
    .unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.kind == RefusalKind::UserCanClear && r.words.contains("isn't on a branch")),
        "{:?}",
        plan.refusals
    );
}

/// R8: the conversation's side-session notes move beside its record, its
/// side session's grant to them points into the target, and Undo brings
/// the notes back where they were.
#[test]
fn e2328_the_notes_folder_and_its_grants_move() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-4", "claude/wt-4");
    let notes = a.host.join("side-session-notes/local_main");
    std::fs::create_dir_all(notes.join("deep")).unwrap();
    std::fs::write(notes.join("plan.md"), b"the plan").unwrap();
    std::fs::write(notes.join("deep/log.txt"), b"a log").unwrap();
    let grant = notes.display().to_string();
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-4", &[]);
    worktree_record(
        &a,
        "local_side",
        "side",
        Some("local_main"),
        &wt,
        "claude/wt-4",
        &[grant],
    );
    history(&a, SLUG, "main", 2, "a");
    history(&a, SLUG, "side", 2, "s");
    let request = lab.request(&a, &b, "main");
    let plan = ClaudeMove.plan(&request).unwrap();
    assert!(
        plan.carried
            .iter()
            .any(|c| matches!(c, juggler_adapter::moving::Carried::Notes { files: 2, .. })),
        "{:?}",
        plan.carried
    );
    let (mut txn, _) = run(&lab, &request, "e2328-c");
    let moved = b.host.join("side-session-notes/local_main");
    assert_eq!(std::fs::read(moved.join("plan.md")).unwrap(), b"the plan");
    assert!(!notes.exists(), "the notes left the source");
    let side = read(&b.folder.join("local_side.json"));
    assert_eq!(
        side["sessionPermissionUpdates"][0]["directories"][0],
        moved.display().to_string(),
        "the grant follows the notes"
    );
    txn.undo(&Stopped::checked(), 9).unwrap();
    assert_eq!(std::fs::read(notes.join("deep/log.txt")).unwrap(), b"a log");
    assert!(!moved.exists());
}

/// R8: Undo restores both profiles' ownership: the source owns the
/// worktree again, the target's own entries are as they were, byte for
/// byte, never one entry in each.
#[test]
fn e2328_undo_restores_both_leases() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-5", "claude/wt-5");
    let theirs = worktree(&lab, "wt-b", "claude/b");
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-5", &[]);
    history(&a, SLUG, "main", 2, "a");
    worktree_file(
        &a.host,
        &[("wt-5", &wt, "local_main", "claude/wt-5")],
        &[&wt],
    );
    worktree_file(
        &b.host,
        &[("wt-b", &theirs, "local_b_own", "claude/b")],
        &[&wt],
    );
    let (a_before, b_before) = (
        std::fs::read(a.host.join("git-worktrees.json")).unwrap(),
        std::fs::read(b.host.join("git-worktrees.json")).unwrap(),
    );
    let (mut txn, _) = run(&lab, &lab.request(&a, &b, "main"), "e2328-d");
    let both = read(&b.host.join("git-worktrees.json"));
    assert_eq!(both["worktrees"]["wt-5"]["leasedBy"], "local_main");
    assert_eq!(both["worktrees"]["wt-b"]["leasedBy"], "local_b_own");
    txn.undo(&Stopped::checked(), 9).unwrap();
    assert_eq!(
        std::fs::read(a.host.join("git-worktrees.json")).unwrap(),
        a_before
    );
    assert_eq!(
        std::fs::read(b.host.join("git-worktrees.json")).unwrap(),
        b_before
    );
}

/// R8/R1/R9: a worktree file of a schema it doesn't know, or a Claude app
/// version the known formats weren't checked on, refuses the move before
/// anything, in words that say to update AI Juggler (never a half-carried
/// move).
#[test]
fn e2328_the_format_gate_refuses_an_unknown_schema_and_version() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-6", "claude/wt-6");
    worktree_record(&a, "local_main", "main", None, &wt, "claude/wt-6", &[]);
    history(&a, SLUG, "main", 2, "a");
    std::fs::write(
        a.host.join("git-worktrees.json"),
        br#"{"schemaVersion": 3, "leases": {"wt-6": "local_main"}}"#,
    )
    .unwrap();
    let plan = ClaudeMove.plan(&lab.request(&a, &b, "main")).unwrap();
    assert!(!plan.can_move());
    assert!(
        plan.refusals.iter().any(|r| r.kind == RefusalKind::Setup
            && r.words.contains("worktrees")
            && r.words.contains("update AI Juggler")),
        "{:?}",
        plan.refusals
    );
    assert!(!plan.shape.is_empty(), "named for the gate watch");
    assert!(
        !plan
            .steps
            .iter()
            .any(|s| matches!(s, MoveStep::WriteAppFile { .. })),
        "nothing of the worktree carried"
    );
    // A version the formats weren't checked on.
    std::fs::remove_file(a.host.join("git-worktrees.json")).unwrap();
    let mut request = lab.request(&a, &b, "main");
    request.target.version = Some("5.11610.2".into());
    let plan = ClaudeMove.plan(&request).unwrap();
    assert!(
        plan.refusals
            .iter()
            .any(|r| r.words.contains("5.11610.2") && r.words.contains("update AI Juggler")),
        "{:?}",
        plan.refusals
    );
}

/// R8/R7: a session a 0.4.x move left split (its record in profile B, its
/// worktree still owned by profile A, the branch moved on): the health
/// check finds it; the repair moves the ownership to B, drops A's sighting,
/// records the real branch, never touches the worktree's files, and is
/// undone byte for byte.
#[test]
fn e2328_the_repair_puts_a_04x_split_right() {
    needs_clones!();
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let wt = worktree(&lab, "wt-7", "claude/wt-7");
    // As the 0.4.x move left it: the record in B, the lease in A.
    worktree_record(&b, "local_main", "main", None, &wt, "claude/wt-7", &[]);
    worktree_file(
        &a.host,
        &[("wt-7", &wt, "local_main", "claude/wt-7")],
        &[&wt],
    );
    worktree_file(&b.host, &[], &[]);
    checkout(&lab, "wt-7", "release/0.5.1");
    let tree_before = hash_tree(&wt);
    let hosts = vec![a.host.clone(), b.host.clone()];
    let found = ClaudeMove.splits(&hosts).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    let split = &found[0];
    assert_eq!(split.owner, a.host);
    assert_eq!(split.holder, b.host);
    assert_eq!(split.conversation, "main");
    assert!(split.branch_differs);
    assert_eq!(split.branch.as_deref(), Some("release/0.5.1"));
    // The repair refuses an unchecked version, plainly.
    let refused = ClaudeMove.repair(split, Some("5.11610.2"));
    assert!(!refused.can_move());
    assert!(
        refused.refusals[0]
            .words
            .contains("Repair becomes available")
    );
    let plan = ClaudeMove.repair(split, Some("5.11492.5"));
    assert!(plan.can_move(), "{:?}", plan.refusals);
    for step in &plan.steps {
        for path in step.paths() {
            assert!(
                !path.starts_with(&wt),
                "never a step inside the worktree: {path:?}"
            );
        }
    }
    let a_before = std::fs::read(a.host.join("git-worktrees.json")).unwrap();
    let mut txn = Transaction::begin(
        &lab.moves,
        "e2328-repair",
        "claude-code",
        lab.request(&a, &b, "main"),
        plan,
        None,
        1,
    )
    .unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    assert!(txn.verify_cold(Vec::new()).unwrap().passed());
    let owner = read(&b.host.join("git-worktrees.json"));
    assert_eq!(owner["worktrees"]["wt-7"]["leasedBy"], "local_main");
    assert_eq!(owner["worktrees"]["wt-7"]["branch"], "release/0.5.1");
    assert_eq!(
        read(&b.folder.join("local_main.json"))["branch"],
        "release/0.5.1"
    );
    assert!(
        read(&a.host.join("git-worktrees.json"))["worktrees"]
            .get("wt-7")
            .is_none()
    );
    assert!(!sweep_would_remove(&a.host, &wt));
    assert!(
        ClaudeMove.splits(&hosts).unwrap().is_empty(),
        "nothing left to repair"
    );
    assert_eq!(
        hash_tree(&wt),
        tree_before,
        "the worktree's files never touched"
    );
    txn.undo(&stopped, 9).unwrap();
    assert_eq!(
        std::fs::read(a.host.join("git-worktrees.json")).unwrap(),
        a_before
    );
    assert_eq!(
        ClaudeMove.splits(&hosts).unwrap().len(),
        1,
        "undone: as it was"
    );
}
