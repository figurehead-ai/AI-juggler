//! Sections and their typed schemas (§1, §2).
//!
//! A feature module registers a [`SectionSpec`]: a section name, the
//! schema version it reads, and the **controls** it compiles in. A control
//! is something that already exists in the code, with the only changes the
//! file may make to it:
//!
//! - [`Control::Switch`]: turn a compiled feature on or off. Its
//!   [`RemoteMode`] says which: `OffOnly` (a kill switch: the file may only
//!   say `false`), `OnOff`, or `Rollout` (on only for a bucketed rule).
//! - [`Control::Versions`]: widen or narrow a version range, never past the
//!   compiled `bounds` (what the code's shape checks support).
//! - [`Control::Number`]: a tunable, clamped into its compiled bounds.
//! - [`Control::Choice`]: pick one of the compiled options.
//!
//! There is no control that carries code, markup, a path, a command or a
//! URL: the file can only select among behaviour already in the app.
//!
//! # A section in the file
//!
//! ```text
//! "flags": {
//!   "schema": 1,
//!   "values": {
//!     "new_feed":  false,
//!     "fast_path": { "rules": [ { "when": { "buckets": [0, 9] }, "value": true } ] }
//!   }
//! }
//! ```
//!
//! A value is either plain (for everyone) or a list of rules, the first
//! matching rule wins, and no match leaves the control to the layer below.
//!
//! # What is ignored and what is rejected
//!
//! - An unknown section, or an unknown control in a known section, is
//!   ignored (an old app meeting a new file).
//! - A section whose `schema` is newer than the module reads is ignored
//!   until the app updates.
//! - Anything else that doesn't fit the schema (a wrong type, a kill switch
//!   turned on, a range past its bounds, an option not compiled in, a
//!   malformed rule) drops that whole section, alone; the other sections
//!   still apply.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::target::{Facts, When};
use crate::version::VersionRange;

/// The longest section or control name.
pub const MAX_NAME_LEN: usize = 64;

/// The most rules one control may carry.
pub const MAX_RULES: usize = 32;

/// What the remote file may do to a switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteMode {
    /// The file may only turn it off (a kill switch).
    OffOnly,
    /// The file may turn it on or off.
    OnOff,
    /// The file may turn it off, or on for a bucketed rule only.
    Rollout,
}

/// One compiled-in thing the file may adjust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Control {
    /// A feature switch.
    Switch {
        /// The compiled default.
        default: bool,
        /// What the file may do.
        remote: RemoteMode,
    },
    /// A version range, within compiled bounds.
    Versions {
        /// The compiled default.
        default: VersionRange,
        /// The widest range the code supports.
        bounds: VersionRange,
    },
    /// An integer tunable, clamped to `[min, max]`.
    Number {
        /// The compiled default.
        default: i64,
        /// The lowest allowed value.
        min: i64,
        /// The highest allowed value.
        max: i64,
    },
    /// One of the compiled options.
    Choice {
        /// The compiled default.
        default: String,
        /// Every option the code has.
        options: Vec<String>,
    },
}

/// A control's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Setting {
    /// A switch's state.
    Switch(bool),
    /// A version range.
    Versions(VersionRange),
    /// A number.
    Number(i64),
    /// An option.
    Choice(String),
}

impl Control {
    /// The compiled default.
    #[must_use]
    pub fn default_setting(&self) -> Setting {
        match self {
            Self::Switch { default, .. } => Setting::Switch(*default),
            Self::Versions { default, .. } => Setting::Versions(default.clone()),
            Self::Number { default, .. } => Setting::Number(*default),
            Self::Choice { default, .. } => Setting::Choice(default.clone()),
        }
    }

