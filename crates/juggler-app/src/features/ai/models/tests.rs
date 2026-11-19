//! AJ-E3810 (ADR-0035 §11.10): the `models` section's schema, record
//! check, URL rules, built-in list, listing states and signing.

use std::collections::BTreeMap;

use juggler_remote_config::publish::{self, Source as ConfigSource, Stamp};
use juggler_remote_config::{Channel, KeyRing, KeyRole, PinnedKey, Version};
use serde_json::{Value, json};

use super::source::{self, Source};
use super::{
    Chip, Engine, InstalledState, Job, Listing, MAX_MODELS, ModelEntry, Verdict, builtin,
    installed_state, listing, offered, read_raw,
};
use crate::features::remote_config::Snapshot;
use crate::features::remote_config::snapshot_fixtures::{facts, file, registry};

const SHA: &str = "061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a";
const COMMIT: &str = "90862c4b9d2787eaed51d12237eafdfe7c5f6077";

fn hf_url(org: &str, commit: &str, file: &str) -> String {
    format!("https://huggingface.co/{org}/Qwen3-1.7B-GGUF/resolve/{commit}/{file}")
}

/// The ADR's worked Qwen3 entry.
fn qwen() -> Value {
    json!({
        "id": "qwen3-1-7b-q8",
        "name": "Qwen3 1.7B",
        "engine": "gguf_local",
        "url": hf_url("Qwen", COMMIT, "Qwen3-1.7B-Q8_0.gguf"),
        "sha256": SHA,
        "size_bytes": 1_834_426_016u64,
        "licence_name": "Apache-2.0",
        "licence_url": format!("https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/blob/{COMMIT}/LICENSE"),
        "min_chip": "m1",
        "min_memory_gb": 16,
        "speed": "quick",
        "feed": "not_yet",
        "review": "beta",
        "review_note": "Preferred over the plain review in 89 % of blind checks.",
        "attention": "ships",
        "chat": "beta",
        "digest": "not_yet",
        "watches": "not_yet"
    })
}

fn apple() -> Value {
    json!({
        "id": "apple-on-device",
        "name": "Apple's on-device model",
        "engine": "apple_builtin",
        "min_chip": "m1",
        "min_memory_gb": 8,
        "review": "beta",
        "attention": "ships",
        "chat": "beta"
    })
}

/// The entries the app reads from `items`, through the app's registry.
fn read(items: Value) -> Vec<ModelEntry> {
    read_raw(&json!({ "schema": 1, "items": items }), &facts(1)).unwrap()
}

fn ids(entries: &[ModelEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.id.as_str()).collect()
}

fn with(mut entry: Value, key: &str, value: Value) -> Value {
    entry[key] = value;
    entry
}

fn without(mut entry: Value, key: &str) -> Value {
    entry.as_object_mut().unwrap().remove(key);
    entry
}

#[test]
fn a_valid_entry_of_each_engine_is_read_typed() {
    let entries = read(json!([apple(), qwen()]));
    assert_eq!(ids(&entries), ["apple-on-device", "qwen3-1-7b-q8"]);
    let apple = &entries[0];
    assert_eq!(apple.engine, Engine::AppleBuiltin);
    assert!(apple.download.is_none());
    assert_eq!(apple.free_disk_needed(), 0);
    let qwen = &entries[1];
    assert_eq!(qwen.engine, Engine::GgufLocal);
    assert_eq!(qwen.min_chip, Chip::M1);
    assert_eq!(qwen.min_memory_gb, 16);
    let download = qwen.download.as_ref().unwrap();
    assert_eq!(download.source, Source::HuggingFace);
    assert_eq!(download.sha256, SHA);
    assert_eq!(download.size_bytes, 1_834_426_016);
    assert_eq!(qwen.free_disk_needed(), 1_834_426_016 + 10_000_000_000);
    assert_eq!(qwen.verdict(Job::Review), Verdict::Beta);
    assert_eq!(qwen.verdict(Job::Attention), Verdict::Ships);
    assert_eq!(qwen.verdict(Job::Feed), Verdict::NotYet);
    assert_eq!(
        qwen.verdicts[&Job::Review].note.as_deref(),
        Some("Preferred over the plain review in 89 % of blind checks.")
    );
    assert!(qwen.passes_any_job());
}

