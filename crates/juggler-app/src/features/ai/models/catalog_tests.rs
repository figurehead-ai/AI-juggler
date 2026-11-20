//! AJ-E3812: the Models screen's list shows only what this Mac can use
//! (D-127), keeps an installed model the list dropped, and carries the
//! same recommendation Automatic uses.

use serde_json::{Value, json};

use super::{
    AppleAiState, CatalogModel, ModelJob, ModelVerdict, NothingWhy, RecommendedWhy, catalog,
};
use crate::features::ai::models::recommend::{AppleAi, HostChip, MacFacts};
use crate::features::ai::models::{Chip, Listing, ModelEntry, offered, read_untargeted};
use crate::features::ai::service::{recommendation, usable};

const MATRIX: &str = include_str!("fixtures/recommend-matrix.json");

/// The matrix's list (Apple's model and four downloads), as read.
fn list() -> Vec<ModelEntry> {
    let matrix: Value = serde_json::from_str(MATRIX).unwrap();
    read_untargeted(&json!({ "schema": 1, "items": matrix["list"] })).unwrap()
}

fn mac(memory_gb: u64, free_gb: u64, apple_ai: AppleAi) -> MacFacts {
    MacFacts {
        chip: HostChip::Apple(Chip::M2),
        memory_gb,
        free_disk_bytes: free_gb * 1_000_000_000,
        apple_ai,
    }
}

fn ids(models: &[CatalogModel]) -> Vec<&str> {
    models.iter().map(|m| m.id.as_str()).collect()
}

#[test]
fn a_16_gb_mac_with_apple_intelligence_sees_every_model_and_apple_is_recommended() {
    let listing = Listing::Read(list());
    let offer = offered(&listing);
    let mac = mac(18, 212, AppleAi::On);
    let pick = recommendation(&mac, &usable(&offer, &[]), &[]);
    let shown = catalog(
        Some(&mac),
        Some(19_327_352_832),
        &listing,
        &offer,
        &[],
        Some(&pick),
    );
    assert!(shown.listed);
    assert_eq!(shown.models.len(), 5, "Apple's and four downloads");
    assert!(shown.models[0].built_in);
    let rec = shown.recommendation.unwrap();
    assert_eq!(rec.model_id.as_deref(), Some("apple-on-device"));
    assert_eq!(rec.why, Some(RecommendedWhy::BuiltIn));
    let info = shown.mac.unwrap();
    assert_eq!(info.apple_ai, AppleAiState::On);
    assert_eq!((info.chip, info.memory_gb), (Some(2), 18));
    // Every job's verdict, attention among them, with the list's line.
    let review = shown.models[1]
        .verdicts
        .iter()
        .find(|v| v.job == ModelJob::Review)
        .unwrap();
    assert_eq!(review.verdict, ModelVerdict::Beta);
    assert!(review.note.as_deref().unwrap().contains("blind checks"));
    assert_eq!(shown.models[1].verdicts.len(), 6);
}

#[test]
fn apple_s_model_shows_only_with_apple_intelligence_on() {
    let listing = Listing::Read(list());
    let offer = offered(&listing);
    for apple_ai in [AppleAi::Off, AppleAi::Unavailable] {
        let mac = mac(16, 48, apple_ai);
        let pick = recommendation(&mac, &usable(&offer, &[]), &[]);
        let shown = catalog(Some(&mac), None, &listing, &offer, &[], Some(&pick));
        assert!(shown.models.iter().all(|m| !m.built_in), "{apple_ai:?}");
        let rec = shown.recommendation.unwrap();
        assert_eq!(rec.why, Some(RecommendedWhy::Fits), "{apple_ai:?}");
    }
}

