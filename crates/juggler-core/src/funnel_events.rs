//! AJ-E309 Req.6/7: the LOCAL onboarding-funnel event store.
//!
//! Events are written to a JSONL file in the app-data directory and
//! never leave this Mac (there is no network path in this epic —
//! telemetry stays opt-in only). The store is capped at 10 MB in
//! total (two 5 MB segments, the oldest dropped on rotation).
//!
//! The schema is an ALLOW-LIST, not a deny-list: every event kind
//! names the exact field keys it may carry, and every key has a value
//! shape (a count, a fixed vocabulary word, or a lowercase code).
//! Free text cannot pass — no `/`, `@`, spaces, or capitals — so a
//! path, label, email, or credential can never be smuggled in through
//! a permitted key. [`FORBIDDEN_KEYS`] is kept as an explicit second
//! assertion (pinned by test): none of them is ever in the schema.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Keys that must NEVER appear in event fields (pinned by test).
pub const FORBIDDEN_KEYS: &[&str] = &[
    "path", "label", "token", "dir", "source", "account", "email", "key", "name", "secret",
];

/// One segment's cap; two segments bound the store at 10 MB total.
const SEGMENT_BYTES: u64 = 5 * 1024 * 1024;

/// Serialises appends + rotation inside this process (concurrent
/// async commands never interleave a rotation with a write).
static APPEND_LOCK: Mutex<()> = Mutex::new(());

/// The value shape a field key accepts.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// A non-negative integer, at most 10 digits.
    Count,
    /// One word from a fixed vocabulary.
    OneOf(&'static [&'static str]),
    /// A lowercase machine code (`[a-z][a-z0-9_-]{0,47}`).
    Code,
}

/// Field keys and their shapes (the whole vocabulary).
const KEY_SHAPES: &[(&str, Shape)] = &[
    ("providers", Shape::Count),
    ("found", Shape::Count),
    ("items", Shape::Count),
    ("count", Shape::Count),
    ("seconds_since_install", Shape::Count),
    (
        "health",
        Shape::OneOf(&["ok", "partial", "denied", "cancelled"]),
    ),
    (
        "field",
        Shape::OneOf(&["purpose", "plan", "login", "activity"]),
    ),
    (
        "surface",
        Shape::OneOf(&["wizard", "launch", "new-project", "accounts"]),
    ),
    (
        "choice",
        Shape::OneOf(&["strip", "keep", "ask", "decide_later"]),
    ),
    ("degraded", Shape::OneOf(&["true", "false"])),
    (
        "flow",
        Shape::OneOf(&["a-first-install", "b-new-project", "c-add-provider"]),
    ),
    ("step", Shape::Code),
    ("reason_code", Shape::Code),
];

/// Every event kind and the field keys it may carry (Req.6 + the
/// Req.7 inference pair + the policy choice + the flow-step mirror).
const KINDS: &[(&str, &[&str])] = &[
    ("scan_started", &["providers"]),
    ("scan_completed", &["providers", "found", "health"]),
    ("proposal_shown", &["items"]),
    ("proposal_accepted", &["items"]),
    ("proposal_customised", &[]),
    ("import_started", &[]),
    ("import_completed", &["degraded"]),
    ("import_failed", &["reason_code"]),
    ("login_queued", &["count"]),
    ("login_completed", &[]),
    ("login_skipped", &[]),
    ("login_expired", &[]),
    ("expert_opened", &["surface"]),
    ("first_launch", &["seconds_since_install"]),
    ("inference_shown", &["field"]),
    ("inference_edit", &["field"]),
    ("policy_chosen", &["choice"]),
    ("flow_step", &["flow", "step"]),
];

/// One funnel event — a kind, a timestamp, and schema-checked fields.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct FunnelEvent {
    /// `scan_started`, `proposal_accepted`, `login_skipped`, …
    pub kind: String,
    /// Emission time, epoch ms.
    pub at_ms: i64,
    /// Aggregate-safe fields only (counts, vocabulary words, codes).
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

/// The allowed field keys for `kind`, or `None` for an unknown kind.
#[must_use]
pub fn allowed_keys(kind: &str) -> Option<&'static [&'static str]> {
    KINDS
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, keys)| *keys)
}

/// Every key the schema can ever accept (for the guardrail test).
#[must_use]
pub fn schema_keys() -> Vec<&'static str> {
    KEY_SHAPES.iter().map(|(key, _)| *key).collect()
}

fn value_fits(shape: Shape, value: &str) -> bool {
    match shape {
        Shape::Count => {
            !value.is_empty() && value.len() <= 10 && value.bytes().all(|b| b.is_ascii_digit())
        }
        Shape::OneOf(words) => words.contains(&value),
        Shape::Code => {
            let mut bytes = value.bytes();
            matches!(bytes.next(), Some(b'a'..=b'z'))
                && value.len() <= 48
                && bytes.all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
        }
    }
}

