//! 0.5.2 G57, G62, G66: the check's clause and place rules, apart from
//! [`super::check`]'s own: a sentence is a whole clause (never opening on
//! "and", never ending on a word that leads on), and a quoted name is a
//! place only right after a place word ("in “web”"), never a record's name
//! said by where it is.

use super::check::{names_of, quote_segments, words};
use super::dto::{BotRowInfo, BotRowKind};

/// 0.5.2 G62: words a whole clause never opens with: it joins a clause
/// before it. English and German.
const OPENS_ON: &[&str] = &["and", "or", "but", "nor", "und", "oder", "aber", "sondern"];

/// 0.5.2 G62: words a whole clause never ends on: an article, a
/// preposition, a joining or leading word, a possessive. Words that do end
/// one are not among them: German's separable prefixes ("stellt … um",
/// "hört auf"), "out", "up", "off" and "over" ("timed out", "is over"),
/// a demonstrative ("matches that") or an object ("asked her").
const ENDS_ON: &[&str] = &[
    "a", "an", "the", "of", "to", "for", "with", "from", "by", "at", "in", "on", "into", "onto",
    "under", "about", "as", "than", "via", "and", "or", "but", "nor", "because", "while", "if",
    "whose", "its", "their", "your", "our", "is", "are", "was", "were", "der", "die", "den", "dem",
    "des", "ein", "eine", "einen", "einem", "einer", "und", "oder", "aber", "mit", "von", "für",
    "im", "bei", "nach", "über", "unter", "dass", "weil", "wenn", "seine", "ihre",
];

/// 0.5.2 G62: a part that is no whole clause, the word that shows it: one
/// opening on a joining word, or ending, outside a quoted name, on a word
/// that leads on. Fail closed: a line that may be a fragment isn't shown.
pub(super) fn fragment(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches(|c: char| {
        c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '\u{2026}')
    });
    let all = words(trimmed);
    if let Some(first) = all.first()
        && !trimmed.starts_with(['\u{201c}', '"', '\u{201e}', '\u{ab}'])
        && OPENS_ON.contains(&first.as_str())
    {
        return Some(first.clone());
    }
    // A sentence ending on a quoted name ends on that name.
    if trimmed.ends_with(['\u{201d}', '"', '\u{201c}', '\u{bb}']) {
        return None;
    }
    // 0.5.2 G66: a citation mark's word left behind (", records."), its
    // numbers gone: a stray mark, never a sentence's end.
    if let Some(last) = all.last()
        && matches!(last.as_str(), "record" | "records" | "eintrag" | "einträge")
        && trimmed[..trimmed.len() - last.len()]
            .trim_end()
            .ends_with([',', '(', '['])
    {
        return Some(last.clone());
    }
    all.last()
        .filter(|last| ENDS_ON.contains(&last.as_str()))
        .cloned()
}

/// 0.5.2 G57: the words a quoted name is a place after ("in “web”", "in
/// project “web”", "on branch “main”", "held by “Ana”"): where a record is,
/// never the record it names. English and German.
pub(super) const PLACE: &[&str] = &[
    "in", "im", "on", "auf", "at", "from", "von", "by", "project", "projekt", "pool", "branch",
    "zweig", "app", "account", "konto",
];

/// Whether the own text before a quote ends with a [`PLACE`] word.
fn is_place(before: &str) -> bool {
    words(before)
        .last()
        .is_some_and(|w| PLACE.contains(&w.as_str()))
}

/// 0.5.2 G57: which quoted names of a sentence (one flag per
/// [`quote_segments`] segment, false for the first) are places: after a
/// [`PLACE`] word.
pub(super) fn place_flags(segments: &[(Option<String>, String)]) -> Vec<bool> {
    let mut flags = Vec::with_capacity(segments.len());
    let mut before = "";
    for (name, after) in segments {
        // 0.5.2 G66: a name joined to a place by "and" ("in “docs” and
        // “Work · work”") is no second place: one record is in one place,
        // so it stands as a name, and a place's name as a name is refused.
        let place = name.is_some() && is_place(before);
        flags.push(place);
        before = after;
    }
    flags
}

/// 0.5.2 G57: a quoted name standing as a record's own name (not after a
/// [`PLACE`] word) that is no cited record's own name but a cited
/// session's or task's project, App, pool, holder or branch: the record
/// named by where it is ("“api” is finished with “Add retries…” in project
/// “api”", "“web” is the project"). A session's name is its one name
/// (`needs::session_name`); a project is said only as "in" it. A memory's
/// own words ("You called “api” …") are its names.
pub(super) fn named_by_place(text: &str, cited: &[&BotRowInfo]) -> Option<String> {
    let segments = quote_segments(text);
    let places = place_flags(&segments);
    for ((name, _), at_place) in segments.iter().zip(places) {
        if let Some(name) = name {
            let name = name.trim();
            let same = |n: &str| n.to_lowercase() == name.to_lowercase();
            let own = cited.iter().any(|row| {
                same(&row.title) || (row.kind == BotRowKind::Memory && names_of(row).any(same))
            });
            let by_where = cited.iter().any(|row| {
                matches!(row.kind, BotRowKind::Session | BotRowKind::Task)
                    && names_of(row).skip(1).any(same)
            });
            if by_where && !own && !at_place {
                return Some(name.to_lowercase());
            }
        }
    }
    None
}
