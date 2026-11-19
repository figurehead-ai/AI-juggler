//! ADR-0035 §11.5: the publisher's check of every channel's `models`
//! section against `config/remote/models-ledger.json`, run as a test so a
//! source that breaks it can't merge.
//!
//! An entry is refused unless:
//! - the ledger has its id, with the same engine and the same file (URL,
//!   hash, size, mirror);
//! - every verdict is the ledger's or lower (a file may narrow without a
//!   new test, never raise);
//! - a job other than attention passes;
//! - each passing job's evidence exists and Opus confirmed it;
//! - its basis is a full harness run, or round 3's after-fix floor for
//!   the six ids that basis names (D-131);
//! - its words are plain (no link, no internal terms).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Job, ModelEntry, Verdict, read_raw};

/// Words a user-facing line may not hold: links and the test's own terms.
const NOT_PLAIN: &[&str] = &[
    "://", "harness", "critical", "floor", "gguf", "sha256", "sha-256", "quant", "q4_", "q8_",
    "pack", "target", "d-1",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn neutral_facts() -> juggler_remote_config::Facts {
    juggler_remote_config::Facts {
        app_version: juggler_remote_config::Version::new(0, 5, 0),
        channel: juggler_remote_config::Channel::Stable,
        os: juggler_remote_config::Os::Macos,
        os_version: None,
        tools: BTreeMap::new(),
        install: juggler_remote_config::InstallId::from_bytes([0; 16]),
    }
}

fn verdict_named(name: &str) -> Option<Verdict> {
    Verdict::ALL
        .iter()
        .find(|(_, known)| *known == name)
        .map(|(verdict, _)| *verdict)
}

fn plain(text: &str) -> bool {
    let lower = text.to_lowercase();
    !NOT_PLAIN.iter().any(|word| lower.contains(word))
}

/// Every problem with `section` (a channel's raw `models` value) against
/// `ledger`, in words; empty when it may be published. `root` is where
/// evidence paths resolve.
pub(super) fn check(section: &Value, ledger: &Value, root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let raw_items = section["items"].as_array().cloned().unwrap_or_default();
    // Targeting is the app's business; the publisher reads every record.
    let stripped: Vec<Value> = raw_items
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if let Some(object) = item.as_object_mut() {
                object.remove("when");
            }
            item
        })
        .collect();
    let read = read_raw(
        &json!({ "schema": section["schema"], "items": stripped }),
        &neutral_facts(),
    )
    .unwrap_or_default();
    if read.len() != raw_items.len() {
        problems.push(format!(
            "{} of {} entries fail the record check",
            raw_items.len() - read.len(),
            raw_items.len()
        ));
    }
    let lines: BTreeMap<&str, &Value> = ledger["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|line| Some((line["id"].as_str()?, line)))
        .collect();
    for entry in &read {
        let Some(line) = lines.get(entry.id.as_str()) else {
            problems.push(format!("{}: not in the ledger", entry.id));
            continue;
        };
        problems.extend(
            check_entry(entry, line, ledger, root)
                .into_iter()
                .map(|p| format!("{}: {p}", entry.id)),
        );
    }
    problems
}

fn check_entry(entry: &ModelEntry, line: &Value, ledger: &Value, root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let engine = super::Engine::ALL
        .iter()
        .find(|(engine, _)| *engine == entry.engine)
        .map(|(_, name)| *name);
    if line["engine"].as_str() != engine {
        problems.push("another engine than the ledger's".to_owned());
    }
    // The same file, or none for Apple's model.
    let file = &line["file"];
    match &entry.download {
        None if file.is_null() || line.get("file").is_none() => {}
        None => problems.push("the ledger has a file for it".to_owned()),
        Some(_) if file.is_null() => {
            problems.push("the ledger records no file for it yet".to_owned());
        }
        Some(download) => {
            let same = file["url"].as_str() == Some(download.url.as_str())
                && file["sha256"].as_str() == Some(download.sha256.as_str())
                && file["size_bytes"].as_u64() == Some(download.size_bytes)
                && file["mirror_url"].as_str() == download.mirror_url.as_deref();
            if !same {
                problems.push("a changed file under an old id (a new file is a new id)".to_owned());
            }
            if !plain(&download.licence_name) {
                problems.push("licence_name isn't plain".to_owned());
            }
        }
    }
    // The basis.
    let basis = line["basis"].as_str().unwrap_or_default();
    match basis {
        "harness" => {}
        "round3_after_fix_floor" => {
            let allowed = ledger["bases"]["round3_after_fix_floor"]["ids"]
                .as_array()
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(&entry.id)));
            if !allowed {
                problems.push("round 3's basis covers only its six models".to_owned());
            }
        }
        _ => problems.push("no basis recorded".to_owned()),
    }
    // Verdicts: the ledger's or lower, evidence for each pass.
    for (job, name) in Job::ALL {
        let recorded = &line["jobs"][name];
        let ledger_verdict = recorded["verdict"]
            .as_str()
            .and_then(verdict_named)
            .unwrap_or_default();
        let verdict = entry.verdict(job);
        if verdict > ledger_verdict {
            problems.push(format!("{name}: raised above the ledger's verdict"));
        }
        if verdict.passed() {
            let evidence = recorded["evidence"].as_str().unwrap_or_default();
            if evidence.is_empty() || !root.join(evidence).is_file() {
                problems.push(format!("{name}: no evidence at {evidence:?}"));
            }
            if recorded["confirmed_by"].as_str() != Some("opus") {
                problems.push(format!("{name}: not confirmed by Opus"));
            }
        }
        if let Some(note) = &entry.verdicts[&job].note
            && !plain(note)
        {
            problems.push(format!("{name}_note isn't plain"));
        }
    }
    if !entry.passes_any_job() {
        problems.push("passes no job but attention".to_owned());
    }
    if !plain(&entry.name) {
        problems.push("name isn't plain".to_owned());
    }
    problems
}