fn reject(message: String) -> crate::CoreError {
    crate::CoreError::Validation {
        subject: "funnel event",
        message,
    }
}

/// Validates one event against the allow-list schema.
///
/// # Errors
///
/// Returns [`crate::CoreError::Validation`] naming the unknown kind,
/// the forbidden or unexpected key, or the malformed value.
pub fn validate(event: &FunnelEvent) -> Result<(), crate::CoreError> {
    let Some(allowed) = allowed_keys(&event.kind) else {
        return Err(reject(format!("unknown kind '{}'", event.kind)));
    };
    for (key, value) in &event.fields {
        if FORBIDDEN_KEYS.contains(&key.as_str()) {
            return Err(reject(format!(
                "field '{key}' is forbidden (no paths, labels, or credentials)"
            )));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(reject(format!(
                "field '{key}' is not part of '{}'",
                event.kind
            )));
        }
        let shape = KEY_SHAPES
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, shape)| *shape)
            .ok_or_else(|| reject(format!("field '{key}' has no shape")))?;
        if !value_fits(shape, value) {
            return Err(reject(format!("field '{key}' has a malformed value")));
        }
    }
    Ok(())
}

/// The local event log (JSONL, two 5 MB segments).
pub struct FunnelLog {
    path: PathBuf,
}

impl FunnelLog {
    /// Opens (or creates on first append) the store under `app_data`.
    #[must_use]
    pub fn open(app_data: &Path) -> Self {
        FunnelLog {
            path: app_data.join("funnel-events.jsonl"),
        }
    }

    fn rotated(&self) -> PathBuf {
        self.path.with_extension("jsonl.1")
    }

    /// Appends one validated event; rotates first when the current
    /// segment reached its cap (the previous `.1` segment is dropped,
    /// so the store never exceeds 10 MB).
    ///
    /// # Errors
    ///
    /// Returns [`crate::CoreError`] on validation or I/O failure.
    pub fn append(&self, event: &FunnelEvent) -> Result<(), crate::CoreError> {
        validate(event)?;
        let line = serde_json::to_string(event).map_err(|e| crate::CoreError::Io(e.to_string()))?;
        let _guard = APPEND_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(meta) = std::fs::metadata(&self.path)
            && meta.len() >= SEGMENT_BYTES
        {
            std::fs::rename(&self.path, self.rotated())
                .map_err(|e| crate::CoreError::Io(e.to_string()))?;
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::CoreError::Io(e.to_string()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| crate::CoreError::Io(e.to_string()))?;
        writeln!(file, "{line}").map_err(|e| crate::CoreError::Io(e.to_string()))?;
        Ok(())
    }

    /// Reads every stored event, oldest first (the rotated segment,
    /// then the current one). Corrupt or schema-failing lines are
    /// skipped, never fatal.
    ///
    /// # Errors
    ///
    /// Infallible today (missing files read as empty); the `Result`
    /// keeps the seam honest for future storage backends.
    pub fn read(&self) -> Result<Vec<FunnelEvent>, crate::CoreError> {
        let mut events = Vec::new();
        for path in [self.rotated(), self.path.clone()] {
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            events.extend(
                raw.lines()
                    .filter_map(|line| serde_json::from_str::<FunnelEvent>(line).ok())
                    .filter(|event| validate(event).is_ok()),
            );
        }
        Ok(events)
    }
}

/// One field's inference-override rate (Req.7).
#[derive(Clone, PartialEq, Debug)]
pub struct OverrideRate {
    /// The inferred field (`purpose`, …).
    pub field: String,
    /// How many times an inferred value was shown.
    pub shown: u64,
    /// How many of those the user overrode.
    pub edited: u64,
    /// `edited / shown` (0 when nothing was shown).
    pub rate: f64,
    /// True when the rate exceeds 30% — the rule needs retuning.
    pub flagged: bool,
}

/// Retune threshold: a rule overridden more than this is flagged.
pub const RETUNE_THRESHOLD: f64 = 0.30;

/// Req.7: the override rate per inferred field. `inference_shown`
/// counts every displayed inference; `inference_edit` counts the ones
/// the user changed before committing.
#[must_use]
pub fn override_rates(events: &[FunnelEvent]) -> Vec<OverrideRate> {
    let mut counts: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for event in events {
        let Some(field) = event.fields.get("field") else {
            continue;
        };
        let entry = counts.entry(field.clone()).or_insert((0, 0));
        match event.kind.as_str() {
            "inference_shown" => entry.0 += 1,
            "inference_edit" => entry.1 += 1,
            _ => {}
        }
    }
    counts
        .into_iter()
        .map(|(field, (shown, edited))| {
            // An edit always follows a show; clamp so a lost show
            // event can never report an impossible >100% rate.
            let shown = shown.max(edited);
            #[allow(clippy::cast_precision_loss)]
            let rate = if shown == 0 {
                0.0
            } else {
                edited as f64 / shown as f64
            };
            OverrideRate {
                field,
                shown,
                edited,
                rate,
                flagged: rate > RETUNE_THRESHOLD,
            }
        })
        .collect()
}

/// Req.7: the LOCAL funnel report (markdown) — counts per kind, the
/// flow-step sequence counts, the policy choices, and the override
/// rates. Deterministic for a given event list (no clock reads), so a
/// stored dry-run can be regression-checked.
#[must_use]
pub fn report(events: &[FunnelEvent]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# Onboarding funnel — local export\n\n\
         Stored only on this Mac (no network call). Fields are counts,\n\
         fixed vocabulary words, or lowercase codes — never paths,\n\
         labels, or credentials.\n\n",
    );
    let _ = write!(out, "Events: {}\n\n", events.len());

