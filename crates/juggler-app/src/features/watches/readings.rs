//! AJ-E3830: code's readings of a watch request's words (the cue table
//! and its closed paraphrase map, numbers with their units, lifetimes in
//! words, German compounds), shared by the draft and its cover.

use super::{Ask, holds, request_numbers, request_times, words};
use crate::features::watches::model::LifetimeChoice;

const FAIL_STEMS: &[&str] = &[
    "fail",
    "error",
    "crash",
    "fehl",
    "absturz",
    "abgestürzt",
    "abstürz",
    "scheiter",
    "schief",
];
const STUCK_WORDS: &[&str] = &[
    "stuck",
    "hangs",
    "hanging",
    "stalls",
    "stalled",
    "frozen",
    "freezes",
    "hängt",
    "festhängt",
    "hängen",
    "hängenbleibt",
];

/// The conditions the request's cue words name, in its order (harness
/// v7's `CUE_CONDITIONS`), read with names and exclusions blanked.
#[must_use]
pub fn cue_conditions(text: &str) -> Vec<String> {
    let w = words(text);
    let at = |pred: &dyn Fn(usize) -> bool| (0..w.len()).find(|&i| pred(i));
    let is = |i: usize, set: &[&str]| set.contains(&w[i].as_str());
    let pre = |i: usize, set: &[&str]| set.iter().any(|p| w[i].starts_with(p));
    let next = |i: usize, set: &[&str]| w.get(i + 1).is_some_and(|n| set.contains(&n.as_str()));
    let mut hits: Vec<(usize, &str)> = Vec::new();
    let table: [(&str, &dyn Fn(usize) -> bool); 11] = [
        ("fails", &|i| {
            pre(i, FAIL_STEMS)
                || is(i, &["breaks", "broke", "broken", "kaputt"])
                || (is(i, &["wrong"]) && i > 0 && is(i - 1, &["goes", "go", "went"]))
        }),
        ("finishes", &|i| {
            pre(
                i,
                &["finish", "complet", "fertig", "erledigt", "abgeschlossen"],
            ) || is(i, &["done", "ready"])
                || (is(i, &["wraps", "wrapped", "wrap"]) && next(i, &["up"]))
        }),
        ("stuck", &|i| {
            is(i, STUCK_WORDS)
                || (is(i, &["stops", "stopped"]) && next(i, &["moving", "making", "progressing"]))
                || (is(i, &["no"]) && next(i, &["progress"]))
        }),
        ("idle_for", &|i| {
            is(i, &["idle", "inactive", "untätig", "inaktiv"])
                || (is(i, &["no"]) && next(i, &["activity"]))
        }),
        ("asks_you", &|i| {
            pre(
                i,
                &[
                    "ask",
                    "question",
                    "approv",
                    "frag",
                    "rückfrag",
                    "freigabe",
                    "input",
                ],
            ) && !(i > 0 && is(i - 1, &["i", "you"]))
                || (is(i, &["waiting", "waits"])
                    && next(i, &["on", "for"])
                    && w.get(i + 2)
                        .is_some_and(|n| ["me", "my", "you", "your"].contains(&n.as_str())))
        }),
        ("hits_limit", &|i| {
            pre(i, &["limit", "rate-limit", "throttl", "nutzungslimit"])
        }),
        ("runway_below", &|i| {
            (is(i, &["below", "under", "unter"])
                && w.get(i + 1)
                    .is_some_and(|n| n.chars().next().is_some_and(|c| c.is_ascii_digit())))
                || (is(i, &["runs", "running"]) && next(i, &["low"]))
                || (is(i, &["less", "weniger"])
                    && next(i, &["than", "als"])
                    && w.get(i + 2)
                        .is_some_and(|n| n.chars().next().is_some_and(|c| c.is_ascii_digit())))
        }),
        ("runway_resets", &|i| {
            pre(i, &["reset", "zurückgesetzt", "spielraum"])
                || (is(i, &["room"]) && next(i, &["again"]))
        }),
        ("ends", &|i| {
            is(
                i,
                &[
                    "ends", "ended", "closes", "closed", "exits", "quits", "endet", "beendet",
                ],
            ) && !next(i, &["up"])
        }),
        ("starts", &|i| {
            (is(
                i,
                &[
                    "starts", "started", "startet", "starting", "begins", "beginnt",
                ],
            ) && !next(i, &["to", "failing", "asking"]))
                || (is(i, &["new"]) && next(i, &["session"]))
                || (is(i, &["neue", "neuer"]) && next(i, &["sitzung"]))
                || (is(i, &["spins"]) && next(i, &["up"]))
        }),
        ("leaves_processes", &|i| pre(i, &["process", "prozess"])),
    ];
    for (code, test) in &table {
        if let Some(i) = at(*test) {
            hits.push((i, code));
        }
    }
    hits.extend(paraphrased(&w));
    hits.sort_unstable();
    let mut out: Vec<String> = hits.into_iter().map(|(_, c)| c.to_owned()).collect();
    composites(&mut out);
    if out.iter().any(|c| c == "runway_below") && !w.iter().any(|x| x.starts_with("limit")) {
        out.retain(|c| c != "hits_limit");
    }
    out.dedup();
    out
}

