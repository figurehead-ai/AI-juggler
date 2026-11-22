//! List sections (§2): a section that carries records instead of (or as
//! well as) controls, such as short in-app messages.
//!
//! A module declares the shape of its records as a [`ListSpec`]: how many
//! the file may carry, and each field's kind. A field is as closed as a
//! control:
//!
//! - [`FieldKind::Text`]: plain text of at most a compiled length; no
//!   control characters, no direction overrides, no `<` or `>` (so no
//!   markup), never blank.
//! - [`FieldKind::Choice`]: one of the compiled options.
//! - [`FieldKind::Flag`]: true or false.
//! - [`FieldKind::Time`]: Unix seconds.
//! - [`FieldKind::Link`]: an `https://` link to one of the compiled hosts:
//!   no other scheme, host, port or user part.
//! - [`FieldKind::Count`]: a whole number within compiled bounds (a size,
//!   an amount of memory).
//! - [`FieldKind::Sha256`]: a SHA-256, exactly 64 lower-case hex
//!   characters.
//!
//! Nothing carries code, a path or a command, and what a record *does* is
//! the module's own compiled behaviour.
//!
//! # A list in the file
//!
//! ```text
//! "notices": {
//!   "schema": 1,
//!   "items": [
//!     { "id": "tool_a_2_3", "when": { "tools": { "tool-a": { "min": "2.3.0" } } },
//!       "level": "needs_you", "title": "…" }
//!   ]
//! }
//! ```
//!
//! Every record has an `id` (`[a-z0-9_-]`, at most 64 characters, unique
//! in the list) and may have a `when`, the same targeting a rule uses
//! ([`crate::target`]), evaluated here; its rollout bucket is keyed by
//! `section.id`.
//!
//! # Tolerant by record (an old app meets a new file)
//!
//! - A record that doesn't match this app is skipped without reading it,
//!   and a `when` this app can't read never matches.
//! - Unknown fields are ignored.
//! - A record missing a required field, or whose required field this app
//!   can't read (a newer option, a longer text), is dropped alone and
//!   reported; an optional field it can't read (a link to a host it
//!   doesn't allow) is dropped alone and the record kept.
//! - A section whose `schema` is newer than this app reads gives no
//!   records: nothing in a list is a kill switch.
//!
//! The publisher's check ([`crate::Registry::validate`]) is strict: each of these
//! is a mistake for the app the schema came from.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::{Reading, SectionIssue, valid_name};
use crate::target::{Facts, When};

/// The most records a list may declare it carries.
pub const MAX_ITEMS: u32 = 64;

/// The longest text field a list may declare, in characters.
pub const MAX_TEXT_CHARS: u32 = 2_000;

/// The longest link, in bytes.
pub const MAX_LINK_LEN: usize = 512;

/// The longest record id.
pub const MAX_ID_LEN: usize = 64;

/// What a field holds.
///
/// Serialised (externally tagged, `snake_case`) only for the schema the
/// publisher validates against ([`crate::Registry::export`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldKind {
    /// Plain text of 1 to `max_chars` characters.
    Text {
        /// The most characters.
        max_chars: u32,
    },
    /// One of the compiled options.
    Choice {
        /// Every option the code has.
        options: Vec<String>,
    },
    /// True or false.
    Flag,
    /// Unix seconds.
    Time,
    /// An `https://` link to one of `hosts`.
    Link {
        /// The only hosts it may point at, lower case.
        hosts: Vec<String>,
    },
    /// A whole number from `min` to `max`, both included.
    Count {
        /// The least.
        min: u64,
        /// The most (at most the canonical form's largest integer).
        max: u64,
    },
    /// A SHA-256: exactly 64 lower-case hex characters.
    Sha256,
}

/// One field of a record: its kind, and whether a record may leave it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// What it holds.
    pub kind: FieldKind,
    /// A record may leave it out (and loses only it when it can't be read).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

impl Field {
    /// A field every record must carry.
    #[must_use]
    pub const fn required(kind: FieldKind) -> Self {
        Self {
            kind,
            optional: false,
        }
    }

    /// A field a record may leave out.
    #[must_use]
    pub const fn optional(kind: FieldKind) -> Self {
        Self {
            kind,
            optional: true,
        }
    }
}

/// The records a section lists: how many at most, and their fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSpec {
    max_items: u32,
    fields: BTreeMap<String, Field>,
}

/// A field's value in a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// Plain text.
    Text(String),
    /// A compiled option.
    Choice(String),
    /// True or false.
    Flag(bool),
    /// Unix seconds.
    Time(u64),
    /// An `https://` link to an allowed host.
    Link(String),
    /// A whole number within its bounds.
    Count(u64),
    /// A SHA-256, 64 lower-case hex characters.
    Sha256(String),
}

