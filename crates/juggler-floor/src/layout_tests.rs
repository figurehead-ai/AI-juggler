//! The office layout's rules (SPEC §9.7, AJ-E3503), by example and by
//! property.

#![allow(clippy::expect_used, clippy::panic)]

use proptest::prelude::*;

use super::{
    AREA_MAX, DEPTH, Free, Office, Plan, ROW_D, SHRINK_AFTER_MS, Size, arranged, desks_for, pack,
    plan_area, units,
};
use crate::family::{Family, Layout};

const DAY: i64 = SHRINK_AFTER_MS;

fn open(apps: &[(&str, usize)]) -> Vec<(String, Family, usize)> {
    apps.iter()
        .map(|(id, n)| ((*id).to_owned(), Family::Workshop, *n))
        .collect()
}

fn area<'a>(plan: &'a Plan, id: &str, annex: usize) -> &'a super::Placed {
    plan.areas
        .iter()
        .find(|a| a.id == id && a.annex == annex)
        .expect("placed")
}

#[test]
fn packed_is_a_view_only_and_kept_places_come_back_exactly() {
    let mut office = Office::new(7, 10);
    office.observe(
        &open(&[("a", 1), ("b", 1), ("c", 1), ("d", 1), ("e", 1)]),
        0,
    );
    let before = office.clone();
    let kept = pack(&office);
    let active: std::collections::BTreeSet<String> = ["c".to_owned(), "e".to_owned()].into();
    let view = arranged(&office, &active, true);
    // the office itself is untouched: nothing to save
    assert_eq!(office, before, "Packed never writes the office");
    // the active Apps take the first places, in their order
    let packed = pack(&view);
    let first: Vec<&str> = packed
        .areas
        .iter()
        .filter(|a| a.annex == 0)
        .take(2)
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(first, ["c", "e"]);
    // Kept places brings every area back exactly
    assert_eq!(pack(&arranged(&office, &active, false)), kept);
}

#[test]
fn sizes_follow_sessions_and_annexes_open_beyond_twenty_four() {
    assert_eq!(desks_for(0), 6);
    assert_eq!(desks_for(6), 6);
    assert_eq!(desks_for(7), 12);
    assert_eq!(desks_for(12), 12);
    assert_eq!(desks_for(13), 24);
    assert_eq!(desks_for(24), 24);
    assert_eq!(desks_for(25), 30);
    assert_eq!(desks_for(49), 54);
    assert_eq!(units(24), vec![Size::L]);
    assert_eq!(units(30), vec![Size::L, Size::S]);
    assert_eq!(units(60), vec![Size::L, Size::L, Size::M]);
    assert_eq!(Size::L.capacity(), AREA_MAX);
}

#[test]
fn apps_keep_the_order_they_were_first_seen_in() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("b", 2), ("a", 2)]), 0);
    office.observe(&open(&[("c", 2), ("a", 2), ("b", 2)]), 1);
    assert_eq!(office.placed, ["b", "a", "c"]);
    let plan = pack(&office);
    assert_eq!((area(&plan, "b", 0).col, area(&plan, "b", 0).row), (0, 0));
    assert_eq!((area(&plan, "a", 0).col, area(&plan, "a", 0).row), (0, 1));
    assert_eq!((area(&plan, "c", 0).col, area(&plan, "c", 0).row), (1, 0));
}

#[test]
fn an_app_grows_at_once_and_shrinks_only_after_a_day() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("a", 3)]), 0);
    let grew = office.observe(&open(&[("a", 9)]), 10);
    assert!(grew.reflow);
    assert_eq!(office.apps["a"].desks, 12);

    let quiet = office.observe(&open(&[("a", 2)]), 20);
    assert!(
        quiet.changed && !quiet.reflow,
        "it starts the clock, nothing moves"
    );
    assert_eq!(office.apps["a"].fits_smaller_since, Some(20));
    assert!(!office.observe(&open(&[("a", 2)]), 20 + DAY - 1).reflow);
    let shrank = office.observe(&open(&[("a", 2)]), 20 + DAY);
    assert!(shrank.reflow);
    assert_eq!(office.apps["a"].desks, 6);
}

#[test]
fn needing_the_desks_again_stops_the_shrink_clock() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("a", 10)]), 0);
    office.observe(&open(&[("a", 2)]), 1);
    let back = office.observe(&open(&[("a", 10)]), 2);
    assert!(back.changed && !back.reflow);
    assert_eq!(office.apps["a"].fits_smaller_since, None);
    assert!(
        !office.observe(&open(&[("a", 2)]), 2 + DAY - 1).reflow,
        "the clock started again"
    );
}