/// Composites (v11's W04, W30): a crash and a start again is a restart;
/// ending and leaving processes is the latter alone.
fn composites(out: &mut Vec<String>) {
    if out.iter().any(|c| c == "restarted") {
        out.retain(|c| c != "fails" && c != "starts");
    }
    if out.iter().any(|c| c == "leaves_processes") {
        out.retain(|c| c != "ends");
    }
}

/// The closed paraphrase map (v12): phrasings the cue table's single words
/// don't reach, each to one condition, with where it starts.
fn paraphrased(w: &[String]) -> Vec<(usize, &'static str)> {
    const MAP: &[(&str, &str)] = &[
        ("gets started again", "restarted"),
        ("started again", "restarted"),
        ("starts again", "restarted"),
        ("comes back up", "restarted"),
        ("is restarted", "restarted"),
        ("gets restarted", "restarted"),
        ("restarts", "restarted"),
        ("wieder gestartet", "restarted"),
        ("neu gestartet", "restarted"),
        ("neu startet", "restarted"),
        ("blows up", "fails"),
        ("falls over", "fails"),
        ("dies", "fails"),
        ("bricht ab", "fails"),
        ("abgebrochen", "fails"),
        ("needs me", "asks_you"),
        ("wartet auf mich", "asks_you"),
        ("wartet auf eine antwort", "asks_you"),
        ("braucht mich", "asks_you"),
        ("stops responding", "stuck"),
        ("freezes up", "stuck"),
        ("bleibt hängen", "stuck"),
        ("shuts down", "ends"),
        ("geschlossen", "ends"),
        ("hits the cap", "hits_limit"),
        ("still ist", "idle_for"),
        ("still sind", "idle_for"),
        ("goes quiet", "idle_for"),
        ("gone quiet", "idle_for"),
        ("stays quiet", "idle_for"),
        ("durch ist", "finishes"),
        ("durch sind", "finishes"),
        ("zu ende", "ends"),
        ("wieder luft", "runway_resets"),
        ("wieder platz", "runway_resets"),
    ];
    MAP.iter()
        .filter_map(|(p, code)| {
            let p = words(p);
            (0..w.len())
                .find(|&i| w.len() >= i + p.len() && w[i..i + p.len()] == p[..])
                .map(|i| (i, *code))
        })
        .collect()
}

/// The number the request says for `code`'s parameter (a percent for
/// runway, minutes otherwise), by code.
#[must_use]
pub fn param_for(text: &str, code: &str) -> Option<String> {
    match code {
        "stuck" | "asks_you" | "idle_for" => {}
        "runway_below" => {
            // "30%" loses its sign to the word split: the text keeps it.
            let w = words(text);
            return w.iter().enumerate().find_map(|(i, x)| {
                let n: String = x.chars().take_while(char::is_ascii_digit).collect();
                let pct = x.ends_with('%')
                    || text.contains(&format!("{n}%"))
                    || text.contains(&format!("{n} %"))
                    || w.get(i + 1)
                        .is_some_and(|s| ["%", "percent", "prozent"].contains(&s.as_str()));
                (!n.is_empty() && pct).then_some(n)
            });
        }
        "task_changes" => {
            let w = words(text);
            return ["done", "blocked"]
                .iter()
                .find(|s| w.iter().any(|x| x == *s))
                .map(|s| (*s).to_owned())
                .or_else(|| {
                    (w.windows(2).any(|p| p[0] == "in" && p[1] == "review"))
                        .then(|| "in_review".to_owned())
                });
        }
        _ => return None,
    }
    // v10: a minutes value only with a time unit ("10 minutes", "an
    // hour"): v9's W28 read "the … one" as one minute.
    minutes_said(text).map(|n| n.to_string())
}

/// The minutes a request states with a time unit, hours as minutes.
fn minutes_said(text: &str) -> Option<u32> {
    let w = words(text);
    (1..w.len()).find_map(|i| {
        let unit = w[i].as_str();
        let hours = ["h", "hour", "hours", "hrs", "stunde", "stunden"].contains(&unit);
        let minutes = ["min", "mins", "minute", "minutes", "minuten"].contains(&unit);
        if !(hours || minutes) {
            return None;
        }
        let half = i >= 2 && w[i - 1] == "an" && w[i - 2] == "half";
        let n = match w[i - 1].as_str() {
            "a" | "an" | "eine" | "einer" => 1,
            x => request_numbers(x).into_iter().next()?,
        };
        Some(if half {
            30
        } else if hours {
            n * 60
        } else {
            n
        })
    })
}

