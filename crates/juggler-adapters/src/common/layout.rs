//! AJ-E2016: what differs between tools' config homes (their [`Layout`]),
//! for the scan, the inspection and the copy (AJ-E1302). Moved from
//! `claude_environment.rs`: Claude Code, Codex, Kimi Code and Grok Build
//! each supply one.

use std::path::Path;

/// What differs between Providers' config homes for the scan, the
/// inspection and the copy (AJ-E1302); the machinery in this module is
/// shared — Codex supplies its own [`Layout`].
pub(crate) struct Layout {
    /// The Provider id written into discovered installs and the marker.
    pub(crate) provider: &'static str,
    /// Entry names skipped as rebuildable state.
    pub(crate) skip_names: &'static [&'static str],
    /// File-name suffixes skipped as rebuildable state.
    pub(crate) skip_suffixes: &'static [&'static str],
    /// The skips apply to the home's own top-level entries only (a
    /// `log/` or `packages/` deeper down is user content, kept).
    pub(crate) top_level_skips_only: bool,
    /// How deep the scan's size estimate walks (Claude keeps its bulk
    /// two levels down; Codex's sessions sit at `sessions/Y/M/D/`).
    pub(crate) estimate_depth: usize,
    /// Entries never copied, with the plain reason listed in the skips
    /// (Codex's `auth.json`: see `codex_environment`), each by its path
    /// in the home: a top-level name, or `folder/name` for one deeper
    /// down (`ZCode`'s `v2/credentials.json`).
    pub(crate) kept_back: &'static [(&'static str, &'static str)],
    /// Top-level names that make a directory a real config home.
    pub(crate) config_evidence: &'static [&'static str],
    /// Top-level history entries the settings-only import leaves behind.
    pub(crate) history: &'static [&'static str],
    /// Files kept owner-only (0600) in the copy.
    pub(crate) private_files: &'static [&'static str],
    /// The plain words when a folder is not this Provider's config.
    pub(crate) not_a_config: &'static str,
    /// Signed in at the source, not in the copy.
    pub(crate) login_stays_note: &'static str,
    /// Signed in at both.
    pub(crate) login_moves_note: &'static str,
}

impl Layout {
    /// Whether `name` (at the top level of the home, or deeper) is
    /// rebuildable state, never copied.
    pub(crate) fn skips(&self, name: &str, top_level: bool) -> bool {
        if self.top_level_skips_only && !top_level {
            return false;
        }
        self.skip_names.contains(&name)
            || self
                .skip_suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix))
    }

    /// Why the entry at `relative` (its path in the home) stays with the
    /// original, when it does.
    pub(crate) fn kept_back(&self, relative: &Path) -> Option<&'static str> {
        self.kept_back
            .iter()
            .find(|(kept, _)| Path::new(kept) == relative)
            .map(|(_, why)| *why)
    }

    /// Whether `dir` carries evidence of a real config home.
    pub(crate) fn is_config_dir(&self, dir: &Path) -> bool {
        self.config_evidence
            .iter()
            .any(|name| dir.join(name).exists())
    }
}