#[test]
fn a_closed_app_keeps_its_cells_until_the_next_reflow() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("a", 2), ("b", 2), ("c", 2)]), 0);
    let gone = office.observe(&open(&[("a", 2), ("c", 2)]), 1);
    assert!(!gone.reflow, "b's cells stay as a lounge");
    assert_eq!(office.placed, ["a", "b", "c"]);
    office.observe(&open(&[("a", 20), ("c", 2)]), 2);
    assert_eq!(
        office.placed,
        ["a", "c"],
        "the next reflow lets b's cells go"
    );
    office.observe(&open(&[("a", 20), ("b", 2), ("c", 2)]), 3);
    assert_eq!(
        office.placed,
        ["a", "b", "c"],
        "b keeps its first place in the order"
    );
}

#[test]
fn a_big_app_spans_the_aisle_and_free_cells_are_a_kitchen_then_lounges() {
    let mut office = Office::new(1, 8);
    office.observe(&open(&[("big", 20), ("mid", 9)]), 0);
    let plan = pack(&office);
    let big = area(&plan, "big", 0);
    assert_eq!((big.size, big.col, big.row), (Size::L, 0, 0));
    assert!(
        big.rect.z0 < ROW_D && (big.rect.z1 - DEPTH).abs() < 1e-4,
        "across the aisle"
    );
    let mid = area(&plan, "mid", 0);
    assert_eq!((mid.size, mid.col, mid.row), (Size::M, 2, 0));
    assert_eq!(plan.cols, 4);
    assert_eq!(plan.free.len(), 2);
    assert_eq!(plan.free[0].1, Free::Kitchen);
    assert_eq!(plan.free[1].1, Free::Lounge);
}

#[test]
fn a_floor_holds_at_most_its_limit_then_the_next_floor_opens() {
    let mut office = Office::new(1, 4);
    let apps: Vec<(String, usize)> = (0..9).map(|n| (format!("app-{n}"), 1)).collect();
    let list: Vec<(&str, usize)> = apps.iter().map(|(id, n)| (id.as_str(), *n)).collect();
    office.observe(&open(&list), 0);
    let plan = pack(&office);
    assert_eq!(plan.floors, 3);
    assert_eq!(plan.areas.iter().filter(|a| a.floor == 0).count(), 4);
    assert_eq!(area(&plan, "app-8", 0).floor, 2);
    assert_eq!(Office::new(1, 99).per_floor, 16);
    assert_eq!(Office::new(1, 1).per_floor, 4);
}

#[test]
fn an_annex_follows_its_app_in_order() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("huge", 30), ("next", 2)]), 0);
    let plan = pack(&office);
    let ids: Vec<(&str, usize)> = plan
        .areas
        .iter()
        .map(|a| (a.id.as_str(), a.annex))
        .collect();
    assert_eq!(ids, [("huge", 0), ("huge", 1), ("next", 0)]);
    assert_eq!(area(&plan, "huge", 1).size, Size::S);
}

#[test]
fn re_create_changes_looks_not_order() {
    let mut office = Office::new(1, 10);
    office.observe(&open(&[("a", 2), ("b", 2), ("c", 2)]), 0);
    let before = office.clone();
    office.recreate(99);
    assert_eq!(office.placed, before.placed);
    for (id, app) in &office.apps {
        assert_eq!(app.order, before.apps[id].order);
    }
    assert_ne!(
        office.apps.values().map(|a| a.choice).collect::<Vec<_>>(),
        before.apps.values().map(|a| a.choice).collect::<Vec<_>>()
    );
}

#[test]
fn every_size_and_layout_seats_every_session_inside_its_area() {
    for size in [Size::S, Size::M, Size::L] {
        let (cw, ch) = size.cells();
        for row in 0..=2 - ch {
            let rect = super::cell_rect(0, row, cw, ch);
            for layout in Layout::ALL {
                for seed in 0..12 {
                    let plan = plan_area(rect, size, layout, 0.3, seed, row);
                    assert_eq!(
                        plan.spots.len(),
                        size.capacity(),
                        "{size:?} {layout:?} row {row} seed {seed}"
                    );
                    for spot in &plan.spots {
                        assert!(
                            rect.contains(spot.x, spot.z),
                            "{size:?} {layout:?}: a desk outside"
                        );
                    }
                    assert_eq!(
                        plan,
                        plan_area(rect, size, layout, 0.3, seed, row),
                        "the same every time"
                    );
                }
            }
        }
    }
}

