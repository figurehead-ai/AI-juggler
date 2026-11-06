//! AJ-E2013: the app's one log. Every `tracing` event at info level or
//! above, from every crate (the libraries use `tracing` only; the print
//! lints are on), is scrubbed as a crash report is — the home folder as
//! `~`, the user's name, keys and anything that looks like a secret —
//! then written as one line to `logs/app.log` in the data folder,
//! owner-only, rolling to `logs/app.1.log` past [`MAX_BYTES`]. A Dev build
//! also prints it to the terminal it runs in. Switch and launch steps
//! (`debug_log::TARGET`) stay in debug mode's own log.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The log, inside the data folder.
pub const RELATIVE: &str = "logs/app.log";
/// The older log.
pub const OLDER: &str = "logs/app.1.log";
/// The most one file holds.
pub const MAX_BYTES: u64 = 4 * 1024 * 1024;

static PATH: OnceLock<PathBuf> = OnceLock::new();
static WRITE: Mutex<()> = Mutex::new(());

/// Where the log goes (set once, at start).
pub fn set_folder(data_dir: &Path) {
    let _ = PATH.set(data_dir.join(RELATIVE));
}

/// An event's fields as `message name=value …`.
#[derive(Default)]
struct Fields(String);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, " {value:?}");
        } else {
            let _ = write!(self.0, " {}={value:?}", field.name());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            let _ = write!(self.0, " {value}");
        } else {
            let _ = write!(self.0, " {}={value:?}", field.name());
        }
    }
}

/// The log line for an event: when, level, where (the span it ran in),
/// what; scrubbed.
#[must_use]
pub fn line(at_ms: i64, level: &tracing::Level, scope: &str, text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let raw = format!("{at_ms} {level} {scope}{flat}");
    super::Scrub::here().text(&crate::error_log::without_keys(&raw))
}

/// What the app log keeps: info and above, not the switch and launch
/// steps (debug mode's own log has those).
#[must_use]
pub fn filter() -> tracing_subscriber::filter::FilterFn<impl Fn(&tracing::Metadata<'_>) -> bool> {
    tracing_subscriber::filter::filter_fn(|metadata| {
        metadata.target() != super::debug_log::TARGET && *metadata.level() <= tracing::Level::INFO
    })
}

/// The layer.
#[derive(Debug, Default)]
pub struct Layer;

impl<S> tracing_subscriber::Layer<S> for Layer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let scope = ctx
            .event_scope(event)
            .map(|spans| {
                spans
                    .from_root()
                    .map(|span| span.name())
                    .collect::<Vec<_>>()
                    .join(">")
            })
            .unwrap_or_default();
        let scope = if scope.is_empty() {
            event.metadata().target().to_owned()
        } else {
            scope
        };
        let text = line(
            juggler_core::model::Timestamp::now().millis(),
            event.metadata().level(),
            &scope,
            &fields.0,
        );
        if cfg!(debug_assertions) {
            #[allow(
                clippy::print_stderr,
                reason = "a Dev build shows its log in the terminal it runs in"
            )]
            {
                eprintln!("{text}");
            }
        }
        if let Some(path) = PATH.get() {
            let _ = append(path, &text);
        }
    }
}

fn append(path: &Path, text: &str) -> std::io::Result<()> {
    let _guard = WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        std::fs::rename(path, path.with_file_name("app.1.log"))?;
    }
    let mut file = juggler_platform::private_fs::append_private(path)?;
    writeln!(file, "{text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_never_keeps_a_secret_or_the_home_folder() {
        let home = dirs::home_dir()
            .map(|h| h.display().to_string())
            .unwrap_or_default();
        let text = format!(
            " couldn't read {home}/project/.env ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789 token ghp_abcdefghijklmnopqrstuvwxyz0123456789"
        );
        let line = line(1, &tracing::Level::WARN, "command>import", &text);
        assert!(!line.contains("sk-ant-api03-abcdefghij"), "{line}");
        assert!(!line.contains("ghp_abcdefghijklmnop"), "{line}");
        if home.len() > 1 {
            assert!(!line.contains(&home), "{line}");
        }
        assert!(line.contains("WARN command>import"), "{line}");
    }

    #[test]
    fn what_reaches_the_file_is_scrubbed_and_inside_its_command() {
        use tracing_subscriber::Layer as _;
        use tracing_subscriber::layer::SubscriberExt as _;
        let dir = tempfile::tempdir().unwrap();
        set_folder(dir.path());
        let Some(path) = PATH.get().cloned() else {
            return;
        };
        if !path.starts_with(dir.path()) {
            return; // another test in this process set the folder first
        }
        let subscriber = tracing_subscriber::registry()
            .with(super::super::debug_log::Layer.with_filter(super::super::debug_log::steps()))
            .with(Layer.with_filter(filter()));
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("command", what = "import the account");
            let _in = span.enter();
            tracing::warn!(
                "copy failed: ANTHROPIC_API_KEY=sk-ant-api03-zyxwvutsrqponmlkjihgfedcba9876543210"
            );
            tracing::debug!("never kept: debug");
        });
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("WARN command copy failed"), "{text}");
        assert!(!text.contains("sk-ant-api03-zyxwvutsrq"), "{text}");
        assert!(!text.contains("never kept"), "{text}");
    }

    #[test]
    fn the_log_rolls_over_and_stays_the_owners() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        append(&path, "first").unwrap();
        std::fs::write(&path, vec![b'x'; usize::try_from(MAX_BYTES).unwrap() + 1]).unwrap();
        append(&path, "after").unwrap();
        assert!(path.with_file_name("app.1.log").exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "after\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
                0
            );
        }
    }
}
