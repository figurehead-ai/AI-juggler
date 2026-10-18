//! Desktop entry point for AI Juggler; everything lives in the library
//! crate so tests and future headless modes share one implementation.

// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    juggler_app::run();
}
