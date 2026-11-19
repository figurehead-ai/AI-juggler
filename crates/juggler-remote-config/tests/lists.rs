//! ADR-0035 §2 and §5: list sections (records such as short messages),
//! each field as closed as a control, targeted on this computer, tolerant
//! by record, strict for the publisher.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

use std::collections::BTreeMap;

use juggler_remote_config::schema::RegistryError;
use juggler_remote_config::{
    Channel, Facts, Field, FieldKind, InstallId, ListSpec, Os, Registry, RemoteMode, SectionIssue,
    SectionSpec, Version,
};
use serde_json::{Value, json};

/// A made-up module's list; this crate knows no real feature.
fn spec() -> ListSpec {
    ListSpec::new(3)
        .field(
            "kind",
            Field::required(FieldKind::Choice {
                options: vec!["low".to_owned(), "high".to_owned()],
            }),
        )
        .field("title", Field::required(FieldKind::Text { max_chars: 20 }))
        .field(
            "link",
            Field::optional(FieldKind::Link {
                hosts: vec!["example.test".to_owned()],
            }),
        )
        .field("quiet", Field::optional(FieldKind::Flag))
        .field("until", Field::optional(FieldKind::Time))
        .field(
            "size",
            Field::optional(FieldKind::Count {
                min: 1,
                max: 8_589_934_592,
            }),
        )
        .field("hash", Field::optional(FieldKind::Sha256))
}

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(SectionSpec::new("messages", 1).list(spec()))
        .unwrap();
    registry
        .register(SectionSpec::new("flags", 1).switch("x", true, RemoteMode::OffOnly))
        .unwrap();
    registry
}

fn facts() -> Facts {
    Facts {
        app_version: "0.5.0".parse::<Version>().unwrap(),
        channel: Channel::Stable,
        os: Os::Macos,
        os_version: None,
        tools: BTreeMap::from([("tool-a".to_owned(), "2.3.0".parse().unwrap())]),
        install: InstallId::from_bytes([7; 16]),
    }
}

fn items(items: Value) -> (Vec<juggler_remote_config::Item>, Vec<SectionIssue>) {
    let sections = BTreeMap::from([(
        "messages".to_owned(),
        json!({ "schema": 1, "items": items }),
    )]);
    let (mut lists, issues) = registry().resolve_lists(&sections, &facts());
    (lists.remove("messages").unwrap_or_default(), issues)
}

fn ids(read: &[juggler_remote_config::Item]) -> Vec<&str> {
    read.iter().map(|item| item.id.as_str()).collect()
}

#[test]
fn records_are_read_with_their_typed_fields() {
    let (read, issues) = items(json!([{
        "id": "a-1", "kind": "high", "title": "Tool A 2.3",
        "link": "https://example.test/notes#a", "quiet": true, "until": 1_900_000_000u64,
        "future_field": {"anything": 1}
    }]));
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(ids(&read), ["a-1"]);
    let item = &read[0];
    assert_eq!(item.choice("kind"), Some("high"));
    assert_eq!(item.text("title"), Some("Tool A 2.3"));
    assert_eq!(item.link("link"), Some("https://example.test/notes#a"));
    assert_eq!(item.flag("quiet"), Some(true));
    assert_eq!(item.time("until"), Some(1_900_000_000));
    assert!(
        !item.fields.contains_key("future_field"),
        "unknown fields are ignored"
    );
}

#[test]
fn counts_and_hashes_are_closed() {
    let hash = "061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a";
    let (read, issues) = items(json!([{
        "id": "a", "kind": "low", "title": "t", "size": 1_834_426_016u64, "hash": hash
    }]));
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(read[0].count("size"), Some(1_834_426_016));
    assert_eq!(read[0].sha256("hash"), Some(hash));
    for (size, why) in [
        (json!(0), "under min"),
        (json!(8_589_934_593u64), "over max"),
        (json!(-1), "negative"),
        (json!(1.5), "not whole"),
        (json!("12"), "a string"),
    ] {
        let (read, issues) =
            items(json!([{ "id": "a", "kind": "low", "title": "t", "size": size }]));
        assert_eq!(read[0].count("size"), None, "{why}");
        assert_eq!(issues.len(), 1, "{why}");
    }
    for bad in [
        hash.to_uppercase(),
        hash[..63].to_owned(),
        format!("{hash}0"),
        format!("{}g", &hash[..63]),
        String::new(),
    ] {
        let (read, issues) =
            items(json!([{ "id": "a", "kind": "low", "title": "t", "hash": bad }]));
        assert_eq!(read[0].sha256("hash"), None, "{bad}");
        assert_eq!(issues.len(), 1, "{bad}");
    }
}