/// One record, read and targeted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Its id, unique in its list.
    pub id: String,
    /// Its targeting (it matched this app).
    pub when: When,
    /// The fields this app could read.
    pub fields: BTreeMap<String, FieldValue>,
}

impl Item {
    /// A text field.
    #[must_use]
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.fields.get(name)? {
            FieldValue::Text(text) => Some(text),
            _ => None,
        }
    }

    /// A choice field.
    #[must_use]
    pub fn choice(&self, name: &str) -> Option<&str> {
        match self.fields.get(name)? {
            FieldValue::Choice(pick) => Some(pick),
            _ => None,
        }
    }

    /// A flag field.
    #[must_use]
    pub fn flag(&self, name: &str) -> Option<bool> {
        match self.fields.get(name)? {
            FieldValue::Flag(on) => Some(*on),
            _ => None,
        }
    }

    /// A time field.
    #[must_use]
    pub fn time(&self, name: &str) -> Option<u64> {
        match self.fields.get(name)? {
            FieldValue::Time(at) => Some(*at),
            _ => None,
        }
    }

    /// A link field.
    #[must_use]
    pub fn link(&self, name: &str) -> Option<&str> {
        match self.fields.get(name)? {
            FieldValue::Link(url) => Some(url),
            _ => None,
        }
    }

    /// A whole-number field.
    #[must_use]
    pub fn count(&self, name: &str) -> Option<u64> {
        match self.fields.get(name)? {
            FieldValue::Count(count) => Some(*count),
            _ => None,
        }
    }

    /// A SHA-256 field, 64 lower-case hex characters.
    #[must_use]
    pub fn sha256(&self, name: &str) -> Option<&str> {
        match self.fields.get(name)? {
            FieldValue::Sha256(hash) => Some(hash),
            _ => None,
        }
    }
}

/// One layer's records, by section.
pub type Lists = BTreeMap<String, Vec<Item>>;

/// The keys every record has besides its fields.
const RESERVED: [&str; 2] = ["id", "when"];

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && host.contains('.')
        && host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

/// A character plain text may not hold: control characters (line breaks
/// too: a notice is short), the bidirectional overrides and isolates
/// (they can make text read differently from what it is), and `<` or `>`
/// (no markup).
fn refused_char(c: char) -> bool {
    c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '<' | '>')
}

impl ListSpec {
    /// A list of at most `max_items` records.
    #[must_use]
    pub const fn new(max_items: u32) -> Self {
        Self {
            max_items,
            fields: BTreeMap::new(),
        }
    }

    /// Adds a field (a later one with the same name replaces it; the
    /// registry rejects bad declarations).
    #[must_use]
    pub fn field(mut self, name: &str, field: Field) -> Self {
        self.fields.insert(name.to_owned(), field);
        self
    }

    /// The most records.
    #[must_use]
    pub const fn max_items(&self) -> u32 {
        self.max_items
    }

