//! ADR-0035 §1, §2, §4 and §10: the section registry, what a value may
//! do, isolation of a bad section, and the layer precedence table.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests assert by unwrapping; the pedantic style lints are for shipped code"
)]

use std::collections::{BTreeMap, BTreeSet};

use juggler_remote_config::schema::RegistryError;
use juggler_remote_config::{
    Channel, Control, Facts, InstallId, Layer, LayerValues, Os, Registry, RemoteMode, SectionIssue,
    SectionSpec, Setting, SettingKey, UserChoices, Version, VersionRange, merge,
};
use proptest::prelude::*;
use serde_json::{Value, json};

fn v(text: &str) -> Version {
    text.parse().unwrap()
}

fn range(min: &str, max: &str) -> VersionRange {
    VersionRange::new(Some(v(min)), Some(v(max)))
}

/// Two made-up modules' sections; this crate knows no real feature.
fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(
            SectionSpec::new("flags", 1)
                .switch("kill_me", true, RemoteMode::OffOnly)
                .switch("toggle", false, RemoteMode::OnOff)
                .switch("rollout", false, RemoteMode::Rollout)
                .control(
                    "mode",
                    Control::Choice {
                        default: "steady".to_owned(),
                        options: vec!["steady".to_owned(), "eager".to_owned()],
                    },
                )
                .control(
                    "interval_s",
                    Control::Number {
                        default: 60,
                        min: 30,
                        max: 3600,
                    },
                ),
        )
        .unwrap();
    registry
        .register(SectionSpec::new("compat", 2).control(
            "tool_move",
            Control::Versions {
                default: range("1.0.0", "2.0.0"),
                bounds: range("0.9.0", "3.0.0"),
            },
        ))
        .unwrap();
    registry
}

fn facts() -> Facts {
    Facts {
        app_version: v("0.5.0"),
        channel: Channel::Stable,
        os: Os::Macos,
        os_version: Some(v("26.1.0")),
        tools: BTreeMap::new(),
        install: InstallId::from_bytes([3; 16]),
    }
}

fn layer(sections: Value) -> (LayerValues, Vec<SectionIssue>) {
    let sections: BTreeMap<String, Value> = serde_json::from_value(sections).unwrap();
    registry().resolve(&sections, &facts())
}

fn key(section: &str, control: &str) -> SettingKey {
    SettingKey::new(section, control)
}

#[test]
fn unknown_sections_controls_and_newer_schemas_are_ignored() {
    let (values, issues) = layer(json!({
        "future": {"schema": 1, "values": {"x": 1}},
        "flags": {"schema": 1, "values": {"toggle": true, "not_yet": {"rules": "anything"}}},
        "compat": {"schema": 3, "values": {"tool_move": {"min": "1.5.0", "max": "9.9.9"}}},
    }));
    assert_eq!(
        values,
        LayerValues::from([(key("flags", "toggle"), Setting::Switch(true))])
    );
    assert!(issues.contains(&SectionIssue::Unknown("future".to_owned())));
    assert!(issues.contains(&SectionIssue::Newer {
        section: "compat".to_owned(),
        schema: 3
    }));
}