#[test]
fn a_link_off_the_allowed_hosts_is_dropped_and_the_record_kept() {
    for link in [
        "https://evil.test/x",
        "https://example.test.evil.test/x",
        "http://example.test/x",
        "https://user@example.test/x",
        "https://example.test:8443/x",
        "https://EXAMPLE.test/x",
        "javascript:alert(1)",
        "https://example.test/a b",
        "https://example.test\\@evil.test/",
        "//example.test/x",
    ] {
        let (read, issues) =
            items(json!([{ "id": "a", "kind": "low", "title": "t", "link": link }]));
        assert_eq!(ids(&read), ["a"], "{link}");
        assert_eq!(read[0].link("link"), None, "{link}");
        assert!(
            matches!(&issues[..], [SectionIssue::ValueDropped { control, .. }] if control == "a"),
            "{link}: {issues:?}"
        );
    }
}

#[test]
fn text_is_plain_short_and_never_blank() {
    for title in [
        "<b>bold</b>",
        "a > b",
        "line\nbreak",
        "spoof\u{202e}txt",
        "   ",
        "twenty-one characters",
    ] {
        let (read, issues) = items(json!([{ "id": "a", "kind": "low", "title": title }]));
        assert!(
            read.is_empty(),
            "{title:?}: a required field unread drops the record"
        );
        assert_eq!(issues.len(), 1, "{title:?}");
    }
    let (read, _) = items(json!([{ "id": "a", "kind": "low", "title": "Ünïcødé — fine" }]));
    assert_eq!(read[0].text("title"), Some("Ünïcødé — fine"));
}

#[test]
fn a_bad_record_is_dropped_alone() {
    let (read, issues) = items(json!([
        { "id": "ok", "kind": "low", "title": "t" },
        { "id": "newer", "kind": "urgent", "title": "t" },
        { "id": "no-title", "kind": "low" },
        { "kind": "low", "title": "no id" },
        { "id": "Bad Id", "kind": "low", "title": "t" },
        "not an object",
        { "id": "ok", "kind": "high", "title": "twice" },
        { "id": "also-ok", "kind": "high", "title": "t", "quiet": "yes" }
    ]));
    assert_eq!(ids(&read), ["ok", "also-ok"]);
    assert_eq!(
        read[1].flag("quiet"),
        None,
        "an optional field unread is dropped alone"
    );
    assert_eq!(issues.len(), 7, "{issues:?}");
}

#[test]
fn records_are_targeted_here_and_capped_in_number() {
    let (read, _) = items(json!([
        { "id": "beta", "when": {"channels": ["beta"]}, "kind": "low", "title": "t" },
        { "id": "tool", "when": {"tools": {"tool-a": {"min": "2.0.0"}}}, "kind": "low", "title": "t" },
        { "id": "newer-when", "when": {"region": "x"}, "kind": "low", "title": "t" },
        { "id": "unread", "when": {"channels": ["beta"]}, "kind": "nonsense" }
    ]));
    assert_eq!(
        ids(&read),
        ["tool"],
        "a when this app can't read never matches"
    );
    // The rollout bucket is salted by section and id, and stable.
    let bucket = facts().install.bucket("messages.r");
    let rule =
        json!([{ "id": "r", "when": {"buckets": [bucket, bucket]}, "kind": "low", "title": "t" }]);
    assert_eq!(ids(&items(rule.clone()).0), ["r"]);
    assert_eq!(ids(&items(rule).0), ["r"]);
    let (read, issues) = items(json!([
        { "id": "a", "kind": "low", "title": "t" },
        { "id": "b", "kind": "low", "title": "t" },
        { "id": "c", "kind": "low", "title": "t" },
        { "id": "d", "kind": "low", "title": "t" }
    ]));
    assert_eq!(ids(&read), ["a", "b", "c"]);
    assert_eq!(issues.len(), 1);
}

#[test]
fn a_newer_or_broken_list_gives_nothing() {
    let registry = registry();
    for raw in [
        json!({ "schema": 2, "items": [{ "id": "a", "kind": "low", "title": "t" }] }),
        json!({ "items": [{ "id": "a", "kind": "low", "title": "t" }] }),
        json!({ "schema": 1, "items": {"a": 1} }),
        json!([1]),
    ] {
        let sections = BTreeMap::from([("messages".to_owned(), raw.clone())]);
        let (lists, _) = registry.resolve_lists(&sections, &facts());
        assert!(!lists.contains_key("messages"), "{raw}: unread, not empty");
    }
    // A section this app reads with no records is there, and empty.
    for raw in [json!({ "schema": 1 }), json!({ "schema": 1, "items": [] })] {
        let sections = BTreeMap::from([("messages".to_owned(), raw.clone())]);
        let (lists, _) = registry.resolve_lists(&sections, &facts());
        assert_eq!(lists.get("messages").map(Vec::len), Some(0), "{raw}");
    }
    let (lists, _) = registry.resolve_lists(&BTreeMap::new(), &facts());
    assert!(
        !lists.contains_key("messages"),
        "a missing section isn't empty"
    );
    // A list section's controls still resolve as before; its items don't
    // disturb them.
    let sections = BTreeMap::from([(
        "flags".to_owned(),
        json!({ "schema": 1, "values": { "x": false } }),
    )]);
    assert_eq!(registry.resolve(&sections, &facts()).0.len(), 1);
}

