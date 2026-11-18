//! AJ-E2023: notices from the remote file: plain, capped to what the facts
//! justify, linked only to our website, dismissed for good on this Mac.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::dto::{RemoteNoticeInfo, RemoteNoticeLevel};
use super::{DISMISSED_KEY, MAX_DISMISSED, active, dismissed, remember_dismissed};
use crate::features::remote_config::Snapshot;
use crate::features::remote_config::dto::RemoteLayerInfo;
use crate::features::remote_config::snapshot_fixtures::{facts, file, registry};

const NOW: u64 = 1_800_000_100;

fn snapshot(items: Value) -> Snapshot {
    snapshot_with(json!({ "notices": { "schema": 1, "items": items } }))
}

fn snapshot_with(sections: Value) -> Snapshot {
    Snapshot::build(&registry(), Some(&file(sections)), &facts(1))
}

fn shown(items: Value) -> Vec<RemoteNoticeInfo> {
    active(&snapshot(items), &BTreeSet::new(), NOW)
}

fn one(item: Value) -> RemoteNoticeInfo {
    let mut notices = shown(json!([item]));
    assert_eq!(notices.len(), 1, "{notices:?}");
    notices.remove(0)
}

#[test]
fn a_notice_is_read_whole() {
    let notice = one(json!({
        "id": "tool-a-2-3", "level": "needs_you",
        "when": { "tools": { "tool-a": { "min": "2.3.0" } } },
        "title": "Tool A 2.3 changed how sessions resume",
        "body": "Switching back into Tool A may take a second try.",
        "link": "https://figurehead.ai/docs/known-issues#tool-a-2-3",
        "link_label": "Read more"
    }));
    assert_eq!(
        notice,
        RemoteNoticeInfo {
            id: "tool-a-2-3".to_owned(),
            level: RemoteNoticeLevel::NeedsYou,
            title: "Tool A 2.3 changed how sessions resume".to_owned(),
            body: Some("Switching back into Tool A may take a second try.".to_owned()),
            link: Some("https://figurehead.ai/docs/known-issues#tool-a-2-3".to_owned()),
            link_label: Some("Read more".to_owned()),
            dismissable: true,
        }
    );
}

#[test]
fn a_notice_is_never_louder_than_the_facts_justify() {
    let level = |when: Value, asked: &str| {
        let mut item = json!({ "id": "n", "level": asked, "title": "t" });
        if !when.is_null() {
            item["when"] = when;
        }
        one(item).level
    };
    let tools = json!({ "tools": { "tool-a": { "min": "2.0.0" } } });
    let app = json!({ "app": { "max": "0.6.0" } });
    let os = json!({ "os": ["macos"] });
    let channel = json!({ "channels": ["stable"] });
    let bucket = json!({ "buckets": [0, 99] });
    // Broken: only when the targeting names the versions this Mac runs.
    assert_eq!(level(tools.clone(), "broken"), RemoteNoticeLevel::Broken);
    assert_eq!(level(app.clone(), "broken"), RemoteNoticeLevel::Broken);
    assert_eq!(level(os.clone(), "broken"), RemoteNoticeLevel::NeedsYou);
    assert_eq!(level(Value::Null, "broken"), RemoteNoticeLevel::Info);
    assert_eq!(level(channel.clone(), "broken"), RemoteNoticeLevel::Info);
    assert_eq!(level(bucket, "broken"), RemoteNoticeLevel::Info);
    // Needs you: only when it targets this setup at all.
    assert_eq!(level(os, "needs_you"), RemoteNoticeLevel::NeedsYou);
    assert_eq!(level(tools, "needs_you"), RemoteNoticeLevel::NeedsYou);
    assert_eq!(level(Value::Null, "needs_you"), RemoteNoticeLevel::Info);
    assert_eq!(level(channel, "needs_you"), RemoteNoticeLevel::Info);
    // Never louder than asked.
    assert_eq!(level(app, "info"), RemoteNoticeLevel::Info);
    // A note can always be dismissed, whatever the file says.
    let note = one(json!({ "id": "n", "level": "broken", "title": "t", "dismissable": false }));
    assert_eq!(note.level, RemoteNoticeLevel::Info);
    assert!(note.dismissable);
}

#[test]
fn a_link_to_a_host_not_allowed_is_dropped() {
    for link in [
        "https://evil.example/x",
        "https://figurehead.ai.evil.example/x",
        "http://figurehead.ai/x",
        "https://figurehead.ai@evil.example/x",
        "https://figurehead.ai:444/x",
        "javascript:alert(1)",
        "file:///etc/passwd",
    ] {
        let notice = one(json!({
            "id": "n", "level": "info", "title": "t", "link": link, "link_label": "Open"
        }));
        assert_eq!(notice.link, None, "{link}");
        assert_eq!(notice.link_label, None, "{link}: no label without a link");
    }
    let kept = one(
        json!({ "id": "n", "level": "info", "title": "t", "link": "https://www.figurehead.ai/x" }),
    );
    assert_eq!(kept.link.as_deref(), Some("https://www.figurehead.ai/x"));
}

#[test]
fn notices_are_plain_targeted_and_expire() {
    let notices = shown(json!([
        { "id": "markup", "level": "info", "title": "<img src=x onerror=alert(1)>" },
        { "id": "beta-only", "level": "info", "title": "t", "when": { "channels": ["beta"] } },
        { "id": "expired", "level": "info", "title": "t", "expires_at": NOW },
        { "id": "later", "level": "info", "title": "t", "expires_at": NOW + 1 },
        { "id": "newer-level", "level": "critical", "title": "t" },
        { "id": "plain", "level": "info", "title": "Plain words — fine" }
    ]));
    let ids: Vec<&str> = notices.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["later", "plain"]);
}

