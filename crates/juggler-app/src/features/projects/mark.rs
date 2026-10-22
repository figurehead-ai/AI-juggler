//! AJ-E1532: a project's mark, decided here and served as data (ADR-0019)
//! so every screen draws the same one. It is the project's own icon when
//! it has one ([`super::icon`]), else a monogram (one or two letters) on
//! one of the palette's twelve swatches. Clones of one repository share
//! their mark and carry a small number. The user may choose the letters,
//! an emoji or the colour from the project's menu; that choice is kept in
//! the app's settings (`projects.mark.<id>`) and wins.
//!
//! Swatches go to repositories in the order AI Juggler first saw them, so
//! a project keeps its colour while others come and go, and the first
//! twelve never share one. Letters are kept apart the same way: a later
//! project whose first choice is taken gets its next.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::model::ProjectId;
use serde::{Deserialize, Serialize};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// How many swatches the palette has (the UI's `PALETTE`, in this order).
pub const SWATCHES: u8 = 12;

/// The settings key a project's chosen mark is kept under.
#[must_use]
pub fn choice_key(project_id: &str) -> String {
    format!("projects.mark.{project_id}")
}

/// A project's mark, as every screen draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProjectMarkInfo {
    /// The monogram: one or two letters, the user's when they chose them.
    pub letters: String,
    /// The user's emoji, drawn instead of the letters.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// The palette swatch, `0..12`.
    #[ts(type = "number")]
    pub swatch: u8,
    /// The project's own icon, when it has one (a version: fetch it with
    /// `projects_icon_read`), even while letters or an emoji show.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// What the mark shows.
    pub shows: MarkShows,
    /// Which clone of its repository this is (1, 2, …), when there is
    /// more than one.
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone: Option<u32>,
    /// What the user chose for it, when they chose anything (the mark
    /// sheet starts from it).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice: Option<MarkChoice>,
}

/// What a mark shows: the project's icon (its own, else the user's
/// choice), or else its letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum MarkShows {
    /// The project's own icon.
    Icon,
    /// Its letters.
    Letters,
    /// The user's emoji.
    Emoji,
}

/// What the user chose for a project's mark. Every part is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MarkChoice {
    /// One or two letters or digits.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letters: Option<String>,
    /// One emoji.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// A palette swatch, `0..12`.
    #[ts(optional, type = "number")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch: Option<u8>,
}

impl MarkChoice {
    /// Nothing chosen.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.letters.is_none() && self.emoji.is_none() && self.swatch.is_none()
    }

    /// The choice, trimmed, or why it can't be kept.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`], in the user's words.
    pub fn checked(self) -> AppResult<Self> {
        let blank =
            |text: Option<String>| text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
        let letters = blank(self.letters);
        let emoji = blank(self.emoji);
        if letters.is_some() && emoji.is_some() {
            return Err(AppError::Refused(
                "Pick letters or an emoji, not both".into(),
            ));
        }
        if let Some(letters) = &letters
            && (letters.chars().count() > 2 || !letters.chars().all(char::is_alphanumeric))
        {
            return Err(AppError::Refused("Use one or two letters or digits".into()));
        }
        if let Some(emoji) = &emoji
            && !is_one_emoji(emoji)
        {
            return Err(AppError::Refused("Use one emoji".into()));
        }
        if self.swatch.is_some_and(|swatch| swatch >= SWATCHES) {
            return Err(AppError::Refused(
                "That colour isn't one of the twelve".into(),
            ));
        }
        Ok(Self {
            letters,
            emoji,
            swatch: self.swatch,
        })
    }
}

/// One emoji, roughly: short, and made only of symbols and the joiners
/// and selectors emoji are built from — never letters, digits, spaces or
/// control characters.
fn is_one_emoji(text: &str) -> bool {
    text.len() <= 32
        && text.chars().count() <= 10
        && text.chars().any(|c| u32::from(c) >= 0x2100)
        && text.chars().all(|c| {
            !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control()
                || matches!(c, '\u{200d}' | '\u{fe0f}' | '\u{20e3}')
        })
}

