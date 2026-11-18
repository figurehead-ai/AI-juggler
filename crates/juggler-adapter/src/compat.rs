//! D-128, ADR-0035 §2: the `compat` section of the signed remote
//! configuration — per tool and per feature, which tool versions a
//! feature is offered for, which revision of its shape check is needed,
//! and a kill switch.
//!
//! **The compiled list is layer 0.** Each adapter declares the features it
//! gates as [`CompatFeature`]s ([`crate::ToolIdentity::compat_features`]):
//! a compiled range of tool versions, the hard limits its code supports,
//! and its shape check's revision. With no remote file (none fetched, none
//! valid, an old one, or the check turned off) every feature behaves
//! exactly as compiled.
//!
//! **The remote file only narrows, widens within the hard limits, or turns
//! off** (§2). Each feature becomes three controls in one section, named
//! after its key:
//!
//! | Control | Kind | What the file may do |
//! |---|---|---|
//! | `<key>_on` | switch, `OffOnly` | turn the feature off (a kill switch); never on |
//! | `<key>_versions` | version range | narrow it, or widen it up to the compiled hard limits (a range past them is clamped) |
//! | `<key>_shape` | number | require a newer shape-check revision than this build has (which turns the feature off here until an update); never an older one |
//!
//! None of these adds behaviour: the feature, its code and its shape check
//! are compiled in, and the file can only stop offering it.
//!
//! **Where it's read.** The app resolves the layers and [`install`]s a
//! [`Gate`]; an adapter asks [`current`] where it decides whether to offer
//! the feature, with the tool version it found, and turns any [`Off`] into
//! its own refusal in words. No code outside the adapters names a tool or
//! a feature (ADR-0019).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, RwLock};

use juggler_remote_config::schema::RegistryError;
use juggler_remote_config::{Control, Effective, RemoteMode, SectionSpec, Setting};
pub use juggler_remote_config::{Version, VersionRange};

/// The section's name in the file.
pub const SECTION: &str = "compat";

/// The section's schema version this build reads.
pub const SCHEMA: u32 = 1;

/// The highest shape-check revision a file may require.
pub const MAX_SHAPE: i64 = 1_000_000;

/// The longest feature key (so its controls' names stay within the
/// registry's 64 characters).
pub const MAX_KEY_LEN: usize = 48;

/// A compiled version range, `[min, max)`, either end open, as text (so it
/// can be a `const`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The lowest version included, if any.
    pub min: Option<&'static str>,
    /// The first version excluded, if any.
    pub max: Option<&'static str>,
}

impl Span {
    /// Every version.
    pub const ANY: Self = Self {
        min: None,
        max: None,
    };

    /// `min` and later.
    #[must_use]
    pub const fn from(min: &'static str) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// The range, or why its text isn't versions.
    fn range(&self) -> Result<VersionRange, String> {
        let parse = |text: Option<&str>| {
            text.map(str::parse::<Version>)
                .transpose()
                .map_err(|e| e.to_string())
        };
        Ok(VersionRange::new(parse(self.min)?, parse(self.max)?))
    }
}

/// One feature an adapter gates by its tool's version: what the remote
/// `compat` section may adjust, and its compiled values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompatFeature {
    /// The controls' name prefix, `[a-z0-9_]`, unique across adapters by
    /// starting with the tool's own name (`claude_app_move`).
    pub key: &'static str,
    /// The tool versions the feature is offered for, compiled.
    pub versions: Span,
    /// The widest range the code supports: the file can't widen past it.
    pub bounds: Span,
    /// This build's shape-check revision for the feature (bump it when the
    /// check learns a new shape).
    pub shape: u32,
}

impl CompatFeature {
    /// The control names, `(on, versions, shape)`.
    fn names(&self) -> [String; 3] {
        [
            format!("{}_on", self.key),
            format!("{}_versions", self.key),
            format!("{}_shape", self.key),
        ]
    }
}