#[test]
fn louder_notices_come_first() {
    let notices = shown(json!([
        { "id": "a", "level": "info", "title": "t" },
        { "id": "b", "level": "broken", "title": "t", "when": { "app": { "min": "0.5.0" } } },
        { "id": "c", "level": "needs_you", "title": "t", "when": { "os": ["macos"] } },
        { "id": "d", "level": "info", "title": "t" }
    ]));
    let ids: Vec<&str> = notices.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["b", "c", "a", "d"]);
}

#[test]
fn the_kill_switch_silences_every_notice() {
    let items = json!([{ "id": "a", "level": "info", "title": "t" }]);
    let killed = snapshot_with(json!({
        "flags": { "schema": 1, "values": { "notices_on": false } },
        "notices": { "schema": 1, "items": items }
    }));
    assert!(active(&killed, &BTreeSet::new(), NOW).is_empty());
}

#[test]
fn a_missing_or_invalid_file_gives_no_notices() {
    let none = Snapshot::build(&registry(), None, &facts(1));
    assert!(active(&none, &BTreeSet::new(), NOW).is_empty());
    for notices in [
        json!("x"),
        json!({ "schema": 1, "items": "x" }),
        json!({ "schema": 2, "items": [{ "id": "a", "level": "info", "title": "t" }] }),
    ] {
        let snapshot = snapshot_with(json!({ "notices": notices.clone() }));
        assert!(
            active(&snapshot, &BTreeSet::new(), NOW).is_empty(),
            "{notices}"
        );
    }
}

#[test]
fn about_lists_each_value_and_record_with_its_layer() {
    let snapshot = snapshot_with(json!({
        "flags": { "schema": 1, "values": { "notices_on": false } },
        "notices": { "schema": 1, "items": [{ "id": "a", "level": "info", "title": "t" }] }
    }));
    let about = snapshot.about();
    assert!(about.verified);
    let file = about.file.unwrap();
    assert_eq!((file.serial, file.channel.as_str()), (418, "stable"));
    assert_eq!(file.issued_at_ms, 1_800_000_000_000);
    let flag = about
        .values
        .iter()
        .find(|v| v.section == "flags" && v.control == "notices_on")
        .unwrap();
    assert_eq!(
        (flag.value.as_str(), flag.layer),
        ("off", RemoteLayerInfo::Public)
    );
    // The adapters' compat values are there too, at their defaults.
    assert!(
        about
            .values
            .iter()
            .any(|v| v.section == "compat" && v.layer == RemoteLayerInfo::Default)
    );
    assert_eq!(about.records.len(), 1);
    assert_eq!(about.records[0].id, "a");
    // With no file: nothing verified, every value built in.
    let none = Snapshot::build(&registry(), None, &facts(1)).about();
    assert!(!none.verified && none.file.is_none() && none.records.is_empty());
    assert!(
        none.values
            .iter()
            .all(|v| v.layer == RemoteLayerInfo::Default)
    );
}

#[test]
fn a_dismissal_persists() {
    let data = tempfile::tempdir().unwrap();
    let items = json!([
        { "id": "a", "level": "info", "title": "t" },
        { "id": "b", "level": "info", "title": "t" },
        { "id": "stays", "level": "broken", "title": "t", "dismissable": false,
          "when": { "app": { "min": "0.5.0" } } }
    ]);
    {
        let state = crate::test_support::bare_state(data.path().to_path_buf());
        remember_dismissed(&state, "a").unwrap();
        // Not an id: refused, nothing kept.
        assert!(remember_dismissed(&state, "A b").is_err());
        assert!(remember_dismissed(&state, "").is_err());
    }
    // A new launch over the same data still has it.
    let state = crate::test_support::bare_state(data.path().to_path_buf());
    let gone = dismissed(&state).unwrap();
    assert_eq!(gone, BTreeSet::from(["a".to_owned()]));
    let ids: Vec<String> = active(&snapshot(items.clone()), &gone, NOW)
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(ids, ["stays", "b"]);
    // A notice that can't be dismissed shows even if its id was.
    let all = BTreeSet::from(["a".to_owned(), "b".to_owned(), "stays".to_owned()]);
    let ids: Vec<String> = active(&snapshot(items), &all, NOW)
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(ids, ["stays"]);
}

#[test]
fn dismissals_are_kept_newest_first_and_bounded() {
    let data = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(data.path().to_path_buf());
    for i in 0..MAX_DISMISSED + 5 {
        remember_dismissed(&state, &format!("n{i}")).unwrap();
    }
    // Dismissing one again moves it to the newest: the next one in
    // pushes out the oldest (n6), not it.
    remember_dismissed(&state, "n5").unwrap();
    remember_dismissed(&state, "fresh").unwrap();
    let kept = dismissed(&state).unwrap();
    assert_eq!(kept.len(), MAX_DISMISSED);
    assert!(kept.contains("n5") && kept.contains("fresh"));
    assert!(!kept.contains("n4") && !kept.contains("n6"));
    // A damaged setting reads as nothing dismissed, never an error.
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(DISMISSED_KEY, "{not json")
        .unwrap();
    assert!(dismissed(&state).unwrap().is_empty());
}