/// A project, as its mark is decided from.
#[derive(Debug, Clone)]
pub struct Subject<'a> {
    /// Its id.
    pub id: &'a str,
    /// Its name (the user's label, else its folder's).
    pub name: &'a str,
    /// Its repository (`github.com/owner/name`), when it is a checkout.
    pub repo: Option<&'a str>,
    /// When AI Juggler first saw it, epoch ms.
    pub created_ms: i64,
    /// Its own icon's version, when it has one.
    pub icon: Option<String>,
}

/// Every project's mark, by id. `choices` are the user's, by project id.
#[must_use]
pub fn marks<S: std::hash::BuildHasher>(
    subjects: &[Subject<'_>],
    choices: &HashMap<String, MarkChoice, S>,
) -> HashMap<String, ProjectMarkInfo> {
    // Repositories (or lone folders) in the order they were first seen.
    let mut groups: Vec<(String, Vec<&Subject<'_>>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut sorted: Vec<&Subject<'_>> = subjects.iter().collect();
    sorted.sort_by(|a, b| a.created_ms.cmp(&b.created_ms).then_with(|| a.id.cmp(b.id)));
    for subject in sorted {
        let key = subject
            .repo
            .map_or_else(|| format!("folder:{}", subject.id), str::to_owned);
        let slot = *index.entry(key.clone()).or_insert_with(|| {
            groups.push((key, Vec::new()));
            groups.len() - 1
        });
        groups[slot].1.push(subject);
    }
    let chosen = |members: &[&Subject<'_>]| {
        members
            .iter()
            .find_map(|m| choices.get(m.id))
            .cloned()
            .unwrap_or_default()
    };
    // The user's letters are taken first, so no project is given them.
    let mut taken: HashSet<String> = groups
        .iter()
        .filter_map(|(_, members)| chosen(members).letters.map(|l| l.to_lowercase()))
        .collect();
    let mut out = HashMap::new();
    for (order, (_, members)) in groups.iter().enumerate() {
        let choice = chosen(members);
        let letters = choice.letters.clone().unwrap_or_else(|| {
            // The first clone's own name: a repository's is lower-cased.
            let picked = monogram(members[0].name, &taken);
            taken.insert(picked.to_lowercase());
            picked
        });
        let icon = members.iter().find_map(|m| m.icon.clone());
        let shows = if choice.emoji.is_some() {
            MarkShows::Emoji
        } else if choice.letters.is_none() && icon.is_some() {
            MarkShows::Icon
        } else {
            MarkShows::Letters
        };
        let swatch = choice
            .swatch
            .unwrap_or_else(|| u8::try_from(order % usize::from(SWATCHES)).unwrap_or(0));
        for (n, member) in members.iter().enumerate() {
            let clone = (members.len() > 1).then(|| u32::try_from(n + 1).unwrap_or(u32::MAX));
            out.insert(
                member.id.to_owned(),
                ProjectMarkInfo {
                    letters: letters.clone(),
                    emoji: choice.emoji.clone(),
                    swatch,
                    icon: icon.clone(),
                    shows,
                    clone,
                    choice: (!choice.is_empty()).then(|| choice.clone()),
                },
            );
        }
    }
    out
}

/// A name's words: split at anything that isn't a letter or digit and
/// where a lower-case letter meets an upper-case one (`AIJuggler` →
/// "AI", "Juggler"). Words of digits only are left out.
fn words(name: &str) -> Vec<Vec<char>> {
    let mut out: Vec<Vec<char>> = Vec::new();
    let mut word: Vec<char> = Vec::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            out.push(std::mem::take(&mut word));
            continue;
        }
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1).copied();
        let camel = c.is_uppercase()
            && prev.is_some_and(|p| {
                p.is_lowercase() || (p.is_uppercase() && next.is_some_and(char::is_lowercase))
            });
        if camel {
            out.push(std::mem::take(&mut word));
        }
        word.push(c);
    }
    out.push(word);
    out.into_iter()
        .filter(|w| w.iter().any(|c| !c.is_ascii_digit()))
        .collect()
}

