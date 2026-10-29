//! AJ-E1518: the Privacy page's links. The page never hands the app an
//! address to open: it names what it wants (a tool's terms, the source of
//! a test the page cites, the private security report) and this picks the
//! address, from the tool's adapter or the page's own claims list.

pub mod commands;

use std::sync::Arc;

use crate::state::AppState;

/// Where the repository's files and its private report live.
const REPOSITORY: &str = "https://github.com/figurehead-ai/AI-juggler";
/// The user guide's pages the app links to (docs/guide).
const GUIDE_PAGES: &[&str] = &["switching", "accounts", "first-run"];

/// The claims the page shows, as the app was built with them.
const CLAIMS: &str = include_str!("../../../../../ui/src/lib/trust-claims.json");

/// The files the page's claims cite, from the claims list.
fn cited_files() -> Vec<String> {
    let Ok(claims) = serde_json::from_str::<serde_json::Value>(CLAIMS) else {
        return Vec::new();
    };
    claims["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|group| group["claims"].as_array().into_iter().flatten())
        .flat_map(|claim| claim["tests"].as_array().into_iter().flatten())
        .filter_map(|test| {
            test.as_str()?
                .split_once("::")
                .map(|(path, _)| path.to_owned())
        })
        .collect()
}

/// The address for what the page asks to open, or `None` for anything
/// it doesn't know.
///
/// - `terms`, a tool's id: that tool's terms, from its adapter;
/// - `source`, a file a claim cites: the file in the repository;
/// - `report`, nothing: the repository's private security report.
#[must_use]
pub fn link_url(state: &Arc<AppState>, kind: &str, id: &str) -> Option<String> {
    match kind {
        "terms" => {
            let adapter = state
                .registry
                .get(&juggler_core::model::HarnessId::new(id))?;
            adapter.spec().terms.map(|terms| terms.url)
        }
        "source" => cited_files()
            .iter()
            .any(|file| file == id)
            .then(|| format!("{REPOSITORY}/blob/main/{id}")),
        "report" => Some(format!("{REPOSITORY}/security/advisories/new")),
        // A section of the user guide ("switching#keep-or-stop-the-
        // original"): a page the guide has, and a plain anchor.
        "guide" => {
            let (page, anchor) = id.split_once('#').unwrap_or((id, ""));
            let known = GUIDE_PAGES.contains(&page);
            let plain = anchor
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            (known && plain).then(|| {
                let section = if anchor.is_empty() {
                    String::new()
                } else {
                    format!("#{anchor}")
                };
                format!("{REPOSITORY}/blob/main/docs/guide/{page}.md{section}")
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(
                dir.path().join("app-data"),
                crate::state::default_registry(),
            )
            .unwrap(),
        );
        (dir, state)
    }

    #[test]
    fn a_guide_section_opens_only_for_a_known_page_and_a_plain_anchor() {
        let (_dir, state) = state();
        assert_eq!(
            link_url(&state, "guide", "switching#keep-or-stop-the-original").as_deref(),
            Some(
                "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/guide/switching.md#keep-or-stop-the-original"
            )
        );
        assert_eq!(
            link_url(&state, "guide", "../SECURITY#x"),
            None,
            "only the guide's own pages"
        );
        assert_eq!(
            link_url(&state, "guide", "switching#a?b=c"),
            None,
            "a plain anchor only"
        );
    }

    #[test]
    fn it_opens_only_what_it_knows() {
        let (_dir, state) = state();
        // A tool's terms come from its adapter.
        for id in state.registry.ids() {
            let spec = state.registry.get(&id).unwrap().spec();
            assert_eq!(
                link_url(&state, "terms", id.as_str()),
                spec.terms.map(|terms| terms.url)
            );
        }
        // A cited file opens in the repository; anything else doesn't open.
        let cited = cited_files();
        assert!(!cited.is_empty());
        assert_eq!(
            link_url(&state, "source", &cited[0]),
            Some(format!("{REPOSITORY}/blob/main/{}", cited[0]))
        );
        assert_eq!(link_url(&state, "source", "README.md"), None);
        assert_eq!(link_url(&state, "source", "https://example.com/x"), None);
        assert_eq!(link_url(&state, "terms", "no-such-tool"), None);
        assert_eq!(link_url(&state, "https", "example.com"), None);
        assert!(
            link_url(&state, "report", "")
                .unwrap()
                .starts_with(REPOSITORY)
        );
    }
}