/// Why the compiled list can't be registered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompatError {
    /// Two features share a key.
    #[error("compat feature {0:?} is declared twice")]
    Duplicate(String),
    /// A key that's too long or not `[a-z0-9_]`.
    #[error("compat feature key {0:?} is not allowed")]
    BadKey(String),
    /// A span that isn't versions.
    #[error("compat feature {0:?}: {1}")]
    BadSpan(String, String),
    /// The remote-config registry refused the section.
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// The `compat` section for `features` (every adapter's), to register with
/// the app's remote-config registry.
///
/// # Errors
///
/// [`CompatError`] for a duplicate or malformed key, a span that isn't
/// versions, or compiled values outside their own hard limits (the
/// registry's own check, run here first so a test catches it).
pub fn section<'a>(
    features: impl IntoIterator<Item = &'a CompatFeature>,
) -> Result<SectionSpec, CompatError> {
    let mut spec = SectionSpec::new(SECTION, SCHEMA);
    let mut keys = BTreeSet::new();
    for feature in features {
        let key = feature.key;
        if key.is_empty()
            || key.len() > MAX_KEY_LEN
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(CompatError::BadKey(key.to_owned()));
        }
        if !keys.insert(key) {
            return Err(CompatError::Duplicate(key.to_owned()));
        }
        let span = |span: &Span| {
            span.range()
                .map_err(|e| CompatError::BadSpan(key.into(), e))
        };
        let [on, versions, shape] = feature.names();
        spec = spec
            .switch(&on, true, RemoteMode::OffOnly)
            .control(
                &versions,
                Control::Versions {
                    default: span(&feature.versions)?,
                    bounds: span(&feature.bounds)?,
                },
            )
            .control(
                &shape,
                Control::Number {
                    default: i64::from(feature.shape),
                    min: i64::from(feature.shape),
                    max: MAX_SHAPE.max(i64::from(feature.shape)),
                },
            );
    }
    // The registry's own declaration checks, so a bad compiled list fails
    // here (and in the adapters' test), not only at start-up.
    juggler_remote_config::Registry::new().register(spec.clone())?;
    Ok(spec)
}

/// Why a feature isn't offered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Off {
    /// Its kill switch is off.
    Killed,
    /// The tool's version is outside the range it's offered for.
    Version {
        /// The tool's version.
        version: String,
    },
    /// The range was narrowed and the tool's version couldn't be read.
    VersionUnknown,
    /// A newer shape check than this build's is required.
    Shape {
        /// The revision required.
        required: i64,
        /// This build's.
        have: u32,
    },
}

impl fmt::Display for Off {
    /// A clause that follows the feature's name ("Move is …").
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Killed => {
                f.write_str("turned off for now while AI Juggler checks a problem with it")
            }
            Self::Version { version } => write!(
                f,
                "turned off for version {version} for now while AI Juggler checks that version"
            ),
            Self::VersionUnknown => f.write_str(
                "limited to checked versions for now, and AI Juggler couldn't read this one",
            ),
            Self::Shape { .. } => {
                f.write_str("waiting for an AI Juggler update that knows this version")
            }
        }
    }
}

/// The compat values in effect, read through [`Gate::check`].
///
/// A control the gate has no value for is at its compiled default, so
/// [`Gate::default`] is the compiled list itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    values: BTreeMap<String, Setting>,
}

impl Gate {
    /// The compat section's values from the merged configuration.
    #[must_use]
    pub fn from_effective(effective: &Effective) -> Self {
        let values = effective
            .iter()
            .filter(|(key, _)| key.section == SECTION)
            .map(|(key, resolved)| (key.control.clone(), resolved.value.clone()))
            .collect();
        Self { values }
    }

    /// Whether `feature` is offered for its tool at `tool_version` (as the
    /// adapter read it; `None` when it couldn't).
    ///
    /// # Errors
    ///
    /// [`Off`] when it isn't: killed, a version outside its range, a
    /// narrowed range with an unreadable version, or a newer shape check
    /// required.
    pub fn check(&self, feature: &CompatFeature, tool_version: Option<&str>) -> Result<(), Off> {
        let [on, versions, shape] = feature.names();
        if matches!(self.values.get(&on), Some(Setting::Switch(false))) {
            return Err(Off::Killed);
        }
        if let Some(Setting::Number(required)) = self.values.get(&shape)
            && *required > i64::from(feature.shape)
        {
            return Err(Off::Shape {
                required: *required,
                have: feature.shape,
            });
        }
        let compiled = feature.versions.range().unwrap_or_default();
        let range = match self.values.get(&versions) {
            Some(Setting::Versions(range)) => range.clone(),
            _ => compiled.clone(),
        };
        match tool_version.and_then(parse_tool_version) {
            Some(version) if range.contains(&version) => Ok(()),
            Some(_) => Err(Off::Version {
                version: tool_version.unwrap_or_default().trim().to_owned(),
            }),
            // An unread version passes only what the compiled list
            // passes: once the file narrows the range, it's refused.
            None if compiled.within(&range) => Ok(()),
            None => Err(Off::VersionUnknown),
        }
    }
}