#[test]
fn a_big_area_keeps_its_aisle_clear() {
    let rect = super::cell_rect(0, 0, 2, 2);
    for layout in Layout::ALL {
        let plan = plan_area(rect, Size::L, layout, 0.0, 5, 0);
        for group in &plan.groups {
            assert!(
                group.z < ROW_D - 0.5 || group.z > ROW_D + super::AISLE + 0.5,
                "{layout:?}: a group in the aisle"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Whatever opens, grows or closes: no two areas overlap, no floor
    /// holds more than its limit, every App open is placed whole, and an
    /// area moves only when an App before it changed.
    #[test]
    fn the_office_always_packs_cleanly(
        per_floor in 4usize..=16,
        steps in prop::collection::vec(prop::collection::vec((0usize..14, 1usize..60), 1..10), 1..6),
    ) {
        let mut office = Office::new(3, per_floor);
        let mut now = 0;
        for step in steps {
            let mut seen = std::collections::BTreeSet::new();
            let list: Vec<(String, Family, usize)> = step
                .into_iter()
                .filter(|(n, _)| seen.insert(*n))
                .map(|(n, sessions)| (format!("app-{n}"), Family::ALL[n % Family::ALL.len()], sessions))
                .collect();
            let before = pack(&office);
            office.observe(&list, now);
            now += DAY / 3;
            let plan = pack(&office);
            for floor in 0..plan.floors {
                let on: Vec<_> = plan.areas.iter().filter(|a| a.floor == floor).collect();
                prop_assert!(on.len() <= office.per_floor);
                for (i, a) in on.iter().enumerate() {
                    for b in &on[i + 1..] {
                        prop_assert!(!a.rect.overlaps(&b.rect), "{} and {} overlap", a.id, b.id);
                    }
                }
            }
            for (id, _, _) in &list {
                let floors: Vec<usize> = plan.areas.iter().filter(|a| &a.id == id).map(|a| a.floor).collect();
                prop_assert!(floors.windows(2).all(|w| w[0] <= w[1]), "an annex before its App");
                let desks: usize = plan.areas.iter().filter(|a| &a.id == id).map(|a| a.size.capacity()).sum();
                prop_assert_eq!(desks, office.apps[id].desks);
            }
            // The first area whose App changed: everything before it stays.
            let unchanged = before
                .areas
                .iter()
                .zip(&plan.areas)
                .take_while(|(a, b)| a.id == b.id && a.annex == b.annex && a.size == b.size)
                .count();
            for (a, b) in before.areas.iter().zip(&plan.areas).take(unchanged) {
                prop_assert_eq!((a.floor, a.col, a.row), (b.floor, b.col, b.row));
            }
        }
    }
}

/// AJ-E3503's budget: placing 20 Apps and 150 sessions (their areas and
/// every desk) ≤ 5 ms, a reflow ≤ 2 ms, on a release build. A debug
/// build gets ten times as long.
#[test]
fn placing_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick() {
    let scale = if cfg!(debug_assertions) { 10 } else { 1 };
    let apps: Vec<(String, Family, usize)> = (0..20)
        .map(|n| {
            (
                format!("app-{n}"),
                Family::ALL[n % Family::ALL.len()],
                [2, 5, 9, 14, 7][n % 5] + usize::from(n == 0) * 2,
            )
        })
        .collect();
    assert_eq!(apps.iter().map(|a| a.2).sum::<usize>(), 150);
    let started = std::time::Instant::now();
    let mut office = Office::new(5, 10);
    office.observe(&apps, 0);
    let plan = pack(&office);
    let desks: usize = plan
        .areas
        .iter()
        .map(|a| {
            let app = &office.apps[&a.id];
            plan_area(
                a.rect,
                a.size,
                app.choice.layout,
                app.family.jitter(),
                app.choice.seed,
                a.row,
            )
            .spots
            .len()
        })
        .sum();
    let placing = started.elapsed();
    assert!(desks >= 150);
    assert!(
        placing.as_micros() <= 5_000 * scale,
        "placing took {placing:?}"
    );

    let started = std::time::Instant::now();
    let _ = pack(&office);
    let reflow = started.elapsed();
    assert!(
        reflow.as_micros() <= 2_000 * scale,
        "a reflow took {reflow:?}"
    );
    eprintln!("placing {placing:?}, reflow {reflow:?}");
}
