//! AJ-E3811: the recommendation over the hardware matrix
//! (`fixtures/recommend-matrix.json`): each Mac and the one answer it must
//! get, deterministic, Apple first, the size plus 10 GB margin at its
//! boundary.

use serde_json::Value;

use super::recommend::{AppleAi, HostChip, MacFacts, Recommendation, Why, WhyNot, recommend};
use super::{Chip, Listing, offered, read_raw};
use crate::features::remote_config::snapshot_fixtures::facts;

const MATRIX: &str = include_str!("fixtures/recommend-matrix.json");

fn chip(name: &str) -> HostChip {
    let chips = [
        ("intel", HostChip::Intel),
        ("newer", HostChip::AppleNewer),
        ("m1", HostChip::Apple(Chip::M1)),
        ("m2", HostChip::Apple(Chip::M2)),
        ("m3", HostChip::Apple(Chip::M3)),
        ("m4", HostChip::Apple(Chip::M4)),
        ("m5", HostChip::Apple(Chip::M5)),
    ];
    chips
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, chip)| *chip)
        .unwrap()
}

fn mac(raw: &Value) -> MacFacts {
    let free_disk_bytes = raw["free_bytes"]
        .as_u64()
        .unwrap_or_else(|| raw["free_gb"].as_u64().unwrap() * 1_000_000_000);
    MacFacts {
        chip: chip(raw["chip"].as_str().unwrap()),
        memory_gb: raw["memory_gb"].as_u64().unwrap(),
        free_disk_bytes,
        apple_ai: match raw["apple_ai"].as_str().unwrap() {
            "on" => AppleAi::On,
            "off" => AppleAi::Off,
            _ => AppleAi::Unavailable,
        },
    }
}

/// The listing a case reads: the matrix's list, changed as the case says.
fn case_listing(list: &Value, raw: &Value) -> Listing {
    match raw["list"].as_str() {
        Some("unread") => return Listing::Unread,
        Some("empty") => return Listing::Read(Vec::new()),
        _ => {}
    }
    let mut items = list.as_array().unwrap().clone();
    for item in &mut items {
        let id = item["id"].as_str().unwrap().to_owned();
        if raw["withdraw"]
            .as_array()
            .is_some_and(|ids| ids.iter().any(|w| w == id.as_str()))
        {
            item["withdrawn"] = Value::Bool(true);
        }
        if raw["narrow_apple"].as_bool() == Some(true) && item["engine"] == "apple_builtin" {
            item["review"] = Value::from("not_yet");
            item["chat"] = Value::from("not_yet");
        }
    }
    let raw_section = serde_json::json!({ "schema": 1, "items": items });
    Listing::Read(read_raw(&raw_section, &facts(1)).unwrap())
}

fn expected(raw: &Value) -> Recommendation {
    if let Some(id) = raw["id"].as_str() {
        let why = match raw["why"].as_str().unwrap() {
            "built_in" => Why::BuiltIn,
            "installed" => Why::Installed,
            _ => Why::Fits { size_bytes: 0 },
        };
        return Recommendation::Model {
            id: id.to_owned(),
            why,
        };
    }
    let apple_ai_off = raw["apple_ai_off"].as_bool().unwrap_or(false);
    Recommendation::Nothing(match raw["none"].as_str().unwrap() {
        "not_apple_silicon" => WhyNot::NotAppleSilicon,
        "not_enough_memory" => WhyNot::NotEnoughMemory { apple_ai_off },
        "not_enough_disk" => WhyNot::NotEnoughDisk {
            needed_bytes: raw["needed_bytes"].as_u64().unwrap(),
            apple_ai_off,
        },
        _ => WhyNot::NothingListed,
    })
}

/// The answer with a download's size blanked (the matrix names the
/// model; its size is checked against the list below).
fn blank_size(answer: Recommendation, offered: &[super::ModelEntry]) -> Recommendation {
    match answer {
        Recommendation::Model {
            id,
            why: Why::Fits { size_bytes },
        } => {
            let listed = offered
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.download.as_ref())
                .map(|d| d.size_bytes);
            assert_eq!(
                listed,
                Some(size_bytes),
                "{id}: the size shown is the list's"
            );
            Recommendation::Model {
                id,
                why: Why::Fits { size_bytes: 0 },
            }
        }
        other => other,
    }
}

#[test]
fn every_mac_in_the_matrix_gets_its_one_answer() {
    let matrix: Value = serde_json::from_str(MATRIX).unwrap();
    let cases = matrix["cases"].as_array().unwrap();
    assert!(cases.len() >= 12);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let listing = case_listing(&matrix["list"], &case["mac"]);
        let offered = offered(&listing);
        let installed: Vec<&str> = case["mac"]["installed"]
            .as_array()
            .map(|ids| ids.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mac = mac(&case["mac"]);
        let answer = recommend(&mac, &offered, &installed);
        // Pure: the same facts and list give the same answer.
        assert_eq!(answer, recommend(&mac, &offered, &installed), "{name}");
        assert_eq!(
            blank_size(answer, &offered),
            expected(&case["expect"]),
            "{name}"
        );
    }
}