    let mut by_kind: BTreeMap<&str, u64> = BTreeMap::new();
    for event in events {
        *by_kind.entry(event.kind.as_str()).or_insert(0) += 1;
    }
    out.push_str("## Events by kind\n\n| kind | count |\n|---|---|\n");
    for (kind, count) in &by_kind {
        let _ = writeln!(out, "| {kind} | {count} |");
    }

    // First-seen order, so the table reads as the funnel's shape
    // (gate → scan → confirm …), not alphabetically.
    let mut steps: Vec<(String, String, u64)> = Vec::new();
    for event in events.iter().filter(|event| event.kind == "flow_step") {
        let flow = event.fields.get("flow").cloned().unwrap_or_default();
        let step = event.fields.get("step").cloned().unwrap_or_default();
        match steps
            .iter_mut()
            .find(|(seen_flow, seen_step, _)| *seen_flow == flow && *seen_step == step)
        {
            Some(entry) => entry.2 += 1,
            None => steps.push((flow, step, 1)),
        }
    }
    out.push_str(
        "\n## Flow steps reached (first-seen order)\n\n| flow | step | times |\n|---|---|---|\n",
    );
    if steps.is_empty() {
        out.push_str("| — | — | 0 |\n");
    }
    for (flow, step, count) in &steps {
        let _ = writeln!(out, "| {flow} | {step} | {count} |");
    }

    let mut choices: BTreeMap<&str, u64> = BTreeMap::new();
    for event in events.iter().filter(|event| event.kind == "policy_chosen") {
        if let Some(choice) = event.fields.get("choice") {
            *choices.entry(choice.as_str()).or_insert(0) += 1;
        }
    }
    out.push_str("\n## Pack policy choices (first run)\n\n");
    if choices.is_empty() {
        out.push_str("- none recorded\n");
    }
    for (choice, count) in &choices {
        let _ = writeln!(out, "- {choice}: {count}");
    }

