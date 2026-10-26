//! Where a window's reading came from, in words (the maintainer's third
//! report: a reading names its source and its age, "from the Claude app,
//! 55 min ago"), and whether a plan has a weekly window at all. The words
//! come from the tool's spec (its name, its app's name), never from a
//! branch on a tool.

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, UsageSource, WEEK_MS};

/// The source of the reading taken at `measured_at`, among `samples`.
#[must_use]
pub fn reading_source(
    samples: &[(QuotaSample, Option<UsageSource>)],
    measured_at: Option<i64>,
) -> Option<UsageSource> {
    let at = measured_at?;
    samples
        .iter()
        .filter(|(sample, _)| sample.ts == at)
        .find_map(|(_, source)| *source)
}

/// A source, in words, for `spec`'s tool ("the Claude app", "Claude
/// Code's last usage check"). `None` for a source that isn't a reading.
#[must_use]
pub fn source_words(spec: &juggler_adapter::ToolSpec, source: UsageSource) -> Option<String> {
    let tool = &spec.display_name;
    Some(match source {
        UsageSource::Statusline => format!("{tool}'s status line"),
        UsageSource::Rollout => format!("{tool}'s own log"),
        UsageSource::DesktopApp => spec
            .desktop_app
            .as_ref()
            .map_or_else(|| format!("{tool}'s app"), |app| format!("the {app} app")),
        UsageSource::UsageCache => format!("{tool}'s last usage check"),
        UsageSource::ApiRetry => format!("{tool}, when it hit the limit"),
        UsageSource::Transcript | UsageSource::Sqlite => return None,
    })
}

/// True when the plan has no weekly window, as far as its own records
/// say: a source that names every window reported the 5-hour one in the
/// last week, and nothing at all reported a week (a team seat's meter
/// shows only its 5-hour use).
#[must_use]
pub fn week_not_reported(
    five_hour: &[(QuotaSample, Option<UsageSource>)],
    weekly: &[(QuotaSample, Option<UsageSource>)],
    now_ms: i64,
) -> bool {
    let recent = |sample: &QuotaSample| {
        sample.confidence == Confidence::Measured && now_ms.saturating_sub(sample.ts) <= WEEK_MS
    };
    five_hour.iter().any(|(sample, source)| {
        recent(sample) && source.is_some_and(UsageSource::names_its_windows)
    }) && !weekly.iter().any(|(sample, _)| recent(sample))
}

/// Why a plan shows no all-models week, in words (D-06: an unknown says
/// why, never a guess), when its own records leave the week out:
/// - the tool reports a week on one model only: that, by name ("Claude
///   Code reports a weekly limit only for Fable");
/// - the newest reading that left the week out names the models' weeks
///   too: the tool reports no weekly limit;
/// - it came from a source that can't see a model's week (the app's
///   meter): no all-models week there, and when the rest shows.
#[must_use]
pub fn week_omitted_words(
    spec: &juggler_adapter::ToolSpec,
    models: &[&str],
    five_hour: &[(QuotaSample, Option<UsageSource>)],
    now_ms: i64,
) -> String {
    let tool = &spec.display_name;
    if !models.is_empty() {
        return format!(
            "{tool} reports a weekly limit only for {}",
            models.join(" and ")
        );
    }
    let newest = five_hour
        .iter()
        .filter(|(sample, source)| {
            sample.confidence == Confidence::Measured
                && now_ms.saturating_sub(sample.ts) <= WEEK_MS
                && source.is_some_and(UsageSource::names_its_windows)
        })
        .max_by_key(|(sample, _)| sample.ts)
        .and_then(|(_, source)| *source);
    match newest {
        Some(source) if !source.names_model_weeks() => {
            let from = source_words(spec, source).unwrap_or_else(|| format!("{tool}'s records"));
            format!(
                "No all-models weekly limit in {from}; a weekly limit on one model shows once {tool} checks its usage"
            )
        }
        _ => format!("{tool} reports no weekly limit for this account"),
    }
}