    fn check_declaration(&self) -> Result<(), &'static str> {
        match self {
            Self::Switch { .. } => Ok(()),
            Self::Versions { default, bounds } if default.within(bounds) && !bounds.is_empty() => {
                Ok(())
            }
            Self::Versions { .. } => Err("the default range is outside the bounds"),
            Self::Number { default, min, max } if min <= max && (min..=max).contains(&default) => {
                Ok(())
            }
            Self::Number { .. } => Err("the default number is outside the bounds"),
            Self::Choice { default, options } if options.contains(default) => Ok(()),
            Self::Choice { .. } => Err("the default is not one of the options"),
        }
    }

    /// Parses and checks a remote value; `bucketed` says whether its rule
    /// is limited to rollout buckets.
    fn remote_value(&self, value: &Value, bucketed: bool) -> Result<Setting, String> {
        match self {
            Self::Switch { remote, .. } => {
                let on = value.as_bool().ok_or("a switch takes true or false")?;
                match (remote, on, bucketed) {
                    (_, false, _)
                    | (RemoteMode::OnOff, true, _)
                    | (RemoteMode::Rollout, true, true) => Ok(Setting::Switch(on)),
                    (RemoteMode::OffOnly, true, _) => {
                        Err("a kill switch can't be turned on".into())
                    }
                    (RemoteMode::Rollout, true, false) => {
                        Err("a rollout turns on only for buckets".into())
                    }
                }
            }
            Self::Versions { bounds, .. } => {
                let range: VersionRange =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                if range.within(bounds) {
                    Ok(Setting::Versions(range))
                } else {
                    Err("a range past its compiled bounds".into())
                }
            }
            Self::Number { min, max, .. } => {
                let number = value.as_i64().ok_or("a number takes an integer")?;
                let clamped = number.clamp(*min, *max);
                if clamped != number {
                    tracing::warn!(number, clamped, "remote config: a number was clamped");
                }
                Ok(Setting::Number(clamped))
            }
            Self::Choice { options, .. } => {
                let pick = value.as_str().ok_or("a choice takes a string")?;
                if options.iter().any(|option| option == pick) {
                    Ok(Setting::Choice(pick.to_owned()))
                } else {
                    Err("an option the app doesn't have".into())
                }
            }
        }
    }

    /// Checks a user's own value: any switch state, a range only narrowed
    /// (applied in the merge), a number clamped, a compiled option.
    pub(crate) fn user_value(&self, setting: &Setting) -> Option<Setting> {
        match (self, setting) {
            (Self::Switch { .. }, Setting::Switch(_))
            | (Self::Versions { .. }, Setting::Versions(_)) => Some(setting.clone()),
            (Self::Number { min, max, .. }, Setting::Number(n)) => {
                Some(Setting::Number((*n).clamp(*min, *max)))
            }
            (Self::Choice { options, .. }, Setting::Choice(pick)) if options.contains(pick) => {
                Some(setting.clone())
            }
            _ => None,
        }
    }
}

/// A section a module registers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpec {
    name: String,
    schema: u32,
    controls: BTreeMap<String, Control>,
}

impl SectionSpec {
    /// A section named `name` whose module reads schema versions
    /// `1..=schema`.
    #[must_use]
    pub fn new(name: &str, schema: u32) -> Self {
        Self {
            name: name.to_owned(),
            schema,
            controls: BTreeMap::new(),
        }
    }

    /// Adds a control (a later one with the same name replaces it; the
    /// registry rejects bad declarations).
    #[must_use]
    pub fn control(mut self, name: &str, control: Control) -> Self {
        self.controls.insert(name.to_owned(), control);
        self
    }

    /// Adds a switch.
    #[must_use]
    pub fn switch(self, name: &str, default: bool, remote: RemoteMode) -> Self {
        self.control(name, Control::Switch { default, remote })
    }

    /// The section's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The section's controls.
    #[must_use]
    pub const fn controls(&self) -> &BTreeMap<String, Control> {
        &self.controls
    }
}

/// A section that can't be registered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// Two modules registered one section name.
    #[error("section {0:?} is registered twice")]
    Duplicate(String),
    /// A name that's empty, overlong, or not `[a-z0-9_]`.
    #[error("name {0:?} is not allowed")]
    BadName(String),
    /// A control whose default breaks its own bounds or options.
    #[error("{section}.{control}: {reason}")]
    BadControl {
        /// The section.
        section: String,
        /// The control.
        control: String,
        /// What's wrong.
        reason: &'static str,
    },
    /// Schema version 0.
    #[error("section {0:?} has schema 0")]
    BadSchema(String),
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// A control's address: section, then control.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingKey {
    /// The section.
    pub section: String,
    /// The control.
    pub control: String,
}

impl SettingKey {
    /// `section.control`.
    #[must_use]
    pub fn new(section: &str, control: &str) -> Self {
        Self {
            section: section.to_owned(),
            control: control.to_owned(),
        }
    }

    /// `section.control`, the rollout salt.
    fn salt(&self) -> String {
        format!("{}.{}", self.section, self.control)
    }
}

/// One layer's values, after validation and targeting.
pub type LayerValues = BTreeMap<SettingKey, Setting>;

/// Why a section of a file wasn't used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionIssue {
    /// No module registered it (kept in the file, ignored).
    Unknown(String),
    /// Its schema is newer than this app reads.
    Newer {
        /// The section.
        section: String,
        /// Its schema.
        schema: u64,
    },
    /// It broke its schema and was dropped.
    Invalid {
        /// The section.
        section: String,
        /// Why.
        reason: String,
    },
}