#[test]
fn a_model_this_mac_cant_run_is_never_shown_but_short_disk_is() {
    let listing = Listing::Read(list());
    let offer = offered(&listing);
    // 8 GB, Apple Intelligence off: no download fits the memory (D-127).
    let small = mac(8, 212, AppleAi::Off);
    let pick = recommendation(&small, &usable(&offer, &[]), &[]);
    let shown = catalog(Some(&small), None, &listing, &offer, &[], Some(&pick));
    assert!(shown.models.is_empty());
    let rec = shown.recommendation.unwrap();
    assert_eq!(rec.nothing, Some(NothingWhy::NotEnoughMemory));
    assert!(rec.apple_ai_off);
    // 16 GB with 11 GB free: every card shows, none recommended for disk.
    let full = mac(16, 11, AppleAi::Unavailable);
    let pick = recommendation(&full, &usable(&offer, &[]), &[]);
    let shown = catalog(Some(&full), None, &listing, &offer, &[], Some(&pick));
    assert_eq!(shown.models.len(), 4);
    let rec = shown.recommendation.unwrap();
    assert_eq!(rec.nothing, Some(NothingWhy::NotEnoughDisk));
    assert_eq!(
        rec.needed_bytes,
        Some(
            shown
                .models
                .iter()
                .map(|m| m.free_disk_needed_bytes)
                .min()
                .unwrap()
        )
    );
    // An Intel Mac: nothing at all.
    let intel = MacFacts {
        chip: HostChip::Intel,
        ..mac(32, 500, AppleAi::Unavailable)
    };
    let shown = catalog(Some(&intel), None, &listing, &offer, &[], None);
    assert!(shown.models.is_empty());
    assert!(!shown.mac.unwrap().apple_silicon);
}

#[test]
fn an_installed_model_the_list_dropped_still_shows_and_offline_shows_only_apple() {
    let all = list();
    let nemotron = all
        .iter()
        .find(|m| m.id == "nemotron-3-nano-4b-q4km")
        .unwrap()
        .clone();
    let listing = Listing::Read(all.into_iter().filter(|m| m.id != nemotron.id).collect());
    let offer = offered(&listing);
    let mac = mac(18, 212, AppleAi::On);
    let shown = catalog(
        Some(&mac),
        None,
        &listing,
        &offer,
        std::slice::from_ref(&nemotron),
        None,
    );
    let dropped = shown.models.iter().find(|m| m.id == nemotron.id).unwrap();
    assert!(!dropped.offered, "installed, no longer offered");
    // Offline (the list unread): Apple's model, and the installed copy.
    let unread = Listing::Unread;
    let offer = offered(&unread);
    let shown = catalog(
        Some(&mac),
        None,
        &unread,
        &offer,
        std::slice::from_ref(&nemotron),
        None,
    );
    assert!(!shown.listed);
    assert_eq!(
        ids(&shown.models),
        ["apple-on-device", nemotron.id.as_str()]
    );
    // Not read yet: nothing offered is shown.
    let shown = catalog(None, None, &unread, &offer, &[], None);
    assert!(shown.models.is_empty() && shown.mac.is_none());
}

/// The 0.5 walk's F-02: a job that isn't in this build (its flag off) is
/// "not yet" on every card, whatever the list says; the jobs that are on,
/// and attention (code), keep the list's verdict.
#[test]
fn a_job_off_in_this_build_is_not_yet_on_every_card() {
    let listing = Listing::Read(list());
    let offer = offered(&listing);
    let mac = mac(18, 212, AppleAi::On);
    let pick = recommendation(&mac, &usable(&offer, &[]), &[]);
    let shown = catalog(Some(&mac), None, &listing, &offer, &[], Some(&pick))
        .in_this_build(|job| job != crate::features::ai::models::Job::Review);
    for model in &shown.models {
        let of = |job: ModelJob| model.verdicts.iter().find(|v| v.job == job).unwrap();
        assert_eq!(
            of(ModelJob::Review).verdict,
            ModelVerdict::NotYet,
            "{}",
            model.id
        );
        assert_eq!(of(ModelJob::Review).note, None);
    }
    let qwen = &shown.models[1];
    let bot = qwen
        .verdicts
        .iter()
        .find(|v| v.job == ModelJob::Bot)
        .unwrap();
    assert_eq!(
        bot.verdict,
        ModelVerdict::Beta,
        "a job that is on keeps the list's verdict"
    );
}