#[test]
fn a_value_may_only_select_what_is_compiled_in() {
    let refused = [
        json!({"kill_me": true}),
        json!({"rollout": true}),
        json!({"toggle": "yes"}),
        json!({"mode": "reckless"}),
        json!({"interval_s": 1.5}),
        json!({"toggle": {"rules": [{"value": "on"}]}}),
    ];
    for values in refused {
        let (got, issues) = layer(json!({"flags": {"schema": 1, "values": values.clone()}}));
        assert!(got.is_empty(), "{values} applied");
        assert!(
            matches!(issues.as_slice(), [SectionIssue::ValueDropped { section, .. }] if section == "flags"),
            "{values}: {issues:?}"
        );
    }
    // A `when` this app can't read never matches (it isn't an error).
    let (got, issues) = layer(json!({"flags": {"schema": 1, "values":
        {"toggle": {"rules": [{"when": {"buckets": [9, 0]}, "value": true}]}}}}));
    assert!(got.is_empty() && issues.is_empty());
    // Ranges: never past the compiled bounds (clamped to them, every limit
    // the file set kept); anything within is fine.
    let (got, issues) =
        layer(json!({"compat": {"schema": 1, "values": {"tool_move": {"min": "0.8.0"}}}}));
    assert_eq!(
        got[&key("compat", "tool_move")],
        Setting::Versions(range("0.9.0", "3.0.0"))
    );
    assert!(
        matches!(issues.as_slice(), [SectionIssue::Clamped { control, .. }] if control == "tool_move")
    );
    let (got, _) = layer(
        json!({"compat": {"schema": 1, "values": {"tool_move": {"min": "0.5.0", "max": "1.8.0"}}}}),
    );
    assert_eq!(
        got[&key("compat", "tool_move")],
        Setting::Versions(range("0.9.0", "1.8.0")),
        "the file's own narrowing at the top still holds"
    );
    let (got, _) = layer(
        json!({"compat": {"schema": 1, "values": {"tool_move": {"min": "0.9.0", "max": "3.0.0"}}}}),
    );
    assert_eq!(
        got[&key("compat", "tool_move")],
        Setting::Versions(range("0.9.0", "3.0.0"))
    );
    // Numbers are clamped into their bounds (§2).
    let (got, _) = layer(json!({"flags": {"schema": 1, "values": {"interval_s": 5}}}));
    assert_eq!(got[&key("flags", "interval_s")], Setting::Number(30));
}

#[test]
fn a_bad_value_is_dropped_alone() {
    let (values, issues) = layer(json!({
        "flags": {"schema": 1, "values": {"toggle": true, "kill_me": true}},
        "compat": {"schema": 1, "values": {"tool_move": {"min": "1.5.0", "max": "2.0.0"}}},
    }));
    assert_eq!(
        values,
        LayerValues::from([
            (
                key("compat", "tool_move"),
                Setting::Versions(range("1.5.0", "2.0.0"))
            ),
            (key("flags", "toggle"), Setting::Switch(true)),
        ]),
        "flags.kill_me can't be turned on; flags.toggle still applies"
    );
    assert!(matches!(issues.as_slice(),
        [SectionIssue::ValueDropped { control, .. }] if control == "kill_me"));
}

/// H2 (the reviewer's proof of concept): one value a 0.5.0 app can't read
/// must not take the section's kill switch, or the compat narrowing, with it.
#[test]
fn one_unreadable_value_keeps_the_sections_kill_switches() {
    let (values, issues) = layer(json!({
        "flags": {"schema": 1, "values": {
            "kill_me": false,
            "mode": "turbo",
        }},
        "compat": {"schema": 1, "values": {
            "tool_move": {"min": "1.2.0", "max": "1.4.0", "exclude": ["1.3.0"]},
        }},
    }));
    assert_eq!(
        values.get(&key("flags", "kill_me")),
        Some(&Setting::Switch(false))
    );
    assert!(
        !values.contains_key(&key("flags", "mode")),
        "the new option is dropped alone"
    );
    assert!(
        !values.contains_key(&key("compat", "tool_move")),
        "a new range field is unreadable"
    );
    assert_eq!(issues.len(), 2);
}

/// H2: a rule gated to newer app versions is never read by this app, so a
/// value only they understand can't break anything here.
#[test]
fn a_rule_gated_to_newer_apps_is_not_read() {
    let (values, issues) = layer(json!({"flags": {"schema": 1, "values": {
        "kill_me": {"rules": [
            {"when": {"app": {"min": "0.6.0"}}, "value": true},
            {"value": false},
        ]},
        "mode": {"rules": [
            {"when": {"app": {"min": "0.6.0"}}, "value": "turbo"},
            {"when": {"app": {"min": "0.6.0", "future_clause": 1}}, "value": {"new": "shape"}},
        ]},
    }}}));
    assert_eq!(
        values.get(&key("flags", "kill_me")),
        Some(&Setting::Switch(false))
    );
    assert!(!values.contains_key(&key("flags", "mode")));
    assert!(issues.is_empty(), "{issues:?}");
}

