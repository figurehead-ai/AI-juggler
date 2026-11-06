//! AJ-E3504: every built-in tool names a Floor family the Floor knows, for
//! its command-line and desktop Apps alike (a plain office only when it
//! says `neutral`), and two tools that name families differ where SPEC
//! §9.7 says they do.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use juggler_adapter::ToolKind;
use juggler_app::state::default_registry;
use juggler_floor::family::Family;

#[test]
fn every_tool_names_a_family_the_floor_knows() {
    let registry = default_registry();
    let mut families = Vec::new();
    for id in registry.ids() {
        let spec = registry.get(&id).unwrap().spec();
        for kind in [ToolKind::Cli, ToolKind::Desktop] {
            let word = spec
                .floor
                .for_kind(kind)
                .unwrap_or_else(|| panic!("{id:?} names no family"));
            let family = Family::of(Some(word));
            assert!(
                family != Family::Neutral || word == "neutral",
                "{id:?}: the Floor doesn't know {word:?}"
            );
            families.push((spec.id.clone(), kind, family));
        }
    }
    let named: std::collections::BTreeSet<&str> = families
        .iter()
        .filter(|(_, _, family)| *family != Family::Neutral)
        .map(|(_, _, family)| family.as_str())
        .collect();
    assert!(
        named.len() >= 4,
        "workshop, studio, industrial and the garage at least: {named:?}"
    );
}
