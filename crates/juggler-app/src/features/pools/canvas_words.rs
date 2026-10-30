//! The pool canvas's live stream in plain words (D-25): who did what to
//! which task or wrote to whom, by the names they had then.

use juggler_core::storage::{SENDER_USER, StreamEntry};

use super::dto::StreamLineInfo;

/// A name as the stream says it ("You" for the user).
fn who(actor_ref: &str, name: &str) -> String {
    if actor_ref == SENDER_USER {
        "You".to_owned()
    } else {
        name.to_owned()
    }
}

/// The recipients of a message: one or two by name, more counted.
fn whom(to: &[(String, String)]) -> String {
    let names: Vec<String> = to.iter().map(|(r, name)| who(r, name)).collect();
    match names.as_slice() {
        [] => "the pool".to_owned(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        [one, rest @ ..] => format!("{one} and {} others", rest.len()),
    }
}

/// What a task event says, by its verb.
fn task_words(actor: &str, verb: &str, title: &str, detail: Option<&str>) -> String {
    match (verb, detail) {
        ("created", _) => format!("{actor} wrote «{title}»"),
        ("claimed", _) => format!("{actor} took «{title}»"),
        ("assigned", Some(to)) => format!("{actor} gave «{title}» to {to}"),
        ("unassigned" | "released", _) => format!("{actor} let go of «{title}»"),
        ("status", Some(change)) => format!("{actor} moved «{title}»: {change}"),
        ("approved", _) => format!("{actor} approved «{title}»"),
        ("sent_back", _) => format!("{actor} sent «{title}» back"),
        ("freed", _) => format!("{actor} freed «{title}»"),
        ("cancelled", _) => format!("{actor} cancelled «{title}»"),
        ("noted", _) => format!("{actor} added a note to «{title}»"),
        _ => format!("{actor} changed «{title}»"),
    }
}

/// One stream line; `session_of` names the canvas's member a ref stands
/// for, when it's shown.
pub fn line(entry: &StreamEntry, session_of: impl Fn(&str) -> Option<String>) -> StreamLineInfo {
    let actor = who(&entry.actor_ref, &entry.actor_name);
    let words = if entry.source == "task" {
        task_words(&actor, &entry.verb, &entry.title, entry.detail.as_deref())
    } else if entry.verb == "question" {
        format!("{actor} asked {}: «{}»", whom(&entry.to), entry.title)
    } else {
        format!("{actor} wrote to {}: «{}»", whom(&entry.to), entry.title)
    };
    StreamLineInfo {
        at_ms: entry.at,
        words,
        source: entry.source.to_owned(),
        session_id: session_of(&entry.actor_ref),
        other_session_id: entry.to.first().and_then(|(r, _)| session_of(r)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        source: &'static str,
        verb: &str,
        to: &[(&str, &str)],
        detail: Option<&str>,
    ) -> StreamEntry {
        StreamEntry {
            at: 1,
            source,
            actor_ref: "session:a".to_owned(),
            actor_name: "Planner".to_owned(),
            to: to
                .iter()
                .map(|(r, n)| ((*r).to_owned(), (*n).to_owned()))
                .collect(),
            verb: verb.to_owned(),
            title: "Round tax".to_owned(),
            detail: detail.map(str::to_owned),
        }
    }

    #[test]
    fn says_who_did_what_in_plain_words() {
        let none = |_: &str| None;
        let gave = line(&entry("task", "assigned", &[], Some("Cart totals")), none);
        assert_eq!(gave.words, "Planner gave «Round tax» to Cart totals");
        let wrote = line(
            &entry("message", "note", &[("session:b", "Cart totals")], None),
            none,
        );
        assert_eq!(wrote.words, "Planner wrote to Cart totals: «Round tax»");
        let asked = line(
            &entry("message", "question", &[("user", "You")], None),
            none,
        );
        assert_eq!(asked.words, "Planner asked You: «Round tax»");
        let many = line(
            &entry(
                "message",
                "note",
                &[("session:b", "B"), ("session:c", "C"), ("session:d", "D")],
                None,
            ),
            none,
        );
        assert_eq!(many.words, "Planner wrote to B and 2 others: «Round tax»");
        let other = line(&entry("task", "linked", &[], None), none);
        assert_eq!(other.words, "Planner changed «Round tax»");
    }

    #[test]
    fn names_the_members_on_the_canvas() {
        let shown = |r: &str| (r != "user").then(|| r.trim_start_matches("session:").to_owned());
        let wrote = line(
            &entry("message", "note", &[("session:b", "B")], None),
            shown,
        );
        assert_eq!(wrote.session_id.as_deref(), Some("a"));
        assert_eq!(wrote.other_session_id.as_deref(), Some("b"));
    }
}