/// The monogram for `name` not in `taken` (lower-cased): a two-word
/// name's initials ("AW"), a one-word name's first two letters ("Pa"),
/// then the next pairs that start the same way, then a digit.
#[must_use]
pub fn monogram<S: std::hash::BuildHasher>(name: &str, taken: &HashSet<String, S>) -> String {
    let words = words(name);
    let Some(first) = words.first().and_then(|w| w.first()).copied() else {
        return "?".into();
    };
    let lead: String = first.to_uppercase().collect();
    let mut candidates: Vec<String> = Vec::new();
    // A name that ends in a number keeps it: projects told apart only by
    // it (`api-1` … `api-5`) read A1 … A5, not five unrelated pairs (QA,
    // the real setup).
    // Only a number that is its own word (`api-2`, not `web2`).
    let trimmed = name.trim_end_matches(|c: char| !c.is_alphanumeric());
    let digits = trimmed
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .count();
    let own_word = trimmed
        .chars()
        .rev()
        .nth(digits)
        .is_some_and(|c| !c.is_alphanumeric());
    let number: String = if own_word {
        trimmed[trimmed.len() - digits..].to_owned()
    } else {
        String::new()
    };
    match number.len() {
        1 => candidates.push(format!("{lead}{number}")),
        2 => candidates.push(number.clone()),
        _ => {}
    }
    if words.len() >= 2 {
        for word in &words[1..] {
            candidates.push(format!("{lead}{}", word[0].to_uppercase()));
        }
        for &c in words[1..].iter().flatten().chain(words[0].iter().skip(1)) {
            candidates.push(format!("{lead}{}", c.to_uppercase()));
        }
    } else {
        for &c in words[0].iter().skip(1) {
            candidates.push(format!("{lead}{}", c.to_lowercase()));
        }
    }
    candidates.extend((2..=9).map(|n| format!("{lead}{n}")));
    candidates
        .iter()
        .find(|c| !taken.contains(&c.to_lowercase()))
        .or_else(|| candidates.first())
        .cloned()
        .unwrap_or(lead)
}

/// The user's choices, by project id, from the app's settings.
///
/// # Errors
///
/// Storage errors.
pub fn choices(storage: &juggler_core::storage::Storage) -> AppResult<HashMap<String, MarkChoice>> {
    Ok(storage
        .app_settings()
        .list()?
        .into_iter()
        .filter_map(|(key, value)| {
            let id = key.strip_prefix("projects.mark.")?.to_owned();
            Some((id, serde_json::from_str::<MarkChoice>(&value).ok()?))
        })
        .collect())
}

/// Keeps `choice` for the project and its clones (they share a mark); an
/// empty one goes back to the mark it would have.
///
/// # Errors
///
/// [`AppError::Refused`] for a choice that can't be kept;
/// [`AppError::NotFound`]; storage errors.
pub fn mark_set_impl(state: &Arc<AppState>, project_id: &str, choice: MarkChoice) -> AppResult<()> {
    let choice = choice.checked()?;
    let projects = lock_storage_arc(state)?.projects().list()?;
    let Some(project) = projects.iter().find(|p| p.id == ProjectId::new(project_id)) else {
        return Err(AppError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        });
    };
    // Repositories are read from disk, never under the storage lock.
    let repo = crate::commands::repo_identity_cached(&project.root);
    let members: Vec<String> = match &repo {
        Some(repo) => projects
            .iter()
            .filter(|p| crate::commands::repo_identity_cached(&p.root).as_deref() == Some(repo))
            .map(|p| p.id.to_string())
            .collect(),
        None => vec![project_id.to_owned()],
    };
    let value = if choice.is_empty() {
        String::new()
    } else {
        serde_json::to_string(&choice).map_err(|e| AppError::Validation {
            subject: "mark",
            message: e.to_string(),
        })?
    };
    let storage = lock_storage_arc(state)?;
    for id in members {
        storage.app_settings().set(&choice_key(&id), &value)?;
    }
    Ok(())
}