/// Whether a model's own week is current: read in the last week, and not
/// left out of a newer reading from the same source (a model the tool no
/// longer lists has no week now). `model` is its readings, `all` the
/// all-models windows' readings.
#[must_use]
pub fn model_week_current(
    model: &[(QuotaSample, Option<UsageSource>)],
    all: &[&[(QuotaSample, Option<UsageSource>)]],
    now_ms: i64,
) -> bool {
    let Some((last, source)) = model
        .iter()
        .filter(|(sample, _)| sample.confidence == Confidence::Measured)
        .max_by_key(|(sample, _)| sample.ts)
    else {
        return false;
    };
    let superseded = source.is_some_and(|source| {
        all.iter()
            .flat_map(|rows| rows.iter())
            .any(|(sample, from)| *from == Some(source) && sample.ts > last.ts)
    });
    now_ms.saturating_sub(last.ts) <= WEEK_MS && !superseded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: i64) -> QuotaSample {
        QuotaSample {
            ts,
            used_pct: 40.0,
            resets_at: None,
            confidence: Confidence::Measured,
        }
    }

    fn spec(app: Option<&str>) -> juggler_adapter::ToolSpec {
        let mut spec = juggler_adapter::ToolSpec::generic("tool-x");
        spec.display_name = "Tool X".into();
        spec.desktop_app = app.map(str::to_owned);
        spec
    }

    #[test]
    fn a_source_reads_in_the_tools_own_words() {
        let with_app = spec(Some("Tool"));
        assert_eq!(
            source_words(&with_app, UsageSource::DesktopApp).as_deref(),
            Some("the Tool app")
        );
        assert_eq!(
            source_words(&spec(None), UsageSource::DesktopApp).as_deref(),
            Some("Tool X's app")
        );
        assert_eq!(
            source_words(&with_app, UsageSource::UsageCache).as_deref(),
            Some("Tool X's last usage check")
        );
        assert_eq!(source_words(&with_app, UsageSource::Transcript), None);
    }

    #[test]
    fn the_reading_names_the_source_of_its_own_sample() {
        let rows = [
            (sample(10), Some(UsageSource::ApiRetry)),
            (sample(20), Some(UsageSource::DesktopApp)),
            (sample(30), None),
        ];
        assert_eq!(
            reading_source(&rows, Some(20)),
            Some(UsageSource::DesktopApp)
        );
        assert_eq!(
            reading_source(&rows, Some(30)),
            None,
            "stored before sources were kept"
        );
        assert_eq!(reading_source(&rows, None), None);
    }

    #[test]
    fn no_week_only_when_a_whole_reading_left_it_out() {
        let now = 1_792_000_000_000;
        let meter = [(sample(now - 60_000), Some(UsageSource::DesktopApp))];
        assert!(week_not_reported(&meter, &[], now));
        // A limit hit names only the window it hit.
        let hit = [(sample(now - 60_000), Some(UsageSource::ApiRetry))];
        assert!(!week_not_reported(&hit, &[], now));
        // A week reported in the last week: it's there.
        let week = [(sample(now - 3_600_000), Some(UsageSource::UsageCache))];
        assert!(!week_not_reported(&meter, &week, now));
        // Nothing recent: nothing is said.
        let old = [(sample(now - 8 * 86_400_000), Some(UsageSource::DesktopApp))];
        assert!(!week_not_reported(&old, &[], now));
    }

    /// The maintainer's report (0.1.6): "Claude Code reports no weekly
    /// limit" for a team seat whose week is on one model. Never said while
    /// a model's week is there, nor from a source that can't see one.
    #[test]
    fn a_missing_week_says_what_is_known_never_no_limit_by_default() {
        let now = 1_792_000_000_000;
        let spec = spec(Some("Tool"));
        let cache = [(sample(now - 60_000), Some(UsageSource::UsageCache))];
        assert_eq!(
            week_omitted_words(&spec, &["Fable"], &cache, now),
            "Tool X reports a weekly limit only for Fable"
        );
        assert_eq!(
            week_omitted_words(&spec, &["Fable", "Opus"], &cache, now),
            "Tool X reports a weekly limit only for Fable and Opus"
        );
        // A reading that names the models' weeks too, and lists none.
        assert_eq!(
            week_omitted_words(&spec, &[], &cache, now),
            "Tool X reports no weekly limit for this account"
        );
        // Only the app's meter: it can't see a model's week.
        let meter = [
            (sample(now - 3_600_000), Some(UsageSource::UsageCache)),
            (sample(now - 60_000), Some(UsageSource::DesktopApp)),
        ];
        assert_eq!(
            week_omitted_words(&spec, &[], &meter, now),
            "No all-models weekly limit in the Tool app; a weekly limit on one model shows once Tool X checks its usage"
        );
    }

    #[test]
    fn a_models_week_is_current_until_a_newer_reading_leaves_it_out() {
        let now = 1_792_000_000_000;
        let week = [(sample(now - 3_600_000), Some(UsageSource::UsageCache))];
        let same: [(QuotaSample, Option<UsageSource>); 1] =
            [(sample(now - 3_600_000), Some(UsageSource::UsageCache))];
        assert!(model_week_current(&week, &[&same], now));
        // The app's meter read since: it can't see the week, so it stays.
        let meter = [(sample(now - 60_000), Some(UsageSource::DesktopApp))];
        assert!(model_week_current(&week, &[&same, &meter], now));
        // A newer usage check without it: gone.
        let newer = [(sample(now - 60_000), Some(UsageSource::UsageCache))];
        assert!(!model_week_current(&week, &[&newer], now));
        // Older than a week: gone.
        let old = [(sample(now - 8 * 86_400_000), Some(UsageSource::UsageCache))];
        assert!(!model_week_current(&old, &[], now));
        assert!(!model_week_current(&[], &[], now));
    }
}