    /// The fields.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, Field> {
        &self.fields
    }

    /// Whether the declaration holds: the offending field and why if not.
    pub(crate) fn check_declaration(&self) -> Result<(), (String, &'static str)> {
        if self.max_items == 0 || self.max_items > MAX_ITEMS {
            return Err(("items".to_owned(), "a list carries 1 to 64 records"));
        }
        for (name, field) in &self.fields {
            let bad = |reason| Err((name.clone(), reason));
            if !valid_name(name) || RESERVED.contains(&name.as_str()) {
                return bad("a field name is [a-z0-9_] and not id or when");
            }
            match &field.kind {
                FieldKind::Text { max_chars } if *max_chars == 0 || *max_chars > MAX_TEXT_CHARS => {
                    return bad("a text field holds 1 to 2000 characters");
                }
                FieldKind::Choice { options }
                    if options.is_empty()
                        || !options.iter().all(|option| valid_name(option))
                        || options.iter().collect::<BTreeSet<_>>().len() != options.len() =>
                {
                    return bad("a choice has distinct [a-z0-9_] options");
                }
                FieldKind::Link { hosts }
                    if hosts.is_empty() || !hosts.iter().all(|host| valid_host(host)) =>
                {
                    return bad("a link names lower-case hosts");
                }
                FieldKind::Count { min, max }
                    if min > max || *max > crate::canonical::MAX_SAFE_INTEGER.unsigned_abs() =>
                {
                    return bad("a count runs from min to max, within the canonical integers");
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Reads one field's value, or says why it can't.
    fn read(kind: &FieldKind, value: &Value) -> Result<FieldValue, String> {
        match kind {
            FieldKind::Text { max_chars } => {
                let text = value.as_str().ok_or("text takes a string")?;
                let chars = text.chars().count();
                if text.trim().is_empty() {
                    Err("text is blank".to_owned())
                } else if chars > usize::try_from(*max_chars).unwrap_or(usize::MAX) {
                    Err(format!("text is over {max_chars} characters"))
                } else if text.chars().any(refused_char) {
                    Err("text is plain: no control, direction or markup characters".to_owned())
                } else {
                    Ok(FieldValue::Text(text.to_owned()))
                }
            }
            FieldKind::Choice { options } => {
                let pick = value.as_str().ok_or("a choice takes a string")?;
                if options.iter().any(|option| option == pick) {
                    Ok(FieldValue::Choice(pick.to_owned()))
                } else {
                    Err("an option the app doesn't have".to_owned())
                }
            }
            FieldKind::Flag => value
                .as_bool()
                .map(FieldValue::Flag)
                .ok_or_else(|| "a flag takes true or false".to_owned()),
            FieldKind::Time => value
                .as_u64()
                .map(FieldValue::Time)
                .ok_or_else(|| "a time takes Unix seconds".to_owned()),
            FieldKind::Link { hosts } => {
                let url = value.as_str().ok_or("a link takes a string")?;
                let clean = url.len() <= MAX_LINK_LEN
                    && url.is_ascii()
                    && !url
                        .bytes()
                        .any(|b| b.is_ascii_control() || b == b' ' || b == b'\\');
                let allowed = clean
                    && crate::fetch::https_host(url)
                        .is_some_and(|host| hosts.iter().any(|allowed| allowed == host));
                if allowed {
                    Ok(FieldValue::Link(url.to_owned()))
                } else {
                    Err("a link goes only to an allowed https host".to_owned())
                }
            }
            FieldKind::Count { min, max } => match value.as_u64() {
                Some(count) if (*min..=*max).contains(&count) => Ok(FieldValue::Count(count)),
                _ => Err(format!("a count is a whole number from {min} to {max}")),
            },
            FieldKind::Sha256 => match value.as_str() {
                Some(hash)
                    if hash.len() == 64
                        && hash
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
                {
                    Ok(FieldValue::Sha256(hash.to_owned()))
                }
                _ => Err("a SHA-256 is 64 lower-case hex characters".to_owned()),
            },
        }
    }

    /// Reads one record: `Err` drops it whole, the issues are the optional
    /// fields dropped alone.
    fn read_item(
        &self,
        object: &serde_json::Map<String, Value>,
        id: &str,
        when: When,
    ) -> Result<(Item, Vec<String>), String> {
        let mut fields = BTreeMap::new();
        let mut dropped = Vec::new();
        for (name, field) in &self.fields {
            match (object.get(name), field.optional) {
                (None, true) => {}
                (None, false) => return Err(format!("{name}: missing")),
                (Some(value), optional) => match Self::read(&field.kind, value) {
                    Ok(read) => {
                        fields.insert(name.clone(), read);
                    }
                    Err(why) if optional => dropped.push(format!("{name}: {why}")),
                    Err(why) => return Err(format!("{name}: {why}")),
                },
            }
        }
        let item = Item {
            id: id.to_owned(),
            when,
            fields,
        };
        Ok((item, dropped))
    }

    /// The records of section `section` that match `facts`, and what was
    /// left out and why (see the module docs).
    pub(crate) fn resolve(
        &self,
        section: &str,
        items: &Value,
        facts: &Facts,
        reading: Reading,
    ) -> (Vec<Item>, Vec<SectionIssue>) {
        let mut out = Vec::new();
        let mut issues = Vec::new();
        let issue = |id: &str, reason: String| SectionIssue::ValueDropped {
            section: section.to_owned(),
            control: id.to_owned(),
            reason,
        };
        if reading != Reading::Full {
            return (out, issues);
        }
        let Some(items) = items.as_array() else {
            issues.push(SectionIssue::Invalid {
                section: section.to_owned(),
                reason: "`items` is a list".to_owned(),
            });
            return (out, issues);
        };
        let max = usize::try_from(self.max_items).unwrap_or(usize::MAX);
        let mut seen = BTreeSet::new();
        for (index, raw) in items.iter().enumerate() {
            let Some(object) = raw.as_object() else {
                issues.push(issue(
                    &format!("#{index}"),
                    "a record is an object".to_owned(),
                ));
                continue;
            };
            let Some(id) = object
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| valid_id(id))
            else {
                issues.push(issue(
                    &format!("#{index}"),
                    "a record needs an id".to_owned(),
                ));
                continue;
            };
            if !seen.insert(id.to_owned()) {
                issues.push(issue(id, "the id is listed twice".to_owned()));
                continue;
            }
            let when = match object.get("when") {
                None => When::Always,
                Some(when) => When::parse(when).unwrap_or(When::Unknown),
            };
            if !when.matches(facts, &format!("{section}.{id}")) {
                continue;
            }
            if out.len() == max {
                issues.push(issue(id, format!("more than {max} records")));
                continue;
            }
            match self.read_item(object, id, when) {
                Ok((item, dropped)) => {
                    for reason in dropped {
                        tracing::warn!(%section, %id, %reason, "remote config: a field was dropped");
                        issues.push(issue(id, reason));
                    }
                    out.push(item);
                }
                Err(reason) => {
                    tracing::warn!(%section, %id, %reason, "remote config: a record was dropped");
                    issues.push(issue(id, reason));
                }
            }
        }
        (out, issues)
    }

    /// The publisher's strict check of a section's `items` (see the module
    /// docs): every problem, in words.
    pub(crate) fn validate(&self, items: &Value) -> Vec<String> {
        let mut problems = Vec::new();
        let Some(items) = items.as_array() else {
            return vec!["`items` is a list".to_owned()];
        };
        if items.len() > usize::try_from(self.max_items).unwrap_or(usize::MAX) {
            problems.push(format!("at most {} records", self.max_items));
        }
        let mut seen = BTreeSet::new();
        for (index, raw) in items.iter().enumerate() {
            let Some(object) = raw.as_object() else {
                problems.push(format!("#{index}: a record is an object"));
                continue;
            };
            let label = object
                .get("id")
                .and_then(Value::as_str)
                .map_or_else(|| format!("#{index}"), str::to_owned);
            let mut problem = |what: String| problems.push(format!("{label}: {what}"));
            match object.get("id").and_then(Value::as_str) {
                Some(id) if valid_id(id) => {
                    if !seen.insert(id.to_owned()) {
                        problem("the id is listed twice".to_owned());
                    }
                }
                _ => problem("an id is [a-z0-9_-], 1 to 64 characters".to_owned()),
            }
            match object.get("when").map(When::parse) {
                None | Some(Ok(When::Always | When::Target(_))) => {}
                Some(Ok(When::Unknown)) => {
                    problem("`when` uses a key the app doesn't know".to_owned());
                }
                Some(Err(e)) => problem(e.to_string()),
            }
            for key in object.keys() {
                if !RESERVED.contains(&key.as_str()) && !self.fields.contains_key(key) {
                    problem(format!("unknown field {key:?}"));
                }
            }
            for (name, field) in &self.fields {
                match object.get(name) {
                    None if field.optional => {}
                    None => problem(format!("{name}: missing")),
                    Some(value) => {
                        if let Err(why) = Self::read(&field.kind, value) {
                            problem(format!("{name}: {why}"));
                        }
                    }
                }
            }
        }
        problems
    }
}

impl crate::Registry {
    /// The records every list section of a file carries for this app
    /// ([`crate::list`]), and what was left out and why. Section-level
    /// problems (an unknown section, a newer schema) are reported by
    /// [`crate::Registry::resolve`], not again here.
    ///
    /// A section is in the result only when this app read it: its schema
    /// is one it knows and `items` is a list or left out (an empty list).
    /// A missing section, a newer or missing schema, or `items` that isn't
    /// a list is left out, so a reader can tell "this file lists nothing"
    /// from "this app couldn't read the list".
    #[must_use]
    pub fn resolve_lists(
        &self,
        sections: &BTreeMap<String, Value>,
        facts: &Facts,
    ) -> (Lists, Vec<SectionIssue>) {
        let mut lists = Lists::new();
        let mut issues = Vec::new();
        for (name, raw) in sections {
            let Some(spec) = self.section(name) else {
                continue;
            };
            let Some(list) = spec.listed() else {
                continue;
            };
            let (Some(reading), _) = spec.reading(raw) else {
                continue;
            };
            if reading != Reading::Full {
                continue;
            }
            let Some(items) = raw.get("items") else {
                lists.insert(name.clone(), Vec::new());
                continue;
            };
            let unreadable = !items.is_array();
            let (read, problems) = list.resolve(name, items, facts, reading);
            issues.extend(problems);
            if !unreadable {
                lists.insert(name.clone(), read);
            }
        }
        (lists, issues)
    }
}
