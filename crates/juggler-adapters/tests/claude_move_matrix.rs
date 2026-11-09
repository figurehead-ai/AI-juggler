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
    InPlaceMove, MemberSide, MovePlan, MoveRequest, MoveSide, NotesChoice,
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
            version: Some("4.16336.0".into()),
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

/// Row 9: what isn't proven is refused before anything: an app version not
/// proven, a link, a hard-linked history, app to terminal, another disk.
#[test]
fn row09_unsupported_version_link_and_device() {
    let lab = Lab::new();
    let (a, b) = (lab.account("a"), lab.account("b"));
    let cwd = lab.cwd();
    record(&a, "local_main", "main", None, false, &cwd);
    let main = history(&a, SLUG, "main", 2, "a");
    // A version not proven.
    let mut request = lab.request(&a, &b, "main");
    request.target.version = Some("5.0.0".into());
    assert!(
        ClaudeMove
            .plan(&request)
            .unwrap()
            .refusals
            .iter()
            .any(|r| r.contains("proven for version"))
    );
    // App to terminal.
    let mut request = lab.request(&a, &b, "main");
    request.target.host_dir = None;
    assert!(
        ClaudeMove
            .plan(&request)
            .unwrap()
            .refusals
            .iter()
            .any(|r| r.contains("isn't proven yet"))
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
            plan.refusals.iter().any(|r| r.contains("hard link")),
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
            .any(|r| r.contains("another disk") || r.contains("clone")),
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
        plan.refusals.iter().any(|r| r.contains("can't be read")),
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
            .any(|r| r.contains("isn't part of this family")),
        "{:?}",
        plan.refusals
    );
}
