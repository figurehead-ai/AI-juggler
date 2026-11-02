//! A walk's live check of a switch on real lab setups, through the same
//! functions the switch screen calls: the targets, the preflight's two
//! choices (Desktop and CLI), then the switch itself to the one named.
//!
//! Only AI Juggler Dev's data folder is ever opened, and only when it
//! exists (a walk fills it with `just dev-bridge-data`). Run it on lab
//! setups only: it hands a session to a real app profile.
//!
//! It prints constant words chosen by branch, never anything read from an
//! account (no names, ids or error texts that could carry them).
//!
//! `cargo run -p juggler-app --example switch_lab -- <source key> <target account label> [desktop|cli]`
//! (no third word: the targets and the preflight only, nothing switched)

use std::process::ExitCode;
use std::sync::Arc;

use juggler_app::edition::{self, Edition};

fn fail(words: &str) -> ExitCode {
    eprintln!("switch_lab: {words}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    edition::set_current(Edition::Dev);
    let mut args = std::env::args().skip(1);
    let (Some(source), Some(target)) = (args.next(), args.next()) else {
        eprintln!("usage: switch_lab <source key> <target account label> [desktop|cli]");
        return ExitCode::from(2);
    };
    let destination = args.next();
    let Ok(data_dir) = juggler_app::state::resolve_data_dir() else {
        return fail("no data folder");
    };
    if data_dir.file_name().and_then(|name| name.to_str()) != Some(Edition::Dev.data_dir_name())
        || !data_dir.exists()
    {
        return fail("only an existing AI Juggler Dev data folder");
    }
    let Ok(state) = juggler_app::state::AppState::init(data_dir).map(Arc::new) else {
        return fail("AI Juggler Dev's data couldn't be opened");
    };
    // The conversations index, as the app's first refresh builds it.
    if juggler_app::features::conversations::commands::list_impl(&state).is_err() {
        return fail("the conversations couldn't be listed");
    }
    let Ok(targets) = juggler_app::commands::switch_targets_impl(&state, &source) else {
        return fail("the targets couldn't be read");
    };
    let Some(pick) = targets.iter().find(|row| row.account_label == target) else {
        return fail("no such target");
    };
    let Ok(preflight) =
        juggler_app::commands::switch_preflight_impl(&state, &source, &pick.recipe_id)
    else {
        return fail("the preflight refused");
    };
    let suggested = if preflight.destinations.suggested == "desktop" {
        "desktop"
    } else {
        "cli"
    };
    let desktop = match &preflight.destinations.desktop {
        Some(desktop) if desktop.running => "offered, its profile running",
        Some(_) => "offered, its profile closed",
        None => "not offered",
    };
    println!("preflight: suggested {suggested}; desktop {desktop}");
    let Some(destination) = destination else {
        return ExitCode::SUCCESS;
    };
    let Ok(done) = juggler_app::commands::switch_execute_to(
        &state,
        &source,
        &pick.recipe_id,
        "finish-turn",
        &destination,
    ) else {
        return fail("the switch failed (its reason is in the Dev app's log)");
    };
    let place = if done.continued_in == "desktop" {
        "desktop"
    } else {
        "cli"
    };
    println!("switched: continued in {place}");
    ExitCode::SUCCESS
}