    let launches: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == "first_launch")
        .filter_map(|event| {
            event
                .fields
                .get("seconds_since_install")
                .map(String::as_str)
        })
        .collect();
    out.push_str("\n## Time to first Instance\n\n");
    match launches.first() {
        Some(seconds) => {
            let _ = writeln!(out, "- {seconds} s after install");
        }
        None => out.push_str("- no Instance launched yet\n"),
    }

    let _ = write!(
        out,
        "\n## Inference override rates (> {:.0}% flagged for retuning)\n\n",
        RETUNE_THRESHOLD * 100.0
    );
    let rates = override_rates(events);
    if rates.is_empty() {
        out.push_str("- no inferences shown yet\n");
    }
    for rate in rates {
        let _ = writeln!(
            out,
            "- {}: {}/{} overridden ({:.0}%){}",
            rate.field,
            rate.edited,
            rate.shown,
            rate.rate * 100.0,
            if rate.flagged {
                " — FLAGGED for retuning"
            } else {
                ""
            }
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> (tempfile::TempDir, FunnelLog) {
        let dir = tempfile::tempdir().unwrap();
        let store = FunnelLog::open(dir.path());
        (dir, store)
    }

    fn event(kind: &str, fields: &[(&str, &str)]) -> FunnelEvent {
        FunnelEvent {
            kind: kind.to_owned(),
            at_ms: 0,
            fields: fields
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn forbidden_keys_are_never_in_the_schema_and_always_rejected() {
        for key in FORBIDDEN_KEYS {
            assert!(
                !schema_keys().contains(key),
                "{key} must not be a schema key"
            );
            for (kind, _) in KINDS {
                let err = validate(&event(kind, &[(key, "x")])).unwrap_err();
                assert!(err.to_string().contains("forbidden"), "{kind}/{key}");
            }
        }
    }

    #[test]
    fn values_cannot_carry_paths_labels_or_emails() {
        for smuggled in [
            "/Users/you/.claude-work",
            "Work Max",
            "you@example.com",
            "sk-ant-ABC",
            "../etc",
            "",
        ] {
            assert!(
                validate(&event("import_failed", &[("reason_code", smuggled)])).is_err(),
                "{smuggled:?} must be rejected"
            );
            assert!(
                validate(&event(
                    "flow_step",
                    &[("flow", "a-first-install"), ("step", smuggled)]
                ))
                .is_err()
            );
        }
        assert!(validate(&event("scan_completed", &[("found", "12a")])).is_err());
        assert!(validate(&event("policy_chosen", &[("choice", "yolo")])).is_err());
        assert!(
            validate(&event("nope", &[])).is_err(),
            "unknown kinds are refused"
        );
        assert!(
            validate(&event("login_skipped", &[("count", "1")])).is_err(),
            "keys outside the kind's list are refused"
        );
        assert!(
            validate(&event(
                "scan_completed",
                &[("providers", "3"), ("found", "2"), ("health", "partial")]
            ))
            .is_ok()
        );
        assert!(
            validate(&event(
                "import_failed",
                &[("reason_code", "already_imported")]
            ))
            .is_ok()
        );
    }

    #[test]
    fn append_read_roundtrip_and_corrupt_skip() {
        let (dir, store) = log();
        store
            .append(&event("scan_started", &[("providers", "1")]))
            .unwrap();
        store.append(&event("login_skipped", &[])).unwrap();
        assert_eq!(store.read().unwrap().len(), 2);
        assert!(store.append(&event("x", &[])).is_err());
        assert_eq!(
            store.read().unwrap().len(),
            2,
            "a refused event writes nothing"
        );
        // A corrupt line is skipped, never fatal; a hand-edited line
        // that fails the schema is skipped too.
        std::fs::write(
            dir.path().join("funnel-events.jsonl"),
            "not json\n{\"kind\":\"scan_started\",\"at_ms\":1,\"fields\":{\"path\":\"/x\"}}\n",
        )
        .unwrap();
        assert_eq!(store.read().unwrap().len(), 0);
    }

    #[test]
    fn rotation_keeps_the_store_under_ten_megabytes() {
        let (dir, store) = log();
        let current = dir.path().join("funnel-events.jsonl");
        let rotated = dir.path().join("funnel-events.jsonl.1");
        std::fs::write(&rotated, vec![b'\n'; 4]).unwrap();
        // A full current segment rotates on the next append, dropping
        // the old `.1` (never a third segment).
        let padding = vec![b'\n'; usize::try_from(SEGMENT_BYTES).unwrap()];
        std::fs::write(&current, &padding).unwrap();
        store.append(&event("login_completed", &[])).unwrap();
        assert_eq!(std::fs::metadata(&rotated).unwrap().len(), SEGMENT_BYTES);
        assert!(std::fs::metadata(&current).unwrap().len() < 200);
        let total =
            std::fs::metadata(&rotated).unwrap().len() + std::fs::metadata(&current).unwrap().len();
        assert!(total <= 2 * SEGMENT_BYTES);
        assert_eq!(store.read().unwrap().len(), 1);
    }

    #[test]
    fn override_rate_flags_over_30_percent() {
        // 10 inferences shown, 4 overridden = 40% → flagged; a field
        // at 1 of 10 stays under the line.
        let mut events: Vec<FunnelEvent> = (0..10)
            .map(|_| event("inference_shown", &[("field", "purpose")]))
            .chain((0..4).map(|_| event("inference_edit", &[("field", "purpose")])))
            .collect();
        events.extend((0..10).map(|_| event("inference_shown", &[("field", "activity")])));
        events.push(event("inference_edit", &[("field", "activity")]));
        let rates = override_rates(&events);
        assert_eq!(rates.len(), 2);
        let purpose = rates.iter().find(|r| r.field == "purpose").unwrap();
        assert!((purpose.rate - 0.4).abs() < 1e-9);
        assert!(purpose.flagged, "40% > 30% flags for retuning");
        let activity = rates.iter().find(|r| r.field == "activity").unwrap();
        assert!(!activity.flagged);
        // A lost show event never yields > 100%.
        let orphan = override_rates(&[event("inference_edit", &[("field", "plan")])]);
        assert!((orphan[0].rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_report_is_deterministic_and_carries_no_free_text() {
        let events = vec![
            event(
                "flow_step",
                &[("flow", "a-first-install"), ("step", "gate")],
            ),
            event("policy_chosen", &[("choice", "strip")]),
            event("first_launch", &[("seconds_since_install", "212")]),
        ];
        let first = report(&events);
        assert_eq!(first, report(&events));
        assert!(first.contains("| a-first-install | gate | 1 |"));
        assert!(first.contains("- strip: 1"));
        assert!(first.contains("- 212 s after install"));
        assert!(!first.contains("/Users"));
    }
}
