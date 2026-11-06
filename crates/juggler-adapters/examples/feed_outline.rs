//! A walk's check of the conversation feed on a real transcript: the
//! last turns as a session's page draws them, by shape only (who spoke,
//! each text's length, each fold's title), never the words.
//!
//! `cargo run -q -p juggler-adapters --example feed_outline -- <transcript.jsonl> [turns]`
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example's output is what it prints"
)]

use std::io::BufRead as _;

use juggler_adapters::conversation_feed::{Block, Lines, Speaker, TurnLog, fold_title};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: feed_outline <transcript.jsonl> [turns]");
        std::process::exit(2);
    };
    let shown: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(8);
    let file = std::fs::File::open(&path).unwrap_or_else(|error| {
        eprintln!("can't open it: {error}");
        std::process::exit(1);
    });
    let mut log = TurnLog::new(Lines::Session);
    let mut reader = std::io::BufReader::new(file);
    let (mut at, mut line) = (0_u64, Vec::new());
    while let Ok(read) = reader.read_until(b'\n', &mut line) {
        if read == 0 || line.last() != Some(&b'\n') {
            break;
        }
        let end = at + read as u64;
        log.line(at, end, &line[..line.len() - 1]);
        at = end;
        line.clear();
    }
    let layout = log.layout();
    let turns = log.turns();
    println!(
        "{} turns kept, layout {layout:?}, last open: {}",
        turns.len(),
        log.last_open()
    );
    for turn in turns.iter().skip(turns.len().saturating_sub(shown)) {
        let who = match turn.speaker {
            Speaker::You => "YOU",
            Speaker::Assistant => "CLAUDE",
        };
        println!("\n{who} @{} rev {}", turn.offset, turn.revision);
        for block in &turn.blocks {
            match block {
                Block::Text(text) => println!("  text, {} chars", text.text().chars().count()),
                Block::Narration(text) => {
                    println!("  narration, {} chars", text.text().chars().count());
                }
                Block::Steps(run) => {
                    let title = fold_title(layout, run)
                        .map(|(done, _)| done)
                        .unwrap_or_default();
                    println!(
                        "  fold: {title}  [{} calls, {} hidden, {} thoughts]",
                        run.steps.len(),
                        run.hidden,
                        run.thoughts.len()
                    );
                }
            }
        }
    }
}