fn ledger() -> Value {
    let text = std::fs::read_to_string(root().join("config/remote/models-ledger.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn channel_section(channel: &str) -> Option<Value> {
    let text =
        std::fs::read_to_string(root().join(format!("config/remote/{channel}.json"))).unwrap();
    let source: Value = serde_json::from_str(&text).unwrap();
    source["sections"].get(super::SECTION).cloned()
}

#[test]
fn every_channel_source_passes_the_ledger() {
    let ledger = ledger();
    for channel in ["nightly", "beta", "stable"] {
        if let Some(section) = channel_section(channel) {
            let problems = check(&section, &ledger, &root());
            assert!(problems.is_empty(), "{channel}: {problems:?}");
        }
    }
}

#[test]
fn the_ledger_states_round_3s_basis_honestly() {
    let ledger = ledger();
    let basis = &ledger["bases"]["round3_after_fix_floor"];
    assert_eq!(basis["ids"].as_array().map(Vec::len), Some(6));
    assert_eq!(basis["commit"], "c8b4ec881");
    let statement = basis["statement"].as_str().unwrap();
    for words in [
        "after-fix floor",
        "not on a fresh blind run",
        "full harness run",
    ] {
        assert!(statement.contains(words), "{words}");
    }
    // Qwen3-4B has no file until our conversion is recorded.
    let lines = ledger["models"].as_array().unwrap();
    let qwen4b = lines
        .iter()
        .find(|line| line["id"] == "qwen3-4b-2507-q4km")
        .unwrap();
    assert!(qwen4b["file"].is_null());
}

/// The nightly section with one entry changed by `change`.
fn changed(id: &str, change: impl Fn(&mut serde_json::Map<String, Value>)) -> Value {
    let mut section = channel_section("nightly").unwrap();
    for item in section["items"].as_array_mut().unwrap() {
        if item["id"] == id {
            change(item.as_object_mut().unwrap());
        }
    }
    section
}

fn refused(section: &Value, ledger: &Value, expect: &str) {
    let problems = check(section, ledger, &root());
    assert!(
        problems.iter().any(|p| p.contains(expect)),
        "{expect}: {problems:?}"
    );
}

#[test]
fn the_publisher_refuses_what_the_ledger_does_not_back() {
    let ledger = ledger();
    let qwen = "qwen3-1-7b-q8";
    // A raised verdict.
    refused(
        &changed(qwen, |e| {
            e.insert("feed".into(), json!("beta"));
        }),
        &ledger,
        "feed: raised",
    );
    // A new file under an old id (a downgraded entry).
    refused(
        &changed(qwen, |e| {
            e.insert("sha256".into(), json!("0".repeat(64)));
        }),
        &ledger,
        "changed file",
    );
    // Only attention passing.
    refused(
        &changed(qwen, |e| {
            e.insert("review".into(), json!("not_yet"));
            e.insert("chat".into(), json!("not_yet"));
        }),
        &ledger,
        "passes no job",
    );
    // Words that aren't plain.
    refused(
        &changed(qwen, |e| {
            e.insert(
                "chat_note".into(),
                json!("See https://example.test for the harness"),
            );
        }),
        &ledger,
        "chat_note isn't plain",
    );
    // An id the ledger doesn't have.
    refused(
        &changed(qwen, |e| {
            e.insert("id".into(), json!("someone-else"));
        }),
        &ledger,
        "not in the ledger",
    );
    // A narrowed verdict is fine without a new test.
    let narrowed = changed(qwen, |e| {
        e.insert("chat".into(), json!("not_yet"));
    });
    assert!(check(&narrowed, &ledger, &root()).is_empty());
}

#[test]
fn the_ledger_must_back_each_pass() {
    let section = channel_section("nightly").unwrap();
    let qwen = "qwen3-1-7b-q8";
    let edit = |change: &dyn Fn(&mut Value)| {
        let mut ledger = ledger();
        for line in ledger["models"].as_array_mut().unwrap() {
            if line["id"] == qwen {
                change(line);
            }
        }
        ledger
    };
    refused(
        &section,
        &edit(&|line| line["jobs"]["review"]["evidence"] = json!("docs/qa/evidence/none.md")),
        "review: no evidence",
    );
    refused(
        &section,
        &edit(&|line| line["jobs"]["chat"]["confirmed_by"] = json!("sonnet")),
        "chat: not confirmed by Opus",
    );
    // Round 3's basis covers its six ids only.
    let mut ledger = ledger();
    ledger["bases"]["round3_after_fix_floor"]["ids"] = json!(["apple-on-device"]);
    refused(&section, &ledger, "round 3's basis");
    // No file recorded yet (our own conversion waits for it).
    refused(
        &section,
        &edit(&|line| line["file"] = Value::Null),
        "no file for it yet",
    );
}