/// H2: the four ordinary schema changes the review names, each in a rule
/// that does match this app: only that control is dropped, and a later
/// matching kill or narrowing stands in (fail toward safety).
#[test]
fn newer_values_drop_one_control_and_fall_back_to_safety() {
    let cases = [
        // A newer app's wider compiled bounds.
        (
            "compat",
            "tool_move",
            json!({"min": "0.1.0", "max": "9.0.0"}),
            json!({"min": "1.0.0", "max": "1.5.0"}),
            Setting::Versions(range("1.0.0", "1.5.0")),
        ),
        // A newer app's new option (no safety value for a choice).
        (
            "flags",
            "mode",
            json!("turbo"),
            json!("eager"),
            Setting::Choice("steady".to_owned()),
        ),
        // A switch that a newer app made a rollout: true with buckets.
        (
            "flags",
            "kill_me",
            json!(true),
            json!(false),
            Setting::Switch(false),
        ),
        // A new field inside a range.
        (
            "compat",
            "tool_move",
            json!({"min": "1.1.0", "step": 2}),
            json!({"min": "1.2.0", "max": "2.0.0"}),
            Setting::Versions(range("1.2.0", "2.0.0")),
        ),
    ];
    for (section, control, newer, fallback, expected) in cases {
        let schema = if section == "compat" { 2 } else { 1 };
        let (values, issues) = layer(
            json!({section: {"schema": schema, "values": {control: {"rules": [
                {"value": newer},
                {"value": fallback},
            ]}}}}),
        );
        assert!(
            matches!(issues.as_slice(), [SectionIssue::ValueDropped { .. }]),
            "{section}.{control}: {issues:?}"
        );
        match expected {
            // A choice has no safe value: the control is left to the layer below.
            Setting::Choice(_) => assert!(values.is_empty(), "{values:?}"),
            expected => assert_eq!(values.get(&key(section, control)), Some(&expected)),
        }
    }
}

/// H2: a section with a newer schema, or none, still applies its kills and
/// narrowings, and nothing else.
#[test]
fn a_newer_schema_still_applies_kills_and_narrowing() {
    for schema in [json!(9), json!(null)] {
        let (values, issues) = layer(json!({
            "flags": {"schema": schema, "values": {"kill_me": false, "toggle": true, "interval_s": 99}},
            "compat": {"schema": schema, "values": {"tool_move": {"min": "1.2.0", "max": "1.8.0"}}},
        }));
        assert_eq!(
            values,
            LayerValues::from([
                (
                    key("compat", "tool_move"),
                    Setting::Versions(range("1.2.0", "1.8.0"))
                ),
                (key("flags", "kill_me"), Setting::Switch(false)),
            ]),
            "schema {schema}"
        );
        assert_eq!(issues.len(), 2);
    }
    // A widening is not a safety value: not applied from a newer schema.
    let (values, _) = layer(json!({"compat": {"schema": 9, "values":
        {"tool_move": {"min": "0.9.0", "max": "3.0.0"}}}}));
    assert!(values.is_empty());
}