/// A tool's version as adapters report it (`2.1.4`, `v2.1.4`,
/// `2.1.4 (Claude Code)`): its first word, without a leading `v`. A
/// two-part version reads as `.0`.
fn parse_tool_version(text: &str) -> Option<Version> {
    let word = text.split_whitespace().next()?;
    let word = word.strip_prefix('v').unwrap_or(word);
    word.parse()
        .ok()
        .or_else(|| format!("{word}.0").parse().ok())
}

/// The gate in effect, set by the app once the layers are merged.
static CURRENT: RwLock<Option<Arc<Gate>>> = RwLock::new(None);

/// Puts `gate` in effect for every adapter (the app does this after each
/// configuration change; it applies live).
pub fn install(gate: Gate) {
    let gate = Arc::new(gate);
    match CURRENT.write() {
        Ok(mut current) => *current = Some(gate),
        Err(poisoned) => *poisoned.into_inner() = Some(gate),
    }
}

/// The gate in effect: the compiled list until the app [`install`]s one.
#[must_use]
pub fn current() -> Arc<Gate> {
    let current = match CURRENT.read() {
        Ok(current) => current.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    current.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use juggler_remote_config::{
        Channel, Facts, InstallId, Os, Registry, SectionIssue, UserChoices, merge,
    };
    use serde_json::{Value, json};

    use super::*;

    /// A made-up feature: compiled for 2.x, the code supports 1.5 and later.
    const MOVE: CompatFeature = CompatFeature {
        key: "tool_move",
        versions: Span {
            min: Some("2.0.0"),
            max: Some("3.0.0"),
        },
        bounds: Span::from("1.5.0"),
        shape: 2,
    };

    fn registry() -> Registry {
        let mut registry = Registry::new();
        registry.register(section([&MOVE]).unwrap()).unwrap();
        registry
    }

    fn facts() -> Facts {
        Facts {
            app_version: "0.5.0".parse().unwrap(),
            channel: Channel::Stable,
            os: Os::Macos,
            os_version: None,
            tools: BTreeMap::new(),
            install: InstallId::from_bytes([5; 16]),
        }
    }

    /// The gate an app on 0.5.0 gets from a file with `values` in its
    /// compat section (`None`: no file at all).
    fn gate_for(values: Option<Value>) -> (Gate, Vec<SectionIssue>) {
        let registry = registry();
        let (layer, issues) = match values {
            Some(values) => {
                let sections: BTreeMap<String, Value> =
                    serde_json::from_value(json!({SECTION: {"schema": 1, "values": values}}))
                        .unwrap();
                let (layer, issues) = registry.resolve(&sections, &facts());
                (Some(layer), issues)
            }
            None => (None, Vec::new()),
        };
        let effective = merge(&registry, layer.as_ref(), None, &UserChoices::new());
        (Gate::from_effective(&effective), issues)
    }

    #[test]
    fn the_compiled_list_is_layer_zero() {
        for gate in [Gate::default(), gate_for(None).0] {
            assert_eq!(gate.check(&MOVE, Some("2.4.1")), Ok(()));
            assert_eq!(
                gate.check(&MOVE, Some("3.0.0")),
                Err(Off::Version {
                    version: "3.0.0".to_owned()
                })
            );
            assert!(gate.check(&MOVE, Some("1.9.0 (Tool)")).is_err());
            // An unread version gets what the compiled list gives it.
            assert_eq!(gate.check(&MOVE, None), Ok(()));
        }
    }

    #[test]
    fn narrowing_hides_a_feature() {
        let (gate, issues) = gate_for(Some(json!({"tool_move_versions":
            {"min": "2.0.0", "max": "2.4.0"}})));
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(gate.check(&MOVE, Some("2.3.9")), Ok(()));
        assert_eq!(
            gate.check(&MOVE, Some("v2.4.0")),
            Err(Off::Version {
                version: "v2.4.0".to_owned()
            })
        );
        // Narrowed: an unread version is no longer given the benefit.
        assert_eq!(gate.check(&MOVE, None), Err(Off::VersionUnknown));
    }

    #[test]
    fn a_kill_turns_it_off() {
        let (gate, _) = gate_for(Some(json!({"tool_move_on": false})));
        assert_eq!(gate.check(&MOVE, Some("2.4.1")), Err(Off::Killed));
        assert_eq!(gate.check(&MOVE, None), Err(Off::Killed));
        // A kill switch can't be turned on: the value is dropped.
        let (gate, issues) = gate_for(Some(json!({"tool_move_on": true})));
        assert_eq!(
            gate,
            Gate::from_effective(&merge(&registry(), None, None, &UserChoices::new()))
        );
        assert!(matches!(
            issues.as_slice(),
            [SectionIssue::ValueDropped { .. }]
        ));
    }

    #[test]
    fn widening_within_the_hard_limit_applies_and_past_it_is_clamped() {
        let (gate, _) = gate_for(Some(
            json!({"tool_move_versions": {"min": "1.5.0", "max": "4.0.0"}}),
        ));
        assert_eq!(gate.check(&MOVE, Some("1.6.0")), Ok(()));
        assert_eq!(gate.check(&MOVE, Some("3.5.0")), Ok(()));
        // Past the hard limit (the code supports 1.5 and later): clamped
        // to it, while the file's own upper limit still holds.
        let (gate, issues) = gate_for(Some(
            json!({"tool_move_versions": {"min": "0.1.0", "max": "3.5.0"}}),
        ));
        assert!(
            matches!(issues.as_slice(), [SectionIssue::Clamped { .. }]),
            "{issues:?}"
        );
        assert_eq!(gate.check(&MOVE, Some("1.5.0")), Ok(()));
        assert!(
            gate.check(&MOVE, Some("1.4.9")).is_err(),
            "never below the hard limit"
        );
        assert!(gate.check(&MOVE, Some("3.5.0")).is_err());
    }

    #[test]
    fn a_newer_shape_check_turns_it_off_until_an_update() {
        let (gate, _) = gate_for(Some(json!({"tool_move_shape": 3})));
        assert_eq!(
            gate.check(&MOVE, Some("2.4.1")),
            Err(Off::Shape {
                required: 3,
                have: 2
            })
        );
        // An older requirement can't lower anything: clamped to this build's.
        let (gate, _) = gate_for(Some(json!({"tool_move_shape": 1})));
        assert_eq!(gate.check(&MOVE, Some("2.4.1")), Ok(()));
    }

    #[test]
    fn a_missing_invalid_or_old_file_falls_back_to_compiled() {
        let compiled = gate_for(None).0;
        let cases = [
            // An invalid value: dropped alone.
            json!({"tool_move_versions": "2.0.0"}),
            json!({"tool_move_on": "no"}),
            // Controls this build doesn't have (a newer file).
            json!({"other_tool_move_on": false}),
            // An empty section (an old file that never set anything).
            json!({}),
        ];
        for values in cases {
            assert_eq!(gate_for(Some(values.clone())).0, compiled, "{values}");
        }
        // A section that isn't readable at all, or a newer schema's
        // widening: compiled too.
        let registry = registry();
        for raw in [
            json!("broken"),
            json!({"schema": 9, "values":
            {"tool_move_versions": {"min": "1.5.0"}}}),
        ] {
            let sections = BTreeMap::from([(SECTION.to_owned(), raw)]);
            let (layer, _) = registry.resolve(&sections, &facts());
            let effective = merge(&registry, Some(&layer), None, &UserChoices::new());
            assert_eq!(Gate::from_effective(&effective), compiled);
        }
        // An expired, unsigned or old file never reaches the merge: the
        // fetcher gives no layer (`None`), which is the compiled list.
        assert_eq!(compiled.check(&MOVE, Some("2.0.0")), Ok(()));
    }

    #[test]
    fn the_compiled_list_is_checked() {
        let bad_key = CompatFeature {
            key: "Tool-Move",
            ..MOVE
        };
        assert!(matches!(section([&bad_key]), Err(CompatError::BadKey(_))));
        assert!(matches!(
            section([&MOVE, &MOVE]),
            Err(CompatError::Duplicate(_))
        ));
        let outside = CompatFeature {
            bounds: Span::from("2.5.0"),
            ..MOVE
        };
        assert!(matches!(section([&outside]), Err(CompatError::Registry(_))));
        let not_versions = CompatFeature {
            versions: Span::from("two"),
            ..MOVE
        };
        assert!(matches!(
            section([&not_versions]),
            Err(CompatError::BadSpan(..))
        ));
    }

    #[test]
    fn the_gate_in_effect_is_the_installed_one() {
        // The only test that touches the process-wide gate.
        assert_eq!(*current(), Gate::default());
        let (killed, _) = gate_for(Some(json!({"tool_move_on": false})));
        install(killed.clone());
        assert_eq!(*current(), killed);
        install(Gate::default());
        assert_eq!(current().check(&MOVE, Some("2.0.0")), Ok(()));
    }
}