/// The registered sections. Built in one place at start-up (ENGINEERING-
/// GUIDE A5: one explicit registry).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    sections: BTreeMap<String, SectionSpec>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a module's section.
    ///
    /// # Errors
    ///
    /// [`RegistryError`] for a duplicate section, a bad name, schema 0, or
    /// a control whose default breaks its own declaration.
    pub fn register(&mut self, spec: SectionSpec) -> Result<(), RegistryError> {
        if !valid_name(&spec.name) {
            return Err(RegistryError::BadName(spec.name));
        }
        if spec.schema == 0 {
            return Err(RegistryError::BadSchema(spec.name));
        }
        for (control, declared) in &spec.controls {
            if !valid_name(control) {
                return Err(RegistryError::BadName(control.clone()));
            }
            declared
                .check_declaration()
                .map_err(|reason| RegistryError::BadControl {
                    section: spec.name.clone(),
                    control: control.clone(),
                    reason,
                })?;
        }
        if self.sections.contains_key(&spec.name) {
            return Err(RegistryError::Duplicate(spec.name));
        }
        self.sections.insert(spec.name.clone(), spec);
        Ok(())
    }

    /// The control at `key`, if registered.
    #[must_use]
    pub fn control(&self, key: &SettingKey) -> Option<&Control> {
        self.sections.get(&key.section)?.controls.get(&key.control)
    }

    /// Every registered control.
    pub fn controls(&self) -> impl Iterator<Item = (SettingKey, &Control)> {
        self.sections.values().flat_map(|spec| {
            spec.controls
                .iter()
                .map(|(name, control)| (SettingKey::new(&spec.name, name), control))
        })
    }

    /// Validates a file's sections and evaluates their rules against
    /// `facts`: the values this layer sets, and what was left out and why.
    ///
    /// Each section is validated whole before any of it is used, so a bad
    /// section never half-applies.
    #[must_use]
    pub fn resolve(
        &self,
        sections: &BTreeMap<String, Value>,
        facts: &Facts,
    ) -> (LayerValues, Vec<SectionIssue>) {
        let mut values = LayerValues::new();
        let mut issues = Vec::new();
        for (name, raw) in sections {
            let Some(spec) = self.sections.get(name) else {
                issues.push(SectionIssue::Unknown(name.clone()));
                continue;
            };
            match parse_section(spec, raw) {
                Ok(Parsed::Newer(schema)) => issues.push(SectionIssue::Newer {
                    section: name.clone(),
                    schema,
                }),
                Ok(Parsed::Rules(controls)) => {
                    for (control, rules) in controls {
                        let key = SettingKey::new(name, &control);
                        let salt = key.salt();
                        if let Some((_, setting)) = rules
                            .into_iter()
                            .find(|(when, _)| when.matches(facts, &salt))
                        {
                            values.insert(key, setting);
                        }
                    }
                }
                Err(reason) => {
                    tracing::warn!(section = %name, %reason, "remote config: section dropped");
                    issues.push(SectionIssue::Invalid {
                        section: name.clone(),
                        reason,
                    });
                }
            }
        }
        (values, issues)
    }
}

enum Parsed {
    Newer(u64),
    Rules(Vec<(String, Vec<(When, Setting)>)>),
}

fn parse_section(spec: &SectionSpec, raw: &Value) -> Result<Parsed, String> {
    let object = raw.as_object().ok_or("a section is an object")?;
    let schema = object
        .get("schema")
        .and_then(Value::as_u64)
        .filter(|schema| *schema >= 1)
        .ok_or("a section needs a schema number")?;
    if schema > u64::from(spec.schema) {
        return Ok(Parsed::Newer(schema));
    }
    let values = match object.get("values") {
        None => return Ok(Parsed::Rules(Vec::new())),
        Some(values) => values.as_object().ok_or("`values` is an object")?,
    };
    let mut out = Vec::new();
    for (name, entry) in values {
        // An unknown control: a newer file; ignored.
        let Some(control) = spec.controls.get(name) else {
            continue;
        };
        let rules = match entry.as_object().and_then(|o| o.get("rules")) {
            Some(rules) => parse_rules(control, rules).map_err(|e| format!("{name}: {e}"))?,
            None => vec![(
                When::Always,
                control
                    .remote_value(entry, false)
                    .map_err(|e| format!("{name}: {e}"))?,
            )],
        };
        out.push((name.clone(), rules));
    }
    Ok(Parsed::Rules(out))
}

fn parse_rules(control: &Control, rules: &Value) -> Result<Vec<(When, Setting)>, String> {
    let rules = rules.as_array().ok_or("`rules` is a list")?;
    if rules.len() > MAX_RULES {
        return Err("too many rules".to_owned());
    }
    rules
        .iter()
        .map(|rule| {
            let rule = rule.as_object().ok_or("a rule is an object")?;
            let when = match rule.get("when") {
                Some(when) => When::parse(when).map_err(|e| e.to_string())?,
                None => When::Always,
            };
            let value = rule.get("value").ok_or("a rule needs a value")?;
            // A rule this app can't read never applies, so a newer file's
            // rollout isn't refused for lacking buckets it may well have.
            let bucketed = when.is_bucketed() || when == When::Unknown;
            let setting = control.remote_value(value, bucketed)?;
            Ok((when, setting))
        })
        .collect()
}