#[test]
fn rules_target_and_the_first_match_wins() {
    let bucket = facts().install.bucket("flags.rollout");
    let (values, _) = layer(json!({"flags": {"schema": 1, "values": {
        "rollout": {"rules": [{"when": {"buckets": [bucket, bucket]}, "value": true}]},
        "toggle": {"rules": [
            {"when": {"channels": ["beta"]}, "value": true},
            {"when": {"os": ["macos"], "app": {"min": "0.5.0"}}, "value": false},
            {"value": true}
        ]},
        "mode": {"rules": [{"when": {"channels": ["nightly"]}, "value": "eager"}]},
        "interval_s": {"rules": [{"when": {"region": "eu"}, "value": 99}]},
    }}}));
    assert_eq!(values[&key("flags", "rollout")], Setting::Switch(true));
    assert_eq!(values[&key("flags", "toggle")], Setting::Switch(false));
    assert!(
        !values.contains_key(&key("flags", "mode")),
        "no match leaves the layer below"
    );
    assert!(
        !values.contains_key(&key("flags", "interval_s")),
        "an unknown `when` key never matches"
    );
    // Outside the bucket, the rollout doesn't apply.
    let other = if bucket == 0 { 1 } else { 0 };
    let (values, _) = layer(json!({"flags": {"schema": 1, "values": {
        "rollout": {"rules": [{"when": {"buckets": [other, other]}, "value": true}]},
    }}}));
    assert!(values.is_empty());
}

#[test]
fn the_registry_refuses_bad_declarations() {
    let mut registry = registry();
    assert_eq!(
        registry.register(SectionSpec::new("flags", 1)),
        Err(RegistryError::Duplicate("flags".to_owned()))
    );
    assert!(matches!(
        registry.register(SectionSpec::new("Bad-Name", 1)),
        Err(RegistryError::BadName(_))
    ));
    assert!(matches!(
        registry.register(SectionSpec::new("zero", 0)),
        Err(RegistryError::BadSchema(_))
    ));
    let out_of_bounds = SectionSpec::new("p", 1).control(
        "n",
        Control::Number {
            default: 0,
            min: 1,
            max: 2,
        },
    );
    assert!(matches!(
        registry.register(out_of_bounds),
        Err(RegistryError::BadControl { .. })
    ));
    let wide = SectionSpec::new("c", 1).control(
        "r",
        Control::Versions {
            default: VersionRange::default(),
            bounds: range("1.0.0", "2.0.0"),
        },
    );
    assert!(matches!(
        registry.register(wide),
        Err(RegistryError::BadControl { .. })
    ));
}

/// The precedence table of ADR-0035 §4, row by row.
#[test]
fn the_merge_precedence_holds() {
    let registry = registry();
    let public = LayerValues::from([
        (key("flags", "kill_me"), Setting::Switch(false)),
        (key("flags", "toggle"), Setting::Switch(true)),
        (key("flags", "mode"), Setting::Choice("eager".to_owned())),
        (
            key("compat", "tool_move"),
            Setting::Versions(range("1.0.0", "1.5.0")),
        ),
    ]);
    let account = LayerValues::from([
        (key("flags", "toggle"), Setting::Switch(false)),
        (key("flags", "interval_s"), Setting::Number(120)),
    ]);
    let user = UserChoices::from([
        (key("flags", "kill_me"), Setting::Switch(true)),
        (key("flags", "mode"), Setting::Choice("steady".to_owned())),
        (key("flags", "interval_s"), Setting::Number(999_999)),
        (
            key("compat", "tool_move"),
            Setting::Versions(range("0.9.0", "2.5.0")),
        ),
        (key("flags", "rollout"), Setting::Number(1)),
    ]);
    let got = merge(&registry, Some(&public), Some(&account), &user);
    let at = |s, c| got.get(s, c).unwrap().clone();

    // A remote kill beats the user (safety).
    assert_eq!(
        (at("flags", "kill_me").value, at("flags", "kill_me").layer),
        (Setting::Switch(false), Layer::Public)
    );
    // Account beats public.
    assert_eq!(
        (at("flags", "toggle").value, at("flags", "toggle").layer),
        (Setting::Switch(false), Layer::Account)
    );
    // The user beats a remote preference.
    assert_eq!(
        (at("flags", "mode").value, at("flags", "mode").layer),
        (Setting::Choice("steady".to_owned()), Layer::User)
    );
    // The user's number is clamped into the compiled bounds.
    assert_eq!(at("flags", "interval_s").value, Setting::Number(3600));
    // The user can only narrow a range: remote narrowing stands.
    assert_eq!(
        at("compat", "tool_move").value,
        Setting::Versions(range("1.0.0", "1.5.0"))
    );
    // A user value of the wrong kind is ignored; untouched controls keep
    // their compiled default.
    assert_eq!(
        (at("flags", "rollout").value, at("flags", "rollout").layer),
        (Setting::Switch(false), Layer::Default)
    );

    // With no remote kill, the user's own choice on a kill switch stands.
    let user_off = UserChoices::from([(key("flags", "kill_me"), Setting::Switch(false))]);
    let got = merge(&registry, None, None, &user_off);
    assert_eq!(got.get("flags", "kill_me").unwrap().layer, Layer::User);
    // A user range that leaves nothing is ignored.
    let user_empty = UserChoices::from([(
        key("compat", "tool_move"),
        Setting::Versions(range("5.0.0", "6.0.0")),
    )]);
    let got = merge(&registry, None, None, &user_empty);
    assert_eq!(
        got.get("compat", "tool_move").unwrap().layer,
        Layer::Default
    );
}