#[test]
fn an_unknown_engine_or_format_drops_the_whole_entry() {
    let entries = read(json!([
        with(qwen(), "engine", json!("mlx_local")),
        with(
            qwen(),
            "url",
            json!(hf_url("Qwen", COMMIT, "model.safetensors"))
        ),
    ]));
    assert!(entries.is_empty(), "{entries:?}");
}

#[test]
fn a_url_off_the_rules_drops_the_whole_entry() {
    for url in [
        "https://evil.test/Qwen/Qwen3-1.7B-GGUF/resolve/x/a.gguf".to_owned(),
        hf_url("someone", COMMIT, "Qwen3-1.7B-Q8_0.gguf"),
        hf_url("qwen", COMMIT, "Qwen3-1.7B-Q8_0.gguf"),
        hf_url("Qwen", "main", "Qwen3-1.7B-Q8_0.gguf"),
        hf_url("Qwen", &COMMIT.to_uppercase(), "Qwen3-1.7B-Q8_0.gguf"),
        format!("https://huggingface.co/Qwen/R/resolve/{COMMIT}/../a.gguf"),
        format!("http://huggingface.co/Qwen/R/resolve/{COMMIT}/a.gguf"),
        format!("https://user@huggingface.co/Qwen/R/resolve/{COMMIT}/a.gguf"),
        format!("https://huggingface.co:443/Qwen/R/resolve/{COMMIT}/a.gguf"),
        format!("https://huggingface.co/Qwen/R/resolve/{COMMIT}/a%2e.gguf"),
        format!("https://huggingface.co/Qwen/R/resolve/{COMMIT}/a.gguf?download=1"),
        format!("https://huggingface.co/Qwen/R/resolve/{COMMIT}/sub/a.gguf"),
        format!("https://models.figurehead.ai/v1/{}/a.gguf", "0".repeat(64)),
        format!("https://models.figurehead.ai/{SHA}/a.gguf"),
    ] {
        assert!(
            read(json!([with(qwen(), "url", json!(url))])).is_empty(),
            "{url}"
        );
    }
    // A bad mirror drops the entry too.
    let mirror = format!("https://models.figurehead.ai/v1/{}/a.gguf", "0".repeat(64));
    assert!(read(json!([with(qwen(), "mirror_url", json!(mirror))])).is_empty());
    // Our own path, addressed by the entry's hash, is fine.
    let own = format!("https://models.figurehead.ai/v1/{SHA}/Qwen3-1.7B-Q8_0.gguf");
    let entries = read(json!([with(qwen(), "url", json!(own))]));
    assert_eq!(entries[0].download.as_ref().unwrap().source, Source::Own);
}

#[test]
fn a_missing_or_malformed_file_field_drops_the_whole_entry() {
    for broken in [
        without(qwen(), "sha256"),
        with(qwen(), "sha256", json!(SHA.to_uppercase())),
        with(qwen(), "sha256", json!(&SHA[..63])),
        without(qwen(), "size_bytes"),
        with(qwen(), "size_bytes", json!(0)),
        with(qwen(), "size_bytes", json!(8u64 * 1024 * 1024 * 1024 + 1)),
        without(qwen(), "url"),
        without(qwen(), "licence_name"),
        without(qwen(), "licence_url"),
        with(qwen(), "licence_url", json!("https://evil.test/LICENSE")),
        without(qwen(), "min_chip"),
        with(qwen(), "min_memory_gb", json!(4)),
        with(qwen(), "name", json!("a".repeat(41))),
        with(qwen(), "name", json!("<b>Qwen</b>")),
    ] {
        assert!(read(json!([broken.clone()])).is_empty(), "{broken}");
    }
}

#[test]
fn apples_entry_can_never_fetch_and_only_one_is_read() {
    let with_file = with(apple(), "url", qwen()["url"].clone());
    assert!(read(json!([with_file])).is_empty());
    let with_hash = with(apple(), "sha256", json!(SHA));
    assert!(read(json!([with_hash])).is_empty());
    let second = with(apple(), "id", json!("apple-again"));
    let entries = read(json!([apple(), second]));
    assert_eq!(ids(&entries), ["apple-on-device"]);
}