#[test]
fn the_registry_refuses_bad_list_declarations() {
    let bad = [
        ListSpec::new(0),
        ListSpec::new(65),
        ListSpec::new(1).field("id", Field::required(FieldKind::Flag)),
        ListSpec::new(1).field("when", Field::required(FieldKind::Flag)),
        ListSpec::new(1).field("Title", Field::required(FieldKind::Flag)),
        ListSpec::new(1).field("t", Field::required(FieldKind::Text { max_chars: 0 })),
        ListSpec::new(1).field("t", Field::required(FieldKind::Text { max_chars: 2_001 })),
        ListSpec::new(1).field("c", Field::required(FieldKind::Choice { options: vec![] })),
        ListSpec::new(1).field(
            "c",
            Field::required(FieldKind::Choice {
                options: vec!["a".to_owned(), "a".to_owned()],
            }),
        ),
        ListSpec::new(1).field("l", Field::required(FieldKind::Link { hosts: vec![] })),
        ListSpec::new(1).field(
            "l",
            Field::required(FieldKind::Link {
                hosts: vec!["Example.test".to_owned()],
            }),
        ),
        ListSpec::new(1).field(
            "l",
            Field::required(FieldKind::Link {
                hosts: vec!["localhost".to_owned()],
            }),
        ),
        ListSpec::new(1).field("n", Field::required(FieldKind::Count { min: 2, max: 1 })),
        ListSpec::new(1).field(
            "n",
            Field::required(FieldKind::Count {
                min: 0,
                max: u64::MAX,
            }),
        ),
    ];
    for list in bad {
        let mut registry = Registry::new();
        let result = registry.register(SectionSpec::new("m", 1).list(list.clone()));
        assert!(
            matches!(result, Err(RegistryError::BadControl { .. })),
            "{list:?}"
        );
    }
}

#[test]
fn the_export_carries_the_list_and_imports_back() {
    let registry = registry();
    let exported = registry.export();
    let again = Registry::import(&exported).unwrap();
    assert_eq!(again.export(), exported);
    assert_eq!(
        again.section("messages").and_then(SectionSpec::listed),
        Some(&spec())
    );
    // A section of controls only exports as it always did: no `list` key.
    let flags = exported["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "flags")
        .unwrap();
    assert!(flags.get("list").is_none(), "{flags}");
}

#[test]
fn the_publisher_is_strict_about_lists() {
    let registry = registry();
    let check =
        |section: Value| registry.validate(&BTreeMap::from([("messages".to_owned(), section)]));
    assert!(
        check(json!({ "schema": 1, "items": [{ "id": "a", "kind": "low", "title": "t" }] }))
            .is_empty()
    );
    assert!(
        check(json!({ "schema": 1 })).is_empty(),
        "an empty list section is fine"
    );
    for (section, expect) in [
        (json!({ "schema": 1, "items": {} }), "`items` is a list"),
        (
            json!({ "schema": 1, "items": [{ "kind": "low", "title": "t" }] }),
            "an id",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "kind": "low", "title": "t" }, { "id": "a", "kind": "low", "title": "t" }] }),
            "twice",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "kind": "low", "title": "t", "extra": 1 }] }),
            "unknown field",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "kind": "low" }] }),
            "title: missing",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "kind": "low", "title": "t", "link": "https://evil.test/" }] }),
            "allowed https host",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "when": {"region": 1}, "kind": "low", "title": "t" }] }),
            "doesn't know",
        ),
        (
            json!({ "schema": 1, "items": [{ "id": "a", "kind": "low", "title": "t" }, { "id": "b", "kind": "low", "title": "t" }, { "id": "c", "kind": "low", "title": "t" }, { "id": "d", "kind": "low", "title": "t" }] }),
            "at most 3",
        ),
        (
            json!({ "schema": 1, "items": [], "values": 1 }),
            "`values` is an object",
        ),
    ] {
        let problems = check(section.clone());
        assert!(
            problems.iter().any(|p| p.contains(expect)),
            "{section}: {problems:?}"
        );
    }
    // `items` is unknown on a section that lists nothing.
    let problems = registry.validate(&BTreeMap::from([(
        "flags".to_owned(),
        json!({ "schema": 1, "values": {}, "items": [] }),
    )]));
    assert!(
        problems.iter().any(|p| p.contains("unknown field")),
        "{problems:?}"
    );
}