#[test]
fn the_defaults_alone_are_the_compiled_behaviour() {
    let registry = registry();
    let got = merge(&registry, None, None, &UserChoices::new());
    let keys: BTreeSet<_> = got.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(keys.len(), 6);
    for (key, resolved) in got.iter() {
        assert_eq!(resolved.layer, Layer::Default);
        assert_eq!(
            resolved.value,
            registry.control(key).unwrap().default_setting()
        );
    }
    assert_eq!(got.switch("flags", "kill_me"), Some(true));
    assert_eq!(got.number("flags", "interval_s"), Some(60));
}

fn arb_layer() -> impl Strategy<Value = LayerValues> {
    (
        any::<Option<bool>>(),
        any::<Option<bool>>(),
        proptest::option::of(-10_000i64..10_000),
        any::<Option<bool>>(),
    )
        .prop_map(|(kill, toggle, interval, eager)| {
            let mut layer = LayerValues::new();
            if kill == Some(false) {
                layer.insert(key("flags", "kill_me"), Setting::Switch(false));
            }
            if let Some(on) = toggle {
                layer.insert(key("flags", "toggle"), Setting::Switch(on));
            }
            if let Some(n) = interval {
                layer.insert(
                    key("flags", "interval_s"),
                    Setting::Number(n.clamp(30, 3600)),
                );
            }
            if let Some(eager) = eager {
                let pick = if eager { "eager" } else { "steady" };
                layer.insert(key("flags", "mode"), Setting::Choice(pick.to_owned()));
            }
            layer
        })
}

fn values(effective: &juggler_remote_config::Effective) -> Vec<(SettingKey, Setting)> {
    effective
        .iter()
        .map(|(k, r)| (k.clone(), r.value.clone()))
        .collect()
}

proptest! {
    /// The merge is idempotent: a layer applied twice (as public and as
    /// account) gives the values it gives once.
    #[test]
    fn merge_is_idempotent(layer in arb_layer(), user in arb_layer()) {
        let registry = registry();
        let once = merge(&registry, Some(&layer), None, &user);
        let twice = merge(&registry, Some(&layer), Some(&layer), &user);
        prop_assert_eq!(values(&once), values(&twice));
    }

    /// A remote kill always holds, whatever the user and account layers say.
    #[test]
    fn a_kill_switch_always_holds(account in arb_layer(), mut user in arb_layer()) {
        user.insert(key("flags", "kill_me"), Setting::Switch(true));
        let registry = registry();
        let public = LayerValues::from([(key("flags", "kill_me"), Setting::Switch(false))]);
        let got = merge(&registry, Some(&public), Some(&account), &user);
        prop_assert_eq!(got.switch("flags", "kill_me"), Some(false));
    }
}