/// Choose a project's mark (and its clones'); an empty choice resets it.
///
/// # Errors
///
/// As [`mark_set_impl`].
#[tauri::command]
pub async fn projects_mark_set(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    choice: MarkChoice,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "changing the project's mark", move |state| {
        mark_set_impl(state, &project_id, choice)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject<'a>(
        id: &'a str,
        name: &'a str,
        repo: Option<&'a str>,
        created_ms: i64,
    ) -> Subject<'a> {
        Subject {
            id,
            name,
            repo,
            created_ms,
            icon: None,
        }
    }

    #[test]
    fn a_name_gives_its_initials_or_its_first_two_letters() {
        let none = HashSet::new();
        assert_eq!(monogram("acme-web", &none), "AW");
        assert_eq!(monogram("payments_api", &none), "PA");
        assert_eq!(monogram("infra", &none), "In");
        assert_eq!(monogram("AIJuggler", &none), "AJ");
        assert_eq!(monogram("dataPipeline", &none), "DP");
        assert_eq!(monogram("acme-web2", &none), "AW");
        assert_eq!(monogram("2024", &none), "?");
        assert_eq!(monogram("日本語", &none), "日本");
    }

    #[test]
    fn a_name_that_ends_in_a_number_keeps_it() {
        let mut taken = HashSet::new();
        let picked: Vec<String> = ["acme-1", "acme-2", "acme-3", "acme-4", "acme-5"]
            .iter()
            .map(|name| {
                let letters = monogram(name, &taken);
                taken.insert(letters.to_lowercase());
                letters
            })
            .collect();
        assert_eq!(picked, ["A1", "A2", "A3", "A4", "A5"]);
        assert_eq!(monogram("build-12", &HashSet::new()), "12");
        assert_eq!(monogram("v2", &HashSet::new()), "V2");
        assert_eq!(
            monogram("acme-web2", &HashSet::new()),
            "AW",
            "a number in a word stays part of it"
        );
        assert_eq!(
            monogram("acme-web", &HashSet::new()),
            "AW",
            "no number: initials as before"
        );
    }

    #[test]
    fn a_taken_monogram_moves_to_the_next_that_starts_the_same_way() {
        let taken: HashSet<String> = ["aw".to_owned()].into();
        assert_eq!(monogram("acme-widgets", &taken), "AI");
        let taken: HashSet<String> = ["in".to_owned()].into();
        assert_eq!(monogram("infra", &taken), "If");
        let taken: HashSet<String> = ["xx".to_owned(), "x2".to_owned()].into();
        assert_eq!(monogram("x", &taken), "X3");
    }

    #[test]
    fn a_repositorys_letters_come_from_its_first_clones_name_not_its_lower_cased_remote() {
        let marks = marks(
            &[subject(
                "a",
                "AIJuggler",
                Some("github.com/figurehead-ai/aijuggler"),
                1,
            )],
            &HashMap::new(),
        );
        assert_eq!(marks["a"].letters, "AJ");
    }

    #[test]
    fn clones_share_a_mark_and_are_numbered_in_the_order_they_were_seen() {
        let repo = Some("github.com/acme/acme-web");
        let subjects = [
            subject("b", "acme-web2", repo, 20),
            subject("a", "acme-web", repo, 10),
            subject("c", "payments-api", None, 15),
        ];
        let marks = marks(&subjects, &HashMap::new());
        assert_eq!(marks["a"].letters, "AW");
        assert_eq!(marks["b"].letters, "AW");
        assert_eq!(marks["a"].swatch, marks["b"].swatch);
        assert_eq!((marks["a"].clone, marks["b"].clone), (Some(1), Some(2)));
        assert_eq!(marks["c"].clone, None);
        assert_ne!(marks["a"].swatch, marks["c"].swatch);
    }

    #[test]
    fn the_first_twelve_repositories_never_share_a_swatch_and_keep_theirs() {
        let names = [
            "acme-web",
            "payments-api",
            "marketing-site",
            "mobile-app",
            "infra",
            "docs-portal",
            "data-pipeline",
            "design-system",
            "cli-tools",
            "ml-experiments",
            "blog",
            "AIJuggler",
            "scratch",
            "codex-playground",
            "monitoring",
        ];
        let ids: Vec<String> = (0..15).map(|n| format!("id{n:02}")).collect();
        let subjects: Vec<Subject<'_>> = (0..15)
            .map(|n| subject(&ids[n], names[n], None, i64::try_from(n).unwrap()))
            .collect();
        let before = marks(&subjects[..13], &HashMap::new());
        let after = marks(&subjects, &HashMap::new());
        let first: HashSet<u8> = (0..12).map(|n| after[&ids[n]].swatch).collect();
        assert_eq!(first.len(), 12);
        for id in &ids[..13] {
            assert_eq!(
                before[id].swatch, after[id].swatch,
                "a newer project moved {id}'s colour"
            );
        }
        let letters: HashSet<&str> = after.values().map(|m| m.letters.as_str()).collect();
        assert_eq!(letters.len(), 15, "two projects got the same letters");
    }

    #[test]
    fn the_users_choice_wins_and_the_icon_gives_way_only_to_letters_or_an_emoji() {
        let mut a = subject("a", "acme-web", None, 1);
        a.icon = Some("v1".into());
        let mut b = subject("b", "acme-api", None, 2);
        b.icon = Some("v1".into());
        let choices = HashMap::from([
            (
                "a".to_owned(),
                MarkChoice {
                    swatch: Some(7),
                    ..MarkChoice::default()
                },
            ),
            (
                "b".to_owned(),
                MarkChoice {
                    emoji: Some("🚀".into()),
                    ..MarkChoice::default()
                },
            ),
        ]);
        let marks = marks(&[a, b], &choices);
        assert_eq!((marks["a"].swatch, marks["a"].shows), (7, MarkShows::Icon));
        assert_eq!(marks["a"].choice.as_ref().and_then(|c| c.swatch), Some(7));
        assert_eq!(
            (marks["b"].emoji.as_deref(), marks["b"].shows),
            (Some("🚀"), MarkShows::Emoji)
        );
        assert_eq!(
            marks["b"].icon.as_deref(),
            Some("v1"),
            "the sheet can offer its icon back"
        );
    }

    #[test]
    fn a_chosen_monogram_is_never_given_to_another_project() {
        let choices = HashMap::from([(
            "b".to_owned(),
            MarkChoice {
                letters: Some("AW".into()),
                ..MarkChoice::default()
            },
        )]);
        let marks = marks(
            &[
                subject("a", "acme-web", None, 1),
                subject("b", "zeta", None, 2),
            ],
            &choices,
        );
        assert_eq!(marks["b"].letters, "AW");
        assert_ne!(marks["a"].letters, "AW");
    }

    #[test]
    fn a_choice_is_checked_in_the_users_words() {
        let ok = MarkChoice {
            letters: Some(" Pa ".into()),
            swatch: Some(11),
            emoji: None,
        }
        .checked()
        .unwrap();
        assert_eq!(ok.letters.as_deref(), Some("Pa"));
        for (choice, words) in [
            (
                MarkChoice {
                    letters: Some("ABC".into()),
                    ..MarkChoice::default()
                },
                "one or two",
            ),
            (
                MarkChoice {
                    letters: Some("a!".into()),
                    ..MarkChoice::default()
                },
                "one or two",
            ),
            (
                MarkChoice {
                    emoji: Some("hi".into()),
                    ..MarkChoice::default()
                },
                "one emoji",
            ),
            (
                MarkChoice {
                    emoji: Some("🚀 🚀 🚀 🚀 🚀".into()),
                    ..MarkChoice::default()
                },
                "one emoji",
            ),
            (
                MarkChoice {
                    swatch: Some(12),
                    ..MarkChoice::default()
                },
                "twelve",
            ),
            (
                MarkChoice {
                    letters: Some("A".into()),
                    emoji: Some("🚀".into()),
                    swatch: None,
                },
                "not both",
            ),
        ] {
            let error = choice.checked().unwrap_err().to_string();
            assert!(error.to_lowercase().contains(words), "{error}");
        }
        for emoji in ["🚀", "👩‍💻", "❤️", "🇸🇪", "#️⃣"] {
            assert!(is_one_emoji(emoji), "{emoji}");
        }
        assert_eq!(
            MarkChoice {
                letters: Some("  ".into()),
                ..MarkChoice::default()
            }
            .checked()
            .unwrap(),
            MarkChoice::default()
        );
    }
}
