//! A walk's live check of a switch on real lab setups, through the same
//! functions the switch screen calls: the targets, the preflight's two
//! choices (Desktop and CLI), then the switch itself to the one named.
//!
//! Only AI Juggler Dev's data folder is ever opened, and only when it
//! exists (a walk fills it with `just dev-bridge-data`). Run it on lab
//! setups only: it hands a session to a real app profile.
//!
//! `cargo run -p juggler-app --example switch_lab -- <source key> <target account label> [desktop|cli]`
//! (no third word: the targets and the preflight only, nothing switched)

use std::process::ExitCode;
use std::sync::Arc;

use juggler_app::edition::{self, Edition};

fn main() -> ExitCode {
    edition::set_current(Edition::Dev);
    let mut args = std::env::args().skip(1);
    let (Some(source), Some(target)) = (args.next(), args.next()) else {
        eprintln!("usage: switch_lab <source key> <target account label> [desktop|cli]");
        return ExitCode::from(2);
    };
    let destination = args.next();
    let Ok(data_dir) = juggler_app::state::resolve_data_dir() else {
        eprintln!("switch_lab: no data folder");
        return ExitCode::FAILURE;
    };
    if data_dir.file_name().and_then(|name| name.to_str()) != Some(Edition::Dev.data_dir_name())
        || !data_dir.exists()
    {
        eprintln!("switch_lab: only an existing AI Juggler Dev data folder");
        return ExitCode::FAILURE;
    }
    let state = match juggler_app::state::AppState::init(data_dir) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            eprintln!("switch_lab: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The conversations index, as the app's first refresh builds it.
    if let Err(error) = juggler_app::features::conversations::commands::list_impl(&state) {
        eprintln!("switch_lab: {error}");
        return ExitCode::FAILURE;
    }
    let targets = match juggler_app::commands::switch_targets_impl(&state, &source) {
        Ok(targets) => targets,
        Err(error) => {
            eprintln!("switch_lab: targets: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Some(pick) = targets.iter().find(|row| row.account_label == target) else {
        eprintln!(
            "switch_lab: no target {target:?}; there are {:?}",
            targets
                .iter()
                .map(|row| &row.account_label)
                .collect::<Vec<_>>()
        );
        return ExitCode::FAILURE;
    };
    match juggler_app::commands::switch_preflight_impl(&state, &source, &pick.recipe_id) {
        Ok(preflight) => println!(
            "preflight: suggested {}, desktop {:?}, why not {:?}",
            preflight.destinations.suggested,
            preflight
                .destinations
                .desktop
                .as_ref()
                .map(|d| (&d.name, d.running)),
            preflight.destinations.desktop_why_not
        ),
        Err(error) => {
            eprintln!("switch_lab: preflight: {error}");
            return ExitCode::FAILURE;
        }
    }
    let Some(destination) = destination else {
        return ExitCode::SUCCESS;
    };
    match juggler_app::commands::switch_execute_to(
        &state,
        &source,
        &pick.recipe_id,
        "finish-turn",
        &destination,
    ) {
        Ok(done) => {
            println!(
                "switched: continued in {}, session {}, pid {}, conversation {:?}",
                done.continued_in, done.session_id, done.pid, done.conversation_key
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("switch_lab: switch: {error}");
            ExitCode::FAILURE
        }
    }
}