/// "Until it ends" in words.
pub(crate) const LIFE_ENDS: &[&str] = &[
    "until it ends",
    "until they end",
    "until it is done",
    "until they are done",
    "until both are done",
    "until it finishes",
    "until the session ends",
    "bis sie endet",
    "bis es fertig ist",
];
/// Once, in words; a bare "once" is "when" (v8's W13, W24).
pub(crate) const LIFE_ONCE: &[&str] = &[
    "just this once",
    "just once",
    "only once",
    "once only",
    "one time",
    "the next time",
    "next time",
    "just the first time",
    "only the first time",
    "nur beim nächsten mal",
    "beim nächsten mal",
    "nur einmal",
    "einmal",
    "einmalig",
    "the first time",
    "one heads-up is enough",
];
/// Always, in words. "Any time" is "when" (v10's W54).
pub(crate) const LIFE_ALWAYS: &[&str] = &[
    "whenever",
    "every time",
    "each time",
    "from now on",
    "until i say stop",
    "until i remove it",
    "always",
    "permanently",
    "permanent",
    "keep watching",
    "keep that on",
    "keep it on",
    "for good",
    "forever",
    "indefinitely",
    "dauerhaft",
    "auf dauer",
    "immer wenn",
    "immer",
    "jedes mal",
    "ab jetzt",
    "until i delete the watch",
    "until i delete it",
    "bis ich es wieder lösche",
    "bis ich es lösche",
];
/// Today, in words.
pub(crate) const LIFE_TODAY: &[&str] = &[
    "for the rest of today",
    "for the rest of the day",
    "für den rest des tages",
    "den rest des tages",
    "for today",
    "today only",
    "only today",
    "just today",
    "nur heute",
    "until midnight",
    "until tonight",
    "bis mitternacht",
    "today",
    "tonight",
    "this evening",
    "bis heute abend",
    "für heute",
    "heute noch",
    "heute",
];

/// The lifetime the request's words say, by code; unsaid when none.
#[must_use]
pub fn lifetime_for(ask: &Ask<'_>) -> LifetimeChoice {
    let w = words(ask.request);
    // "immer noch" is "still", not "always" (v12's W38).
    let mut w = w;
    while let Some(i) =
        (0..w.len().saturating_sub(1)).find(|&i| w[i] == "immer" && w[i + 1] == "noch")
    {
        w.drain(i..i + 2);
    }
    let has = |p: &&str| holds(&w, &words(p));
    for (i, x) in w.iter().enumerate() {
        if ["until", "till", "by", "bis"].contains(&x.as_str())
            && let Some(t) = w
                .get(i + 1)
                .map(|n| format!("{n} {}", w.get(i + 2).map_or("", String::as_str)))
                .and_then(|s| request_times(&s).into_iter().next())
            && let Some((h, m)) = t
                .split_once(':')
                .and_then(|(h, m)| Some((h.parse::<i64>().ok()?, m.parse::<i64>().ok()?)))
        {
            let mut at = ask.midnight_ms + (h * 60 + m) * 60_000;
            if at <= ask.now_ms {
                at += 86_400_000;
            }
            return LifetimeChoice::Until { at_ms: at };
        }
    }
    if LIFE_ENDS.iter().any(has) {
        LifetimeChoice::UntilEnds
    } else if LIFE_ONCE.iter().any(has) {
        LifetimeChoice::Once
    } else if LIFE_ALWAYS.iter().any(has) {
        LifetimeChoice::Always
    } else if LIFE_TODAY.iter().any(has) {
        LifetimeChoice::Today
    } else {
        LifetimeChoice::Unsaid
    }
}

/// A German compound's kind word apart: "Fahrplan-Import-Sitzung" is
/// "Fahrplan-Import Sitzung" (also -Session, -Lauf, -Projekt).
#[must_use]
pub fn uncompound(text: &str) -> String {
    const KINDS: &[&str] = &[
        "-sitzung", "-session", "-lauf", "-projekt", "-project", "-agent", "-konto", "-account",
    ];
    let lower = text.to_lowercase();
    if lower.len() != text.len() {
        return text.to_owned(); // a case change that moves bytes: left as said
    }
    let mut out = text.to_owned();
    for k in KINDS {
        let mut from = 0;
        while let Some(at) = lower[from..].find(k).map(|x| x + from) {
            let end = at + k.len();
            let whole = lower[end..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric());
            let joined = lower[..at]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
            if whole && joined {
                out.replace_range(at..=at, " ");
            }
            from = end;
        }
    }
    out
}