#[test]
fn unknown_fields_are_ignored_and_unknown_verdicts_read_as_not_tested() {
    let entry = with(
        with(qwen(), "future_field", json!({"x": 1})),
        "chat",
        json!("glowing"),
    );
    let entries = read(json!([entry]));
    assert_eq!(ids(&entries), ["qwen3-1-7b-q8"]);
    assert_eq!(entries[0].verdict(Job::Chat), Verdict::NotTested);
    let long_note = with(qwen(), "chat_note", json!("a".repeat(121)));
    assert_eq!(read(json!([long_note]))[0].verdicts[&Job::Chat].note, None);
}

#[test]
fn the_list_is_capped() {
    let many: Vec<Value> = (0..=MAX_MODELS)
        .map(|n| with(qwen(), "id", json!(format!("m{n}"))))
        .collect();
    assert_eq!(read(Value::Array(many)).len(), MAX_MODELS as usize);
}

#[test]
fn the_built_in_list_is_apple_only_through_the_same_reader() {
    let built_in = builtin();
    assert_eq!(ids(&built_in), ["apple-on-device"]);
    assert_eq!(built_in[0].engine, Engine::AppleBuiltin);
    assert_eq!(built_in[0].verdict(Job::Review), Verdict::Beta);
    assert_eq!(built_in[0].verdict(Job::Chat), Verdict::Beta);
    assert_eq!(built_in[0].verdict(Job::Attention), Verdict::Ships);
    for job in [Job::Feed, Job::Digest, Job::Watches] {
        assert_eq!(built_in[0].verdict(job), Verdict::NotYet, "{job:?}");
    }
    let raw: Value = serde_json::from_str(include_str!("builtin.json")).unwrap();
    assert!(
        raw["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i.get("when").is_none()),
        "the built-in list carries no targeting"
    );
    // With nothing read, the built-in list is offered.
    assert_eq!(offered(&Listing::Unread), built_in);
}

fn snapshot(sections: Value) -> Snapshot {
    Snapshot::build(&registry(), Some(&file(sections)), &facts(1))
}

#[test]
fn the_listing_tells_unread_from_empty() {
    // No file at all.
    let none = Snapshot::build(&registry(), None, &facts(1));
    assert_eq!(listing(&none.lists), Listing::Unread);
    // A file without the section, or with a newer schema.
    assert_eq!(listing(&snapshot(json!({})).lists), Listing::Unread);
    let newer = snapshot(json!({ "models": { "schema": 2, "items": [qwen()] } }));
    assert_eq!(listing(&newer.lists), Listing::Unread);
    // A section read with no entries: Apple's built-in entry is still offered.
    let empty = snapshot(json!({ "models": { "schema": 1, "items": [] } }));
    assert_eq!(listing(&empty.lists), Listing::Read(Vec::new()));
    assert_eq!(ids(&offered(&listing(&empty.lists))), ["apple-on-device"]);
    // The section's own Apple entry replaces the built-in one.
    let own = snapshot(json!({ "models": { "schema": 1, "items": [
        with(apple(), "review", json!("not_yet")), qwen()
    ] } }));
    let offered_now = offered(&listing(&own.lists));
    assert_eq!(ids(&offered_now), ["apple-on-device", "qwen3-1-7b-q8"]);
    assert_eq!(offered_now[0].verdict(Job::Review), Verdict::NotYet);
}

#[test]
fn withdrawn_models_stay_usable_with_a_note() {
    let copy = read(json!([qwen()])).remove(0);
    // Withdrawn: not offered, installed still works, with the note.
    let withdrawn = Listing::Read(read(json!([with(qwen(), "withdrawn", json!(true))])));
    assert!(offered(&withdrawn).iter().all(|e| e.id != copy.id));
    assert!(matches!(
        installed_state(&copy, &withdrawn),
        InstalledState::Withdrawn(_)
    ));
    // No longer listed: the same.
    let gone = Listing::Read(Vec::new());
    assert_eq!(
        installed_state(&copy, &gone),
        InstalledState::Withdrawn(copy.clone())
    );
    // Unread: the copy stands, no note.
    assert_eq!(
        installed_state(&copy, &Listing::Unread),
        InstalledState::Unknown(copy.clone())
    );
    // Listed: the copy is refreshed (a narrowed verdict applies).
    let narrowed = Listing::Read(read(json!([with(qwen(), "chat", json!("not_yet"))])));
    let state = installed_state(&copy, &narrowed);
    assert!(
        matches!(&state, InstalledState::Listed(now) if now.verdict(Job::Chat) == Verdict::NotYet),
        "{state:?}"
    );
}

#[test]
fn a_downgraded_entry_never_replaces_the_installed_file() {
    let copy = read(json!([qwen()])).remove(0);
    let other = "f".repeat(64);
    let changed = Listing::Read(read(json!([with(qwen(), "sha256", json!(other))])));
    assert_eq!(
        installed_state(&copy, &changed),
        InstalledState::Unknown(copy.clone()),
        "an entry with another file under the same id is ignored here"
    );
}

#[test]
fn redirects_go_only_to_the_sources_file_hosts() {
    let ok = "https://cas-server.xethub.hf.co/x/y?X-Amz-Signature=ab%2Fcd&Expires=1";
    assert!(source::redirect_allowed(Source::HuggingFace, ok));
    for bad in [
        "https://evil.test/x",
        "https://cdn-lfs.hf.co.evil.test/x",
        "http://cdn-lfs-us-1.hf.co/x",
        "https://user@cdn-lfs-us-1.hf.co/x",
        "https://cdn-lfs-us-1.hf.co:8443/x",
        "https://127.0.0.1/x",
        "https://[::1]/x",
    ] {
        assert!(!source::redirect_allowed(Source::HuggingFace, bad), "{bad}");
    }
    assert!(
        !source::redirect_allowed(Source::Own, "https://models.figurehead.ai/v1/x"),
        "our own host redirects nowhere"
    );
}

/// A ring pinning one fixed test key, and that key.
fn lab() -> (ed25519_dalek::SigningKey, KeyRing) {
    let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let pinned =
        PinnedKey::new("rc-test", KeyRole::Current, &key.verifying_key().to_bytes()).unwrap();
    (key, KeyRing::new(vec![pinned]).unwrap())
}

fn signed(sections: Value, key: &ed25519_dalek::SigningKey, ring: &KeyRing) -> Vec<u8> {
    let source = ConfigSource {
        channel: Channel::Stable,
        min_app: None,
        max_app: None,
        sections: serde_json::from_value::<BTreeMap<String, Value>>(sections).unwrap(),
    };
    let stamp = Stamp::new(1, 1_800_000_000, 14).unwrap();
    publish::sign(&source, &stamp, key, ring, &[]).unwrap()
}

#[test]
fn an_unsigned_or_forged_models_section_is_never_read() {
    let (key, ring) = lab();
    let app: Version = "0.5.0".parse().unwrap();
    let now = 1_800_000_100;
    let models = json!({ "models": { "schema": 1, "items": [qwen()] } });
    let genuine = signed(models.clone(), &key, &ring);
    let verified = publish::verify(&genuine, &ring, Channel::Stable, &app, now).unwrap();
    let read_now = Snapshot::build(&registry(), Some(&verified), &facts(1));
    assert_eq!(
        ids(&offered(&listing(&read_now.lists))),
        ["apple-on-device", "qwen3-1-7b-q8"]
    );

    // The models payload under another file's signature: forged.
    let plain = signed(json!({}), &key, &ring);
    let mut forged: Value = serde_json::from_slice(&genuine).unwrap();
    let other: Value = serde_json::from_slice(&plain).unwrap();
    forged["config"]["signature"] = other["config"]["signature"].clone();
    // No signature at all: unsigned.
    let mut unsigned = forged.clone();
    unsigned.as_object_mut().unwrap().remove("config");
    for bad in [forged, unsigned] {
        let bytes = serde_json::to_vec(&bad).unwrap();
        assert!(publish::verify(&bytes, &ring, Channel::Stable, &app, now).is_err());
    }
    // What the app has then: no file, so the built-in list only.
    let none = Snapshot::build(&registry(), None, &facts(1));
    assert_eq!(ids(&offered(&listing(&none.lists))), ["apple-on-device"]);
}
